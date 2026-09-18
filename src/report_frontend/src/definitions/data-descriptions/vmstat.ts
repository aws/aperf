import type { DataDescription } from "../data-descriptions";
import { DIRTY_WRITEBACK_INVESTIGATION, MEMORY_COMPACTION_INVESTIGATION } from "./optimization-guides";
import { HUGE_PAGE_SIZES_KB } from "./mem-settings";

function perSizeThpStatsFieldDescriptions(): DataDescription["fieldDescriptions"] {
  const fieldDescriptions: DataDescription["fieldDescriptions"] = {};
  for (const sizeKb of HUGE_PAGE_SIZES_KB) {
    fieldDescriptions[`thp_fault_alloc_${sizeKb}kB`] = {
      readableName: `${sizeKb}kB Fault Alloc`,
      description: `Number of ${sizeKb}kB transparent huge pages allocated during page fault handling. This is the per-size twin of thp_fault_alloc, which counts only PMD-sized pages.`,
      desired: "lower",
    };
    fieldDescriptions[`thp_fault_fallback_${sizeKb}kB`] = {
      readableName: `${sizeKb}kB THP Fault Fallback`,
      description: `Number of times a ${sizeKb}kB transparent huge page allocation failed during page fault handling and fell back to smaller pages. This is the per-size twin of thp_fault_fallback.`,
      desired: "lower",
      unit: "Count",
    };
    fieldDescriptions[`nr_anon_transparent_hugepages_${sizeKb}kB`] = {
      readableName: `${sizeKb}kB Anonymous Transparent Hugepages Count`,
      description: `Number of ${sizeKb}kB transparent huge pages currently backing anonymous memory. This is the per-size twin of nr_anon_transparent_hugepages, which counts only PMD-sized pages.`,
      desired: "higher",
    };
    fieldDescriptions[`nr_anon_partially_mapped_${sizeKb}kB`] = {
      readableName: `${sizeKb}kB Partially Mapped THP Count`,
      description: `Number of ${sizeKb}kB transparent huge pages that are likely only partially mapped and possibly wasting memory, queued for deferred splitting. This is the live gauge whose cumulative event twin is thp_deferred_split_page.`,
      desired: "lower",
    };
  }
  return fieldDescriptions;
}

export const VMSTAT_DATA_DESCRIPTION: DataDescription = {
  readableName: "Virtual Memory Stats",
  summary:
    "Virtual memory metrics measure the usage of the system's virtual memory. The data were collected from the system pseudo-file /proc/vmstat. Note that for some metrics, the values were computed using the delta of two snapshots, so that first value is always zero.",
  defaultUnit: "Pages",
  defaultHelpfulLinks: ["https://www.man7.org/linux/man-pages/man5/proc_vmstat.5.html"],
  fieldDescriptions: {
    dirty_utilization: {
      readableName: "Dirty Page Cache Utilization",
      description:
        "How full the kernel's dirty page buffer is, calculated as nr_dirty over nr_dirty_threshold. As this value approaches 100%, processes that write are made to stop and flush data to disk themselves before they can continue. The threshold is a trigger point rather than a hard limit, so this value can go above 100%.",
      unit: "Utilization (%)",
      desired: "depends",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    thp_fault_fallback: {
      readableName: "THP Fault Fallback",
      description:
        "Number of times a PMD-sized transparent huge page could not be allocated for a page fault and regular pages were used instead. The cause may be that no contiguous block was free, that a memory cgroup limit refused the charge, or simply that the defrag policy chose not to work for one. Like thp_fault_alloc, this counts only the PMD size.",
      desired: "lower",
      unit: "Count",
    },
    thp_collapse_alloc: {
      readableName: "THP Collapse Allocation",
      description:
        "Successful allocations of Transparent Huge Pages through memory compaction and page migration processes.",
      desired: "higher",
      unit: "Count",
    },
    nr_anon_pages: {
      readableName: "Anonymous Pages Count",
      description: "Number of anonymous memory pages not backed by files including process heap and stack memory.",
      desired: "depends",
    },
    nr_inactive_file: {
      readableName: "Inactive File Pages Count",
      description:
        "Number of file-backed memory pages in the inactive LRU list that are candidates for reclamation during memory pressure.",
      desired: "moderate",
    },
    oom_kill: {
      readableName: "Out-of-Memory Kills",
      description:
        "Number of processes terminated by the kernel's Out of Memory killer when a memory allocation could not be satisfied. The killer picks its victim mainly by memory footprint, so the process terminated is often not the one that caused the pressure. This also counts processes killed for exceeding the memory limit of their own control group, such as a container hitting its limit while the host still has memory free.",
      desired: "lower",
      unit: "Count",
    },
    thp_file_mapped: {
      readableName: "THP File Mapped",
      description:
        "File-backed memory pages mapped using Transparent Huge Pages for improved memory access performance and reduced TLB pressure.",
      desired: "higher",
    },
    pgdeactivate: {
      readableName: "Page Deactivations",
      description:
        "Number of memory pages moved from active to inactive LRU lists as part of the kernel's memory reclamation process.",
      desired: "lower",
    },
    thp_deferred_split_page: {
      readableName: "THP Deferred Split Page",
      description:
        "Number of times a huge page was queued to be split because part of it was unmapped while the rest stayed mapped. The split is left to a shrinker that runs under memory pressure, so the whole huge page keeps occupying memory until then. This counts queueing events; nr_anon_partially_mapped is the live count of huge pages currently in that state.",
      desired: "lower",
    },
    htlb_buddy_alloc_success: {
      readableName: "Huge TLB Buddy Alloc Success",
      description:
        "Successful allocations of huge pages through the buddy allocator system for applications requiring large contiguous memory blocks.",
      desired: "higher",
    },
    nr_slab_reclaimable: {
      readableName: "Slab Reclaimable Pages",
      description:
        "Number of kernel slab memory pages that can be reclaimed when the system experiences memory pressure.",
      desired: "moderate",
    },
    pgfree: {
      readableName: "Page Frees",
      description:
        "Number of memory pages freed by the kernel memory management system for reallocation to other processes.",
      desired: "higher",
    },
    workingset_nodes: {
      readableName: "Working Set Nodes",
      description:
        "Number of working set nodes used by the kernel to track memory access patterns for efficient page reclamation decisions.",
      desired: "moderate",
      unit: "Count",
    },
    nr_dirty_background_threshold: {
      readableName: "Dirty Background Threshold",
      description:
        "The number of dirty pages at which the kernel starts writing them back in the background. Unlike nr_dirty_threshold, reaching this point does not stop the application from writing. The kernel derives it from vm.dirty_background_ratio applied to the memory currently available to hold dirty data.",
      desired: "depends",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    nr_vmscan_immediate_reclaim: {
      readableName: "VM Scan Immediate Reclaim",
      description:
        "Number of pages immediately reclaimed during memory scanning when the system is under severe memory pressure.",
      desired: "lower",
    },
    compact_stall: {
      readableName: "Compaction Stalls",
      description:
        "Number of times a process needed a physically contiguous block of memory, could not get one, and stopped to compact memory itself by migrating pages out of the way. The process is paused for the whole attempt. Every stall ends in either compact_success or compact_fail, so those two always add up to this.",
      desired: "lower",
      unit: "Count",
      optimization: [MEMORY_COMPACTION_INVESTIGATION],
    },
    htlb_buddy_alloc_fail: {
      readableName: "Huge TLB Buddy Alloc Fail",
      description:
        "Failed attempts to allocate huge pages through the buddy allocator indicating memory fragmentation or insufficient huge page pool.",
      desired: "lower",
      unit: "Count",
    },
    nr_file_pmdmapped: {
      readableName: "File PMD Mapped Pages",
      description:
        "Number of file-backed memory pages mapped using Page Middle Directory entries for large page optimizations.",
      desired: "lower",
    },
    pgsteal_kswapd: {
      readableName: "Page Steal Kswapd",
      description:
        "Number of pages freed by the background kswapd thread. This work happens outside the application, so a large value is normal on a busy system and costs the workload nothing directly, unlike the reclaim counted in pgsteal_direct.",
      desired: "lower",
    },
    nr_free_cma: {
      readableName: "Free CMA Pages",
      description:
        "Number of free pages in the Contiguous Memory Allocator pool available for devices requiring physically contiguous memory.",
      desired: "higher",
    },
    nr_dirty_threshold: {
      readableName: "Dirty Memory Threshold",
      description:
        "The number of dirty pages at which processes that write are made to stop and flush data to disk themselves. The kernel derives this from vm.dirty_ratio applied to the memory currently available to hold dirty data, so it moves during a run as free memory changes.",
      desired: "depends",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    pgpgin: {
      readableName: "Pages Paged In",
      description: "Number of pages read from storage devices into memory indicating I/O activity and memory pressure.",
      desired: "lower",
    },
    thp_zero_page_alloc_failed: {
      readableName: "THP Zero Page Alloc Failed",
      description:
        "Failed attempts to allocate Transparent Huge Pages for zero-filled memory indicating memory fragmentation or resource constraints.",
      desired: "lower",
    },
    nr_mapped: {
      readableName: "Mapped Pages Count",
      description:
        "Number of memory pages mapped into process address spaces for file-backed memory and shared libraries.",
      desired: "moderate",
    },
    nr_zone_write_pending: {
      readableName: "Zone Write Pending",
      description: "Number of pages in memory zones that are waiting to be written back to storage devices.",
      desired: "lower",
    },
    thp_split_page_failed: {
      readableName: "THP Split Page Failed",
      description:
        "Failed attempts to split Transparent Huge Pages into smaller pages due to memory constraints or fragmentation.",
      desired: "lower",
    },
    workingset_activate_file: {
      readableName: "Working Set Activate File",
      description:
        "File pages activated from inactive to active LRU list based on access patterns for better memory management.",
      desired: "moderate",
    },
    pgscan_direct: {
      readableName: "Page Scan Direct",
      description:
        "Number of pages examined by direct reclaim while looking for memory to free. Comparing this against pgsteal_direct shows how much of the searching paid off: a large gap means the thread spent time walking page lists and found little it was allowed to free.",
      desired: "lower",
    },
    numa_interleave: {
      readableName: "NUMA Interleave",
      description:
        "Memory allocations using NUMA interleave policy to distribute pages across multiple NUMA nodes for balanced memory access.",
      desired: "depends",
      unit: "Count",
    },
    pgscan_anon: {
      readableName: "Page Scan Anonymous",
      description: "Anonymous memory pages scanned during memory reclamation for potential swapping or freeing.",
      desired: "lower",
    },
    thp_split_page: {
      readableName: "THP Split Page",
      description:
        "Transparent Huge Pages that have been split into smaller pages due to memory management requirements.",
      desired: "lower",
    },
    pgreuse: {
      readableName: "Page Reuse",
      description:
        "Memory pages that were reused instead of being freed and reallocated improving memory management efficiency.",
      desired: "higher",
    },
    numa_pages_migrated: {
      readableName: "NUMA Pages Migrated",
      description:
        "Memory pages successfully migrated between NUMA nodes for better memory locality and performance optimization.",
      desired: "lower",
    },
    kswapd_inodesteal: {
      readableName: "Kswapd Inode Steal",
      description: "Inodes reclaimed by the kswapd kernel thread during memory pressure to free up kernel memory.",
      desired: "lower",
      unit: "Count",
    },
    nr_shmem: {
      readableName: "Shared Memory Pages",
      description:
        "Memory pages used for shared memory segments including System V shared memory and tmpfs filesystems.",
      desired: "moderate",
    },
    nr_vmscan_write: {
      readableName: "VM Scan Write",
      description: "Memory pages written to storage during virtual memory scanning and reclamation processes.",
      desired: "lower",
    },
    nr_active_file: {
      readableName: "Active File Pages Count",
      description:
        "Number of file-backed memory pages in the active LRU list that are frequently accessed and less likely to be reclaimed.",
      desired: "moderate",
    },
    nr_inactive_anon: {
      readableName: "Inactive Anonymous Pages Count",
      description:
        "Number of anonymous memory pages in the inactive LRU list that are candidates for swapping to storage.",
      desired: "moderate",
    },
    nr_zone_inactive_file: {
      readableName: "Zone Inactive File Pages",
      description: "Number of file-backed memory pages in zone inactive lists that are candidates for reclamation.",
      desired: "moderate",
    },
    pgscan_file: {
      readableName: "Page Scan File",
      description: "File-backed memory pages scanned during memory reclamation for potential freeing or swapping.",
      desired: "lower",
    },
    nr_zone_inactive_anon: {
      readableName: "Zone Inactive Anonymous Pages",
      description: "Number of anonymous memory pages in zone inactive lists that are candidates for swapping.",
      desired: "moderate",
    },
    slabs_scanned: {
      readableName: "Slabs Scanned",
      description: "Number of kernel slab objects scanned during memory reclamation to free unused kernel memory.",
      desired: "lower",
      unit: "Count",
    },
    compact_daemon_free_scanned: {
      readableName: "Compaction Daemon Free Scanned",
      description:
        "Number of free pages scanned by the compaction daemon to find suitable pages for memory compaction.",
      desired: "lower",
    },
    thp_collapse_alloc_failed: {
      readableName: "THP Collapse Alloc Failed",
      description:
        "Number of failed transparent huge page collapse allocations indicating memory pressure or fragmentation.",
      desired: "lower",
    },
    workingset_refault_anon: {
      readableName: "Workingset Refault Anon",
      description:
        "Number of anonymous page refaults from the working set indicating memory pressure and page reclaim activity.",
      desired: "lower",
    },
    pgpgout: {
      readableName: "Pages Paged Out",
      description: "Number of pages written to storage devices indicating memory pressure and swap activity.",
      desired: "lower",
    },
    nr_writeback_temp: {
      readableName: "Writeback Temp",
      description:
        "Number of pages held in temporary buffers while being written back, which is used by filesystems such as FUSE. It reads zero unless such a filesystem is in use.",
      desired: "lower",
    },
    thp_file_fallback_charge: {
      readableName: "File Fallback Charge",
      description: "Number of transparent huge page file fallback charges when THP allocation fails.",
      desired: "lower",
    },
    compact_free_scanned: {
      readableName: "Compact Free Scanned",
      description: "Number of free pages scanned during memory compaction to reduce fragmentation.",
      desired: "lower",
    },
    kswapd_high_wmark_hit_quickly: {
      readableName: "Kswapd High Wmark Hit Quickly",
      description: "Number of times kswapd quickly reached high watermark indicating efficient memory reclaim.",
      desired: "lower",
    },
    pgsteal_anon: {
      readableName: "Steal Anon",
      description: "Number of anonymous pages reclaimed from memory during memory pressure.",
      desired: "lower",
    },
    thp_split_pmd: {
      readableName: "Split Pmd",
      description: "Number of transparent huge page PMD splits breaking large pages into smaller ones.",
      desired: "lower",
    },
    thp_fault_alloc: {
      readableName: "Fault Alloc",
      description:
        "Number of PMD-sized transparent huge pages allocated to satisfy a page fault. This counts only the PMD size, so allocations of the smaller multi-size huge pages do not appear here at all and are reported by the per-size thp_fault_alloc metrics instead.",
      desired: "lower",
    },
    pgsteal_direct: {
      readableName: "Steal Direct",
      description:
        "Number of pages freed by direct reclaim, where the thread that asked for memory had to free some itself before its allocation could proceed. Unlike the reclaim kswapd does in the background, this stops the application while it runs.",
      desired: "lower",
    },
    allocstall_dma: {
      readableName: "Allocstall Dma",
      description:
        "Number of times an allocation from the DMA zone had to stop and reclaim memory itself. This zone is small and only used by devices that can address a limited range, so stalls here are rare.",
      desired: "lower",
      unit: "Count",
    },
    pgmajfault: {
      readableName: "Major Page Faults",
      description:
        "Page faults that had to read from storage before the access could complete, so the thread waited on the disk. The common cause is simply touching a file-backed page for the first time, such as a page of an executable, rather than memory pressure.",
      desired: "lower",
      helpfulLinks: ["https://www.kernel.org/doc/html/latest/admin-guide/mm/concepts.html#page-faults"],
    },
    pgminorfault: {
      readableName: "Minor Page Faults",
      description:
        "Page faults resolved without disk I/O where the page is already present in memory. Derived as total page faults minus major page faults.",
      desired: "lower",
      helpfulLinks: ["https://www.kernel.org/doc/html/latest/admin-guide/mm/concepts.html#page-faults"],
    },
    compact_migrate_scanned: {
      readableName: "Compact Migrate Scanned",
      description: "Number of pages scanned for migration during memory compaction to reduce fragmentation.",
      desired: "lower",
    },
    numa_miss: {
      readableName: "Numa Miss",
      description:
        "Number of allocations that could not be satisfied on the node they preferred and fell back to another node, meaning the preferred node had run out. This does not measure remote memory access: a process pinned to one node while allocating from another produces no numa_miss at all. numa_other is the metric that counts pages allocated from a different node than the requesting CPU.",
      desired: "lower",
      unit: "Count",
    },
    numa_huge_pte_updates: {
      readableName: "Numa Huge Pte Updates",
      description:
        "The amount of transparent huge pages that were marked for NUMA hinting faults. In combination with numa_pte_updates the total address space that was marked can be calculated.",
      desired: "lower",
    },
    nr_dirty: {
      readableName: "Dirty",
      description:
        "Number of pages holding data that an application has written but that the kernel has not yet sent to storage.",
      desired: "depends",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    compact_isolated: {
      readableName: "Compact Isolated",
      description: "Number of pages isolated during memory compaction for migration.",
      desired: "lower",
    },
    nr_zone_unevictable: {
      readableName: "Zone Unevictable",
      description: "Number of unevictable pages in memory zones that cannot be reclaimed.",
      desired: "lower",
    },
    pgalloc_movable: {
      readableName: "Alloc Movable",
      description: "Number of page allocations from movable memory zone for migration and compaction.",
      desired: "lower",
    },
    unevictable_pgs_rescued: {
      readableName: "Unevictable Pgs Rescued",
      description: "Number of pages rescued from unevictable list and made available for reclaim.",
      desired: "lower",
    },
    compact_success: {
      readableName: "Compact Success",
      description:
        "Number of compaction attempts that a process stopped to perform and that produced the contiguous block it needed. Compared against compact_fail, this shows whether the time the workload spent compacting was worth anything.",
      desired: "depends",
      unit: "Count",
      optimization: [MEMORY_COMPACTION_INVESTIGATION],
    },
    swap_ra: {
      readableName: "Swap Ra",
      description: "Number of swap readahead operations to optimize swap performance.",
      desired: "lower",
      unit: "Count",
    },
    nr_kernel_stack: {
      readableName: "Kernel Stack",
      description: "Number of pages used for kernel stack memory indicating kernel thread activity.",
      desired: "lower",
    },
    pgskip_dma: {
      readableName: "Skip Dma",
      description: "Number of pages skipped during scanning in DMA zone due to being unsuitable for reclaim.",
      desired: "lower",
    },
    pgmigrate_fail: {
      readableName: "Migrate Fail",
      description: "Number of failed page migrations during memory compaction or NUMA balancing.",
      desired: "lower",
    },
    unevictable_pgs_scanned: {
      readableName: "Unevictable Pgs Scanned",
      description: "Number of unevictable pages scanned during memory reclaim attempts.",
      desired: "lower",
    },
    balloon_migrate: {
      readableName: "Balloon Migrate",
      description: "Number of pages migrated during memory balloon operations in virtualized environments.",
      desired: "lower",
    },
    pgrefill: {
      readableName: "Refill",
      description: "Number of pages moved from active to inactive list during memory reclaim.",
      desired: "lower",
    },
    nr_active_anon: {
      readableName: "Active Anon",
      description: "Number of active anonymous pages in memory that are frequently accessed.",
      desired: "lower",
    },
    workingset_restore_file: {
      readableName: "Workingset Restore File",
      description: "Number of file pages restored to the working set after being reclaimed.",
      desired: "lower",
    },
    pageoutrun: {
      readableName: "Pageoutrun",
      description: "Number of times kswapd ran to reclaim memory indicating memory pressure.",
      desired: "lower",
    },
    nr_mlock: {
      readableName: "Mlock",
      description: "Number of pages locked in memory that cannot be swapped out.",
      desired: "lower",
    },
    workingset_nodereclaim: {
      readableName: "Workingset Nodereclaim",
      description: "Number of times a shadow node has been reclaimed.",
      desired: "lower",
      unit: "Count",
    },
    nr_foll_pin_acquired: {
      readableName: "Foll Pin Acquired",
      description: "Number of pages pinned for follow operations preventing them from being moved.",
      desired: "lower",
    },
    nr_written: {
      readableName: "Written",
      description:
        "Number of pages the kernel wrote out to storage. Comparing this against nr_dirtied shows whether writeback is keeping up with the rate at which the workload is dirtying pages.",
      desired: "depends",
    },
    unevictable_pgs_culled: {
      readableName: "Unevictable Pgs Culled",
      description: "Number of pages removed from unevictable list when they become evictable.",
      desired: "lower",
    },
    pgrotated: {
      readableName: "Rotated",
      description: "Number of pages rotated to the tail of the LRU list during reclaim.",
      desired: "lower",
    },
    workingset_refault_file: {
      readableName: "Workingset Refault File",
      description:
        "Number of file pages that had to be read back after the kernel had evicted them. A large value means the page cache was too small to hold the working set, so the same data is being read from storage repeatedly.",
      desired: "lower",
    },
    workingset_restore_anon: {
      readableName: "Workingset Restore Anon",
      description: "Number of anonymous pages restored to the working set after being reclaimed.",
      desired: "lower",
    },
    nr_zone_active_anon: {
      readableName: "Zone Active Anon",
      description: "Number of active anonymous pages in each memory zone.",
      desired: "lower",
    },
    pgscan_kswapd: {
      readableName: "Scan Kswapd",
      description:
        "Number of pages the background kswapd thread examined while looking for memory to free. Comparing this against pgsteal_kswapd shows how much of the scanning found pages it could actually reclaim.",
      desired: "lower",
    },
    pgsteal_file: {
      readableName: "Steal File",
      description: "Number of file pages reclaimed from memory during memory pressure.",
      desired: "lower",
    },
    allocstall_normal: {
      readableName: "Allocstall Normal",
      description:
        "Number of times an allocation from the Normal zone had to stop and reclaim memory itself before it could be satisfied. Each stall is a pause in the thread that was allocating.",
      desired: "lower",
      unit: "Count",
    },
    nr_unevictable: {
      readableName: "Unevictable",
      description: "Number of pages that cannot be evicted from memory.",
      desired: "lower",
    },
    balloon_deflate: {
      readableName: "Balloon Deflate",
      description: "Number of pages returned from memory balloon operations in virtualized environments.",
      desired: "lower",
    },
    nr_zone_active_file: {
      readableName: "Zone Active File",
      description: "Number of active file pages in each memory zone.",
      desired: "lower",
    },
    thp_file_alloc: {
      readableName: "File Alloc",
      description: "Number of transparent huge pages allocated for file mappings.",
      desired: "lower",
    },
    pgskip_normal: {
      readableName: "Skip Normal",
      description: "Number of pages skipped during scanning in normal zone due to being unsuitable for reclaim.",
      desired: "lower",
    },
    numa_other: {
      readableName: "NUMA Other Node Allocations",
      description: "Number of pages allocated from a NUMA node by a CPU located on a different NUMA node.",
      desired: "lower",
    },
    drop_slab: {
      readableName: "Slab Cache Drop",
      description: "Number of slab cache objects dropped to free kernel memory during memory pressure.",
      desired: "lower",
      unit: "Count",
    },
    nr_isolated_anon: {
      readableName: "Isolated Anonymous Pages",
      description:
        "Number of anonymous memory pages currently isolated for migration or other memory management operations.",
      desired: "lower",
    },
    swap_ra_hit: {
      readableName: "Swap Readahead Hit",
      description: "Number of successful swap readahead operations that found the requested pages already in memory.",
      desired: "higher",
    },
    numa_pte_updates: {
      readableName: "NUMA PTE Updates",
      description: "The amount of base pages that were marked for NUMA hinting faults.",
      desired: "lower",
    },
    nr_unstable: {
      readableName: "Unstable Pages",
      description:
        "Number of pages written to an NFS server but not yet committed to stable storage. Current kernels no longer track this separately and count those pages in nr_writeback instead, so it reads zero.",
      desired: "lower",
    },
    thp_fault_fallback_charge: {
      readableName: "THP Fault Fallback Charge",
      description:
        "Transparent Huge Page allocation failures during fault handling that resulted in memory charge fallback to regular pages.",
      desired: "lower",
    },
    numa_hint_faults: {
      readableName: "NUMA Hint Faults",
      description: "Number of NUMA hinting faults were trapped.",
      desired: "depends",
      unit: "Count",
    },
    thp_migration_fail: {
      readableName: "THP Migration Fail",
      description: "Transparent Huge Page migration failures during memory compaction or NUMA balancing operations.",
      desired: "lower",
    },
    balloon_inflate: {
      readableName: "Balloon Inflate",
      description:
        "Memory pages reclaimed by balloon driver in virtualized environments to return memory to the hypervisor.",
      desired: "depends",
    },
    compact_daemon_migrate_scanned: {
      readableName: "Compact Daemon Migrate Scanned",
      description: "Pages scanned by the kernel compaction daemon during migration to reduce memory fragmentation.",
      desired: "lower",
    },
    nr_slab_unreclaimable: {
      readableName: "Slab Unreclaimable Pages",
      description: "Number of kernel slab memory pages that cannot be reclaimed and remain permanently allocated.",
      desired: "lower",
    },
    pgalloc_normal: {
      readableName: "Page Allocations Normal",
      description: "Number of memory page allocations from the normal memory zone for regular system operations.",
      desired: "depends",
    },
    thp_swpout_fallback: {
      readableName: "THP Swapout Fallback",
      description:
        "Transparent Huge Page swapout operations that failed and fell back to swapping individual 4KB pages.",
      desired: "lower",
    },
    pginodesteal: {
      readableName: "Page Inode Steal",
      description:
        "Memory pages reclaimed by stealing from inode caches during memory pressure to free up system memory.",
      desired: "lower",
    },
    thp_migration_split: {
      readableName: "THP Migration Split",
      description: "Transparent Huge Pages that were split into smaller pages during migration operations.",
      desired: "lower",
    },
    numa_local: {
      readableName: "NUMA Local Allocations",
      description: "Memory allocations that were successfully allocated on the local NUMA node.",
      desired: "higher",
    },
    nr_foll_pin_released: {
      readableName: "Follow Pin Released",
      description: "Number of pages unpinned after follow operations allowing them to be moved or reclaimed.",
      desired: "higher",
    },
    nr_free_pages: {
      readableName: "Free Pages Count",
      description:
        "Number of free memory pages currently available for allocation without requiring memory reclamation.",
      desired: "higher",
    },
    workingset_activate_anon: {
      readableName: "Working Set Activate Anonymous",
      description:
        "Anonymous pages activated from inactive to active LRU list based on access patterns for better memory management.",
      desired: "moderate",
    },
    drop_pagecache: {
      readableName: "Page Cache Drop",
      description:
        "Number of page cache entries dropped to free memory during memory pressure or administrative action.",
      desired: "lower",
    },
    pgscan_direct_throttle: {
      readableName: "Page Scan Direct Throttle",
      description: "Direct memory reclaim operations throttled to prevent excessive CPU usage during memory pressure.",
      desired: "lower",
    },
    thp_file_fallback: {
      readableName: "THP File Fallback",
      description: "File-backed Transparent Huge Page allocations that failed and fell back to regular 4KB pages.",
      desired: "lower",
    },
    thp_zero_page_alloc: {
      readableName: "THP Zero Page Allocation",
      description: "Transparent Huge Page allocations for zero-filled pages to optimize memory initialization.",
      desired: "higher",
    },
    nr_page_table_pages: {
      readableName: "Page Table Pages Count",
      description: "Number of memory pages used for page table structures in the virtual memory management system.",
      desired: "lower",
    },
    pgalloc_dma: {
      readableName: "Page Allocations DMA",
      description:
        "Number of memory page allocations from the DMA memory zone for devices requiring low memory addresses.",
      desired: "depends",
    },
    nr_anon_transparent_hugepages: {
      readableName: "Anonymous Transparent Hugepages Count",
      description:
        "Number of PMD-sized transparent huge pages currently backing anonymous memory. This counts only the PMD size, not the smaller multi-size huge pages, and is the same quantity meminfo reports as AnonHugePages.",
      desired: "higher",
    },
    pgskip_movable: {
      readableName: "Page Skip Movable",
      description: "Memory pages skipped during scanning because they are in movable memory zones.",
      desired: "lower",
    },
    numa_hint_faults_local: {
      readableName: "NUMA Hint Faults Local",
      description:
        "Shows how many of the hinting faults were to local nodes. In combination with numa_hint_faults, the percentage of local versus remote faults can be calculated. A high percentage of local hinting faults indicates that the workload is closer to being converged.",
      desired: "higher",
      unit: "Count",
    },
    nr_dirtied: {
      readableName: "Pages Dirtied",
      description:
        "Number of pages that were newly marked dirty because the workload wrote to them, which shows how fast it is producing data that the kernel has to write out.",
      desired: "depends",
    },
    pgfault: {
      readableName: "Page Faults",
      description:
        "Total page faults including both minor faults (memory already in RAM) and major faults (requiring disk I/O).",
      desired: "lower",
      unit: "Count",
    },
    nr_isolated_file: {
      readableName: "Isolated File Pages",
      description:
        "Number of file-backed memory pages currently isolated for migration or other memory management operations.",
      desired: "lower",
    },
    unevictable_pgs_cleared: {
      readableName: "Unevictable Pages Cleared",
      description: "Number of pages removed from the unevictable list when they became evictable again.",
      desired: "higher",
    },
    pswpout: {
      readableName: "Pages Swapped Out",
      description:
        "Number of pages of application memory written out to swap. The kernel only resorts to this once dropping page cache is not enough, so any value means memory was genuinely short, unless the system uses compressed swap such as zram or zswap by design.",
      desired: "lower",
    },
    pglazyfreed: {
      readableName: "Pages Lazy Freed",
      description:
        "Number of pages freed using lazy freeing mechanism to defer actual memory deallocation for performance optimization.",
      desired: "higher",
    },
    compact_daemon_wake: {
      readableName: "Compaction Daemon Wake",
      description: "Number of times the memory compaction daemon was awakened to reduce memory fragmentation.",
      desired: "lower",
      unit: "Count",
    },
    zone_reclaim_failed: {
      readableName: "Zone Reclaim Failed",
      description:
        "Number of time the kernel attempts to reclaim memory from a local NUMA zone but cannot free up enough pages, forcing it to look elsewhere.",
      desired: "lower",
      unit: "Count",
    },
    nr_file_pages: {
      readableName: "File Pages Count",
      description:
        "Number of memory pages used for file-backed mappings including cached files and memory-mapped files.",
      desired: "depends",
    },
    unevictable_pgs_stranded: {
      readableName: "Unevictable Pages Stranded",
      description: "Memory pages that cannot be evicted and are stranded in the unevictable LRU list.",
      desired: "lower",
    },
    numa_foreign: {
      readableName: "NUMA Foreign Allocations",
      description: "Memory allocations that occurred on foreign NUMA nodes due to local memory unavailability.",
      desired: "lower",
      unit: "Count",
    },
    nr_zspages: {
      readableName: "ZSwap Pages Count",
      description: "Number of pages stored in compressed memory using zswap for memory efficiency.",
      desired: "depends",
    },
    pgmigrate_success: {
      readableName: "Page Migration Success",
      description: "Number of successfull page migrations during memory compaction or NUMA balancing operations.",
      desired: "higher",
    },
    nr_bounce: {
      readableName: "Bounce Buffer Pages",
      description: "Number of pages used for bounce buffers when DMA cannot access high memory directly.",
      desired: "lower",
    },
    compact_fail: {
      readableName: "Memory Compaction Failures",
      description:
        "Number of compaction attempts that a process stopped to perform and that did not produce the contiguous block it needed, so the time spent was wasted. This counts only the compaction a process runs itself, not the work done in the background by kcompactd.",
      desired: "lower",
      unit: "Count",
      optimization: [MEMORY_COMPACTION_INVESTIGATION],
    },
    unevictable_pgs_mlocked: {
      readableName: "Unevictable Pages Memory Locked",
      description: "Memory pages that are locked in memory and cannot be swapped out or evicted.",
      desired: "depends",
    },
    pgskip_dma32: {
      readableName: "Page Skip DMA32",
      description: "Memory pages skipped during scanning because they are in the DMA32 memory zone.",
      desired: "lower",
    },
    allocstall_dma32: {
      readableName: "Allocation Stall DMA32",
      description:
        "Number of times an allocation from the DMA32 zone had to stop and reclaim memory itself. This zone serves devices limited to 32-bit addressing and does not exist on every architecture.",
      desired: "lower",
      unit: "Count",
    },
    pgactivate: {
      readableName: "Page Activations",
      description: "Memory pages moved from inactive to active LRU lists due to recent access patterns.",
      desired: "depends",
    },
    nr_writeback: {
      readableName: "Writeback Pages",
      description:
        "Number of pages that have been handed to the storage device and whose write has not yet completed. Pages leave nr_dirty and are counted here once writeback starts.",
      desired: "lower",
    },
    numa_hit: {
      readableName: "NUMA Hit",
      description:
        "Number of memory allocations that were successfully satisfied on the intended NUMA node providing optimal memory locality.",
      desired: "higher",
      unit: "Count",
    },
    pswpin: {
      readableName: "Pages Swapped In",
      description:
        "Number of pages read back from swap because the workload touched memory that had been swapped out. The thread waits on the disk read, in the same way a major page fault does, so this is where the cost of earlier swapping is actually paid.",
      desired: "lower",
    },
    allocstall_movable: {
      readableName: "Allocation Stall Movable",
      description:
        "Number of times an allocation from the Movable zone had to stop and reclaim memory itself. This zone holds only pages the kernel can migrate, and is empty unless it was configured at boot.",
      desired: "lower",
    },
    pglazyfree: {
      readableName: "Pages Lazy Free",
      description:
        "Number of pages marked for lazy freeing to defer actual memory deallocation for performance optimization.",
      desired: "higher",
    },
    pgalloc_dma32: {
      readableName: "Page Allocations DMA32",
      description: "Number of memory page allocations from the DMA32 memory zone for 32-bit device compatibility.",
      desired: "depends",
    },
    nr_kernel_misc_reclaimable: {
      readableName: "Kernel Miscellaneous Reclaimable",
      description: "Number of kernel miscellaneous memory pages that can be reclaimed during memory pressure.",
      desired: "moderate",
    },
    nr_shmem_hugepages: {
      readableName: "Shared Memory Hugepages",
      description: "Number of shared memory pages allocated as huge pages for improved performance.",
      desired: "higher",
    },
    kswapd_low_wmark_hit_quickly: {
      readableName: "Kswapd Low Watermark Hit Quickly",
      description: "Number of memory pages used for memory mapping structures during system boot.",
      desired: "lower",
    },
    unevictable_pgs_munlocked: {
      readableName: "Unevictable Pages Memory Unlocked",
      description: "Memory pages that were unlocked from the unevictable LRU list and made available for reclaim.",
      desired: "higher",
    },
    thp_split_pud: {
      readableName: "THP Split PUD",
      description: "Transparent Huge Pages split at the Page Upper Directory level during memory management.",
      desired: "lower",
    },
    nr_file_hugepages: {
      readableName: "File Huge Pages Count",
      description:
        "Number of file-backed memory pages using huge page sizes for improved I/O performance and reduced TLB pressure.",
      desired: "depends",
    },
    thp_migration_success: {
      readableName: "THP Migration Success",
      description:
        "Number of transparent huge pages successfully migrated between memory locations for NUMA optimization.",
      desired: "higher",
    },
    thp_swpout: {
      readableName: "THP Swap Out",
      description:
        "Number of transparent huge pages swapped out to storage indicating memory pressure on large page allocations.",
      desired: "lower",
    },
    nr_shmem_pmdmapped: {
      readableName: "Shared Memory PMD Mapped",
      description:
        "Number of shared memory pages mapped using Page Middle Directory entries for large page optimizations.",
      desired: "lower",
    },
    pgsteal_khugepaged: {
      readableName: "Page Steal Khugepaged",
      description:
        "Number of pages freed by khugepaged while it was collapsing regular pages into huge pages, which it does in the background rather than in the application.",
      desired: "lower",
    },
    nr_throttled_written: {
      readableName: "Throttled Written Pages",
      description: "Number of pages whose write operations were throttled to prevent overwhelming storage devices.",
      desired: "lower",
    },
    zswpin: {
      readableName: "ZSwap Pages In",
      description: "Pages swapped in from compressed memory (zswap) back to regular memory.",
      desired: "lower",
    },
    thp_scan_exceed_share_pte: {
      readableName: "THP Scan Exceed Share PTE",
      description: "Transparent Huge Page scans that exceeded shared page table entry limits.",
      desired: "lower",
    },
    pgalloc_device: {
      readableName: "Page Allocations Device",
      description: "Number of memory page allocations from device memory zones for specialized hardware.",
      desired: "depends",
    },
    nr_unaccepted: {
      readableName: "Unaccepted Memory Pages",
      description: "Number of memory pages that have not been accepted by the guest OS in virtualized environments.",
      desired: "lower",
    },
    ksm_swpin_copy: {
      readableName: "KSM Swap In Copy",
      description: "Kernel Same-page Merging pages copied during swap-in operations to maintain memory deduplication.",
      desired: "lower",
    },
    allocstall_device: {
      readableName: "Allocation Stall Device",
      description: "Memory allocation stalls in device memory zones due to resource constraints.",
      desired: "lower",
      unit: "Count",
    },
    direct_map_level2_splits: {
      readableName: "Direct Map Level 2 Splits",
      description: "Page table splits at level 2 of direct memory mapping for large page management.",
      desired: "lower",
    },
    zswpwb: {
      readableName: "ZSwap Writeback",
      description: "Pages written back from compressed memory (zswap) to storage during memory pressure.",
      desired: "lower",
    },
    pgskip_device: {
      readableName: "Page Skip Device",
      description: "Memory pages skipped during scanning because they are in device memory zones.",
      desired: "lower",
    },
    pgdemote_kswapd: {
      readableName: "Page Demote Kswapd",
      description:
        "Number of pages the background kswapd thread moved to a slower memory tier, such as a CXL or persistent memory node, instead of reclaiming them outright. This only happens on systems configured with tiered memory.",
      desired: "lower",
    },
    pgdemote_khugepaged: {
      readableName: "Page Demote Khugepaged",
      description:
        "Number of pages khugepaged moved to a slower memory tier while collapsing regular pages into huge pages. This only applies to systems configured with tiered memory.",
      desired: "lower",
    },
    direct_map_level3_splits: {
      readableName: "Direct Map Level 3 Splits",
      description: "Page table splits at level 3 of direct memory mapping for very large page management.",
      desired: "lower",
      unit: "Count",
    },
    pgscan_khugepaged: {
      readableName: "Page Scan Khugepaged",
      description: "Memory pages scanned by the kernel huge page daemon for consolidation opportunities.",
      desired: "lower",
    },
    pgpromote_success: {
      readableName: "Page Promote Success",
      description: "Successful promotions of memory pages to faster storage tiers for improved performance.",
      desired: "higher",
    },
    zswpout: {
      readableName: "ZSwap Pages Out",
      description: "Pages swapped out from regular memory to compressed memory (zswap) to save space.",
      desired: "depends",
    },
    thp_scan_exceed_none_pte: {
      readableName: "THP Scan Exceed None PTE",
      description: "Transparent Huge Page scans that exceeded limits with no page table entries found.",
      desired: "lower",
    },
    nr_sec_page_table_pages: {
      readableName: "Secondary Page Table Pages",
      description: "Number of secondary page table pages used for virtualization and memory management.",
      desired: "lower",
      unit: "Count",
    },
    cow_ksm: {
      readableName: "Copy-on-Write KSM",
      description: "Copy-on-write operations on Kernel Same-page Merging pages when shared pages are modified.",
      desired: "lower",
      unit: "Count",
    },
    nr_swapcached: {
      readableName: "Swap Cached Pages",
      description: "Number of pages cached in memory that are also present in swap space for faster access.",
      desired: "depends",
    },
    pgpromote_candidate: {
      readableName: "Page Promote Candidate",
      description: "Memory pages identified as candidates for promotion to faster storage tiers.",
      desired: "higher",
    },
    pgdemote_direct: {
      readableName: "Page Demote Direct",
      description:
        "Number of pages moved to a slower memory tier by the allocating thread itself rather than by kswapd, so the application waited while it happened. This only applies to systems configured with tiered memory.",
      desired: "lower",
    },
    thp_scan_exceed_swap_pte: {
      readableName: "THP Scan Exceed Swap PTE",
      description: "Transparent Huge Page scans that exceeded limits while examining swap page table entries.",
      desired: "lower",
    },
    cma_alloc_fail: {
      readableName: "CMA Allocation Failures",
      description:
        "Failed allocations from Contiguous Memory Allocator for devices requiring large contiguous memory blocks.",
      desired: "lower",
      unit: "Count",
    },
    cma_alloc_success: {
      readableName: "CMA Allocation Success",
      description: "Successful allocations from Contiguous Memory Allocator for device memory requirements.",
      desired: "higher",
      unit: "Count",
    },
    nr_shadow_call_stack: {
      readableName: "Shadow Call Stack Pages",
      description: "Number of memory pages used for shadow call stack security feature on ARM processors.",
      desired: "depends",
    },
    nr_memmap_boot_pages: {
      readableName: "Memory Map Boot Pages",
      description: "Number of memory pages used for memory mapping structures during system boot.",
      desired: "lower",
    },
    nr_hugetlb: {
      readableName: "Huge TLB Pages Count",
      description: "Number of huge pages allocated for applications requiring large memory pages.",
      desired: "depends",
    },
    nr_iommu_pages: {
      readableName: "IOMMU Pages Count",
      description: "Number of memory pages used by Input-Output Memory Management Unit for device memory translation.",
      desired: "lower",
    },
    nr_memmap_pages: {
      readableName: "Memory Map Pages Count",
      description: "Number of memory pages used for memory mapping data structures in the kernel.",
      desired: "lower",
    },
    swpout_zero: {
      readableName: "Swap Out Zero Pages",
      description: "Zero-filled pages swapped out to storage without actual data transfer for optimization.",
      desired: "higher",
    },
    swpin_zero: {
      readableName: "Swap In Zero Pages",
      description: "Zero-filled pages swapped in from storage with optimized handling for empty pages.",
      desired: "higher",
    },
    zone_reclaim_success: {
      readableName: "Zone Reclaim Success",
      description:
        "Successful memory reclamation operations from specific NUMA zones during allocation (should be evaluated in conjunction with the system's vm.zone_reclaim_mode setting).",
      desired: "higher",
      unit: "Count",
    },
    thp_underused_split_page: {
      readableName: "THP Underused Split Page",
      description: "Transparent Huge Pages split because they were underutilized to free up memory.",
      desired: "lower",
    },
    ...perSizeThpStatsFieldDescriptions(),
  },
};
