#![cfg(target_os = "linux")]

pub mod parser;

use crate::profiling::symbols::ResolvedSymbol;

/// The information of a parsed Perf sample.
#[derive(Default, Debug)]
struct PerfSample {
    /// The PID that this sample belongs to.
    pid: i32,
    /// The timestamp of the sample in nanoseconds from the EPOCH.
    timestamp: u64,
    /// The symbolicated call chain of the sample.
    /// The order is from leaf to root.
    call_chain: Vec<Option<ResolvedSymbol>>,
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
