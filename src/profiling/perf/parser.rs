use crate::data::common::data_formats::{Profiler, ProfilingData};
use crate::profiling::perf::off_cpu::{self, OffCpuCapture, OffCpuEvent};
use crate::profiling::perf::{
    clock_ns, PerfSample, PERF_CONTEXT_MAX, PERF_CONTEXT_USER, PERF_RECORD_MISC_FORK_EXEC,
};
use crate::profiling::symbols::symbol_resolver::SymbolResolver;
use crate::profiling::symbols::Symbol;
use crate::profiling::ThreadState;
use linux_perf_data::{PerfFile, PerfFileReader, PerfFileRecord};
use linux_perf_event_reader::{EventRecord, RawData, SampleRecord};
use log::{debug, error, warn};
use std::collections::HashMap;
use std::env;
use std::ffi::CStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::iter::Peekable;
use std::path::Path;
use std::time::Instant;

/// The cpu Profiler's inputs.
pub struct OnCpuInputs<'a> {
    pub raw_data_path: &'a Path,
    pub start_time_ms: i64,
    pub events_output_path: Option<&'a Path>,
}

/// The off_cpu Profiler's inputs.
pub struct OffCpuInputs<'a> {
    pub raw_data_path: &'a Path,
    pub events_output_path: Option<&'a Path>,
    pub capture: &'a OffCpuCapture,
}

/// Parse the raw Perf profile once and build both the cpu and (when off-CPU events are present)
/// the off_cpu Profilers in a single chronological pass, interleaving the off-CPU events into the
/// perf.data walk so each resolves against the resolver state at its own timestamp.
pub fn build_profiling_data(on_cpu: OnCpuInputs, off_cpu: Option<OffCpuInputs>) -> ProfilingData {
    debug!("Start parsing raw Perf profile...");

    let perf_parse_start_time = Instant::now();

    let mut cpu_profiler = Profiler::new(on_cpu.start_time_ms);
    let mut off_cpu_profiler = match &off_cpu {
        Some(off_cpu) => Profiler {
            block_width_ms: off_cpu.capture.block_width_ms,
            ..Profiler::new(off_cpu.capture.start_time_ms)
        },
        None => Profiler::default(),
    };
    let mut cpu_output_file = open_events_file(on_cpu.events_output_path, "Perf");
    let mut off_cpu_output_file = open_events_file(
        off_cpu
            .as_ref()
            .and_then(|off_cpu| off_cpu.events_output_path),
        "off-CPU",
    );
    let mut off_cpu_count: usize = 0;

    let handle_parsed_sample = |sample: PerfSample| {
        let (ts_ms, pid, samples, frames) = fold_sample(sample);
        if let Some(file) = cpu_output_file.as_mut() {
            let _ = writeln!(file, "{}|{}|{}", ts_ms, pid, frames.join(";"));
        }
        cpu_profiler.insert_stack("cpu", ts_ms, ThreadState::None, &frames, samples);
    };

    let handle_resolved_off_cpu = |sample: PerfSample| {
        off_cpu_count += 1;
        let (ts_ms, pid, samples, frames) = fold_sample(sample);
        if let Some(file) = off_cpu_output_file.as_mut() {
            let _ = writeln!(file, "{}|{}|{}|{}", ts_ms, pid, samples, frames.join(";"));
        }
        off_cpu_profiler.insert_stack("off_cpu", ts_ms, ThreadState::None, &frames, samples);
    };

    let num_parsed_samples = parse_perf_data(
        on_cpu.raw_data_path,
        handle_parsed_sample,
        off_cpu.as_ref(),
        handle_resolved_off_cpu,
    );

    debug!(
        "Finished parsing {num_parsed_samples} Perf samples in {:?}",
        perf_parse_start_time.elapsed()
    );

    let mut profiling_data = ProfilingData::default();
    profiling_data
        .profilers
        .insert(String::from("cpu"), cpu_profiler);
    if off_cpu_count > 0 {
        profiling_data
            .profilers
            .insert(String::from("off_cpu"), off_cpu_profiler);
    }
    profiling_data
}

fn open_events_file(events_output_path: Option<&Path>, label: &str) -> Option<BufWriter<File>> {
    let events_output_path = events_output_path?;
    match File::create(events_output_path) {
        Ok(file) => Some(BufWriter::new(file)),
        Err(_) => {
            warn!(
                "Failed to create file {} to save the {label} events",
                events_output_path.display()
            );
            None
        }
    }
}

/// Parse every record in the raw Perf profile and pass every parsed sample to
/// on_parsed_sample.
/// Off-CPU events (empty for on-CPU-only parses) are interleaved in time order: each is resolved
/// against the resolver state at its own timestamp and passed to `on_resolved_off_cpu`. The off-CPU
/// flush only reads the resolver, so the cpu samples are unaffected.
fn parse_perf_data(
    perf_data_path: &Path,
    mut on_parsed_sample: impl FnMut(PerfSample),
    off_cpu: Option<&OffCpuInputs>,
    mut on_resolved_off_cpu: impl FnMut(PerfSample),
) -> usize {
    let mut num_processed_samples: usize = 0;
    let mut num_record_parsing_errors: usize = 0;

    let perf_data_file = match File::open(perf_data_path) {
        Ok(perf_data_file) => perf_data_file,
        Err(e) => {
            error!(
                "Failed to open raw Perf data file at {}: {e}",
                perf_data_path.display()
            );
            return num_processed_samples;
        }
    };
    // Read an 1MB chunk at a time - the raw perf data typically has a size of several MB to ~500MB.
    let buf_reader = BufReader::with_capacity(1 << 20, perf_data_file);

    let PerfFileReader {
        mut perf_file,
        mut record_iter,
    } = match PerfFileReader::parse_file(buf_reader) {
        Ok(perf_file_reader) => perf_file_reader,
        Err(e) => {
            error!(
                "Failed to parse raw Perf data file at {}: {e}",
                perf_data_path.display()
            );
            return num_processed_samples;
        }
    };

    let arch = if let Ok(Some(arch)) = perf_file.arch() {
        arch
    } else {
        env::consts::ARCH
    };

    let mut symbol_resolver = SymbolResolver::for_arch(arch);
    let epoch_offset_ns = monotonic_to_epoch_offset_ns(&perf_file);
    // Thread names by thread ID. Idle task is named "swapper".
    let mut thread_names: HashMap<i32, String> = HashMap::from([(0, "swapper".to_string())]);

    // Collect all ELF Build-IDs from the profile, which can be used to find the original
    // build version of an ELF file.
    match perf_file.build_ids() {
        Ok(build_ids) => {
            for dso_info in build_ids.values() {
                let elf_file_path = String::from_utf8_lossy(&dso_info.path).to_string();
                symbol_resolver.add_build_id(&elf_file_path, &dso_info.build_id)
            }
        }
        Err(e) => error!("Failed to read the Build-IDs from the Perf data: {e}"),
    };

    // Off-CPU events are written in switch_in_ns order, so stream them and flush each into the
    // walk once the record cursor reaches its timestamp.
    let mut off_cpu_events = off_cpu
        .into_iter()
        .flat_map(|off_cpu| off_cpu::read_events(off_cpu.raw_data_path))
        .peekable();
    let mut cur_on_cpu_mono_ns: u64 = 0;

    loop {
        let record = match record_iter.next_record(&mut perf_file) {
            Ok(Some(record)) => record,
            Ok(None) => break,
            Err(e) => {
                // An error cannot be recovered from by skipping ahead.
                error!(
                    "Stopped parsing the raw Perf profile after {num_processed_samples} samples: {e}",
                );
                break;
            }
        };
        match record {
            PerfFileRecord::EventRecord { attr_index, record } => {
                if let Some(timestamp) = record.timestamp() {
                    cur_on_cpu_mono_ns = timestamp;
                }
                flush_off_cpu_events(
                    &mut off_cpu_events,
                    cur_on_cpu_mono_ns,
                    &mut symbol_resolver,
                    off_cpu,
                    &mut on_resolved_off_cpu,
                );

                let parsed_record = match record.parse() {
                    Ok(parsed_record) => parsed_record,
                    Err(_) => {
                        num_record_parsing_errors += 1;
                        continue;
                    }
                };

                match parsed_record {
                    EventRecord::Mmap(mmap) => {
                        symbol_resolver.add_mmap(
                            mmap.pid,
                            mmap.address,
                            mmap.length,
                            mmap.page_offset,
                            rawdata_to_string(&mmap.path),
                        );
                    }
                    EventRecord::Mmap2(mmap2) => {
                        symbol_resolver.add_mmap(
                            mmap2.pid,
                            mmap2.address,
                            mmap2.length,
                            mmap2.page_offset,
                            rawdata_to_string(&mmap2.path),
                        );
                    }
                    EventRecord::Comm(comm) => {
                        if comm.is_execve {
                            symbol_resolver.handle_exec(comm.pid);
                        }
                        thread_names.insert(comm.tid, rawdata_to_string(&comm.name));
                    }
                    EventRecord::Fork(fork) => {
                        match thread_names.get(&fork.ptid).cloned() {
                            Some(name) => thread_names.insert(fork.tid, name),
                            // Any earlier name is stale.
                            None => thread_names.remove(&fork.tid),
                        };
                        if fork.ppid != fork.pid && (record.misc & PERF_RECORD_MISC_FORK_EXEC) == 0
                        {
                            symbol_resolver.handle_forked_process_mmap(fork.ppid, fork.pid);
                        }
                    }
                    EventRecord::Sample(sample_record) => {
                        // For every PMU event that Perf record collects, it creates an entry in the
                        // attribute table. We only collect the cpu-clock event for now, so limit the
                        // scope to only handling one type of PMU event (the first one in the table).
                        if attr_index != 0 {
                            continue;
                        }
                        if let Some(mut perf_sample) =
                            handle_sample_event(&sample_record, &thread_names, &mut symbol_resolver)
                        {
                            perf_sample.timestamp =
                                perf_sample.timestamp.saturating_add_signed(epoch_offset_ns);
                            num_processed_samples += 1;
                            on_parsed_sample(perf_sample);
                        }
                    }
                    EventRecord::ContextSwitch(_) => {
                        // Not used for now, but it can be useful for off-CPU profiling in the future.
                    }
                    _ => {}
                };
            }
            _ => {}
        }
    }

    // Flush any off-CPU events that outlast the last perf record against the final resolver state.
    flush_off_cpu_events(
        &mut off_cpu_events,
        u64::MAX,
        &mut symbol_resolver,
        off_cpu,
        &mut on_resolved_off_cpu,
    );

    debug!("Number of Perf profile parsing errors: {num_record_parsing_errors}");

    num_processed_samples
}

/// Symbolize and emit every pending off-CPU event whose timestamp is before `cur_on_cpu_mono_ns`,
/// converting its CLOCK_MONOTONIC time to epoch with the offset the kernel bucketed on, so each
/// lands in its bucket's block.
fn flush_off_cpu_events(
    off_cpu_events: &mut Peekable<impl Iterator<Item = OffCpuEvent>>,
    cur_on_cpu_mono_ns: u64,
    symbol_resolver: &mut SymbolResolver,
    off_cpu: Option<&OffCpuInputs>,
    on_resolved_off_cpu: &mut impl FnMut(PerfSample),
) {
    let Some(OffCpuInputs { capture, .. }) = off_cpu else {
        return;
    };
    // `dedup_leaf` drops the arm64 duplicate kernel leaf.
    let mut resolve_stack =
        |stacks: &HashMap<u32, Vec<u64>>, id: i64, pid: i32, dedup_leaf: bool| -> Vec<Symbol> {
            let ips: &[u64] = if id < 0 {
                &[]
            } else {
                stacks
                    .get(&(id as u32))
                    .map(Vec::as_slice)
                    .unwrap_or_default()
            };
            let ips = if dedup_leaf && ips.len() >= 2 && ips[0] == ips[1] {
                &ips[1..]
            } else {
                ips
            };
            ips.iter()
                .map(|&ip| symbol_resolver.resolve(pid, ip))
                .collect()
        };
    while off_cpu_events
        .peek()
        .is_some_and(|event| event.value.switch_in_ns < cur_on_cpu_mono_ns)
    {
        let event = off_cpu_events.next().unwrap();
        let pid = event.key.pid as i32;
        let user = resolve_stack(&capture.ustacks, event.key.ustack_id, pid, false);
        let kernel = strip_probe_frames(resolve_stack(
            &capture.kstacks,
            event.key.kstack_id,
            pid,
            true,
        ));
        let mut call_chain = kernel;
        call_chain.extend(user);
        on_resolved_off_cpu(PerfSample {
            pid,
            timestamp: event.value.switch_in_ns + capture.epoch_offset_ns,
            thread_name: CStr::from_bytes_until_nul(&event.key.comm)
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            call_chain,
            samples: event.value.off_cpu_ns / 1000,
        });
    }
}

/// The offset that converts a sample's CLOCK_MONOTONIC timestamp (perf record -k 1) into epoch
/// time. Perf records a wall-clock and a CLOCK_MONOTONIC reading taken at the same instant.
fn monotonic_to_epoch_offset_ns(perf_file: &PerfFile) -> i64 {
    if let Ok(Some(clock_data)) = perf_file.clock_data() {
        if clock_data.clockid == libc::CLOCK_MONOTONIC as u32 {
            return clock_data.wall_clock_ns as i64 - clock_data.clockid_time_ns as i64;
        }
    }
    // If the pair was not recorded in Perf, read them now as a fallback.
    clock_ns(libc::CLOCK_REALTIME) as i64 - clock_ns(libc::CLOCK_MONOTONIC) as i64
}

/// Handle a Perf sample event by symbolicating every frame in the call chain and performing
/// leaf frame recovery if needed to.
fn handle_sample_event(
    sample_record: &SampleRecord,
    thread_names: &HashMap<i32, String>,
    symbol_resolver: &mut SymbolResolver,
) -> Option<PerfSample> {
    let pid = match sample_record.pid {
        Some(pid) => pid,
        None => return None,
    };
    let tid = match sample_record.tid {
        Some(tid) => tid,
        None => return None,
    };
    let timestamp = match sample_record.timestamp {
        Some(timestamp) => timestamp,
        None => return None,
    };
    let call_chain = match sample_record.callchain {
        Some(call_chain) => call_chain,
        None => return None,
    };

    let mut leaf_frame_idx: Option<usize> = None;
    let mut frame_addresses: Vec<u64> = Vec::new();
    let mut resolved_call_chain: Vec<Symbol> = Vec::new();

    // Refer to add_callchain_ip in
    // https://github.com/torvalds/linux/blob/master/tools/perf/util/machine.c
    for i in 0..call_chain.len() {
        let frame_addr = match call_chain.get(i) {
            Some(frame_addr) => frame_addr,
            None => continue,
        };
        // The userspace sentinel indicates that the following frames are all
        // from the userspace, and the next frame is the leaf frame.
        if frame_addr == PERF_CONTEXT_USER {
            leaf_frame_idx = Some(frame_addresses.len());
            continue;
        }
        if frame_addr >= PERF_CONTEXT_MAX {
            continue;
        }
        resolved_call_chain.push(symbol_resolver.resolve(pid, frame_addr));
        frame_addresses.push(frame_addr);
    }

    let mut perf_sample = PerfSample {
        pid,
        timestamp,
        thread_name: thread_names
            .get(&tid)
            .cloned()
            // Perf's placeholder for a thread that was never named.
            .unwrap_or_else(|| format!(":{tid}")),
        call_chain: resolved_call_chain,
        samples: 1,
    };

    // On ARM, the caller of the leaf frame might not be in the call chain, due
    // to the fact that the function invocation instruction (bl) saves the return
    // address to the LR register, and it relies on the invoked function's prologue
    // to save it to stack. Therefore, Perf, which relies on the stack to trace
    // and create the call chain, might be missing the leaf frame's caller in the
    // call chain. We need to check if the value of the leaf frame's LR register
    // can be used to recover its caller frame.
    if symbol_resolver.support_leaf_caller_recovery() {
        let (lr, fp, sp) = match &sample_record.user_regs {
            Some(user_regs) => (user_regs.get(30), user_regs.get(29), user_regs.get(31)),
            None => return Some(perf_sample),
        };
        // LR is crucial, while fp and sp are used as fallback.
        if lr.is_none() {
            return Some(perf_sample);
        }

        // Perf only attempts this when there is at least a second userspace frame.
        let leaf_frame_idx = match leaf_frame_idx {
            Some(idx) if idx + 1 < frame_addresses.len() => idx,
            _ => return Some(perf_sample),
        };
        let leaf_addr = frame_addresses[leaf_frame_idx];
        let existing_caller_addr = frame_addresses[leaf_frame_idx + 1];

        if let Some(leaf_caller_addr) =
            symbol_resolver.recover_leaf_frame_caller(pid, leaf_addr, lr.unwrap(), fp, sp)
        {
            // Match perf's check: if (leaf_frame_caller && leaf_frame_caller != ip)
            if leaf_caller_addr != 0 && leaf_caller_addr != existing_caller_addr {
                perf_sample.call_chain.insert(
                    leaf_frame_idx + 1,
                    symbol_resolver.resolve(pid, leaf_caller_addr),
                );
            }
        }
    }

    Some(perf_sample)
}

fn rawdata_to_string(raw: &RawData) -> String {
    let cow = raw.as_slice();
    let bytes: &[u8] = match &cow {
        std::borrow::Cow::Borrowed(b) => b,
        std::borrow::Cow::Owned(b) => b.as_slice(),
    };
    // Strip trailing null bytes (perf data format null-terminates strings)
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

/// Fold a sample into its root-to-leaf frame strings (rooted at the thread name) plus its
/// timestamp (ms), pid, and weight.
fn fold_sample(sample: PerfSample) -> (i64, i32, u64, Vec<String>) {
    // Following Perf, every stack is rooted at the sampled thread's name.
    let frames = std::iter::once(sample.thread_name.replace(' ', "_"))
        .chain(
            // Perf sample's call chain is from leaf to root, so reverse the frames.
            sample
                .call_chain
                .into_iter()
                .rev()
                .map(|symbol| symbol.into_frame_string()),
        )
        .collect();
    (
        (sample.timestamp / 1_000_000) as i64,
        sample.pid,
        sample.samples,
        frames,
    )
}

/// The probe fires from `__schedule`, so drop every kernel frame leaf-ward of it. The probe is
/// unloaded before kallsyms is read, so its own frame can't be matched by name.
fn strip_probe_frames(mut kernel_leaf_first: Vec<Symbol>) -> Vec<Symbol> {
    const ANCHOR_WINDOW: usize = 8;
    let strip = kernel_leaf_first
        .iter()
        .take(ANCHOR_WINDOW)
        .position(|symbol| symbol.name == "__schedule" || symbol.name.starts_with("__schedule."))
        .unwrap_or_else(|| {
            kernel_leaf_first
                .iter()
                .take_while(|symbol| {
                    symbol.name.starts_with("bpf_prog")
                        || symbol.name.starts_with("bpf_trace_run")
                        || symbol.name.starts_with("__bpf_trace_")
                })
                .count()
        });
    kernel_leaf_first.drain(..strip);
    kernel_leaf_first
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiling::symbols::Symbol;
    use crate::profiling::FrameType;

    fn sym(name: &str, frame_type: FrameType) -> Symbol {
        Symbol {
            name: name.to_string(),
            resolved: true,
            source: None,
            frame_type,
        }
    }

    fn unknown(frame_type: FrameType) -> Symbol {
        Symbol {
            name: String::new(),
            resolved: false,
            source: None,
            frame_type,
        }
    }

    #[test]
    fn test_off_cpu_fold_order_suffix_and_unknown() {
        // Off-CPU call chain is kernel-leaf-first then user-leaf-first.
        let call_chain = vec![
            sym("__schedule", FrameType::Kernel),
            unknown(FrameType::Kernel),
            sym("el0_svc", FrameType::Kernel),
            sym("nanosleep", FrameType::Native),
            sym("main", FrameType::Native),
        ];
        let (_, _, samples, frames) = fold_sample(PerfSample {
            pid: 0,
            timestamp: 0,
            thread_name: "blocker".to_string(),
            call_chain,
            samples: 42,
        });
        assert_eq!(samples, 42);
        assert_eq!(
            frames,
            vec![
                "blocker".to_string(),
                "main".to_string(),
                "nanosleep".to_string(),
                "el0_svc_[k]".to_string(),
                "[unknown]_[k]".to_string(),
                "__schedule_[k]".to_string()
            ]
        );
    }

    #[test]
    fn test_off_cpu_strip_probe_frames() {
        let strip = |kernel_leaf_first: Vec<Symbol>| -> Vec<String> {
            strip_probe_frames(kernel_leaf_first)
                .into_iter()
                .map(Symbol::into_frame_string)
                .collect()
        };
        assert_eq!(
            strip(vec![
                unknown(FrameType::Kernel),
                sym("bpf_trace_run4", FrameType::Kernel),
                sym("__bpf_trace_sched_switch", FrameType::Kernel),
                sym("__schedule", FrameType::Kernel),
                sym("schedule", FrameType::Kernel),
            ]),
            vec!["__schedule_[k]", "schedule_[k]"]
        );
        assert_eq!(
            strip(vec![
                sym("bpf_prog_6deef7357e7b4530_off_cpu", FrameType::Kernel),
                sym("bpf_trace_run4", FrameType::Kernel),
                sym("__bpf_trace_sched_switch", FrameType::Kernel),
                sym("__schedule", FrameType::Kernel),
                sym("schedule", FrameType::Kernel),
            ]),
            vec!["__schedule_[k]", "schedule_[k]"]
        );
        assert_eq!(
            strip(vec![
                sym("bpf_trace_run4", FrameType::Kernel),
                unknown(FrameType::Kernel),
                sym("schedule", FrameType::Kernel),
            ]),
            vec!["[unknown]_[k]", "schedule_[k]"]
        );
    }
}
