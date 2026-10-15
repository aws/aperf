use crate::profiling::symbols::elf_build_ids::ElfBuildIds;
use crate::profiling::symbols::elf_symbols::ElfSymbols;
use crate::profiling::symbols::jit_symbols::JitSymbols;
use crate::profiling::symbols::kernel_symbols::KernelSymbols;
use crate::profiling::symbols::mmap_resolver::MmapResolver;
use crate::profiling::symbols::vdso_symbols::read_vdso_elf_data;
use crate::profiling::symbols::{Symbol, VDSO_ELF_FILE_PATH};
use crate::profiling::FrameType;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::fs;

#[derive(Default)]
pub struct SymbolResolver {
    /// Store and resolve an instruction address into a file offset within an ELF file.
    mmap_resolver: MmapResolver,
    /// Store all ELF files' Build-IDs to help locate the original ELF file.
    elf_build_ids: ElfBuildIds,
    /// All ELF symbol tables mapped to ELF file paths.
    elf_symbol_tables: HashMap<String, ElfSymbols>,
    /// All JIT symbol tables mapped to PIDs.
    jit_symbol_tables: HashMap<i32, JitSymbols>,
    /// All Kernel symbols.
    kernel_symbols: KernelSymbols,
    /// The start of kernel space, used to distinguish Kernel instructions from userspace instructions.
    kernel_space_start_address: u64,
    /// Whether to create the frame unwinder to support leaf frame recovery.
    support_leaf_caller_recovery: bool,
}

impl SymbolResolver {
    /// Initialize the Symbol Resolver for a specific architecture.
    pub fn for_arch(arch: &str) -> Self {
        let mut symbol_resolver = Self::default();

        // Expect string values from uname -m.
        let arch = arch.to_lowercase();
        if arch == "aarch64" || arch == "arm64" {
            // https://www.kernel.org/doc/html/next/arm64/memory.html
            symbol_resolver.kernel_space_start_address = 0xffff_0000_0000_0000;
            symbol_resolver.support_leaf_caller_recovery = true;
        } else if arch == "x86_64" {
            // https://www.kernel.org/doc/Documentation/x86/x86_64/mm.txt
            symbol_resolver.kernel_space_start_address = 0xffff_8000_0000_0000;
            symbol_resolver.support_leaf_caller_recovery = false;
        } else {
            warn!("Unrecognized arch value when creating Symbol Resolver: {arch}");
            symbol_resolver.kernel_space_start_address = 0xffff_0000_0000_0000;
            symbol_resolver.support_leaf_caller_recovery = false;
        }

        // Parse /proc/kallsyms and create the Kernel Symbol table.
        match KernelSymbols::from_kallsyms() {
            Ok(kernel_symbols) => symbol_resolver.kernel_symbols = kernel_symbols,
            Err(e) => error!(
                "Failed to parse Kernel symbols from /proc/kallsyms: {:?}",
                e
            ),
        }

        symbol_resolver
    }

    /// Whether the resolver supports leaf frame recovery.
    pub fn support_leaf_caller_recovery(&self) -> bool {
        self.support_leaf_caller_recovery
    }

    /// Store an MMAP event to be used for future symbol resolutions.
    pub fn add_mmap(
        &mut self,
        pid: i32,
        start_addr: u64,
        len: u64,
        page_offset: u64,
        filename: String,
    ) {
        if pid == -1 {
            self.mmap_resolver
                .add_kernel_mmap(start_addr, len, page_offset, filename)
        } else {
            self.mmap_resolver
                .add_mmap(pid, start_addr, len, page_offset, filename);
        }
    }

    /// Update the MMAP table when a process is forked - its parent's MMAP tables
    /// should be inherited, until the process is exec'd and receives its own MMAP events.
    pub fn handle_forked_process_mmap(&mut self, ppid: i32, pid: i32) {
        self.mmap_resolver.fork_process(ppid, pid);
        self.jit_symbol_tables.remove(&pid);
    }

    /// Handle the case where a process execs - its previous address space and JIT
    /// symbols no longer apply to the new image.
    pub fn handle_exec(&mut self, pid: i32) {
        self.mmap_resolver.handle_exec_process(pid);
        self.jit_symbol_tables.remove(&pid);
    }

    /// Store a pair of ELF file path and its Build-ID, to help search for the original build
    /// of the ELF file.
    pub fn add_build_id(&mut self, elf_file_path: &str, build_id: &[u8]) {
        self.elf_build_ids.add_build_id(elf_file_path, build_id)
    }

    /// Resolve an instruction address of a PID into a symbol.
    pub fn resolve(&mut self, pid: i32, addr: u64) -> Symbol {
        if addr >= self.kernel_space_start_address {
            self.resolve_kernel_addr(addr)
        } else {
            self.resolve_userspace_addr(pid, addr)
        }
    }

    /// Resolve a kernel instruction address through the following steps:
    /// 1. Attempt to resolve the address directly using the Kernel symbol table (built from /proc/kallsyms).
    /// 2. If the resolution failed, try to translate the address into file offset and ELF file
    ///    through Kernel MMAP entries, and then resolve it using ELF symbol table.
    fn resolve_kernel_addr(&mut self, addr: u64) -> Symbol {
        if let Some(symbol_name) = self.kernel_symbols.resolve(addr) {
            return Symbol {
                name: symbol_name,
                resolved: true,
                source: Some(("kallsyms".to_string(), addr)),
                frame_type: FrameType::Kernel,
            };
        }

        self.resolve_by_elf_symbols(-1, addr)
    }

    /// Resolve an userspace instruction address through the following steps:
    /// 1. Translating the address into file offset and ELF file through MMAP entries.
    /// 2. Resolve the file offset into symbol through the symbol table built from the ELF file.
    /// 3. If the MMAP translation or ELF symbol resolution failed, resolve the address
    ///    directly using the JIT symbol table.
    fn resolve_userspace_addr(&mut self, pid: i32, addr: u64) -> Symbol {
        let elf_symbol = self.resolve_by_elf_symbols(pid, addr);

        if elf_symbol.resolved {
            return elf_symbol;
        }

        let jit_symbol = self.resolve_by_jit_symbols(pid, addr);

        if jit_symbol.resolved {
            jit_symbol
        } else {
            // If the symbol also cannot be resolved through JIT, return
            // the ELF resolution result, which might contain ELF file info
            // if MMAP entry exists for the address.
            elf_symbol
        }
    }

    fn resolve_by_jit_symbols(&mut self, pid: i32, addr: u64) -> Symbol {
        // Still insert a dummy symbol table in case it could not be built, so that
        // the resolver does not attempt to keep rebuilding the symbol table.
        let symbol_name = self
            .jit_symbol_tables
            .entry(pid)
            .or_insert_with(|| JitSymbols::from_perf_map(pid).unwrap_or_default())
            .resolve(addr);

        let (name, resolved, source, frame_type) = match symbol_name {
            Some(symbol_name) => (
                symbol_name,
                true,
                Some((format!("perf-{pid}.map"), addr)),
                FrameType::Jit,
            ),
            None => (String::default(), false, None, FrameType::default()),
        };

        Symbol {
            name,
            resolved,
            source,
            frame_type,
        }
    }

    fn resolve_by_elf_symbols(&mut self, pid: i32, addr: u64) -> Symbol {
        let mut frame_type = if pid < 0 {
            FrameType::Kernel
        } else {
            FrameType::default()
        };

        let mmap_entry = if pid < 0 {
            self.mmap_resolver.resolve_kernel_addr(addr)
        } else {
            self.mmap_resolver.resolve_addr(pid, addr)
        };

        let (file_offset, elf_file_path) = match mmap_entry {
            Some(mmap_entry) => mmap_entry,
            None => {
                let source = if pid < 0 {
                    Some(("kallsyms".to_string(), addr))
                } else {
                    None
                };
                return Symbol {
                    name: String::new(),
                    resolved: false,
                    source,
                    frame_type,
                };
            }
        };
        if elf_file_path == VDSO_ELF_FILE_PATH {
            frame_type = FrameType::Vdso;
        }

        self.lazy_load_elf_file(pid, &elf_file_path);

        let symbol_name = self
            .elf_symbol_tables
            .get(&elf_file_path)
            .map_or(None, |elf_symbol_table| {
                elf_symbol_table.resolve(file_offset)
            });
        let resolved = symbol_name.is_some();

        Symbol {
            name: symbol_name.unwrap_or_default(),
            resolved,
            source: Some((elf_file_path, file_offset)),
            frame_type,
        }
    }

    /// Attempt to load the data of an ELF file and use it to build the ELF symbol table.
    fn lazy_load_elf_file(&mut self, pid: i32, elf_file_path: &str) {
        if self.elf_symbol_tables.contains_key(elf_file_path) {
            return;
        }

        let load_elf_file_data = || -> Option<Vec<u8>> {
            // First try to find the original build of the ELF file using the Build-ID - they ensure
            // that the instruction address, MMAP entries, and symbol tables all refer to the same ELF file.
            if let Some(original_elf_file_path) =
                self.elf_build_ids.find_original_elf_file(elf_file_path)
            {
                if let Ok(data) = fs::read(&original_elf_file_path) {
                    return Some(data);
                }
            }
            // If dealing with VDSO symbols, load them from APerf's memory - all processes
            // running on the same Kernel share the same VDSO symbol table.
            if elf_file_path == VDSO_ELF_FILE_PATH {
                return read_vdso_elf_data();
            }
            // If failed to find the original ELF file using the Build-ID, attempt to read
            // the ELF file path directly, if its Build-ID still matches.
            if let Ok(data) = fs::read(elf_file_path) {
                if self.elf_build_ids.matches_build_id(elf_file_path, &data) {
                    return Some(data);
                }
            }
            // Fall back to the process's FS mount exposed by the kernel /proc/<pid>/root/<path>
            // Caveats: the process needs to be owned by the same user who ran APerf, or sudo is
            // required. Also, the path will not exist anymore if the process has exited.
            if pid > 0 {
                let process_fs_path = format!("/proc/{}/root{}", pid, elf_file_path);
                // If this path can be read, by this point we've tried all possibilities of finding
                // the elf file for the pid. If the build id still does not match, there's no point
                // continuing to check its siblings below, as at best we'll just read again one of
                // the files found above.
                if let Ok(data) = fs::read(&process_fs_path) {
                    return self
                        .elf_build_ids
                        .matches_build_id(elf_file_path, &data)
                        .then_some(data);
                }

                // In case the process has exited, retrieve the list of PIDs that have MMAP-ed
                // this file path and access through their file system, in case one of them is
                // still running.
                for sibling_pid in self.mmap_resolver.get_file_path_pids(elf_file_path) {
                    if sibling_pid != pid {
                        let sibling_process_fs_path =
                            format!("/proc/{}/root{}", sibling_pid, elf_file_path);
                        if let Ok(data) = fs::read(&sibling_process_fs_path) {
                            return self
                                .elf_build_ids
                                .matches_build_id(elf_file_path, &data)
                                .then_some(data);
                        }
                    }
                }
            }
            // Could not find or open the ELF file.
            None
        };

        if let Some(elf_data) = load_elf_file_data() {
            self.elf_symbol_tables.insert(
                elf_file_path.to_string(),
                ElfSymbols::from_elf_data(&elf_data, self.support_leaf_caller_recovery)
                    .unwrap_or_else(|error| {
                        debug!(
                            "Failed to build ELF symbol table for file {elf_file_path}: {:?}",
                            error
                        );
                        // Still insert a dummy symbol table in case it could not be built, so that
                        // the resolver does not attempt to keep rebuilding the symbol table.
                        ElfSymbols::default()
                    }),
            );
        } else {
            // Same reasoning as above - we only attempt to build the tables once.
            self.elf_symbol_tables
                .insert(elf_file_path.to_string(), ElfSymbols::default());
        }
    }

    pub fn recover_leaf_frame_caller(
        &mut self,
        pid: i32,
        leaf_addr: u64,
        lr: u64,
        fp: Option<u64>,
        sp: Option<u64>,
    ) -> Option<u64> {
        let (leaf_file_offset, elf_file_path) =
            match self.mmap_resolver.resolve_addr(pid, leaf_addr) {
                Some(resolved_mmap) => resolved_mmap,
                None => return None,
            };

        self.lazy_load_elf_file(pid, &elf_file_path);

        self.elf_symbol_tables
            .get(&elf_file_path)
            .and_then(|elf_symbol_table| {
                elf_symbol_table.recover_leaf_frame_caller(leaf_file_offset, lr, fp, sp)
            })
    }
}
