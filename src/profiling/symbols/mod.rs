#![cfg(target_os = "linux")]

mod elf_build_ids;
mod elf_symbols;
mod jit_symbols;
mod kernel_symbols;
mod mmap_resolver;
pub mod symbol_resolver;
mod vdso_symbols;

/// A single entry in the symbol table, indicating that all addresses within
/// [addr, addr + size) should be resolved to name.
pub struct SymbolTableEntry {
    addr: u64,
    size: u64,
    name: String,
}

/// Information of a resolved symbol.
#[derive(Debug)]
pub struct ResolvedSymbol {
    /// The human-readable name of the symbol.
    pub name: String,
    /// The offset from the starting address of the symbol.
    pub offset: u64,
    /// The source file or module that the symbol belongs to.
    pub source: String,
}

/// Attempt to resolve an address into a symbol in the symbol table. Entries in the symbol
/// table is sorted by every symbol's starting address to enable binary search.
pub fn resolve_symbol(
    addr: u64,
    symbol_table: &Vec<SymbolTableEntry>,
    source: &str,
) -> Option<ResolvedSymbol> {
    let idx = match symbol_table
        .binary_search_by_key(&addr, |symbol_table_entry| symbol_table_entry.addr)
    {
        Ok(i) => i,
        Err(0) => return None,
        Err(i) => i - 1,
    };
    let symbol_table_entry = &symbol_table[idx];
    let symbol_offset = addr - symbol_table_entry.addr;
    if symbol_table_entry.size > 0 {
        if symbol_offset >= symbol_table_entry.size {
            return None;
        }
    } else {
        // Handle symbols with zero-size, referring to function symbols__fixup_end in
        // https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol.c
        if idx + 1 < symbol_table.len() {
            let next_addr = symbol_table[idx + 1].addr;
            if addr >= next_addr {
                return None;
            }
        } else if addr >= tail_symbol_bound(symbol_table_entry.addr) {
            return None;
        }
    }

    Some(ResolvedSymbol {
        name: symbol_table_entry.name.clone(),
        offset: symbol_offset,
        source: source.to_string(),
    })
}

/// The information of a symbol retrieved from an ELF file (for userspace symbols) or
/// kallsyms (for Kernel symbols), used to compare and deduplicate symbols with same addresses,
/// to choose the preferable one to create an entry in the final symbol table.
pub struct RawSymbol {
    /// The starting address of the symbol.
    addr: u64,
    /// The size of the symbol in bytes, i.e. any address falling between [addr, addr + size)
    /// should be resolved to this symbol.
    size: u64,
    /// The symbol name.
    name: String,
    /// A Local symbol is private to the translation unit that defines it and never exported
    /// across object file, Equivalent to a C static function or a C++ anonymous-namespace helper.
    /// A global symbol is externally visible - it is the default binding for non-static C/C++
    /// functions and exported kernel symbols.
    is_global: bool,
    /// A weak symbol is a hint saying "only use it if no strong symbol exists". They are
    /// commonly default implementations or aliases.
    is_weak: bool,
    /// It indicates that the symbol exists, but the toolchain did not assert its type. Typical
    /// sources are handwritten assemblies or linker generated symbols.
    is_no_type: bool,
}
impl RawSymbol {
    fn to_symbol_table_entry(self) -> SymbolTableEntry {
        SymbolTableEntry {
            addr: self.addr,
            size: self.size,
            name: demangle_symbol(&self.name),
        }
    }
}

/// Refer to choose_best_symbol function in
/// https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol.c
pub fn prefer_second_symbol(a: &RawSymbol, b: &RawSymbol) -> bool {
    // Prefer non-zero size
    let a_non_zero_size = a.size > 0;
    let b_non_zero_size = b.size > 0;
    if a_non_zero_size != b_non_zero_size {
        return b_non_zero_size;
    }
    // Prefer non-NOTYPE
    if a.is_no_type != b.is_no_type {
        return a.is_no_type;
    }
    // Prefer non-weak
    if a.is_weak != b.is_weak {
        return a.is_weak;
    }
    // Prefer global
    if a.is_global != b.is_global {
        return b.is_global;
    }
    // Prefer fewer leading underscores.
    let a_num_leading_underscores = a.name.bytes().take_while(|&c| c == b'_').count();
    let b_num_leading_underscores = b.name.bytes().take_while(|&c| c == b'_').count();
    if a_num_leading_underscores != b_num_leading_underscores {
        return b_num_leading_underscores < a_num_leading_underscores;
    }
    // Prefer longer name
    b.name.len() > a.name.len()
}

/// Set the bound when dealing with last entry in a symbol table that has
/// no size (so we do not know the actual bound). This refers to the
/// symbols__fixup_end function (the "roundup(prev->end + 4096, 4096)" part) in
/// https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol.c
pub fn tail_symbol_bound(symbol_address: u64) -> u64 {
    ((symbol_address + 4095) & !4095u64) + 4096
}

/// Demangle a Rust or C++ symbol.
pub fn demangle_symbol(name: &str) -> String {
    // Try Rust demangling first (v0 and legacy formats)
    if name.starts_with("_R") || name.starts_with("_ZN") {
        let demangled = rustc_demangle::demangle(name).to_string();
        if demangled != name {
            return demangled;
        }
    }
    // Try C++ demangling — use cpp_demangle with no_params + no_return_type
    // for clean output that matches perf's format
    if name.starts_with("_Z") || name.starts_with("___Z") {
        if let Ok(sym) = cpp_demangle::Symbol::new(name) {
            let options = cpp_demangle::DemangleOptions::default()
                .no_return_type()
                .no_params();
            if let Ok(demangled) = sym.demangle_with_options(&options) {
                return demangled;
            }
        }
    }

    name.to_string()
}
