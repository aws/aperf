#![cfg(target_os = "linux")]

pub mod off_cpu;
pub mod parser;

use crate::profiling::symbols::Symbol;

/// The information of a parsed Perf sample.
#[derive(Default, Debug)]
struct PerfSample {
    /// The PID that this sample belongs to.
    pid: i32,
    /// The timestamp of the sample in nanoseconds from the EPOCH.
    timestamp: u64,
    /// The name of the sampled thread.
    thread_name: String,
    /// The symbolicated call chain of the sample.
    /// The order is from leaf to root.
    call_chain: Vec<Symbol>,
    /// Sample weight: 1 for a Perf sample, off-CPU microseconds for an off-CPU event.
    samples: u64,
    /// `samples` per block as `(time_ms, count)` over the blocks an off-CPU event touched;
    /// empty for a Perf sample.
    splits: Vec<(i64, u64)>,
}

fn clock_ns(clock_id: libc::clockid_t) -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(clock_id, &mut time) };
    (time.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(time.tv_nsec as u64)
}

// See below constants in https://github.com/torvalds/linux/blob/master/include/uapi/linux/perf_event.h

/// It marks the case in a FORK event, where the child process has already exec'd and should
/// be treated as a regular process.
const PERF_RECORD_MISC_FORK_EXEC: u16 = 0x2000;
/// Sentinel inserted by the kernel into Perf sample callchain to mark that the following
/// are userspace addresses.
const PERF_CONTEXT_USER: u64 = 0xffff_ffff_ffff_fe00;
/// The bound of Perf Context.
const PERF_CONTEXT_MAX: u64 = 0xffff_ffff_ffff_f000;
