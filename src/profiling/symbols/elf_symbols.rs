use crate::profiling::symbols::demangle_symbol;
use crate::profiling::symbols::{
    prefer_second_symbol, resolve_symbol, RawSymbol, ResolvedSymbol, SymbolTableEntry,
};
use anyhow::Result;
use object::{
    read::elf::SectionHeader, Architecture, Object, ObjectKind, ObjectSection, ObjectSegment,
    ObjectSymbol, SymbolIterator, SymbolKind, SymbolScope, SymbolSection,
};
use std::collections::HashMap;

/// A single entry in the ELF file's program header table, indicating a segment
/// from [file_offset, file_offset + size) will be loaded at p_vaddr.
struct LoadSegment {
    file_offset: u64,
    size: u64,
    p_vaddr: u64,
}

/// Indicates what a CFI (Call Frame Information) in .eh_frame or .debug_frame sections says
/// about if the current value in the LR register is the correct return address, and can be
/// used for leaf frame recovery.
#[derive(Clone, Copy, PartialEq, Debug)]
enum CfiRule {
    /// The current value in the LR register is the correct return address.
    SameValue,
    /// The current value in the LR register is not the correct return address, which
    /// is instead saved on stack with some offsets.
    Offset,
    /// The current value in the LR register is not the correct return address for other
    /// reasons that we do not handle.
    Other,
    /// There is no CFI rule (used for sentinel entries)
    None,
}
impl CfiRule {
    fn is_none(&self) -> bool {
        *self == CfiRule::None
    }
}

/// An entry in the CFI rule table, which indicates that for any address that lies between
/// addr and the next entry's addr, the rule applies.
struct CfiRuleTableEntry {
    addr: u64,
    rule: CfiRule,
}

/// Store all symbols retrieved from an ELF file to resolve a file offset.
#[derive(Default)]
pub struct ElfSymbols {
    /// All symbols in the ELF file's .symtab and .dynsyms sections, sorted by p_vaddr for fast lookup.
    symbol_table: Vec<SymbolTableEntry>,
    /// All load segments in the ELF file's program header, used to convert a file_offset into p_vaddr.
    load_segments: Vec<LoadSegment>,
    /// All CFI rules sorted by addr for fast look up during leaf frame recovery.
    cfi_rule_table: Vec<CfiRuleTableEntry>,
    /// The name of the ELF file.
    elf_filename: String,
    /// True for ET_DYN (shared libraries and PIE executables).
    is_dyn: bool,
}

impl ElfSymbols {
    /// Parse the contents of an ELF file and build the symbol table.
    pub fn from_elf_data(
        elf_data: &[u8],
        elf_filename: String,
        support_leaf_frame_recovery: bool,
    ) -> Result<Self> {
        let elf_obj = object::File::parse(elf_data)?;
        let is_dyn = elf_obj.kind() == ObjectKind::Dynamic;

        // Populate and sort all load segments from the ELF file's program header
        let mut load_segments: Vec<LoadSegment> = Vec::new();
        for segment in elf_obj.segments() {
            load_segments.push(LoadSegment {
                file_offset: segment.file_range().0,
                size: segment.file_range().1,
                p_vaddr: segment.address(),
            })
        }

        let architecture = elf_obj.architecture();
        let is_x86 = matches!(architecture, Architecture::X86_64 | Architecture::I386);

        // On x86, check for .plt.sec — modern binaries compiled with -fcf-protection or
        // linked with -z now emit the actual PLT entries in the .plt.sec section,
        // while .plt becomes a pure resolver trampoline.
        let (plt_section_name, lazy_plt) =
            if is_x86 && elf_obj.section_by_name(".plt.sec").is_some() {
                (".plt.sec", false)
            } else {
                (".plt", true)
            };

        // Retrieve all sizes of the PLT section. They will be used to filter out symbols when
        // creating the symbol table from .symtab and .dynsym, and to parse the PLT section.
        let (plt_section_header_size, plt_entry_size, plt_entries_offset, plt_section_size) =
            get_plt_section_sizes(&elf_obj, plt_section_name, architecture)
                .unwrap_or_else(|| (0, 0, 0, 0));
        let plt_address_range = (plt_entries_offset, plt_entries_offset + plt_section_size);

        // Collect all raw symbols from the .symtab and .dynsym section of the ELF file, and also
        // create the index-symbol map for the two sections, to be used to create the PLT symbol table.
        let mut raw_symbols: Vec<RawSymbol> = Vec::new();
        let mut symtab_index_name_map: HashMap<usize, &str> = HashMap::new();
        let mut dynsym_index_name_map: HashMap<usize, &str> = HashMap::new();
        collect_raw_symbols(
            elf_obj.symbols(),
            &mut raw_symbols,
            &mut symtab_index_name_map,
            plt_address_range,
        );
        collect_raw_symbols(
            elf_obj.dynamic_symbols(),
            &mut raw_symbols,
            &mut dynsym_index_name_map,
            plt_address_range,
        );
        // Fedora/RHEL/Ubuntu could strip .symtab in distro shared libs but create the .gnu_debugdata
        // section, which is a LZMA-compressed embedded ELF file containing minimal debug symbols (mostly.symtab).
        let mut gnu_debugdata_data: Option<Vec<u8>> = None;
        if let Some(section) = elf_obj.section_by_name(".gnu_debugdata") {
            if let Ok(compressed) = section.data() {
                let mut decompressed = Vec::new();
                if lzma_rs::xz_decompress(&mut std::io::Cursor::new(compressed), &mut decompressed)
                    .is_ok()
                {
                    gnu_debugdata_data = Some(decompressed);
                }
            }
        }
        let mini_elf_obj: Option<object::File> = gnu_debugdata_data
            .as_ref()
            .and_then(|data| object::File::parse(data as &[u8]).ok());
        if let Some(ref mini_elf) = mini_elf_obj {
            collect_raw_symbols(
                mini_elf.symbols(),
                &mut raw_symbols,
                &mut symtab_index_name_map,
                plt_address_range,
            );
        }
        let mut symbol_table = process_raw_symbols(raw_symbols);

        let plt_symbols = parse_plt(
            &elf_obj,
            &dynsym_index_name_map,
            &symtab_index_name_map,
            is_x86,
            plt_section_header_size,
            plt_entry_size,
            plt_entries_offset,
            plt_section_size,
            lazy_plt,
        );

        // Create the final symbol tables
        symbol_table.extend(plt_symbols);
        symbol_table.sort_by_key(|symbol_table_entry| symbol_table_entry.addr);

        // If leaf frame recovery is enabled, build the CFI rule table
        let cfi_rule_table = if support_leaf_frame_recovery {
            create_cfi_rule_table(&elf_obj)
        } else {
            Vec::new()
        };

        Ok(ElfSymbols {
            symbol_table,
            load_segments,
            elf_filename,
            cfi_rule_table,
            is_dyn,
        })
    }

    /// Resolve a file_offset into a symbol in the symbol table.
    pub fn resolve(&self, file_offset: u64) -> Option<ResolvedSymbol> {
        if self.symbol_table.is_empty() {
            return None;
        }
        resolve_symbol(
            self.file_offset_to_p_vaddr(file_offset)?,
            &self.symbol_table,
            &self.elf_filename,
        )
    }

    /// Unwind the leaf caller frame by checking the corresponding CFI rule and deciding if the
    /// value in the LR register is the correct address in the leaf caller frame.
    pub fn recover_leaf_frame_caller(
        &self,
        leaf_file_offset: u64,
        lr: u64,
        fp: Option<u64>,
        sp: Option<u64>,
    ) -> Option<u64> {
        if self.cfi_rule_table.is_empty() {
            return None;
        }
        // Follows Perf's implementation in unwind-libdw.c, where if the ELF file is
        // ET_DYN (shared libraries and PIE), it uses the file offset directly to query
        // the CFI rule table. Otherwise, the p_vaddr should be used.
        let cfi_table_lookup_key = if self.is_dyn {
            leaf_file_offset
        } else {
            if let Some(p_vaddr) = self.file_offset_to_p_vaddr(leaf_file_offset) {
                p_vaddr
            } else {
                leaf_file_offset
            }
        };

        let matched_cfi_rule = match self
            .cfi_rule_table
            .binary_search_by_key(&cfi_table_lookup_key, |cfi_rule_table_entry| {
                cfi_rule_table_entry.addr
            }) {
            Ok(i) => self.cfi_rule_table[i].rule,
            Err(0) => CfiRule::None,
            Err(i) => self.cfi_rule_table[i - 1].rule,
        };
        match matched_cfi_rule {
            // CFI rule says the value in LR is the correct return address of the leaf
            CfiRule::SameValue => Some(lr),
            // CFI rule says the value in LR is not the correct return address of the leaf
            CfiRule::Offset | CfiRule::Other => None,
            // No CFI rule coverage, so fall back to making decision using register values.
            // It follows the same logics in the aarch64_unwind.c of elfutils, which is used
            // by Perf.
            CfiRule::None => {
                // Missing fp/sp are treated as 0 — matching elfutils' aarch64_unwind,
                // which sets `fp = 0; sp = 0` when getfunc fails.
                let fp = fp.unwrap_or(0);
                let sp = sp.unwrap_or(0);
                if lr != 0 {
                    // fp = 0: the function did not set up a frame pointer, so naturally the
                    //         LR value is not saved to the stack yet.
                    // fp + 16 > sp: the stack is moving toward the correct direction.
                    if fp == 0 || fp.wrapping_add(16) > sp {
                        return Some(lr);
                    }
                }
                None
            }
        }
    }

    /// Use the load segment of the ELF file to convert a file_offset into p_vaddr. The number
    /// of load segments is usually small (~4), so linear scan is faster.
    fn file_offset_to_p_vaddr(&self, file_offset: u64) -> Option<u64> {
        for load_segment in &self.load_segments {
            if file_offset >= load_segment.file_offset
                && file_offset < load_segment.file_offset + load_segment.size
            {
                return Some(file_offset - load_segment.file_offset + load_segment.p_vaddr);
            }
        }
        None
    }
}

// ================================== .symtab and .dynsym section ==================================

/// Iterate through all symbols retrieved from the ELF file (through symbol_iterator). Convert each of them
/// into RawSymbol and fill raw_symbols, which will be later processed to deduplicate symbols with the same
/// address and create the final symbol table. Also map every symbol name to index in symbol_index_name_map,
/// which is needed to populate the PLT symbol table.
fn collect_raw_symbols<'a>(
    symbol_iterator: SymbolIterator<'a, 'a>,
    raw_symbols: &mut Vec<RawSymbol>,
    symbol_index_name_map: &mut HashMap<usize, &'a str>,
    plt_address_range: (u64, u64),
) {
    for sym in symbol_iterator {
        let symbol_name = match sym.name() {
            Ok(symbol_name) => symbol_name,
            Err(_) => continue,
        };

        symbol_index_name_map.insert(sym.index().0, symbol_name);

        // Mirrors perf's elf_sym__filter:
        //   accept if elf_sym__is_function()  -- STT_FUNC / STT_GNU_IFUNC
        //   accept if elf_sym__is_object()    -- STT_OBJECT
        //   accept if elf_sym__is_label()     -- STT_NOTYPE with a proper name
        //                                       (not SHN_UNDEF / SHN_ABS)
        // The `object` crate maps these to:
        //   SymbolKind::Text    <- STT_FUNC / STT_GNU_IFUNC
        //   SymbolKind::Data    <- STT_OBJECT / STT_COMMON
        //   SymbolKind::Unknown <- STT_NOTYPE
        // Everything else (File, Section, Tls, ...) is dropped.
        // See https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol-elf.c
        let is_function_symbol = sym.kind() == SymbolKind::Text;
        let is_object_symbol = sym.kind() == SymbolKind::Data;
        let is_label_symbol =
            sym.kind() == SymbolKind::Unknown && sym.section() != SymbolSection::Absolute;
        if symbol_name.is_empty()
            || sym.address() == 0
            || sym.section() == SymbolSection::Undefined
            || !(is_function_symbol || is_object_symbol || is_label_symbol)
        {
            continue;
        }
        // Skip dynamic symbols whose address falls in a PLT section. They will be handled
        // later when parsing the PLT section.
        if sym.address() >= plt_address_range.0 && sym.address() < plt_address_range.1 {
            continue;
        }
        // Filter ARM/AARCH64 mapping symbols ($a, $d, $t, $x)
        if symbol_name.starts_with('$') && symbol_name.len() >= 2 {
            let second = symbol_name.as_bytes()[1];
            if second == b'a' || second == b'd' || second == b't' || second == b'x' {
                continue;
            }
        }
        raw_symbols.push(RawSymbol {
            addr: sym.address(),
            size: sym.size(),
            name: symbol_name.to_string(),
            is_weak: sym.is_weak(),
            is_global: sym.scope() == SymbolScope::Dynamic || sym.scope() == SymbolScope::Linkage,
            is_no_type: sym.kind() == SymbolKind::Unknown,
        });
    }
}

/// Create the final symbol table from the raw symbol tables, by sorting all raw symbols and
/// applying the deduplication logic.
fn process_raw_symbols(mut raw_symbols: Vec<RawSymbol>) -> Vec<SymbolTableEntry> {
    // Sort all raw symbols by address to apply deduplication logics. It also naturally makes the
    // final symbol table sorted and enables its fast lookup.
    raw_symbols.sort_by(|a, b| a.addr.cmp(&b.addr).then(a.name.cmp(&b.name)));

    let mut symbol_table: Vec<SymbolTableEntry> = Vec::new();
    // For symbols with same address, apply a deduplication logic to choose the best one for this address.
    let mut raw_symbols_iterator = raw_symbols.into_iter().peekable();
    while let Some(cur_raw_symbol) = raw_symbols_iterator.next() {
        let cur_raw_symbol_addr = cur_raw_symbol.addr;
        let mut chosen_raw_symbol = cur_raw_symbol;
        while let Some(next_raw_symbol) = raw_symbols_iterator.peek() {
            if next_raw_symbol.addr != cur_raw_symbol_addr {
                break;
            }
            let next_raw_symbol = raw_symbols_iterator.next().unwrap();
            if prefer_second_symbol(&chosen_raw_symbol, &next_raw_symbol) {
                chosen_raw_symbol = next_raw_symbol;
            }
        }
        symbol_table.push(chosen_raw_symbol.to_symbol_table_entry())
    }

    symbol_table
}

// =================================== .plt and .plt.sec section ===================================

/// Parsing the PLT section of the ELF file into a symbol table.
/// The logics refer to dso__synthesize_plt_symbols in
/// https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol-elf.c
fn parse_plt(
    elf_obj: &object::File,
    dynsym_index_name_map: &HashMap<usize, &str>,
    symtab_index_name_map: &HashMap<usize, &str>,
    is_x86: bool,
    plt_section_header_size: u64,
    plt_entry_size: u64,
    mut plt_entries_offset: u64,
    plt_section_size: u64,
    lazy_plt: bool,
) -> Vec<SymbolTableEntry> {
    let mut entries = Vec::new();

    // Find relocation section - prefer .rela.plt (SHT_RELA, 24-byte entries) but fall back to .rel.plt (SHT_REL, 16-byte entries).
    let (relocation_section, use_rela, relocation_entry_size) =
        if let Some(s) = elf_obj.section_by_name(".rela.plt") {
            (
                s,
                true,
                get_elf_section_sh_entsize(elf_obj, ".rela.plt") as usize,
            )
        } else if let Some(s) = elf_obj.section_by_name(".rel.plt") {
            (
                s,
                false,
                get_elf_section_sh_entsize(elf_obj, ".rel.plt") as usize,
            )
        } else {
            return entries;
        };
    let relocation_section_data = match relocation_section.data() {
        Ok(d) => d,
        Err(_) => return entries,
    };
    let relocation_section_size = relocation_section_data.len();
    if relocation_entry_size == 0 || relocation_section_size % relocation_entry_size != 0 {
        return entries;
    }

    // Parse every relocation entry into (relocation_entry_offset, symbol_index). For every entry,
    // relocation_entry_offset is used for sorting on X86 (see below), and symbol_index is to map
    // it back to the symbol table.
    let mut relocation_entries: Vec<(u64, u32)> =
        Vec::with_capacity(relocation_section_size / relocation_entry_size);
    let mut i = 0;
    while i + relocation_entry_size <= relocation_section_size {
        let relocation_entries_offset =
            u64::from_le_bytes(relocation_section_data[i..i + 8].try_into().unwrap());
        let cur_relocation_entry = if use_rela {
            u64::from_le_bytes(relocation_section_data[i + 8..i + 16].try_into().unwrap())
        } else {
            u32::from_le_bytes(relocation_section_data[i + 8..i + 12].try_into().unwrap()) as u64
        };
        let symbol_index = if use_rela {
            (cur_relocation_entry >> 32) as u32
        } else {
            // 32-bit r_info layout: low 8 = type, high 24 = sym.
            (cur_relocation_entry as u32) >> 8
        };
        relocation_entries.push((relocation_entries_offset, symbol_index));
        i += relocation_entry_size;
    }

    if relocation_entries.is_empty() {
        return entries;
    }

    // According to Perf, X86 doesn't insert IFUNC relocations in the same order as PLT, so this is
    // to get it back to order. Refer to the sort_rel function.
    if is_x86 {
        relocation_entries.sort_by_key(|&(off, _)| off);
    }

    let num_relocation_entries = relocation_entries.len() as u64;

    // Matching Perf's implementation (search for lazy_plt), which assumes that if the number of relocation entries
    // match exactly the number of PLT entries, the PLT section is not lazy and does not have a header. In this case,
    // plt_entries_offset is already pointing at the first entry in the PLT section. Otherwise, move the offset over
    // the header to point it to the first entry.
    if lazy_plt && num_relocation_entries * plt_entry_size != plt_section_size {
        plt_entries_offset += plt_section_header_size;
    }

    // For every relocation entry, create a symbol entry in the symbol table. All the relocation entries should
    // now be in the same order as PLT entries, so each entry can be mapped to the corresponding PLT entry's address.
    for &(_, symbol_index) in relocation_entries.iter() {
        let name: Option<&str> = if symbol_index != 0 {
            dynsym_index_name_map
                .get(&(symbol_index as usize))
                .or_else(|| symtab_index_name_map.get(&(symbol_index as usize)))
                .copied()
        } else {
            None
        };
        let symbol_name = match name {
            Some(n) => {
                let demangled_name = demangle_symbol(n);
                let short = if let Some(p) = demangled_name.find('(') {
                    demangled_name[..p].to_string()
                } else {
                    demangled_name
                };
                format!("{}@plt", short)
            }
            None => format!("offset_{:#x}@plt", plt_entries_offset),
        };
        entries.push(SymbolTableEntry {
            addr: plt_entries_offset,
            size: plt_entry_size,
            name: symbol_name,
        });
        plt_entries_offset += plt_entry_size;
    }

    entries.sort_by_key(|entry| entry.addr);
    entries
}

/// Compute the sizes of components within the .plt or .plt.sec section:
/// - plt_header_size: the size of the header of the PLT section
/// - plt_entry_size: the size of an entry in the PLT section
/// - plt_entries_offset: the address of the first entry in the PLT section
/// - plt_section_size: the size of the PLT section
/// Refer to the get_plt_sizes function in
/// https://github.com/torvalds/linux/blob/master/tools/perf/util/symbol-elf.c
fn get_plt_section_sizes(
    elf_obj: &object::File,
    plt_section_name: &str,
    architecture: Architecture,
) -> Option<(u64, u64, u64, u64)> {
    let plt_section = elf_obj.section_by_name(plt_section_name)?;

    let plt_sh_entsize = get_elf_section_sh_entsize(elf_obj, plt_section_name);

    let (plt_header_size, plt_entry_size) = match architecture {
        Architecture::Aarch64 => (32u64, 16u64),
        Architecture::X86_64 | Architecture::I386 => {
            let entry_size = if plt_sh_entsize == 8 || plt_sh_entsize == 16 {
                plt_sh_entsize
            } else if plt_section.align() == 8 {
                8
            } else {
                16
            };
            (entry_size, entry_size)
        }
        Architecture::Arm => (20u64, 12u64),
        _ => (plt_sh_entsize, plt_sh_entsize),
    };

    Some((
        plt_header_size,
        plt_entry_size,
        plt_section.address(),
        plt_section.size(),
    ))
}

/// Retrieve the sh_entsize field inside an ELF section, which describes the size of each entry
/// within the section's data.
fn get_elf_section_sh_entsize(elf_obj: &object::File, section_name: &str) -> u64 {
    match elf_obj {
        object::File::Elf64(elf_64_obj) => {
            if let Some(elf_section) = elf_64_obj.section_by_name(section_name) {
                elf_section
                    .elf_section_header()
                    .sh_entsize(elf_64_obj.endian())
            } else {
                0
            }
        }
        object::File::Elf32(elf_32_obj) => {
            if let Some(elf_section) = elf_32_obj.section_by_name(section_name) {
                elf_section
                    .elf_section_header()
                    .sh_entsize(elf_32_obj.endian()) as u64
            } else {
                0
            }
        }
        _ => 0,
    }
}

// ============================== .eh_frame and .debug_frame section ===============================

/// Create the CFI rule table from the content of an ELF file, by parsing the .eh_frame
/// and .debug_frame sections.
fn create_cfi_rule_table(elf_obj: &object::File) -> Vec<CfiRuleTableEntry> {
    let mut cfi_rule_table: Vec<CfiRuleTableEntry> = Vec::new();

    let mut base_addresses = gimli::BaseAddresses::default();
    // When gimli parses an FDE entry in en_frame, it needs to parse encoded pointers sometimes
    // requiring other ELF sections. Add the possibly required sections' base addresses so
    // that it knows where to find them.
    if let Some(text_section) = elf_obj.section_by_name(".text") {
        base_addresses = base_addresses.set_text(text_section.address())
    };
    if let Some(got_section) = elf_obj.section_by_name(".got") {
        base_addresses = base_addresses.set_got(got_section.address())
    };

    // Collect CFI rules from the .eh_frame section (should present in all ELF files)
    if let Some(eh_frame_section) = elf_obj.section_by_name(".eh_frame") {
        base_addresses = base_addresses.set_eh_frame(eh_frame_section.address());
        if let Ok(eh_frame_section_data) = eh_frame_section.data() {
            let mut eh_frame_unwind_section =
                gimli::EhFrame::new(eh_frame_section_data, gimli::LittleEndian);
            eh_frame_unwind_section.set_vendor(gimli::Vendor::AArch64);
            collect_cfi_rules(
                &eh_frame_unwind_section,
                &base_addresses,
                &mut cfi_rule_table,
            );
        }
    }
    // Collect CFI rules from the .debug_frame section (will not be in stripped binaries)
    if let Some(debug_frame_section) = elf_obj.section_by_name(".debug_frame") {
        if let Ok(debug_frame_section_data) = debug_frame_section
            .uncompressed_data()
            .map(|d| d.into_owned())
        {
            let mut debug_frame_unwind_section =
                gimli::DebugFrame::new(&debug_frame_section_data, gimli::LittleEndian);
            debug_frame_unwind_section.set_address_size(8);
            debug_frame_unwind_section.set_vendor(gimli::Vendor::AArch64);
            collect_cfi_rules(
                &debug_frame_unwind_section,
                &base_addresses,
                &mut cfi_rule_table,
            );
        }
    }

    // Sort all CFI rules by address, and for each address only keep the first rule
    // (the sentinel rules added will be overridden by real rules at the same address).
    cfi_rule_table.sort_by(|a, b| {
        a.addr
            .cmp(&b.addr)
            .then_with(|| a.rule.is_none().cmp(&b.rule.is_none()))
    });
    cfi_rule_table.dedup_by_key(|cfi_rule_table_entry| cfi_rule_table_entry.addr);

    cfi_rule_table
}

/// Parse all FDE (Frame Description Entry) entries in the section into CFI rules.
fn collect_cfi_rules<'a, S>(
    section: &S,
    base_addresses: &gimli::BaseAddresses,
    cfi_rule_table: &mut Vec<CfiRuleTableEntry>,
) where
    S: gimli::UnwindSection<gimli::EndianSlice<'a, gimli::LittleEndian>>,
    S::Offset: gimli::UnwindOffset<usize>,
{
    // All the FDE (Frame Description Entry) entries in .eh_frame and .debug_frame. Every FDE
    // entry has CFI (Call Frame Information) that describes the rules.
    let mut fde_entries = section.entries(base_addresses);
    let mut unwind_context = gimli::UnwindContext::new();
    // The whole frame-unwinding logic is run on ARM only, so we only care
    // about the LR (X30) register, which stores the return address.
    let lr_register = gimli::Register(30);

    while let Ok(Some(fde_entry)) = fde_entries.next() {
        if let gimli::CieOrFde::Fde(partial_fde) = fde_entry {
            if let Ok(fde) = partial_fde.parse(S::cie_from_offset) {
                let fde_start = fde.initial_address();
                let fde_end = fde_start + fde.len();

                // Walk through the CFI rows in the FDE entry
                if let Ok(mut cfi_rows_iter) =
                    fde.rows(section, base_addresses, &mut unwind_context)
                {
                    while let Ok(Some(cfi_row)) = cfi_rows_iter.next_row() {
                        let addr = cfi_row.start_address();

                        let rule = match cfi_row.register(lr_register) {
                            Some(gimli::RegisterRule::SameValue) => CfiRule::SameValue,
                            Some(gimli::RegisterRule::Offset(_)) => CfiRule::Offset,
                            Some(_) => CfiRule::Other,
                            // When there are no explicit rules, use default CFI rules specified
                            // by the platform's ABI. It follows the definitions in aarch64_abi_cfi
                            // in aarch64_cfi.c of elfutils.
                            None => CfiRule::SameValue,
                        };

                        cfi_rule_table.push(CfiRuleTableEntry { addr, rule });
                    }
                }

                // Add sentinel entry to the table that marks the end of an FDE entry, so for all the
                // address starting from the fde_end to the next valid CFI-covered address, there
                // is no valid CFI rule.
                cfi_rule_table.push(CfiRuleTableEntry {
                    addr: fde_end,
                    rule: CfiRule::None,
                })
            }
        }
    }
}
