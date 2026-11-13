#![cfg(target_os = "linux")]
//! Off-CPU half of the perf profile collector: loads and attaches the eBPF
//! program, drains the COUNTS map each interval into the on-disk event stream,
//! and reads the stacks and counters at finish. Events are symbolized later, in
//! `parser.rs`, during the chronological replay of perf.data.

use crate::data::common::utils::get_sub_process_duration_seconds;
use crate::data::TimeEnum;
use crate::data_collection::InitParams;
use crate::ebpf::preconditions;
use crate::profiling::perf::clock_ns;
use crate::PDError;
use anyhow::Result;
use aya::maps::{HashMap as AyaHashMap, PerCpuArray, StackTraceMap};
use aya::programs::BtfTracePoint;
use log::debug;
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::mem::{offset_of, size_of, size_of_val};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

static OFF_CPU_BPF: &[u8] = aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/off_cpu.bpf.o"));

/// Profile only this process (tgid); 0 profiles all processes.
const OFF_CPU_TARGET_TGID: u32 = 0;
/// Ignore off-CPU stretches (switch-out to switch-in) shorter than this; 0 keeps them all.
const OFF_CPU_MIN_BLOCK_NS: u64 = 0;
/// Keep the program running this long past the planned end, since work after the load (static
/// data, a later prepare) delays the collection; finish cuts at the real collection end.
const OFF_CPU_STOP_BUFFER_S: u64 = 3;

// Indices into the per-CPU `OFF_CPU_STATS` map, mirroring `OFF_CPU_STAT_*` in `off_cpu.h`.
pub const OFF_CPU_STAT_START_UPDATE_FAILED: u32 = 0;
pub const OFF_CPU_STAT_COUNTS_UPDATE_FAILED: u32 = 1;
pub const OFF_CPU_STAT_KSTACK_FAILED: u32 = 2;
pub const OFF_CPU_STAT_KSTACK_COLLISION: u32 = 3;
pub const OFF_CPU_STAT_USTACK_FAILED: u32 = 4;
pub const OFF_CPU_STAT_USTACK_COLLISION: u32 = 5;
pub const OFF_CPU_STAT_SWITCHES_TOTAL: u32 = 6;
pub const OFF_CPU_STAT_EVENTS_TOTAL: u32 = 7;
pub const OFF_CPU_STAT_TOTAL_NS: u32 = 8;
pub const OFF_CPU_STAT_KSTACK_COLLISION_NS: u32 = 9;
pub const OFF_CPU_STAT_KSTACK_FAILED_NS: u32 = 10;
pub const OFF_CPU_STAT_USTACK_COLLISION_NS: u32 = 11;
pub const OFF_CPU_STAT_USTACK_FAILED_NS: u32 = 12;
pub const OFF_CPU_STAT_USTACK_KTHREAD: u32 = 21;
pub const OFF_CPU_STAT_USTACK_KTHREAD_NS: u32 = 22;
pub const OFF_CPU_STAT_RUNNABLE: u32 = 23;

/// Off-CPU block-duration bands (counts), mirror of `OFF_CPU_STAT_BLOCK_*` in `off_cpu.h`; emitted
/// to pick a minimum block time to ignore.
const OFF_CPU_BLOCK_BANDS: [(&str, u32); 8] = [
    ("BLOCK_LT_10US", 20),
    ("BLOCK_LT_100US", 13),
    ("BLOCK_LT_1MS", 14),
    ("BLOCK_LT_10MS", 15),
    ("BLOCK_LT_100MS", 16),
    ("BLOCK_LT_1S", 17),
    ("BLOCK_LT_10S", 18),
    ("BLOCK_GE_10S", 19),
];

/// Userspace mirror of `struct off_cpu_key` in `ebpf/programs/off_cpu.h`; its `_Static_assert`s
/// pin the C layout and the asserts below pin this one.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OffCpuKey {
    pub pid: u32,
    pub comm: [u8; 16],
    pub mid_buckets: u32,
    pub kstack_id: i64,
    pub ustack_id: i64,
    pub bucket: u64,
}

const _: () = assert!(size_of::<OffCpuKey>() == 48);
const _: () = assert!(offset_of!(OffCpuKey, kstack_id) == 24);

unsafe impl aya::Pod for OffCpuKey {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OffCpuValue {
    pub switch_in_ns: u64,
    pub head_ns: u64,
    pub mid_ns: u64,
    pub tail_ns: u64,
}

unsafe impl aya::Pod for OffCpuValue {}

/// One COUNTS entry (off-CPU time summed per key), or one thread still off-CPU at finish, written
/// to the per-run events file and symbolized at finish against the resolver state at switch-in.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OffCpuEvent {
    pub key: OffCpuKey,
    pub value: OffCpuValue,
}

const _: () = assert!(size_of::<OffCpuEvent>() == 80);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OffCpuStart {
    pub ts_ns: u64,
    pub tgid: u32,
    pub kstack_miss: u8,
    pub ustack_miss: u8,
    _pad: u16,
    pub kstack_id: i64,
    pub ustack_id: i64,
    pub comm: [u8; 16],
}

const _: () = assert!(size_of::<OffCpuStart>() == 48);

unsafe impl aya::Pod for OffCpuStart {}

/// What finish hands back: the off-CPU start, its bucket grid, both stack maps, and the
/// counters.
pub struct OffCpuCapture {
    pub start_time_ms: i64,
    pub block_width_ms: u64,
    pub epoch_offset_ns: u64,
    pub kstacks: HashMap<u32, Vec<u64>>,
    pub ustacks: HashMap<u32, Vec<u64>>,
    pub stats: Vec<(&'static str, u64)>,
}

/// The off-CPU half of the merged perf collector: holds the eBPF handle and the events writer,
/// drains the COUNTS map every interval, and reads the stacks and counters at finish.
#[derive(Debug)]
pub struct OffCpuProfiler {
    ebpf: aya::Ebpf,
    writer: Option<BufWriter<fs::File>>,
    peak_counts_entries: u64,
    bucket_ns: u64,
    stop_at_ns: u64,
    epoch_offset_ns: u64,
    start_time_ms: i64,
}

impl OffCpuProfiler {
    /// Check preconditions and load the eBPF program into the kernel without attaching it
    pub fn load(init_params: &InitParams) -> Result<Self> {
        let btf = preconditions::check_preconditions().map_err(precondition_error)?;
        preconditions::warn_if_kallsyms_hidden();

        // Ten off-CPU buckets per interval, so the COUNTS-key count stays bounded regardless of -i.
        let bucket_ns = init_params.interval.saturating_mul(100_000_000);
        let stop_at_ns = clock_ns(libc::CLOCK_MONOTONIC).saturating_add(
            (get_sub_process_duration_seconds(init_params) + OFF_CPU_STOP_BUFFER_S)
                .saturating_mul(1_000_000_000),
        );
        let epoch_offset_ns =
            clock_ns(libc::CLOCK_REALTIME).saturating_sub(clock_ns(libc::CLOCK_MONOTONIC));
        let ebpf = match load_program(btf, stop_at_ns, bucket_ns, epoch_offset_ns) {
            Ok(ebpf) => ebpf,
            Err(e) => {
                debug!("Off-CPU eBPF load error: {e:?}");
                let summary = format!("{e:#}");
                let eperm = e.chain().any(|cause| {
                    cause
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|io| io.raw_os_error() == Some(libc::EPERM))
                });
                let hint = match preconditions::memlock_hint() {
                    Some(hint) if eperm => format!("; {hint}"),
                    _ => String::new(),
                };
                return Err(PDError::DependencyError(format!(
                    "Off-CPU eBPF profiling cannot run (load failed): {}{hint}; drop --profile-off-cpu to profile on-CPU only",
                    summary.lines().next().unwrap_or_default()
                ))
                .into());
            }
        };

        Ok(OffCpuProfiler {
            ebpf,
            writer: None,
            peak_counts_entries: 0,
            bucket_ns,
            stop_at_ns,
            epoch_offset_ns,
            start_time_ms: 0,
        })
    }

    pub fn attach(&mut self, run_data_dir: &Path) -> Result<()> {
        let writer = attach_program(&mut self.ebpf, run_data_dir).map_err(|e| {
            PDError::DependencyError(format!(
                "Off-CPU eBPF profiling cannot run (attach failed): {e:#}; drop --profile-off-cpu to profile on-CPU only"
            ))
        })?;
        self.writer = Some(writer);
        self.start_time_ms = (clock_ns(libc::CLOCK_REALTIME) / 1_000_000) as i64;
        debug!("Off-CPU eBPF profiling attached to sched:sched_switch.");
        Ok(())
    }

    pub fn collect(&mut self) {
        // The kernel only writes the current bucket, so drain two behind to avoid racing an in-flight update.
        let cutoff = ((clock_ns(libc::CLOCK_MONOTONIC) + self.epoch_offset_ns) / self.bucket_ns)
            .saturating_sub(2);
        if let Err(e) = self
            .drain_counts(cutoff)
            .and_then(|drained| self.write_events(drained))
        {
            debug!("Off-CPU drain failed this tick: {e:?}");
        }
    }

    /// Detach, drain the remaining COUNTS entries, close threads still off-CPU, flush the events
    /// file, and return the `OffCpuCapture`.
    pub fn finish(&mut self, init_params: &InitParams) -> Result<OffCpuCapture> {
        if let Some(prog) = self.ebpf.program_mut("off_cpu") {
            if let Ok(prog) = <&mut BtfTracePoint>::try_from(prog) {
                let _ = prog.unload();
            }
        }

        // End at the real collection end, on the kernel's clock. The program stops at stop_at_ns,
        // so a START entry may belong to a thread that woke after it.
        let end_ns = match init_params.collection_end {
            Some(TimeEnum::DateTime(end)) => (end.timestamp_nanos_opt().unwrap_or_default() as u64)
                .saturating_sub(self.epoch_offset_ns),
            _ => clock_ns(libc::CLOCK_MONOTONIC),
        }
        .min(self.stop_at_ns);
        let mut drained = self.drain_counts(u64::MAX).unwrap_or_else(|e| {
            debug!("Off-CPU final drain failed: {e:?}");
            Vec::new()
        });
        let (mut unfinished, mut unfinished_ns) = (0, 0);
        if let Some(start) = self
            .ebpf
            .map("START")
            .and_then(|map| AyaHashMap::<_, u32, OffCpuStart>::try_from(map).ok())
        {
            let end_epoch_ns = end_ns + self.epoch_offset_ns;
            for (_, s) in start.iter().flatten() {
                let off_cpu_ns = end_ns.saturating_sub(s.ts_ns);
                unfinished += 1;
                // Blocked after the collection ended.
                if off_cpu_ns == 0 {
                    continue;
                }
                unfinished_ns += off_cpu_ns;
                let (mid_buckets, head_ns, mid_ns, tail_ns) =
                    split_off_cpu_stretch_by_bucket(end_epoch_ns, off_cpu_ns, self.bucket_ns);
                drained.push(OffCpuEvent {
                    key: OffCpuKey {
                        pid: s.tgid,
                        comm: s.comm,
                        mid_buckets,
                        kstack_id: s.kstack_id,
                        ustack_id: s.ustack_id,
                        bucket: end_epoch_ns / self.bucket_ns,
                    },
                    value: OffCpuValue {
                        switch_in_ns: end_ns,
                        head_ns,
                        mid_ns,
                        tail_ns,
                    },
                });
            }
        }
        if let Err(e) = self.write_events(drained) {
            debug!("Off-CPU final event write failed: {e:?}");
        }
        if let Some(writer) = self.writer.as_mut() {
            writer.flush()?;
        }

        // Stack ids are stable (no REUSE_STACKID), so one dump at finish resolves every event's ids.
        let dump_stacks = |name: &str| -> HashMap<u32, Vec<u64>> {
            self.ebpf
                .map(name)
                .and_then(|map| StackTraceMap::try_from(map).ok())
                .map(|stacks| {
                    stacks
                        .iter()
                        .filter_map(Result::ok)
                        .map(|(id, trace)| {
                            (id, trace.frames().iter().map(|frame| frame.ip).collect())
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let kstacks = dump_stacks("KSTACK_TRACES");
        let ustacks = dump_stacks("USTACK_TRACES");

        let stats = self
            .ebpf
            .map("OFF_CPU_STATS")
            .and_then(|map| PerCpuArray::<_, u64>::try_from(map).ok());
        let stat = |idx: u32| -> u64 {
            stats
                .as_ref()
                .and_then(|map| map.get(&idx, 0).ok())
                .map_or(0, |values| values.iter().sum())
        };
        let mut counters = vec![
            (
                "START_UPDATE_FAILED",
                stat(OFF_CPU_STAT_START_UPDATE_FAILED),
            ),
            (
                "COUNTS_UPDATE_FAILED",
                stat(OFF_CPU_STAT_COUNTS_UPDATE_FAILED),
            ),
            ("KSTACK_FAILED", stat(OFF_CPU_STAT_KSTACK_FAILED)),
            ("KSTACK_COLLISION", stat(OFF_CPU_STAT_KSTACK_COLLISION)),
            ("USTACK_FAILED", stat(OFF_CPU_STAT_USTACK_FAILED)),
            ("USTACK_COLLISION", stat(OFF_CPU_STAT_USTACK_COLLISION)),
            ("USTACK_KTHREAD", stat(OFF_CPU_STAT_USTACK_KTHREAD)),
            ("SWITCHES_TOTAL", stat(OFF_CPU_STAT_SWITCHES_TOTAL)),
            ("RUNNABLE", stat(OFF_CPU_STAT_RUNNABLE)),
            ("EVENTS_TOTAL", stat(OFF_CPU_STAT_EVENTS_TOTAL)),
            ("UNFINISHED", unfinished),
            ("COUNTS_PEAK_ENTRIES", self.peak_counts_entries),
            ("KSTACK_TRACES_OCCUPANCY", kstacks.len() as u64),
            ("USTACK_TRACES_OCCUPANCY", ustacks.len() as u64),
            ("TOTAL_US", stat(OFF_CPU_STAT_TOTAL_NS) / 1000),
            ("UNFINISHED_US", unfinished_ns / 1000),
            (
                "KSTACK_COLLISION_US",
                stat(OFF_CPU_STAT_KSTACK_COLLISION_NS) / 1000,
            ),
            (
                "KSTACK_FAILED_US",
                stat(OFF_CPU_STAT_KSTACK_FAILED_NS) / 1000,
            ),
            (
                "USTACK_COLLISION_US",
                stat(OFF_CPU_STAT_USTACK_COLLISION_NS) / 1000,
            ),
            (
                "USTACK_FAILED_US",
                stat(OFF_CPU_STAT_USTACK_FAILED_NS) / 1000,
            ),
            (
                "USTACK_KTHREAD_US",
                stat(OFF_CPU_STAT_USTACK_KTHREAD_NS) / 1000,
            ),
        ];
        for (name, idx) in OFF_CPU_BLOCK_BANDS {
            let count = stat(idx);
            if count > 0 {
                counters.push((name, count));
            }
        }
        Ok(OffCpuCapture {
            start_time_ms: self.start_time_ms,
            block_width_ms: self.bucket_ns / 1_000_000,
            epoch_offset_ns: self.epoch_offset_ns,
            kstacks,
            ustacks,
            stats: counters,
        })
    }

    /// Drain and return COUNTS entries up through `drain_through_bucket`. Callers stay two
    /// buckets behind while attached: get+remove isn't atomic with the kernel's lookup+add, which can
    /// outlive its bucket.
    fn drain_counts(&mut self, drain_through_bucket: u64) -> Result<Vec<OffCpuEvent>> {
        let counts_map = self
            .ebpf
            .map_mut("COUNTS")
            .ok_or_else(|| anyhow::anyhow!("COUNTS map missing"))?;
        let mut counts = AyaHashMap::<_, OffCpuKey, OffCpuValue>::try_from(counts_map)?;
        let keys: Vec<OffCpuKey> = counts.keys().flatten().collect();
        self.peak_counts_entries = self.peak_counts_entries.max(keys.len() as u64);
        let mut drained = Vec::new();
        for key in keys {
            if key.bucket > drain_through_bucket {
                continue;
            }
            if let Ok(value) = counts.get(&key, 0) {
                let _ = counts.remove(&key);
                drained.push(OffCpuEvent { key, value });
            }
        }
        Ok(drained)
    }

    fn write_events(&mut self, mut events: Vec<OffCpuEvent>) -> Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("off-CPU events writer not open"))?;
        // Each drain window is disjoint from the next, so sorting per drain keeps the whole file
        // globally switch_in_ns-ordered for the streaming interleave at finish.
        events.sort_by_key(|event| event.value.switch_in_ns);
        writer.write_all(unsafe {
            std::slice::from_raw_parts(events.as_ptr().cast(), size_of_val(events.as_slice()))
        })?;
        Ok(())
    }
}

pub fn events_file_path(run_data_dir: &Path) -> PathBuf {
    run_data_dir.join("off_cpu_events")
}

/// Stream the per-run events file back as an iterator, so finish can interleave
/// events without buffering the whole file in memory.
pub fn read_events(events_path: &Path) -> impl Iterator<Item = OffCpuEvent> {
    let mut reader = fs::File::open(events_path).ok().map(BufReader::new);
    std::iter::from_fn(move || {
        let mut bytes = [0u8; size_of::<OffCpuEvent>()];
        reader.as_mut()?.read_exact(&mut bytes).ok()?;
        Some(unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast()) })
    })
}

fn precondition_error(reason: String) -> PDError {
    PDError::DependencyError(format!(
        "eBPF profiling cannot run: {reason}; drop --profile-off-cpu to profile on-CPU only"
    ))
}

fn load_program(
    btf: aya::Btf,
    stop_at_ns: u64,
    bucket_ns: u64,
    epoch_offset_ns: u64,
) -> Result<aya::Ebpf> {
    let mut ebpf = aya::EbpfLoader::new()
        .btf(Some(&btf))
        .override_global("target_tgid", &OFF_CPU_TARGET_TGID, true)
        .override_global("min_block_ns", &OFF_CPU_MIN_BLOCK_NS, true)
        .override_global("stop_at_ns", &stop_at_ns, true)
        .override_global("bucket_ns", &bucket_ns, true)
        .override_global("epoch_offset_ns", &epoch_offset_ns, true)
        .load(OFF_CPU_BPF)?;

    let prog: &mut BtfTracePoint = ebpf
        .program_mut("off_cpu")
        .ok_or_else(|| anyhow::anyhow!("off_cpu program missing from eBPF object"))?
        .try_into()?;
    prog.load("sched_switch", &btf)?;

    Ok(ebpf)
}

fn attach_program(ebpf: &mut aya::Ebpf, run_data_dir: &Path) -> Result<BufWriter<fs::File>> {
    let prog: &mut BtfTracePoint = ebpf
        .program_mut("off_cpu")
        .ok_or_else(|| anyhow::anyhow!("off_cpu program missing from eBPF object"))?
        .try_into()?;
    prog.attach()?;
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(events_file_path(run_data_dir))?;
    Ok(BufWriter::new(file))
}

/// Splits one off-CPU stretch ending at `wake_epoch_ns` the way the kernel does: the buckets fully
/// covered between its first and wake-up buckets, and its time in the first bucket (0 if it
/// touched only one), in those middle buckets, and in the wake-up bucket.
fn split_off_cpu_stretch_by_bucket(
    wake_epoch_ns: u64,
    off_cpu_ns: u64,
    bucket_ns: u64,
) -> (u32, u64, u64, u64) {
    let in_bucket_ns = wake_epoch_ns % bucket_ns;
    if off_cpu_ns <= in_bucket_ns {
        return (0, 0, 0, off_cpu_ns);
    }
    let head_ns = off_cpu_ns - in_bucket_ns;
    if head_ns <= bucket_ns {
        return (0, head_ns, 0, in_bucket_ns);
    }
    let mid_buckets = (head_ns - 1) / bucket_ns;
    let mid_ns = mid_buckets * bucket_ns;
    (mid_buckets as u32, head_ns - mid_ns, mid_ns, in_bucket_ns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_off_cpu_stretch_by_bucket() {
        let ns_per_ms = 1_000_000;
        // (wake-up, off-CPU ms) -> (middle buckets, head ms, mid ms, tail ms), 100 ms buckets.
        for (wake, delta, want) in [
            (80, 60, (0, 0, 0, 60)),
            (170, 120, (0, 50, 0, 70)),
            (210, 80, (0, 70, 0, 10)),
            (240, 150, (1, 10, 100, 40)),
            (260, 250, (1, 90, 100, 60)),
            (300, 100, (0, 100, 0, 0)),
            (300, 200, (1, 100, 100, 0)),
            (301, 102, (1, 1, 100, 1)),
            (350, 340, (2, 90, 200, 50)),
        ] {
            let (mid_buckets, head_ns, mid_ns, tail_ns) = split_off_cpu_stretch_by_bucket(
                wake * ns_per_ms,
                delta * ns_per_ms,
                100 * ns_per_ms,
            );
            assert_eq!(
                (
                    mid_buckets,
                    head_ns / ns_per_ms,
                    mid_ns / ns_per_ms,
                    tail_ns / ns_per_ms
                ),
                want,
                "wake {wake} delta {delta}"
            );
        }
    }
}
