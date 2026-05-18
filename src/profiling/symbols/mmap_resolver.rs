use std::collections::{BTreeMap, HashMap, HashSet};

/// This struct contains the information of an MMAP/MMAP2 event. When a process runs, an MMAP event maps
/// a section in file with file_path, located at page_offset and spanning len, to address start_addr
/// of the process's virtual address space.
#[derive(Clone)]
struct MmapEntry {
    start_addr: u64,
    len: u64,
    page_offset: u64,
    file_path: String,
}

/// All MMAP entries of a process.
#[derive(Default)]
struct ProcessMmap {
    /// Sorted map from start_address to MMAP entry, to quickly locate the corresponding MMAP entry
    /// of a virtual address through binary search.
    mmap_table: MmapTable,
    /// If this process is just forked and the MMAP table was copied from its parent.
    forked: bool,
}

type MmapTable = BTreeMap<u64, MmapEntry>;

/// This struct contains MMAP events and related logics to resolve the virtual address in a process into the
/// file offset of an ELF file. The file offset and the ELF file can then be resolved into a symbol name.
#[derive(Default)]
pub struct MmapResolver {
    /// Map from pid to the process's MMAP entries.
    per_process_mmaps: HashMap<i32, ProcessMmap>,
    /// Kernel MMAP table.
    kernel_mmaps: MmapTable,
    /// Cache the list of PIDs that have MMAP-ed a file path. It is used during the fallback of
    /// reading the file from a process's root file system exposed by the Kernel.
    file_path_pids_cache: HashMap<String, HashSet<i32>>,
    /// Used to identify whether a PID belongs to a HotSpot JVM, which might require additional
    /// logics to build the JIT symbol table (see jit_symbols.rs).
    hotspot_jvm_pids: HashSet<i32>,
}

impl MmapResolver {
    /// Store an MMAP entry for a process with PID.
    pub fn add_mmap(
        &mut self,
        pid: i32,
        start_addr: u64,
        len: u64,
        page_offset: u64,
        file_path: String,
    ) {
        self.file_path_pids_cache
            .entry(file_path.clone())
            .or_insert_with(|| HashSet::new())
            .insert(pid);
        let process_map = self
            .per_process_mmaps
            .entry(pid)
            .or_insert_with(|| ProcessMmap::default());

        // When a forked process has an MMAP event, it has exec'd and the MMAP table
        // copied from its parent is no longer valid.
        if process_map.forked {
            process_map.mmap_table.clear();
            process_map.forked = false;
        }

        // HotSpot JVM candidate.
        if file_path.contains("libjvm") {
            self.hotspot_jvm_pids.insert(pid);
        }

        process_map.mmap_table.insert(
            start_addr,
            MmapEntry {
                start_addr,
                len,
                page_offset,
                file_path,
            },
        );
    }

    /// Store a Kernel MMAP entry.
    pub fn add_kernel_mmap(
        &mut self,
        start_addr: u64,
        len: u64,
        page_offset: u64,
        filename: String,
    ) {
        self.kernel_mmaps.insert(
            start_addr,
            MmapEntry {
                start_addr,
                len,
                page_offset,
                file_path: filename,
            },
        );
    }

    /// Handle the case where a process is forked, by copying its parent's MMAP table.
    pub fn fork_process(&mut self, ppid: i32, pid: i32) {
        let cloned_parent_mmap_table = match self.per_process_mmaps.get(&ppid) {
            Some(parent_process_mmaps) => parent_process_mmaps.mmap_table.clone(),
            None => return,
        };
        self.per_process_mmaps
            .entry(pid)
            .or_insert_with(|| ProcessMmap {
                mmap_table: cloned_parent_mmap_table,
                forked: true,
            });
    }

    /// Resolve an instruction address of a process into the corresponding file offset and ELF file path.
    pub fn resolve_addr(&self, pid: i32, addr: u64) -> Option<(u64, String)> {
        let process_mmaps = self.per_process_mmaps.get(&pid)?;
        resolve_mmap(&process_mmaps.mmap_table, addr)
    }

    /// Resolve an instruction address in the Kernel space into the corresponding file offset and ELF file path.
    pub fn resolve_kernel_addr(&self, kernel_addr: u64) -> Option<(u64, String)> {
        resolve_mmap(&self.kernel_mmaps, kernel_addr)
    }

    /// Retrieves the list of PIDs that have MMAP-ed the file path.
    pub fn get_file_path_pids(&self, file_path: &str) -> Vec<i32> {
        self.file_path_pids_cache
            .get(file_path)
            .map(|pids| pids.into_iter().map(|pid| *pid).collect())
            .unwrap_or(Vec::new())
    }

    /// Check if the pid belongs to a HotSpot JVM.
    pub fn is_pid_hotspot_jvm(&self, pid: i32) -> bool {
        self.hotspot_jvm_pids.contains(&pid)
    }
}

/// Resolve an address into the corresponding file offset and ELF file path in the MMAP table.
fn resolve_mmap(mmap_table: &MmapTable, addr: u64) -> Option<(u64, String)> {
    let mmap_entry = mmap_table.range(..=addr).next_back()?.1;
    if addr < mmap_entry.start_addr + mmap_entry.len {
        let file_offset = addr - mmap_entry.start_addr + mmap_entry.page_offset;
        Some((file_offset, mmap_entry.file_path.clone()))
    } else {
        None
    }
}
