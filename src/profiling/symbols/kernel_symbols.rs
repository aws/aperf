use crate::profiling::symbols::{
    prefer_second_symbol, resolve_symbol, tail_symbol_bound, RawSymbol, ResolvedSymbol,
    SymbolTableEntry,
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
        let mut symbol_table_entry = self.raw_symbol.to_symbol_table_entry();
        // A Kernel symbol does not actually have len - the len in raw_symbol is pseudo and
        // only used for deduplication (see below).
        symbol_table_entry.size = 0;
        symbol_table_entry
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
            symbol_table: process_raw_kernel_symbols(
                collect_raw_kernel_symbols(PathBuf::from("/proc/kallsyms"))?,
                load_proc_modules(),
            ),
        })
    }

    /// Resolve a Kernel address into a symbol.
    pub fn resolve(&self, addr: u64) -> Option<ResolvedSymbol> {
        if self.symbol_table.is_empty() {
            return None;
        }
        resolve_symbol(addr, &self.symbol_table, "kallsyms")
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

/// Load /proc/modules so we can attribute a symbol to its module when
/// kallsyms has duplicates at the same address from different modules.
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

/// Read the file (/proc/kallsyms) containing all kernel symbols and parse them into
/// a list of RawKernelSymbol sorted by the symbol address.
fn collect_raw_kernel_symbols(path: PathBuf) -> Result<Vec<RawKernelSymbol>> {
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
    // Pre-compute a pseudo-size for each entry following Perf's symbols__fixup_end function, by
    // computing the difference between two neighbor symbols. The pseudo-size will be then used
    // to deduplicate overlapping symbols.
    // This is to force that, for symbols with the same addr, the last symbol in the kallsyms file
    // will get picked (Rust's sort_by_key is stable), which follows Perf's logic.
    for i in 0..raw_kernel_symbols.len() {
        let end_addr = if i + 1 < raw_kernel_symbols.len() {
            raw_kernel_symbols[i + 1].raw_symbol.addr
        } else {
            tail_symbol_bound(raw_kernel_symbols[i].raw_symbol.addr)
        };
        raw_kernel_symbols[i].raw_symbol.size =
            end_addr.saturating_sub(raw_kernel_symbols[i].raw_symbol.addr)
    }

    Ok(raw_kernel_symbols)
}

/// Create the final Kernel symbol table by applying the deduplication logic on the raw symbols.
fn process_raw_kernel_symbols(
    raw_kernel_symbols: Vec<RawKernelSymbol>,
    proc_module_entries: Vec<ProcModuleEntry>,
) -> Vec<SymbolTableEntry> {
    let mut kernel_symbol_table: Vec<SymbolTableEntry> = Vec::new();

    // Deduplicate kernel symbols that share the same address.
    let mut raw_kernel_symbol_iterator = raw_kernel_symbols.into_iter().peekable();
    while let Some(cur_raw_kernel_symbol) = raw_kernel_symbol_iterator.next() {
        let cur_raw_kernel_symbol_addr = cur_raw_kernel_symbol.raw_symbol.addr;
        // Attempt to retrieve the module that the address belongs to.
        let matched_module = match proc_module_entries
            .binary_search_by_key(&cur_raw_kernel_symbol_addr, |proc_module_entry| {
                proc_module_entry.base_addr
            }) {
            Ok(i) => Some(&proc_module_entries[i].module_name),
            Err(0) => None,
            Err(i) => {
                let matched_proc_module_entry = &proc_module_entries[i - 1];
                if cur_raw_kernel_symbol_addr < matched_proc_module_entry.end_addr {
                    Some(&matched_proc_module_entry.module_name)
                } else {
                    None
                }
            }
        };

        let mut chosen_raw_kernel_symbol = cur_raw_kernel_symbol;
        let mut chosen_with_matched_module =
            matched_module.is_some() && chosen_raw_kernel_symbol.module.as_ref() == matched_module;

        while let Some(next_raw_kernel_symbol) = raw_kernel_symbol_iterator.peek() {
            if next_raw_kernel_symbol.raw_symbol.addr != cur_raw_kernel_symbol_addr {
                break;
            }

            let next_raw_kernel_symbol = raw_kernel_symbol_iterator.next().unwrap();
            let next_raw_kernel_symbol_match_module = matched_module.is_some()
                && next_raw_kernel_symbol.module.as_ref() == matched_module;

            // When choosing between two symbols with the same address, prefer the one whose module
            // matches the /proc/modules. If none or both of them match, apply the choose_best_symbol
            // logic in Perf.
            let chose_next_raw_kernel_symbol =
                if chosen_with_matched_module != next_raw_kernel_symbol_match_module {
                    next_raw_kernel_symbol_match_module
                } else {
                    prefer_second_symbol(
                        &chosen_raw_kernel_symbol.raw_symbol,
                        &next_raw_kernel_symbol.raw_symbol,
                    )
                };
            if chose_next_raw_kernel_symbol {
                chosen_raw_kernel_symbol = next_raw_kernel_symbol;
                chosen_with_matched_module = next_raw_kernel_symbol_match_module;
            }
        }

        kernel_symbol_table.push(chosen_raw_kernel_symbol.to_symbol_table_entry());
    }

    kernel_symbol_table
}
