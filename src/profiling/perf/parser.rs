use crate::data::common::data_formats::Profiler;
use crate::profiling::perf::{
    PerfSample, PERF_CONTEXT_MAX, PERF_CONTEXT_USER, PERF_RECORD_MISC_FORK_EXEC,
};
use crate::profiling::symbols::symbol_resolver::SymbolResolver;
use crate::profiling::symbols::ResolvedSymbol;
use crate::profiling::ThreadState;
use linux_perf_data::{PerfFileReader, PerfFileRecord};
use linux_perf_event_reader::{EventRecord, RawData, SampleRecord};
use log::{debug, error, warn};
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Parse the raw Perf profile and build the Profiler Data.
pub fn build_perf_profiler_data(
    profile_type: &str,
    perf_data_path: &PathBuf,
    profile_start_timestamp_ms: i64,
    events_output_path: Option<&Path>,
) -> Profiler {
    debug!("Start parsing raw Perf profile...");

    let mut profiler = Profiler::new(profile_start_timestamp_ms);

    let perf_parse_start_time = Instant::now();

    let mut stack_output_file = if let Some(events_output_path) = events_output_path {
        if let Ok(file) = File::create(events_output_path) {
            Some(BufWriter::new(file))
        } else {
            warn!(
                "Failed to create file {} to save the Perf events",
                events_output_path.display()
            );
            None
        }
    } else {
        None
    };

    let mut system_boot_timestamp_ms =
        procfs::boot_time().map_or(0, |boot_time| boot_time.timestamp_millis());

    let handle_parsed_sample = |perf_sample: PerfSample| {
        // In the rare case where the system boot timestamp cannot be retrieved, assume the
        // first sample's epoch timestamp matches the profile start timestamp
        if system_boot_timestamp_ms == 0 {
            system_boot_timestamp_ms =
                profile_start_timestamp_ms - (perf_sample.timestamp / 1_000_000) as i64;
        }

        let mut frames: Vec<String> = perf_sample
            .call_chain
            .iter()
            .map(|resolved_symbol| {
                resolved_symbol
                    .as_ref()
                    .map_or("[unknown]".to_string(), |s| {
                        format!("{}{}", s.name, s.frame_type.literal_suffix())
                    })
            })
            .collect();
        // Perf sample's call chain is from leaf to root, so reverse the frames
        frames.reverse();

        let sample_timestamp_ms =
            system_boot_timestamp_ms + (perf_sample.timestamp / 1_000_000) as i64;

        if let Some(file) = stack_output_file.as_mut() {
            let _ = writeln!(
                file,
                "{}|{}|{}",
                sample_timestamp_ms,
                perf_sample.pid,
                frames.join(";")
            );
        }

        profiler.insert_stack(
            profile_type,
            sample_timestamp_ms,
            ThreadState::None,
            &frames,
            1,
        );
    };

    let num_parsed_samples = parse_perf_data(perf_data_path, handle_parsed_sample);

    debug!(
        "Finished parsing {num_parsed_samples} Perf samples in {:?}",
        perf_parse_start_time.elapsed()
    );

    profiler
}

/// Parse every record in the raw Perf profile and pass every parsed sample to
/// on_parsed_sample.
fn parse_perf_data(
    perf_data_path: &PathBuf,
    mut on_parsed_sample: impl FnMut(PerfSample),
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
                    }
                    EventRecord::Fork(fork) => {
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
                        if let Some(perf_sample) =
                            handle_sample_event(&sample_record, &mut symbol_resolver)
                        {
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

    debug!("Number of Perf profile parsing errors: {num_record_parsing_errors}");

    num_processed_samples
}

/// Handle a Perf sample event by symbolicating every frame in the call chain and performing
/// leaf frame recovery if needed to.
fn handle_sample_event(
    sample_record: &SampleRecord,
    symbol_resolver: &mut SymbolResolver,
) -> Option<PerfSample> {
    let pid = match sample_record.pid {
        Some(pid) => pid,
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
    let mut resolved_call_chain: Vec<Option<ResolvedSymbol>> = Vec::new();

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
        call_chain: resolved_call_chain,
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
