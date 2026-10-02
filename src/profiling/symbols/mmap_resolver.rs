use std::collections::{BTreeMap, HashMap, HashSet};

/// This struct contains the information of an MMAP/MMAP2 event. When a process runs, an MMAP event maps
/// a section in file with file_path, located at page_offset and spanning len, to address start_addr
/// of the process's virtual address space.
#[derive(Clone)]
struct MmapEntry {
    start_addr: u64,
    end_addr: u64,
    page_offset: u64,
    file_path: String,
}

/// Sorted map from start_address to MMAP entry, to quickly locate the corresponding MMAP entry
/// of a virtual address through binary search. Entries are kept non-overlapping by
/// insert_with_overlap_fixup.
#[derive(Clone, Default)]
struct MmapTable(BTreeMap<u64, MmapEntry>);

impl MmapTable {
    /// Insert an MMAP entry, truncating or splitting any existing entries it overlaps, so the
    /// table always maps each address to the most recent MMAP covering it. Overlaps can happen
    /// between forkd child and parent, dlclose and dlopen calls, and JIT code being freed and
    /// reallocated.
    ///
    /// See maps__fixup_overlap_and_insert function in
    /// https://github.com/torvalds/linux/blob/master/tools/perf/util/maps.c
    fn insert(&mut self, start_addr: u64, len: u64, page_offset: u64, file_path: String) {
        let end_addr = start_addr.saturating_add(len);
        let new_entry = MmapEntry {
            start_addr,
            end_addr,
            page_offset,
            file_path,
        };

        if end_addr <= start_addr {
            return;
        }

        let mmap_table = &mut self.0;

        // Create the remnant of an old entry when adding an overlapping new entry.
        // Before: |                     old entry                          |
        // Add:                     |     new entry     |
        // Now:    |    old entry   |     new entry     | old entry remnant |
        let old_entry_remnant = |old_entry: &MmapEntry| MmapEntry {
            start_addr: end_addr,
            end_addr: old_entry.end_addr,
            page_offset: old_entry
                .page_offset
                .saturating_add(end_addr - old_entry.start_addr),
            file_path: old_entry.file_path.clone(),
        };

        // Locate the existing entry that starts before the new entry and create remnant
        // if there is overlap.
        if let Some((&left_entry_start_addr, left_entry)) =
            mmap_table.range(..start_addr).next_back()
        {
            if left_entry.end_addr > start_addr {
                if left_entry.end_addr > end_addr {
                    let left_entry_remnant = old_entry_remnant(left_entry);
                    mmap_table.get_mut(&left_entry_start_addr).unwrap().end_addr = start_addr;
                    mmap_table.insert(left_entry_remnant.start_addr, left_entry_remnant);
                    // No other overlapping entries to be handled, so return early.
                    mmap_table.insert(start_addr, new_entry);
                    return;
                } else {
                    mmap_table.get_mut(&left_entry_start_addr).unwrap().end_addr = start_addr;
                }
            }
        }

        // Remove all existing entries starting within the new entry's range, while handling
        // possible remnant.
        // Before:     |    A    |          B            |                  C                  |
        // Add:     |                          new entry                          |
        // Now:     |                          new entry                          | C remnant  |
        let overlap_start_addrs: Vec<u64> = mmap_table
            .range(start_addr..end_addr)
            .map(|(overlap_start_addr, _)| *overlap_start_addr)
            .collect();
        for overlap_start_addr in overlap_start_addrs {
            let overlap_entry = mmap_table.remove(&overlap_start_addr).unwrap();
            if overlap_entry.end_addr > end_addr {
                let overlap_entry_remnant = old_entry_remnant(&overlap_entry);
                mmap_table.insert(overlap_entry_remnant.start_addr, overlap_entry_remnant);
            }
        }

        mmap_table.insert(start_addr, new_entry);
    }

    /// Resolve an address into the corresponding file offset and ELF file path in the MMAP table.
    fn resolve(&self, addr: u64) -> Option<(u64, String)> {
        let mmap_entry = self.0.range(..=addr).next_back()?.1;
        if addr < mmap_entry.end_addr {
            let file_offset = addr - mmap_entry.start_addr + mmap_entry.page_offset;
            Some((file_offset, mmap_entry.file_path.clone()))
        } else {
            None
        }
    }
}

/// This struct contains MMAP events and related logics to resolve the virtual address in a process into the
/// file offset of an ELF file. The file offset and the ELF file can then be resolved into a symbol name.
#[derive(Default)]
pub struct MmapResolver {
    /// Map from pid to the process's MMAP entries.
    per_process_mmaps: HashMap<i32, MmapTable>,
    /// Kernel MMAP table.
    kernel_mmaps: MmapTable,
    /// Cache the list of PIDs that have MMAP-ed a file path. It is used during the fallback of
    /// reading the file from a process's root file system exposed by the Kernel.
    file_path_pids_cache: HashMap<String, HashSet<i32>>,
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

        self.per_process_mmaps.entry(pid).or_default().insert(
            start_addr,
            len,
            page_offset,
            file_path,
        );
    }

    /// Store a Kernel MMAP entry.
    pub fn add_kernel_mmap(
        &mut self,
        start_addr: u64,
        len: u64,
        page_offset: u64,
        file_path: String,
    ) {
        self.kernel_mmaps
            .insert(start_addr, len, page_offset, file_path);
    }

    /// Handle the case where a process is forked, by copying its parent's MMAP table. The copied
    /// entries stay valid and can have new entries, until the process execs.
    pub fn fork_process(&mut self, ppid: i32, pid: i32) {
        // The pid may have belonged to an exited process, so none of its mappings are kept.
        match self.per_process_mmaps.get(&ppid).cloned() {
            Some(parent_mmap_table) => self.per_process_mmaps.insert(pid, parent_mmap_table),
            None => self.per_process_mmaps.remove(&pid),
        };
    }

    /// Handle the case where a process execs, where the old address space is gone and therefore
    /// drop every MMAP entry.
    pub fn handle_exec_process(&mut self, pid: i32) {
        self.per_process_mmaps.remove(&pid);
    }

    /// Resolve an instruction address of a process into the corresponding file offset and ELF file path.
    pub fn resolve_addr(&self, pid: i32, addr: u64) -> Option<(u64, String)> {
        self.per_process_mmaps.get(&pid)?.resolve(addr)
    }

    /// Resolve an instruction address in the Kernel space into the corresponding file offset and ELF file path.
    pub fn resolve_kernel_addr(&self, kernel_addr: u64) -> Option<(u64, String)> {
        self.kernel_mmaps.resolve(kernel_addr)
    }

    /// Retrieves the list of PIDs that have MMAP-ed the file path.
    pub fn get_file_path_pids(&self, file_path: &str) -> Vec<i32> {
        self.file_path_pids_cache
            .get(file_path)
            .map(|pids| pids.into_iter().map(|pid| *pid).collect())
            .unwrap_or(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver_with(entries: &[(u64, u64, u64, &str)]) -> MmapResolver {
        let mut resolver = MmapResolver::default();
        for &(start, len, pgoff, path) in entries {
            resolver.add_mmap(1, start, len, pgoff, path.to_string());
        }
        resolver
    }

    #[test]
    fn splitting_insert_leaves_neighbors_and_boundaries_intact() {
        // A [0x1000,0x6000), C [0x6000,0x7000), then B lands inside A:
        //   | A head | B [0x2000,0x4000) | A remnant |     C     |
        let mut resolver =
            resolver_with(&[(0x1000, 0x5000, 0, "a.so"), (0x6000, 0x1000, 0, "c.so")]);
        resolver.add_mmap(1, 0x2000, 0x2000, 0, "b.so".to_string());

        assert_eq!(
            resolver.resolve_addr(1, 0x1800),
            Some((0x800, "a.so".to_string()))
        );
        assert_eq!(
            resolver.resolve_addr(1, 0x3000),
            Some((0x1000, "b.so".to_string()))
        );
        // The remnant's file offset stays continuous with A's original mapping.
        assert_eq!(
            resolver.resolve_addr(1, 0x4800),
            Some((0x3800, "a.so".to_string()))
        );
        // A split handles every overlap, so the right neighbor must be untouched.
        assert_eq!(
            resolver.resolve_addr(1, 0x6800),
            Some((0x800, "c.so".to_string()))
        );

        // Same start and length as B: an exact replacement, displacing only B.
        resolver.add_mmap(1, 0x2000, 0x2000, 0, "b2.so".to_string());
        assert_eq!(
            resolver.resolve_addr(1, 0x3000),
            Some((0x1000, "b2.so".to_string()))
        );
        assert_eq!(
            resolver.resolve_addr(1, 0x4800),
            Some((0x3800, "a.so".to_string()))
        );

        // A zero-length mapping covers no address and displaces nothing.
        resolver.add_mmap(1, 0x1800, 0, 0, "empty".to_string());
        assert_eq!(
            resolver.resolve_addr(1, 0x1900),
            Some((0x900, "a.so".to_string()))
        );

        // Below the first entry, past the last entry's exclusive end, unknown pid.
        assert_eq!(resolver.resolve_addr(1, 0xfff), None);
        assert_eq!(resolver.resolve_addr(1, 0x7000), None);
        assert_eq!(resolver.resolve_addr(2, 0x3000), None);
    }

    #[test]
    fn covering_insert_truncates_left_and_removes_multiple_entries() {
        // PRE | A | B | C, then NEW [0x800,0x4000) reaches into PRE and covers A, B
        // and C's head:
        //   | PRE head |            NEW            | C remnant |
        let mut resolver = resolver_with(&[
            (0x0, 0x1000, 0, "pre.so"),
            (0x1000, 0x1000, 0, "a.so"),
            (0x2000, 0x1000, 0, "b.so"),
            (0x3000, 0x3000, 0x40, "c.so"),
        ]);
        resolver.add_mmap(1, 0x800, 0x3800, 0, "new.so".to_string());

        assert_eq!(
            resolver.resolve_addr(1, 0x400),
            Some((0x400, "pre.so".to_string()))
        );
        for (addr, offset) in [(0x900, 0x100), (0x1800, 0x1000), (0x2800, 0x2000)] {
            assert_eq!(
                resolver.resolve_addr(1, addr),
                Some((offset, "new.so".to_string()))
            );
        }
        // C's part beyond NEW survives, with its file offset advanced past the
        // covered head (0x40 + 0x1000).
        assert_eq!(
            resolver.resolve_addr(1, 0x4800),
            Some((0x1840, "c.so".to_string()))
        );
    }

    #[test]
    fn saturating_page_offset_on_tail_remnant() {
        let mut resolver = MmapResolver::default();
        resolver.add_kernel_mmap(0x1000, u64::MAX, 0x2000, "vmlinux".to_string());
        resolver.add_kernel_mmap(0x8000, u64::MAX - 0x8100, 0, "module".to_string());
        // Must not panic; the end address and the remnant's page_offset saturate
        // instead of wrapping.
        assert!(resolver.resolve_kernel_addr(0x9000).is_some());
    }

    #[test]
    fn fork_and_exec_lifecycle() {
        // Fork copies the parent's table; a child MMAP (e.g. dlopen) keeps the
        // inherited entries and stays invisible to the parent; exec drops only the
        // child's entries.
        let mut resolver = resolver_with(&[(0x1000, 0x1000, 0, "parent.so")]);
        resolver.fork_process(1, 2);
        resolver.add_mmap(2, 0x9000, 0x1000, 0, "plugin.so".to_string());
        assert_eq!(
            resolver.resolve_addr(2, 0x1800),
            Some((0x800, "parent.so".to_string()))
        );
        assert_eq!(
            resolver.resolve_addr(2, 0x9800),
            Some((0x800, "plugin.so".to_string()))
        );
        assert_eq!(resolver.resolve_addr(1, 0x9800), None);

        resolver.handle_exec_process(2);
        assert_eq!(resolver.resolve_addr(2, 0x1800), None);
        assert_eq!(resolver.resolve_addr(2, 0x9800), None);
        // The parent's address space is unaffected by the child's exec.
        assert_eq!(
            resolver.resolve_addr(1, 0x1800),
            Some((0x800, "parent.so".to_string()))
        );
        resolver.add_mmap(2, 0x5000, 0x1000, 0, "new-image.so".to_string());
        assert_eq!(
            resolver.resolve_addr(2, 0x5800),
            Some((0x800, "new-image.so".to_string()))
        );

        // Pid 2 exits unnoticed and is reused by a fork of pid 3. It should start from pid 3's table
        // instead of the one that the exited process had.
        resolver.add_mmap(3, 0x3000, 0x1000, 0, "other-parent.so".to_string());
        resolver.fork_process(3, 2);
        assert_eq!(
            resolver.resolve_addr(2, 0x3800),
            Some((0x800, "other-parent.so".to_string()))
        );
        assert_eq!(resolver.resolve_addr(2, 0x5800), None);
        // A fork from a parent without a table leaves nothing behind either.
        resolver.fork_process(4, 2);
        assert_eq!(resolver.resolve_addr(2, 0x3800), None);
    }
}
