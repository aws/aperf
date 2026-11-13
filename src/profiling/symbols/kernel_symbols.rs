use crate::profiling::symbols::{
    prefer_second_symbol, resolve_symbol, tail_symbol_bound, RawSymbol, SymbolTableEntry,
};
use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// The information of a symbol retrieved from /proc/kallsyms, which, in addition to
/// the common raw symbol information, can have a designated Kernel module that it belongs to.
struct RawKernelSymbol {
    raw_symbol: RawSymbol,
    module: Option<String>,
}
impl RawKernelSymbol {
    fn to_symbol_table_entry(self) -> SymbolTableEntry {
        self.raw_symbol.to_symbol_table_entry()
    }
}

/// An entry in /proc/modules, indicating the Kernel module that an address range belongs to.
struct ProcModuleEntry {
    base_addr: u64,
    end_addr: u64,
    module_name: String,
}

/// Store all Kernel symbols retrieved from /proc/kallsyms to resolve an address.
#[derive(Default)]
pub struct KernelSymbols {
    symbol_table: Vec<SymbolTableEntry>,
}

impl KernelSymbols {
    /// Parse the content of /proc/kallsyms to build the Kernel symbol table.
    pub fn from_kallsyms() -> Result<Self> {
        Ok(KernelSymbols {
            symbol_table: build_kernel_symbol_table(
                PathBuf::from("/proc/kallsyms"),
                &load_proc_modules(),
            )?,
        })
    }

    /// Resolve a Kernel address into a symbol.
    pub fn resolve(&self, addr: u64) -> Option<String> {
        if self.symbol_table.is_empty() {
            return None;
        }
        resolve_symbol(addr, &self.symbol_table)
    }
}

/// Accepts the symbol type letters that can correspond to runtime code
/// or data we might sample: T/t, W/w, D/d, B/b, plus the handful of
/// special lowercase types perf also accepts (u, l, N, 1).
/// Refer to symbol_type__filter in
/// https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol.c
fn is_valid_type_letter(t: char) -> bool {
    let upper = t.to_ascii_uppercase();
    upper == 'T'
        || upper == 'W'
        || upper == 'D'
        || upper == 'B'
        || t == 'u'
        || t == 'l'
        || t == 'N'
        || t == '1'
}

/// Load /proc/modules so we can tell which module an address belongs to.
fn load_proc_modules() -> Vec<ProcModuleEntry> {
    let mut proc_module_entries: Vec<ProcModuleEntry> = Vec::new();

    if let Ok(proc_module_content) = fs::read_to_string("/proc/modules") {
        for line in proc_module_content.lines() {
            // Format: name size refcount deps state base_addr
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 6 {
                continue;
            }
            let size: u64 = parts[1].parse().unwrap_or(0);
            let base = parts[5].trim_start_matches("0x");
            let base_addr = u64::from_str_radix(base, 16).unwrap_or(0);
            if base_addr == 0 || size == 0 {
                continue;
            }
            let end_addr = base_addr + size;
            let module_name = parts[0].to_string();

            proc_module_entries.push(ProcModuleEntry {
                base_addr,
                end_addr,
                module_name,
            });
        }
        // Sort all entries by base_addr for fast lookup to see if a symbol has matched module.
        proc_module_entries.sort_by_key(|proc_module_entry| proc_module_entry.base_addr);
    }

    proc_module_entries
}

/// The module whose /proc/modules range holds the address, if any.
fn owning_module(addr: u64, proc_module_entries: &[ProcModuleEntry]) -> Option<&str> {
    let index = proc_module_entries.partition_point(|entry| entry.base_addr <= addr);
    let entry = proc_module_entries.get(index.checked_sub(1)?)?;
    (addr < entry.end_addr).then_some(entry.module_name.as_str())
}

/// Read the file (/proc/kallsyms) containing all kernel symbols and parse them into
/// a symbol table sorted by address, with one symbol per address.
fn build_kernel_symbol_table(
    path: PathBuf,
    proc_module_entries: &[ProcModuleEntry],
) -> Result<Vec<SymbolTableEntry>> {
    let kallsyms_file = fs::File::open(path)?;
    let buf_reader = BufReader::new(kallsyms_file);

    let mut raw_kernel_symbols: Vec<RawKernelSymbol> = Vec::new();

    for line_result in buf_reader.lines() {
        let line = match line_result {
            Ok(line) => line,
            Err(_) => continue,
        };

        // Each line is in the format of <address> <type letter> <raw name>, where the
        // raw name part contain the kernel symbol name and the module it belongs to (in "[]")
        // e.g. ffffffffc0619010 t __nf_conntrack_hash_insert   [nf_conntrack]
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() < 3 {
            continue;
        }
        let addr = u64::from_str_radix(parts[0], 16).unwrap_or(0);
        if addr == 0 {
            continue;
        }
        let type_letter = parts[1].chars().next().unwrap_or('?');
        if !is_valid_type_letter(type_letter) {
            continue;
        }
        let raw_name = parts[2];
        // Ignore ARM mapping symbols and similar module-local labels,
        // matching perf's map__process_kallsym_symbol (tools/perf/util/symbol.c):
        //     if (name[0] == '$') return 0;
        if raw_name.starts_with('$') || raw_name.starts_with("__pi_$") {
            continue;
        }
        // Retrieve symbol name and module it belongs to
        let (name, module) = if let Some(tab_pos) = raw_name.find('\t') {
            let module_part = &raw_name[tab_pos + 1..];
            let module = module_part
                .trim_matches(|c| c == '[' || c == ']')
                .to_string();
            (raw_name[..tab_pos].to_string(), Some(module))
        } else {
            (raw_name.to_string(), None)
        };

        // If a loaded module's function at an address that now belongs to another module,
        // it is stale and should be skipped.
        if let Some(module) = &module {
            if type_letter == 'T'
                && matches!(owning_module(addr, proc_module_entries), Some(owner) if owner != module)
            {
                continue;
            }
        }

        raw_kernel_symbols.push(RawKernelSymbol {
            raw_symbol: RawSymbol {
                addr,
                // Give a dummy size first and compute it after all symbols are sorted.
                size: 0,
                name,
                // In kallsyms, an upper case type letter denotes a global symbol.
                is_global: type_letter.is_ascii_uppercase(),
                // In kallsyms, weak symbols use type letters 'W' or 'w'.
                is_weak: type_letter.to_ascii_lowercase() == 'w',
                // All kallsyms symbols have a type.
                is_no_type: false,
            },
            module,
        });
    }

    raw_kernel_symbols.sort_by_key(|raw_kernel_symbol| raw_kernel_symbol.raw_symbol.addr);
    // kallsyms carries no sizes, so each symbol's size has to be synthesized from where the next
    // one starts if it is from the same module, otherwise add a constant bound, following Perf's
    // symbols__fixup_end function.
    for i in 0..raw_kernel_symbols.len() {
        let addr = raw_kernel_symbols[i].raw_symbol.addr;
        let end_addr = match raw_kernel_symbols.get(i + 1) {
            Some(next_raw_kernel_symbol)
                if next_raw_kernel_symbol.module == raw_kernel_symbols[i].module =>
            {
                next_raw_kernel_symbol.raw_symbol.addr
            }
            _ => tail_symbol_bound(addr),
        };
        raw_kernel_symbols[i].raw_symbol.size = end_addr.saturating_sub(addr)
    }

    // Keep one symbol per address by Perf's choose_best_symbol logic.
    let mut kernel_symbol_table: Vec<SymbolTableEntry> = Vec::new();
    let mut raw_kernel_symbol_iterator = raw_kernel_symbols.into_iter().peekable();
    while let Some(mut chosen_raw_kernel_symbol) = raw_kernel_symbol_iterator.next() {
        while let Some(next_raw_kernel_symbol) = raw_kernel_symbol_iterator
            .next_if(|next| next.raw_symbol.addr == chosen_raw_kernel_symbol.raw_symbol.addr)
        {
            if prefer_second_symbol(
                &chosen_raw_kernel_symbol.raw_symbol,
                &next_raw_kernel_symbol.raw_symbol,
            ) {
                chosen_raw_kernel_symbol = next_raw_kernel_symbol;
            }
        }
        kernel_symbol_table.push(chosen_raw_kernel_symbol.to_symbol_table_entry());
    }

    Ok(kernel_symbol_table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// The kernel text, a module region and a BPF region, with a page-sized hole between each:
    /// the gaps are where a synthesized size must stop.
    const KALLSYMS: &str = "\
ffff800080010000 T do_one_thing
ffff800080010040 T do_another_thing
ffff800080010040 t do_another_thing_local
ffff800080011000 t last_vmlinux_symbol
ffff800080a00000 t nf_conntrack_init\t[nf_conntrack]
ffff800080a00100 t nf_conntrack_insert\t[nf_conntrack]
ffff800080b00000 t bpf_prog_1234\t[bpf]
";

    fn kernel_symbols(
        kallsyms_content: &str,
        proc_module_entries: &[ProcModuleEntry],
    ) -> KernelSymbols {
        let mut kallsyms_file = tempfile::NamedTempFile::new().unwrap();
        kallsyms_file
            .write_all(kallsyms_content.as_bytes())
            .unwrap();
        KernelSymbols {
            symbol_table: build_kernel_symbol_table(
                kallsyms_file.path().to_path_buf(),
                proc_module_entries,
            )
            .unwrap(),
        }
    }

    fn resolved_name(kernel_symbols: &KernelSymbols, addr: u64) -> Option<String> {
        kernel_symbols.resolve(addr)
    }

    #[test]
    fn symbol_sizes_stop_at_a_module_boundary() {
        let kernel_symbols = kernel_symbols(KALLSYMS, &[]);

        // Within the kernel text a symbol runs up to the next one.
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_8001_0010).as_deref(),
            Some("do_one_thing")
        );
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_8001_0ff0).as_deref(),
            Some("do_another_thing_local")
        );

        // The last kernel symbol before the module region stops a page in, rather than
        // swallowing the whole unmapped gap up to the first module symbol.
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_8001_1010).as_deref(),
            Some("last_vmlinux_symbol")
        );
        assert_eq!(resolved_name(&kernel_symbols, 0xffff_8000_8001_2000), None);
        assert_eq!(resolved_name(&kernel_symbols, 0xffff_8000_809f_ffff), None);

        // Same for the gap between two different modules, and after the very last symbol.
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_80a0_0180).as_deref(),
            Some("nf_conntrack_insert")
        );
        assert_eq!(resolved_name(&kernel_symbols, 0xffff_8000_80a0_2000), None);
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_80b0_0010).as_deref(),
            Some("bpf_prog_1234")
        );
        assert_eq!(resolved_name(&kernel_symbols, 0xffff_8000_80b0_2000), None);

        // Nothing below the first symbol resolves.
        assert_eq!(resolved_name(&kernel_symbols, 0xffff_8000_8000_ffff), None);
    }

    #[test]
    fn last_duplicate_symbol_at_an_address_wins() {
        // Perf keeps the last of the duplicates listed at an address; the synthesized sizes are
        // what make that happen here, so a change to them must not quietly flip the choice.
        let kernel_symbols = kernel_symbols(KALLSYMS, &[]);
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_8001_0040).as_deref(),
            Some("do_another_thing_local")
        );
    }

    #[test]
    fn stale_init_function_in_another_module_is_skipped() {
        // ftrace still lists vxlan's freed init function where nf_tables was loaded later.
        let kallsyms = "\
ffff800080a00000 t nft_update_chain_stats\t[nf_tables]
ffff800080a00090 T vxlan_init_module\t[vxlan]
ffff800080a00098 T nft_do_chain\t[nf_tables]
ffff800080c00000 t vxlan_xmit\t[vxlan]
";
        let proc_module_entries = [
            ProcModuleEntry {
                base_addr: 0xffff_8000_80a0_0000,
                end_addr: 0xffff_8000_80b0_0000,
                module_name: "nf_tables".to_string(),
            },
            ProcModuleEntry {
                base_addr: 0xffff_8000_80c0_0000,
                end_addr: 0xffff_8000_80c1_0000,
                module_name: "vxlan".to_string(),
            },
        ];
        let kernel_symbols = kernel_symbols(kallsyms, &proc_module_entries);
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_80a0_0094).as_deref(),
            Some("nft_update_chain_stats")
        );
        assert_eq!(
            resolved_name(&kernel_symbols, 0xffff_8000_80a0_0100).as_deref(),
            Some("nft_do_chain")
        );
    }
}
