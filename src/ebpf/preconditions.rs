use log::warn;
use nix::unistd;
use std::fs;
use std::io::{BufRead, BufReader};

const CAP_SYS_ADMIN: u64 = 21;
const CAP_SYS_RESOURCE: u64 = 24;
const CAP_PERFMON: u64 = 38;
const CAP_BPF: u64 = 39;

fn cap_eff() -> u64 {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|line| line.strip_prefix("CapEff:"))
                .and_then(|value| u64::from_str_radix(value.trim(), 16).ok())
        })
        .unwrap_or(0)
}

fn has_cap(eff: u64, bit: u64) -> bool {
    (eff >> bit) & 1 == 1
}

/// General eBPF prep: verify the capabilities any eBPF load needs and parse kernel BTF, returning
/// the `Btf` the loader requires. Every eBPF collector calls this first.
pub fn check_preconditions() -> Result<aya::Btf, String> {
    let root = unistd::geteuid().is_root();
    let eff = cap_eff();

    let bpf_ok =
        root || (has_cap(eff, CAP_BPF) && has_cap(eff, CAP_PERFMON)) || has_cap(eff, CAP_SYS_ADMIN);
    if !bpf_ok {
        return Err(
            "insufficient capabilities for eBPF (need root, or CAP_BPF+CAP_PERFMON, or CAP_SYS_ADMIN)"
                .to_string(),
        );
    }

    aya::Btf::from_sys_fs().map_err(|e| format!("kernel BTF unavailable: {e}"))
}

pub fn memlock_hint() -> Option<&'static str> {
    let applies = !unistd::geteuid().is_root()
        && !has_cap(cap_eff(), CAP_SYS_RESOURCE)
        && aya::util::KernelVersion::current()
            .is_ok_and(|version| version < aya::util::KernelVersion::new(5, 11, 0));
    applies.then_some(
        "kernels before 5.11 charge eBPF maps and programs to RLIMIT_MEMLOCK: run as root, grant CAP_SYS_RESOURCE, or raise the memlock limit",
    )
}

pub fn warn_if_kallsyms_hidden() {
    // kptr_restrict=2 zeros /proc/kallsyms even for root, so check regardless of privilege.
    let hidden = fs::File::open("/proc/kallsyms")
        .map(|file| {
            BufReader::new(file)
                .lines()
                .map_while(Result::ok)
                .all(|line| {
                    line.split_whitespace()
                        .next()
                        .is_none_or(|addr| addr.bytes().all(|b| b == b'0'))
                })
        })
        .unwrap_or(true);
    if hidden {
        warn!(
            "Off-CPU eBPF profiling: /proc/kallsyms addresses are hidden, so off-CPU kernel frames will be [unknown]. Run `sudo sysctl -w kernel.kptr_restrict=0`, and when not running as root also `sudo sysctl -w kernel.perf_event_paranoid=1` (or grant CAP_SYSLOG)"
        );
    }
}
