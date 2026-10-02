use crate::profiling::symbols::{resolve_symbol, ResolvedSymbol, SymbolTableEntry};
use crate::run_command_and_wait;
use anyhow::Result;
use log::debug;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};

/// Store all JIT symbols created by application runtimes to resolve an address.
///
/// Note that jitdump is not currently processed, since it requires nontrivial logics and
/// has limited use cases. Usually a JVM can be configured to write perf maps instead.
#[derive(Default)]
pub struct JitSymbols {
    /// All JIT symbols sorted by addr for fast lookup.
    symbol_table: Vec<SymbolTableEntry>,
}

impl JitSymbols {
    /// Parse the JIT symbols in /tmp/perf-<pid>.map file. Application runtimes are responsible
    /// for writing their symbol mappings to the file.
    /// * For HotSpot JVM, this is done through the JVM argument -XX:+DumpPerfMapAtExit. However,
    ///   this flag only creates the perf maps after the JVM exits. Therefore, we run jcmd as a
    ///   fallback to genarate the perf maps when the JVM is still running.
    /// * For V8, --perf-basic-prof writes perf maps continuously.
    /// * For .NET, setting DOTNET_PerfMapEnabled=1 writes perf maps continuously.
    ///
    /// Refer to the dso__load_perf_map function in
    /// https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol.c
    pub fn from_perf_map(pid: i32) -> Result<Self> {
        let perf_map_file_path = PathBuf::from(format!("/tmp/perf-{}.map", pid));
        if !perf_map_file_path.exists() {
            if is_live_hotspot_jvm(pid) {
                create_hotspot_jvm_perf_map(pid);
            } else {
                return Ok(Self::default());
            }
        }

        let perf_map_file = File::open(&perf_map_file_path)?;
        let buf_reader = BufReader::new(perf_map_file);

        let mut symbol_table: Vec<SymbolTableEntry> = Vec::new();

        // Format: <hex_addr> <hex_size> <name>
        for line in buf_reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() < 3 {
                continue;
            }
            let addr_str = parts[0]
                .strip_prefix("0x")
                .or_else(|| parts[0].strip_prefix("0X"))
                .unwrap_or(parts[0]);
            let size_str = parts[1]
                .strip_prefix("0x")
                .or_else(|| parts[1].strip_prefix("0X"))
                .unwrap_or(parts[1]);
            let addr = u64::from_str_radix(addr_str, 16).unwrap_or(0);
            if addr == 0 {
                continue;
            }
            let size = u64::from_str_radix(size_str, 16).unwrap_or(0);
            let name = parts[2].to_string();
            symbol_table.push(SymbolTableEntry::new(addr, size, name));
        }

        symbol_table.sort_by_key(|symbol_table_entry| symbol_table_entry.addr);

        Ok(JitSymbols { symbol_table })
    }

    /// Resolve an address into a symbol in the symbol table.
    pub fn resolve(&self, addr: u64) -> Option<ResolvedSymbol> {
        if self.symbol_table.is_empty() {
            return None;
        }
        resolve_symbol(addr, &self.symbol_table, "jit")
    }
}

/// Whether the process is a live HotSpot JVM, which is a requirement for running jcmd against it.
/// jcmd from JDK 17 or older sends SIGQUIT to and kills a process that is not a JVM.
fn is_live_hotspot_jvm(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/maps")).is_ok_and(|maps| maps.contains("libjvm"))
}

/// Run `jcmd <pid> Compiler.perfmap` to ask a HotSpot JVM to dump /tmp/perf-<pid>.map.
/// It is best-effort and any failures leave symbols unresolved.
fn create_hotspot_jvm_perf_map(pid: i32) {
    let result = run_command_and_wait("jcmd", [&pid.to_string(), "Compiler.perfmap"], "jcmd", None);
    match result {
        Ok(out) if out.status.success() => debug!("jcmd Compiler.perfmap succeeded for pid {pid}"),
        Ok(out) => debug!(
            "jcmd Compiler.perfmap exited with {} for pid {pid}",
            out.status
        ),
        Err(e) => debug!("jcmd Compiler.perfmap failed for pid {pid}: {e}"),
    }
}
