import { DataType } from "./types";

type DesiredValue = "higher" | "lower" | "moderate" | "fixed" | "depends";

interface DataDescription {
  readonly readableName: string;
  readonly summary: string;
  readonly defaultUnit?: string;
  readonly fieldDescriptions: {
    [key in string]: {
      readonly readableName: string;
      readonly description: string;
      readonly unit?: string;
      readonly desired?: DesiredValue;
    };
  };
  readonly helpfulLinks?: string[];
}

export const DATA_DESCRIPTIONS: { [key in DataType]: DataDescription } = {
  systeminfo: {
    readableName: "Report Home",
    summary:
      "The APerf report homepage provides overviews of each recording run. In this page, you can view every run's system information, analytical findings, and statistical findings. For more details, use the side navigation panel to open a specific data's page.",
    fieldDescriptions: {
      statisticalFinding: {
        readableName: "Statistical Findings",
        description:
          "A statistical finding represents the delta of comparing a time-series metric's stat against the same metric in the base run. The deltas are color-coded based on the desired value of the metric - green means as desired (good) and red means otherwise (bad). Use the filters to select data types, stats, and finding types to be included in the table.",
      },
      analyticalFinding: {
        readableName: "Analytical Findings",
        description:
          "An analytical finding is produced when matching a predefined analytical rule against the data during report generation. They describe how the data in-scope is potentially impacting performance.",
      },
    },
  },
  cpu_utilization: {
    readableName: "CPU Utilization",
    summary:
      "CPU utilization metrics measure the percentage of CPU time spent in various CPU state. The data were collected and computed from the system pseudo-file /proc/stat. Every metric graph shows the percentage of time spent in the corresponding state for each CPU, as well as the aggregate of all CPUs. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for its aggregate series.",
    defaultUnit: "Utilization (%)",
    fieldDescriptions: {
      aggregate: {
        readableName: "Total CPU Utilization",
        description: "Percentage of CPU time spent on all activities (across all CPUs for each type).",
        desired: "higher",
      },
      idle: {
        readableName: "CPU Idle Time",
        description: "Percentage of CPU time spent idle.",
        desired: "lower",
      },
      iowait: {
        readableName: "CPU I/O Wait Time",
        description: "Percentage of CPU time spent waiting for I/O operations to complete.",
        desired: "lower",
      },
      irq: {
        readableName: "Hardware Interrupt Time",
        description: "Percentage of CPU time spent servicing hardware interrupts.",
        desired: "lower",
      },
      nice: {
        readableName: "Nice Process Time",
        description: "Percentage of CPU time spent on low-priority (nice) user processes.",
        desired: "lower",
      },
      softirq: {
        readableName: "Software Interrupt Time",
        description: "Percentage of CPU time spent servicing software interrupts.",
        desired: "lower",
      },
      steal: {
        readableName: "CPU Steal Time",
        description: "Percentage of CPU time stolen by hypervisor for other tasks.",
        desired: "lower",
      },
      system: {
        readableName: "System CPU Time",
        description: "Percentage of CPU time spent in kernel mode.",
        desired: "lower",
      },
      user: {
        readableName: "User CPU Time",
        description: "Percentage of CPU time spent in user mode.",
        desired: "higher",
      },
    },
  },
  processes: {
    readableName: "Processes",
    summary:
      "Processes metrics monitor usage of various resources for processes running on the system during APerf collection. The data were collected and computed from the system pseudo-files /proc/<pid>/stat. Every metric graph contains the top 16 processes in the highest average usage of the corresponding resource. The stats of a metric graph accounts for the process with the highest average.",
    defaultUnit: "Count",
    fieldDescriptions: {
      user_space_time: {
        readableName: "User Space Time (utime)",
        description: "The aggregate percent CPU time spent executing application code for each process.",
        desired: "higher",
        unit: "Utilization (%)",
      },
      kernel_space_time: {
        readableName: "Kernel Space Time (stime)",
        description: "The aggregate percent CPU time spent executing in kernel mode (system calls).",
        desired: "lower",
        unit: "Utilization (%)",
      },
      number_threads: {
        readableName: "Number of Threads (num_threads)",
        description: "Current number of threads in the process.",
        desired: "depends",
      },
      virtual_memory_size: {
        readableName: "Virtual Memory Size (vsize)",
        description: "Total virtual memory used by the process in Bytes.",
        desired: "lower",
        unit: "Bytes",
      },
      resident_set_size: {
        readableName: "Resident Set Size (rss)",
        description:
          "Physical memory currently used by the process in pages, multiply by page size to convert to bytes.",
        desired: "lower",
        unit: "Pages",
      },
    },
  },
  perf_stat: {
    readableName: "PMU Events",
    summary:
      "PMU metrics collect and compute the PMU (Performance Monitoring Unit) counters, which track hardware-level events, across all CPUs. Every graph corresponds to a metric computed using one or more PMU counters for every CPU, as well as the aggregate (average) of all CPUs. The statistics of a metric graph accounts for its aggregate series.",
    defaultUnit: "Counts",
    fieldDescriptions: {
      "data-tlb-mpki": {
        readableName: "Data TLB Misses Per Kilo Instructions",
        description:
          "Translation Lookaside Buffer misses for data accesses per thousand instructions, indicating additional latency required for memory operations.",
        desired: "lower",
      },
      "data-tlb-tw-pki": {
        readableName: "Data TLB Table Walk Per Kilo Instructions",
        description:
          "Translation Lookaside Buffer table walks for data accesses per thousand instructions, indicates much higher latency for some memory accesses.",
        desired: "lower",
      },
      "l3-mpki": {
        readableName: "L3 Cache Misses Per Kilo Instructions",
        description:
          "Level 3 cache misses per thousand instructions executed, indicating how often the CPU has to access memory beyond the L3.",
        desired: "lower",
      },
      "branch-mpki": {
        readableName: "Branch Misses Per Kilo Instructions",
        description:
          "Number of branch prediction misses per thousand instructions indicating CPU pipeline efficiency and code predictability.",
        desired: "lower",
      },
      "inst-tlb-tw-pki": {
        readableName: "Instruction TLB Table Walk Per Kilo Instructions",
        description:
          "Translation Lookaside Buffer table walks for instruction fetches per thousand instructions indicating code size issues and poor code locality.",
        desired: "lower",
      },
      "inst-tlb-mpki": {
        readableName: "Instruction TLB Misses Per Kilo Instructions",
        description:
          "Instruction Translation Lookaside Buffer misses per thousand instructions indicating code locality.",
        desired: "lower",
      },
      "stall-frontend-pkc": {
        readableName: "Frontend Stall Per Kilo Cycles",
        description:
          "Cycle count when frontend could not send any micro-operations to the rename stage because of frontend resource stalls caused by fetch memory latency or branch prediction flow stalls per thousand cycles.",
        desired: "lower",
      },
      "data-l1-mpki": {
        readableName: "Data L1 Cache Misses Per Kilo Instructions",
        description:
          "Level 1 data cache misses per thousand instructions indicating data access patterns and cache efficiency for frequently accessed data.",
        desired: "lower",
      },
      "data-rd-tlb-tw-pki": {
        readableName: "Data Read TLB Table Walk Per Kilo Instructions",
        description:
          "Translation Lookaside Buffer table walks for data read operations per thousand instructions indicating memory management overhead for read accesses.",
        desired: "lower",
      },
      "data-st-tlb-tw-pki": {
        readableName: "Data Store TLB Table Walk Per Kilo Instructions",
        description:
          "Translation Lookaside Buffer table walks for data store operations per thousand instructions indicating memory management overhead for write accesses.",
        desired: "lower",
      },
      "inst-l1-mpki": {
        readableName: "Instruction L1 Cache Misses Per Kilo Instructions",
        description:
          "Level 1 instruction cache misses per thousand instructions indicating instruction fetch efficiency and code locality patterns.",
        desired: "lower",
      },
      ipc: {
        readableName: "Instructions Per Cycle",
        description:
          "Average number of instructions executed per CPU clock cycle indicating overall CPU utilization efficiency and performance.",
        desired: "higher",
      },
      "data-rd-tlb-mpki": {
        readableName: "Data Read TLB Misses Per Kilo Instructions",
        description:
          "Translation Lookaside Buffer misses for data read operations per thousand instructions indicating memory access patterns for read operations.",
        desired: "lower",
      },
      "l2-mpki": {
        readableName: "L2 Cache Misses Per Kilo Instructions",
        description: "Number of level 2 cache accesses missed per thousand instructions executed.",
        desired: "lower",
      },
      "data-st-tlb-mpki": {
        readableName: "Data Store TLB Misses Per Kilo Instructions",
        description:
          "Number of Translation Lookaside Buffer misses for data store operations per thousand instructions.",
        desired: "lower",
      },
      "stall-backend-pkc": {
        readableName: "Backend Stall Per Kilo Cycles",
        description:
          "CPU backend pipeline stalls per thousand cycles caused by execution unit bottlenecks and resource constraints.",
        desired: "lower",
      },
      stall_frontend_pkc: {
        readableName: "Frontend Stall Per Kilo Cycles",
        description:
          "Cycle count when frontend could not send any micro-operations to the rename stage because of frontend resource stalls caused by fetch memory latency or branch prediction flow stalls per thousand cycles.",
        desired: "lower",
      },
      stall_backend_pkc: {
        readableName: "Backend Stall Per Kilo Cycles",
        description:
          "CPU backend pipeline stalls per thousand cycles caused by execution unit bottlenecks and resource constraints on AMD processors.",
        desired: "lower",
      },
      "inst-tlb-tw-mpki": {
        readableName: "Instruction TLB Table Walk Misses Per Kilo Instructions",
        description:
          "Number of Instruction Translation Lookaside Buffer table walk misses per thousand instructions indicating instruction fetch efficiency on AMD processors.",
        desired: "lower",
      },
      "code-sparsity": {
        readableName: "Code Sparsity",
        description:
          "Code sparsity is a measure of how compact the instruction code is packed and how closely related code is placed. Lower sparsity helps branch prediction and the cache subsystem.",
        desired: "lower",
      },
    },
  },
  meminfo: {
    readableName: "Memory Usage",
    summary:
      "Memory usage metrics measure the usage of the system's physical memory. The data were collected from the system pseudo-file /proc/meminfo.",
    defaultUnit: "Bytes",
    fieldDescriptions: {
      vmalloc_used: {
        readableName: "Virtual Memory Allocated Used",
        description:
          "Amount of virtual memory currently allocated through the vmalloc interface for kernel data structures and device drivers.",
        desired: "moderate",
      },
      cached: {
        readableName: "Page Cache Memory",
        description:
          "Memory used by the kernel to cache file system data and metadata to improve I/O performance by reducing disk access.",
        desired: "moderate",
      },
      mem_free: {
        readableName: "Free Memory",
        description:
          "Amount of physical memory currently available for allocation without requiring memory reclamation or swapping.",
        desired: "higher",
      },
      file_pmd_mapped: {
        readableName: "File PMD Mapped",
        description:
          "File-backed memory pages mapped using Page Middle Directory entries for large page optimizations in memory management.",
        desired: "lower",
      },
      direct_map_2m: {
        readableName: "Direct Map 2M Pages",
        description: "Number of bytes of RAM linearly mapped by kernel in 2 MB pages.",
        desired: "higher",
      },
      unevictable: {
        readableName: "Unevictable Memory",
        description: "Memory pages that cannot be swapped out or reclaimed including locked memory and kernel pages.",
        desired: "lower",
      },
      per_cpu: {
        readableName: "Per-CPU Memory",
        description:
          "Memory allocated on a per-CPU basis for CPU-local data structures to avoid cache line contention in multi-processor systems.",
        desired: "moderate",
      },
      anon_hugepages: {
        readableName: "Anonymous Huge Pages",
        description:
          "Anonymous memory pages using huge page sizes for applications that benefit from reduced TLB overhead and improved memory performance.",
        desired: "depends",
      },
      inactive_file: {
        readableName: "Inactive File Pages",
        description:
          "File-backed memory pages in the inactive LRU list that are candidates for reclamation when memory pressure occurs.",
        desired: "moderate",
      },
      cma_total: {
        readableName: "CMA Total Memory",
        description:
          "Total Contiguous Memory Allocator memory reserved for devices requiring physically contiguous memory blocks.",
        desired: "higher",
      },
      swap_free: {
        readableName: "Free Swap Space",
        description:
          "Available swap space that can be used when physical memory is exhausted for virtual memory management.",
        desired: "higher",
      },
      cma_free: {
        readableName: "CMA Free Memory",
        description:
          "Free pages in the Contiguous Memory Allocator pool available for devices requiring physically contiguous memory blocks.",
        desired: "higher",
      },
      committed_as: {
        readableName: "Committed Memory",
        description:
          "Total amount of memory currently allocated or reserved by processes including virtual memory that may not be physically present.",
        desired: "moderate",
      },
      inactive: {
        readableName: "Inactive Memory",
        description:
          "Total memory pages in the inactive LRU lists that are candidates for reclamation when memory pressure occurs.",
        desired: "moderate",
      },
      commit_limit: {
        readableName: "Commit Limit",
        description:
          "Maximum amount of memory that can be allocated by processes based on available physical memory and swap space.",
        desired: "higher",
      },
      s_unreclaim: {
        readableName: "Slab Unreclaimable",
        description:
          "Kernel slab memory that cannot be reclaimed and remains permanently allocated for kernel data structures.",
        desired: "lower",
      },
      mem_total: {
        readableName: "Total Memory",
        description:
          "Total amount of physical memory available to the system including memory used by kernel and applications.",
        desired: "higher",
      },
      slab: {
        readableName: "Slab Memory",
        description: "Total kernel slab memory used for caching frequently used kernel objects and data structures.",
        desired: "moderate",
      },
      direct_map_4k: {
        readableName: "Direct Map 4K Pages",
        description: "Number of bytes of RAM linearly mapped by kernel in 4 kB pages.",
        desired: "moderate",
      },
      swap_total: {
        readableName: "Total Swap Space",
        description:
          "Total amount of swap space available for virtual memory management when physical memory is exhausted.",
        desired: "higher",
      },
      shmem: {
        readableName: "Shared Memory",
        description: "Memory used for shared memory segments including System V shared memory and tmpfs filesystems.",
        desired: "moderate",
      },
      active_file: {
        readableName: "Active File Pages",
        description:
          "File-backed memory pages in the active LRU list that are frequently accessed and less likely to be reclaimed.",
        desired: "moderate",
      },
      mem_available: {
        readableName: "Available Memory",
        description: "Estimate of memory available for starting new applications without causing excessive swapping.",
        desired: "higher",
      },
      swap_cached: {
        readableName: "Swap Cache",
        description:
          "Memory that was swapped out but is now back in RAM and still cached in case it needs to be swapped out again.",
        desired: "lower",
      },
      shmem_pmd_mapped: {
        readableName: "Shared Memory PMD Mapped",
        description:
          "Shared memory pages mapped using Page Middle Directory entries for large page optimizations in shared memory segments.",
        desired: "lower",
      },
      hugepages_total: {
        readableName: "Total Huge Pages",
        description:
          "Total number of huge pages configured in the system for applications requiring large contiguous memory blocks.",
        desired: "depends",
        unit: "Pages",
      },
      kernel_stack: {
        readableName: "Kernel Stack Memory",
        description: "Memory used by kernel stacks for each thread and process in the system.",
        desired: "moderate",
      },
      hugepages_rsvd: {
        readableName: "Reserved Huge Pages",
        description: "Number of huge pages reserved but not yet allocated to applications.",
        desired: "lower",
        unit: "Pages",
      },
      nfs_unstable: {
        readableName: "NFS Unstable Pages",
        description: "Pages that have been written to NFS server but not yet committed to stable storage.",
        desired: "lower",
      },
      k_reclaimable: {
        readableName: "Kernel Reclaimable",
        description: "Kernel memory that can be reclaimed when the system is under memory pressure.",
        desired: "moderate",
      },
      hugepages_surp: {
        readableName: "Surplus Huge Pages",
        description: "Number of huge pages allocated beyond the configured pool size.",
        desired: "lower",
        unit: "Pages",
      },
      shmem_hugepages: {
        readableName: "Shared Memory Huge Pages",
        description: "Shared memory segments using huge pages for improved performance.",
        desired: "lower",
      },
      hugepagesize: {
        readableName: "Huge Page Size",
        description: "Size of each huge page in the system typically 2MB or 1GB.",
        desired: "fixed",
      },
      file_huge_pages: {
        readableName: "File Huge Pages",
        description: "File-backed memory pages using huge page sizes for improved I/O performance.",
        desired: "lower",
      },
      direct_map_1g: {
        readableName: "Direct Map 1G Pages",
        description: "Number of bytes of physical memory directly mapped using 1GB pages for maximum TLB efficiency.",
        desired: "higher",
      },
      bounce: {
        readableName: "Bounce Buffer Memory",
        description: "Memory used for bounce buffers when DMA cannot directly access certain memory regions.",
        desired: "lower",
      },
      direct_map_4m: {
        readableName: "Direct Map 4M Pages",
        description:
          "Number of bytes of physical memory directly mapped using 4MB pages for improved TLB efficiency on some architectures.",
        desired: "higher",
      },
      mlocked: {
        readableName: "Memory Locked",
        description: "Memory pages that have been locked in RAM and cannot be swapped out to storage.",
        desired: "lower",
      },
      writeback: {
        readableName: "Writeback Memory",
        description: "Memory pages currently being written back to storage devices.",
        desired: "lower",
      },
      s_reclaimable: {
        readableName: "Slab Reclaimable",
        description: "Kernel slab memory that can be reclaimed when the system is under memory pressure.",
        desired: "moderate",
      },
      vmalloc_total: {
        readableName: "Virtual Memory Allocator Total",
        description: "Total virtual address space available for vmalloc allocations by the kernel.",
        desired: "higher",
      },
      hardware_corrupted: {
        readableName: "Hardware Corrupted Memory",
        description: "Memory pages marked as corrupted due to hardware errors and excluded from use.",
        desired: "lower",
      },
      anon_pages: {
        readableName: "Anonymous Pages",
        description: "Anonymous memory pages not backed by files including process heap stack and anonymous mappings.",
        desired: "depends",
      },
      active: {
        readableName: "Active Memory",
        description:
          "Total memory pages in active LRU lists that are frequently accessed and less likely to be reclaimed.",
        desired: "moderate",
      },
      active_anon: {
        readableName: "Active Anonymous Memory",
        description: "Anonymous memory pages in the active LRU list that are frequently accessed.",
        desired: "moderate",
      },
      inactive_anon: {
        readableName: "Inactive Anonymous Memory",
        description: "Anonymous memory pages in the inactive LRU list that are candidates for swapping.",
        desired: "moderate",
      },
      mmap_copy: {
        readableName: "Memory Map Copy",
        description: "Memory used for copy-on-write mappings during memory management operations.",
        desired: "lower",
      },
      writeback_tmp: {
        readableName: "Temporary Writeback Memory",
        description: "Memory used for temporary writeback operations during I/O processing.",
        desired: "lower",
      },
      quicklists: {
        readableName: "Quicklist Memory",
        description: "Memory used by quicklists for fast allocation and deallocation of kernel objects.",
        desired: "lower",
      },
      hugetlb: {
        readableName: "Huge TLB Memory",
        description: "Memory reserved for huge page TLB entries to improve virtual memory performance.",
        desired: "depends",
      },
      buffers: {
        readableName: "Buffer Memory",
        description: "Memory used by the kernel for buffering block device I/O operations.",
        desired: "moderate",
      },
      dirty: {
        readableName: "Dirty Memory",
        description: "Memory pages that have been modified but not yet written back to storage.",
        desired: "lower",
      },
      vmalloc_chunk: {
        readableName: "Virtual Memory Allocator Chunk",
        description: "Largest contiguous chunk of virtual address space available for vmalloc allocations.",
        desired: "higher",
      },
      page_tables: {
        readableName: "Page Table Memory",
        description: "Memory used by page tables for virtual to physical address translation.",
        desired: "moderate",
      },
      mapped: {
        readableName: "Mapped Memory",
        description: "Memory pages mapped into process address spaces for files and shared libraries.",
        desired: "moderate",
      },
      hugepages_free: {
        readableName: "Free Huge Pages",
        description: "Number of huge pages currently available for allocation.",
        desired: "higher",
        unit: "Pages",
      },
    },
  },
  vmstat: {
    readableName: "Virtual Memory Stats",
    summary:
      "Virtual memory metrics measure the usage of the system's virtual memory. The data were collected from the system pseudo-file /proc/vmstat. Note that for some metrics, the values were computed using the delta of two snapshots, so that first value is always zero.",
    defaultUnit: "Pages",
    fieldDescriptions: {
      thp_fault_fallback: {
        readableName: "THP Fault Fallback",
        description:
          "Number of times Transparent Huge Page allocation failed and fell back to regular 4KB pages due to memory fragmentation.",
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
        readableName: "Out of Memory Kills",
        description:
          "Number of processes terminated by the kernel's Out of Memory killer when system memory is critically low.",
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
          "Transparent Huge Pages that have been marked for splitting but the operation has been deferred to reduce immediate memory management overhead.",
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
          "Memory threshold at which the kernel begins background writeback of dirty pages to storage to maintain system performance.",
        desired: "higher",
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
          "Number of times memory compaction was stalled waiting for pages to be migrated to reduce memory fragmentation.",
        desired: "lower",
        unit: "Count",
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
          "Number of pages reclaimed by the kswapd kernel thread during background memory reclamation to maintain free memory levels.",
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
          "Memory threshold at which processes are throttled to prevent excessive dirty page accumulation and maintain system responsiveness.",
        desired: "higher",
      },
      pgpgin: {
        readableName: "Pages Paged In",
        description:
          "Number of pages read from storage devices into memory indicating I/O activity and memory pressure.",
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
          "Direct memory reclamation scans performed by processes when memory allocation fails and immediate reclamation is needed.",
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
        description: "Number of pages in temporary writeback state during memory reclaim operations.",
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
        description: "Number of transparent huge pages allocated during page fault handling.",
        desired: "lower",
      },
      pgsteal_direct: {
        readableName: "Steal Direct",
        description: "Number of pages reclaimed through direct memory reclaim indicating memory pressure.",
        desired: "lower",
      },
      allocstall_dma: {
        readableName: "Allocstall Dma",
        description: "Number of allocation stalls in DMA zone indicating memory pressure in low memory areas.",
        desired: "lower",
        unit: "Count",
      },
      pgmajfault: {
        readableName: "Major Page Faults",
        description:
          "Page faults that require loading data from storage devices indicating memory pressure and I/O activity.",
        desired: "lower",
      },
      compact_migrate_scanned: {
        readableName: "Compact Migrate Scanned",
        description: "Number of pages scanned for migration during memory compaction to reduce fragmentation.",
        desired: "lower",
      },
      numa_miss: {
        readableName: "Numa Miss",
        description:
          "Number of memory allocations that missed the preferred NUMA node indicating suboptimal memory placement.",
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
        description: "Number of dirty pages in memory that need to be written to storage.",
        desired: "lower",
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
        description: "Number of successful memory compaction operations that reduced fragmentation.",
        desired: "lower",
        unit: "Count",
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
        description: "Number of pages written to storage devices during writeback operations.",
        desired: "lower",
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
        description: "Number of file page refaults from the working set indicating cache misses.",
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
        description: "Number of pages scanned by kswapd daemon during background memory reclaim.",
        desired: "lower",
      },
      pgsteal_file: {
        readableName: "Steal File",
        description: "Number of file pages reclaimed from memory during memory pressure.",
        desired: "lower",
      },
      allocstall_normal: {
        readableName: "Allocstall Normal",
        description: "Number of allocation stalls in normal memory zone indicating memory pressure.",
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
        description: "Number of pages that have been written to NFS server but not yet committed to stable storage.",
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
        description:
          "Direct memory reclaim operations throttled to prevent excessive CPU usage during memory pressure.",
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
          "Number of anonymous memory pages allocated as Transparent Huge Pages for improved memory performance.",
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
          "Total number of pages that have been marked as dirty since system boot indicating write activity.",
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
        description: "Number of pages written to swap space indicating memory pressure and virtual memory activity.",
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
        description: "Failed attempts to compact memory to create larger contiguous memory blocks.",
        desired: "lower",
        unit: "Count",
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
        description: "Memory allocation stalls in the DMA32 zone due to memory pressure or fragmentation.",
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
          "Number of pages currently being written back to storage devices indicating active I/O operations.",
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
        description: "Number of pages read from swap space back into memory indicating memory pressure recovery.",
        desired: "lower",
      },
      allocstall_movable: {
        readableName: "Allocation Stall Movable",
        description:
          "Number of allocation stalls in movable memory zone indicating memory pressure in reclaimable areas.",
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
        description: "Memory pages reclaimed by the kernel huge page daemon during memory pressure.",
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
        description:
          "Kernel Same-page Merging pages copied during swap-in operations to maintain memory deduplication.",
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
        description: "Memory pages demoted to slower storage tiers by the kernel swap daemon during memory pressure.",
        desired: "lower",
      },
      pgdemote_khugepaged: {
        readableName: "Page Demote Khugepaged",
        description: "Memory pages demoted by the kernel huge page daemon during memory management operations.",
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
        description: "Memory pages directly demoted to slower storage tiers during memory pressure.",
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
        description:
          "Number of memory pages used by Input-Output Memory Management Unit for device memory translation.",
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
    },
  },
  interrupts: {
    readableName: "Interrupts",
    summary:
      "Interrupt metrics measure that number of interrupts handled by each CPU. The data were collected from the system pseudo-file /proc/interrupts. Every metric graph show the number of times a specific interrupt was handled by each CPU, as well as the aggregate (average) of all CPUs. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for its aggregate series.",
    defaultUnit: "Counts",
    fieldDescriptions: {
      // TODO: add once interrupt names are available
    },
  },
  diskstats: {
    readableName: "Disk Stats",
    summary:
      "Disk stats metrics measure the I/O stats for each disk device and partition of the system. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for the device series with the highest average.",
    defaultUnit: "Counts",
    fieldDescriptions: {
      discards: {
        readableName: "Discard Operations",
        description:
          "Discard operations count the total number of TRIM or discard commands issued to the storage device to mark unused blocks for reclamation.",
        desired: "lower",
      },
      discards_merged: {
        readableName: "Merged Discard Operations",
        description: "Number of discard operations merged before being sent to the device.",
        desired: "lower",
      },
      flushes: {
        readableName: "Flush Operations",
        description: "Number of flush operations to ensure data is written to storage.",
        desired: "lower",
      },
      in_progress: {
        readableName: "I/O Operations In Progress",
        description: "Number of I/O operations currently in progress.",
        desired: "lower",
      },
      reads: {
        readableName: "Read Operations",
        description: "Total number of read operations completed.",
        desired: "depends",
      },
      merged: {
        readableName: "Merged Read Operations",
        description: "Number of read operations merged before being sent to the device.",
        desired: "lower",
      },
      sectors_discarded: {
        readableName: "Sectors Discarded",
        description: "Number of 512-byte sectors discarded.",
        desired: "lower",
      },
      sectors_read: {
        readableName: "Sectors Read",
        description: "Number of 512-byte sectors read from storage.",
        desired: "depends",
      },
      sectors_written: {
        readableName: "Sectors Written",
        description: "Number of 512-byte sectors written to storage.",
        desired: "depends",
      },
      time_discarding: {
        readableName: "Time Spent Discarding",
        description: "Total time spent on discard operations in milliseconds.",
        unit: "milliseconds",
        desired: "lower",
      },
      time_flushing: {
        readableName: "Time Spent Flushing",
        description: "Total time spent on flush operations in milliseconds.",
        unit: "milliseconds",
        desired: "lower",
      },
      time_in_progress: {
        readableName: "Time with I/O In Progress",
        description: "Total time with I/O operations in progress in milliseconds.",
        unit: "milliseconds",
        desired: "lower",
      },
      time_reading: {
        readableName: "Time Spent Reading",
        description:
          "Total time spent on read operations in milliseconds (as measured from blk_mq_alloc_request() to __blk_mq_end_request()).",
        unit: "milliseconds",
        desired: "lower",
      },
      time_writing: {
        readableName: "Time Spent Writing",
        description:
          "Total time spent on write operations in milliseconds (as measured from blk_mq_alloc_request() to __blk_mq_end_request()).",
        unit: "milliseconds",
        desired: "lower",
      },
      weighted_time_in_progress: {
        readableName: "Weighted Time In Progress",
        description: "Weighted time with I/O operations in progress accounting for queue depth.",
        unit: "milliseconds",
        desired: "lower",
      },
      writes: {
        readableName: "Write Operations",
        description: "Total number of write operations completed.",
        desired: "depends",
      },
      writes_merged: {
        readableName: "Merged Write Operations",
        description: "Number of write operations merged before being sent to the device.",
        desired: "lower",
      },
    },
  },
  netstat: {
    readableName: "Network Stats",
    summary:
      "Network stats metrics measure various networking stats for different protocols (TCP, IP, etc.). Note that since the metric values were computed using the delta between two snapshots, the first value is always zero.",
    defaultUnit: "Counts",
    fieldDescriptions: {
      "TcpExt:ArpFilter": {
        readableName: "ARP Filter Events",
        description:
          "Number of ARP packets filtered or dropped due to security policies or network configuration rules.",
        desired: "lower",
      },
      "IpExt:InBcastPkts": {
        readableName: "Incoming Broadcast Packets",
        description: "Number of broadcast packets received by the network interface from the local network segment.",
        desired: "depends",
      },
      "MPTcpExt:MPJoinAckRx": {
        readableName: "MPTCP Join ACK Received",
        description:
          "Multipath TCP join acknowledgment packets received for establishing additional subflows in MPTCP connections.",
        desired: "depends",
      },
      "MPTcpExt:RmAddr": {
        readableName: "MPTCP Remove Address",
        description:
          "Multipath TCP remove address operations for managing multiple network paths in MPTCP connections.",
        desired: "lower",
      },
      "TcpExt:TCPFastOpenCookieReqd": {
        readableName: "TCP Fast Open Cookie Required",
        description:
          "TCP Fast Open connections that required cookie validation for security before allowing data transmission.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynAckRx": {
        readableName: "MPTCP Join SYN-ACK Received",
        description:
          "Multipath TCP join SYN-ACK packets received for establishing additional subflows in MPTCP connections.",
        desired: "depends",
      },
      "TcpExt:TCPDSACKRecv": {
        readableName: "TCP DSACK Received",
        description:
          "Duplicate Selective Acknowledgment packets received indicating duplicate data transmission and potential network issues.",
        desired: "lower",
      },
      "TcpExt:TCPDeliveredCE": {
        readableName: "TCP Delivered CE",
        description:
          "TCP packets delivered with Congestion Experienced marking indicating network congestion encountered during transmission.",
        desired: "lower",
      },
      "MPTcpExt:MPTCPRetrans": {
        readableName: "MPTCP Retransmissions",
        description: "Multipath TCP packet retransmissions across multiple network paths for reliable data delivery.",
        desired: "lower",
      },
      "TcpExt:PAWSActive": {
        readableName: "PAWS Active",
        description:
          "Protection Against Wrapped Sequences active connections preventing old duplicate packets from being accepted.",
        desired: "lower",
      },
      "TcpExt:TCPDSACKUndo": {
        readableName: "TCP DSACK Undo",
        description:
          "TCP congestion control undo operations triggered by Duplicate Selective Acknowledgments indicating false loss detection.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynAckHMacFailure": {
        readableName: "MPTCP Join SYN-ACK HMAC Failure",
        description: "Multipath TCP join SYN-ACK packets with HMAC authentication failures indicating security issues.",
        desired: "lower",
      },
      "TcpExt:BusyPollRxPackets": {
        readableName: "Busy Poll RX Packets",
        description:
          "Packets received using busy polling for reduced latency in high-performance networking applications.",
        desired: "higher",
      },
      "TcpExt:TCPACKSkippedChallenge": {
        readableName: "TCP ACK Skipped Challenge",
        description: "TCP acknowledgments skipped due to challenge ACK rate limiting for security protection.",
        desired: "lower",
      },
      "TcpExt:TCPSackMerged": {
        readableName: "TCP SACK Merged",
        description: "TCP Selective Acknowledgment blocks merged to reduce protocol overhead and improve efficiency.",
        desired: "lower",
      },
      "MPTcpExt:DSSCorruptionReset": {
        readableName: "MPTCP DSS Corruption Reset",
        description:
          "Multipath TCP connections reset due to Data Sequence Signal corruption indicating data integrity issues.",
        desired: "lower",
      },
      "IpExt:ReasmOverlaps": {
        readableName: "IP Reassembly Overlaps",
        description:
          "IP packet fragments with overlapping data during reassembly indicating potential fragmentation issues or attacks.",
        desired: "lower",
      },
      "MPTcpExt:MPCapableFallbackACK": {
        readableName: "MPTCP Capable Fallback ACK",
        description: "Multipath TCP connections falling back to regular TCP due to capability negotiation failures.",
        desired: "lower",
      },
      "TcpExt:SpuriousRtxHostQueues": {
        readableName: "TCP Spurious Retransmit Host Queues",
        description:
          "TCP spurious retransmissions caused by host queue management issues that can indicate network stack inefficiencies.",
        desired: "lower",
      },
      "TcpExt:WantZeroWindowAdv": {
        readableName: "TCP Want Zero Window Advertise",
        description:
          "TCP connections that want to advertise a zero window size to pause incoming data when receive buffers are full.",
        desired: "lower",
      },
      "TcpExt:ReqQFullDrop": {
        readableName: "TCP Request Queue Full Drop",
        description:
          "TCP connection requests dropped because the server's request queue is full indicating potential overload conditions.",
        desired: "lower",
      },
      "TcpExt:OutOfWindowIcmps": {
        readableName: "Out of Window ICMPs",
        description:
          "ICMP packets received outside the expected TCP sequence window indicating potential network issues or attacks.",
        desired: "lower",
      },
      "MPTcpExt:RmSubflow": {
        readableName: "MPTCP Remove Subflow",
        description:
          "Multipath TCP subflow removal operations for managing multiple network paths in MPTCP connections.",
        desired: "lower",
      },
      "TcpExt:TCPLossFailures": {
        readableName: "TCP Loss Failures",
        description: "TCP loss recovery failures indicating unsuccessful attempts to recover from packet loss events.",
        desired: "lower",
      },
      "IpExt:OutBcastPkts": {
        readableName: "Outgoing Broadcast Packets",
        description: "Number of broadcast packets transmitted by the network interface to the local network segment.",
        desired: "lower",
      },
      "TcpExt:TCPDSACKRecvSegs": {
        readableName: "TCP DSACK Received Segments",
        description:
          "Duplicate Selective Acknowledgment segments received indicating duplicate data transmission issues.",
        desired: "lower",
      },
      "TcpExt:TCPFastOpenBlackhole": {
        readableName: "TCP Fast Open Blackhole",
        description: "TCP Fast Open connections that encountered blackhole detection indicating network path issues.",
        desired: "lower",
      },
      "IpExt:InNoECTPkts": {
        readableName: "Incoming Non-ECT Packets",
        description:
          "IP packets received without Explicit Congestion Notification marking indicating no congestion awareness.",
        desired: "lower",
      },
      "MPTcpExt:EchoAdd": {
        readableName: "MPTCP Echo Add",
        description: "Multipath TCP echo add operations for managing address advertisements in MPTCP connections.",
        desired: "lower",
      },
      "IpExt:OutMcastOctets": {
        readableName: "Outgoing Multicast Octets",
        description: "Total bytes transmitted in multicast packets for group communication.",
        desired: "lower",
      },
      "TcpExt:TCPAutoCorking": {
        readableName: "TCP Auto Corking",
        description: "TCP connections using automatic corking to batch small writes for improved network efficiency.",
        desired: "lower",
      },
      "MPTcpExt:DuplicateData": {
        readableName: "MPTCP Duplicate Data",
        description: "Multipath TCP duplicate data packets received across multiple subflows.",
        desired: "lower",
      },
      "TcpExt:TCPRcvCoalesce": {
        readableName: "TCP Receive Coalesce",
        description: "TCP segments coalesced in the receive path to reduce processing overhead.",
        desired: "lower",
      },
      "TcpExt:TCPHystartTrainDetect": {
        readableName: "TCP HyStart Train Detect",
        description: "TCP HyStart algorithm train detection events for congestion control optimization.",
        desired: "lower",
      },
      "TcpExt:TWRecycled": {
        readableName: "TIME-WAIT Recycled",
        description: "TCP TIME-WAIT sockets recycled for new connections to conserve resources.",
        desired: "lower",
      },
      "TcpExt:TCPOFODrop": {
        readableName: "TCP Out-of-Order Drop",
        description: "TCP out-of-order packets dropped due to receive buffer limitations or memory pressure.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinNoTokenFound": {
        readableName: "MPTCP Join No Token Found",
        description: "Multipath TCP join attempts that failed because no matching connection token was found.",
        desired: "lower",
      },
      "TcpExt:TCPRetransFail": {
        readableName: "TCP Retransmission Fail",
        description: "TCP retransmission attempts that failed due to network issues or connection problems.",
        desired: "lower",
      },
      "TcpExt:PAWSEstab": {
        readableName: "PAWS Established",
        description:
          "Protection Against Wrapped Sequences established connections preventing old duplicate packets from being accepted.",
        desired: "lower",
      },
      "TcpExt:TCPSynRetrans": {
        readableName: "TCP SYN Retransmissions",
        description: "TCP SYN packet retransmissions due to connection establishment failures or network issues.",
        desired: "lower",
      },
      "TcpExt:TCPPureAcks": {
        readableName: "TCP Pure ACKs",
        description:
          "Count of TCP acknowledgment packets that contain no data payload and are used purely for connection management and flow control.",
        desired: "depends",
      },
      "MPTcpExt:MPCapableACKRX": {
        readableName: "MPTCP Capable ACK Received",
        description: "Multipath TCP capable acknowledgment packets received during connection establishment.",
        desired: "lower",
      },
      "TcpExt:TCPMinTTLDrop": {
        readableName: "TCP Minimum TTL Drop",
        description:
          "TCP packets dropped because their TTL value was below the configured minimum threshold for security.",
        desired: "lower",
      },
      "TcpExt:TW": {
        readableName: "TCP TIME-WAIT",
        description: "TCP connections transitioning to or operating in the TIME-WAIT state for connection cleanup.",
        desired: "lower",
      },
      "TcpExt:IPReversePathFilter": {
        readableName: "IP Reverse Path Filter",
        description:
          "IP packets dropped due to reverse path filtering violations indicating potential spoofing or routing issues.",
        desired: "lower",
      },
      "TcpExt:TCPSackRecovery": {
        readableName: "TCP SACK Recovery",
        description:
          "TCP Selective Acknowledgment based loss recovery events for efficient retransmission of missing segments.",
        desired: "lower",
      },
      "TcpExt:TCPRenoReorder": {
        readableName: "TCP Reno Reorder",
        description: "TCP Reno algorithm packet reordering detections indicating out-of-order packet delivery.",
        desired: "lower",
      },
      "TcpExt:TCPOFOQueue": {
        readableName: "TCP Out-of-Order Queue",
        description:
          "TCP out-of-order packets queued for later processing when segments arrive before expected sequence numbers.",
        desired: "lower",
      },
      "TcpExt:TcpDuplicateDataRehash": {
        readableName: "TCP Duplicate Data Rehash",
        description:
          "TCP connections that required hash table rehashing due to duplicate data detection and processing.",
        desired: "lower",
      },
      "TcpExt:TCPAbortOnMemory": {
        readableName: "TCP Abort On Memory",
        description: "TCP connections aborted due to insufficient memory for connection processing.",
        desired: "lower",
      },
      "IpExt:InTruncatedPkts": {
        readableName: "IP Truncated Packets Received",
        description: "IP packets received that were truncated due to insufficient buffer space or transmission errors.",
        desired: "lower",
      },
      "TcpExt:TCPMemoryPressuresChrono": {
        readableName: "TCP Memory Pressures Chrono",
        description:
          "Chronological count of TCP memory pressure events indicating sustained memory constraints over time.",
        desired: "lower",
      },
      "TcpExt:TCPRcvCollapsed": {
        readableName: "TCP Receive Collapsed",
        description: "TCP receive buffer segments collapsed to save memory during high memory pressure conditions.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynRx": {
        readableName: "MPTCP Join SYN Received",
        description:
          "Multipath TCP join SYN packets received for establishing additional subflows in MPTCP connections.",
        desired: "depends",
      },
      "IpExt:InCEPkts": {
        readableName: "IP CE Packets Received",
        description:
          "IP packets received with Congestion Experienced marking indicating network congestion encountered during transmission.",
        desired: "lower",
      },
      "TcpExt:TCPSlowStartRetrans": {
        readableName: "TCP Slow Start Retransmissions",
        description: "TCP retransmissions that occurred during the slow start phase of congestion control.",
        desired: "lower",
      },
      "TcpExt:TCPFastOpenActive": {
        readableName: "TCP Fast Open Active",
        description: "TCP connections that actively used Fast Open to send data with the initial SYN packet.",
        desired: "higher",
      },
      "TcpExt:PFMemallocDrop": {
        readableName: "PF Memory Allocation Drop",
        description: "Packets dropped due to page frame memory allocation failures during network processing.",
        desired: "lower",
      },
      "TcpExt:TCPBacklogCoalesce": {
        readableName: "TCP Backlog Coalesce",
        description:
          "TCP segments coalesced in the backlog queue to reduce processing overhead and improve efficiency.",
        desired: "higher",
      },
      "TcpExt:TCPSpuriousRTOs": {
        readableName: "TCP Spurious RTOs",
        description: "TCP spurious retransmission timeouts that were later determined to be unnecessary.",
        desired: "lower",
      },
      "TcpExt:TCPMigrateReqFailure": {
        readableName: "TCP Migrate Request Failure",
        description: "TCP connection migration requests that failed during socket migration between CPU cores.",
        desired: "lower",
      },
      "TcpExt:DelayedACKs": {
        readableName: "Delayed ACKs",
        description:
          "TCP acknowledgments that were delayed to potentially piggyback on outgoing data packets for improved network efficiency.",
        desired: "moderate",
      },
      "MPTcpExt:AddAddr": {
        readableName: "MPTCP Add Address",
        description:
          "Multipath TCP add address operations for advertising additional network interfaces to establish multiple subflows.",
        desired: "lower",
      },
      "TcpExt:TCPMD5NotFound": {
        readableName: "TCP MD5 Not Found",
        description:
          "TCP connections that failed MD5 signature verification because the expected MD5 key was not found.",
        desired: "lower",
      },
      "IpExt:InCsumErrors": {
        readableName: "IP Checksum Errors",
        description: "IP packets received with invalid checksums indicating data corruption during transmission.",
        desired: "lower",
      },
      "MPTcpExt:InfiniteMapRx": {
        readableName: "MPTCP Infinite Map Received",
        description:
          "Multipath TCP infinite mapping packets received indicating the entire remaining data stream maps to one subflow.",
        desired: "lower",
      },
      "TcpExt:TCPChallengeACK": {
        readableName: "TCP Challenge ACK",
        description:
          "TCP challenge acknowledgments sent to verify connection state and prevent blind attacks on established connections.",
        desired: "lower",
      },
      "TcpExt:TCPMD5Unexpected": {
        readableName: "TCP MD5 Unexpected",
        description:
          "TCP packets received with unexpected MD5 signatures when MD5 authentication was not expected for the connection.",
        desired: "lower",
      },
      "TcpExt:TCPToZeroWindowAdv": {
        readableName: "TCP To Zero Window Advertise",
        description:
          "TCP connections that advertised a zero window size to pause incoming data when receive buffers became full.",
        desired: "lower",
      },
      "TcpExt:TCPRenoFailures": {
        readableName: "TCP Reno Failures",
        description:
          "TCP Reno congestion control algorithm failures during loss recovery indicating network congestion handling issues.",
        desired: "lower",
      },
      "TcpExt:TCPMTUPSuccess": {
        readableName: "TCP MTU Probe Success",
        description:
          "Successful TCP Maximum Transmission Unit path discovery probes for optimizing packet size across network paths.",
        desired: "higher",
      },
      "TcpExt:TCPSACKDiscard": {
        readableName: "TCP SACK Discard",
        description:
          "TCP Selective Acknowledgment blocks discarded due to invalid or duplicate sequence numbers during loss recovery.",
        desired: "lower",
      },
      "TcpExt:TCPHPHits": {
        readableName: "TCP High Performance Hits",
        description: "TCP high performance path cache hits for optimized packet processing and routing decisions.",
        desired: "higher",
      },
      "TcpExt:TCPHystartDelayDetect": {
        readableName: "TCP HyStart Delay Detect",
        description:
          "TCP HyStart algorithm delay-based congestion detection events for improved slow start exit timing.",
        desired: "lower",
      },
      "TcpExt:TCPAbortOnData": {
        readableName: "TCP Abort On Data",
        description:
          "TCP connections aborted due to unexpected data received during connection termination or invalid states.",
        desired: "lower",
      },
      "TcpExt:TCPFastOpenPassiveAltKey": {
        readableName: "TCP Fast Open Passive Alt Key",
        description: "TCP Fast Open passive connections using alternative key validation for enhanced security.",
        desired: "lower",
      },
      "TcpExt:TCPECNRehash": {
        readableName: "TCP ECN Rehash",
        description:
          "TCP connections requiring hash table rehashing due to Explicit Congestion Notification state changes.",
        desired: "lower",
      },
      "TcpExt:TCPDelivered": {
        readableName: "TCP Delivered",
        description:
          "Total number of TCP data packets successfully delivered to applications indicating successful data transmission.",
        desired: "higher",
      },
      "MPTcpExt:OFOQueueTail": {
        readableName: "MPTCP OFO Queue Tail",
        description: "Multipath TCP out-of-order packets queued at the tail for reordering across multiple subflows.",
        desired: "lower",
      },
      "TcpExt:SyncookiesRecv": {
        readableName: "SYN Cookies Received",
        description:
          "TCP SYN cookies received and validated during connection establishment under high load conditions.",
        desired: "lower",
      },
      "TcpExt:TCPMigrateReqSuccess": {
        readableName: "TCP Migrate Request Success",
        description: "Successful TCP connection migration requests for load balancing or failover scenarios.",
        desired: "higher",
      },
      "TcpExt:TCPLossProbeRecovery": {
        readableName: "TCP Loss Probe Recovery",
        description:
          "TCP loss probe packets that successfully recovered from potential packet loss without triggering full retransmission.",
        desired: "higher",
      },
      "TcpExt:TCPHPAcks": {
        readableName: "TCP High Performance ACKs",
        description:
          "TCP high performance acknowledgment packets processed through optimized fast path for improved throughput.",
        desired: "higher",
      },
      "TcpExt:TCPSackFailures": {
        readableName: "TCP SACK Failures",
        description:
          "TCP Selective Acknowledgment processing failures due to invalid SACK blocks or sequence number issues.",
        desired: "lower",
      },
      "TcpExt:TCPReqQFullDoCookies": {
        readableName: "TCP Request Queue Full Do Cookies",
        description:
          "TCP SYN cookies generated when the request queue is full to handle connection overload situations.",
        desired: "lower",
      },
      "TcpExt:TCPACKSkippedSynRecv": {
        readableName: "TCP ACK Skipped SYN Received",
        description:
          "TCP acknowledgments skipped for connections in SYN-RECEIVED state to optimize connection establishment.",
        desired: "lower",
      },
      "TcpExt:TCPFastOpenPassive": {
        readableName: "TCP Fast Open Passive",
        description:
          "TCP Fast Open passive connections established allowing data transmission during the initial handshake.",
        desired: "higher",
      },
      "IpExt:InNoRoutes": {
        readableName: "IP Input No Routes",
        description: "IP packets received that could not be routed due to missing routing table entries.",
        desired: "lower",
      },
      "TcpExt:TCPOrigDataSent": {
        readableName: "TCP Original Data Sent",
        description:
          "TCP original data segments transmitted before any retransmissions indicating initial transmission efficiency.",
        desired: "depends",
      },
      "TcpExt:TCPLostRetransmit": {
        readableName: "TCP Lost Retransmit",
        description:
          "TCP retransmitted segments that were subsequently lost requiring additional retransmission attempts.",
        desired: "lower",
      },
      "TcpExt:TCPOFOMerge": {
        readableName: "TCP Out-of-Order Merge",
        description: "TCP out-of-order segments successfully merged into the receive queue for proper data sequencing.",
        desired: "higher",
      },
      "IpExt:InBcastOctets": {
        readableName: "IP Input Broadcast Octets",
        description: "Total bytes received from broadcast packets on the network interface.",
        desired: "depends",
      },
      "TcpExt:SyncookiesFailed": {
        readableName: "SYN Cookies Failed",
        description:
          "TCP SYN cookies that failed validation during connection establishment under high load conditions.",
        desired: "lower",
      },
      "TcpExt:SyncookiesSent": {
        readableName: "SYN Cookies Sent",
        description:
          "TCP SYN cookies transmitted when the connection request queue is full to prevent denial of service.",
        desired: "lower",
      },
      "TcpExt:TCPAbortFailed": {
        readableName: "TCP Abort Failed",
        description: "TCP connection abort attempts that failed to properly terminate the connection.",
        desired: "lower",
      },
      "TcpExt:TCPAckCompressed": {
        readableName: "TCP ACK Compressed",
        description:
          "TCP acknowledgment packets compressed to reduce network overhead and improve bandwidth efficiency.",
        desired: "higher",
      },
      "IpExt:OutMcastPkts": {
        readableName: "IP Output Multicast Packets",
        description: "Number of multicast packets transmitted by the network interface.",
        desired: "depends",
      },
      "TcpExt:TCPDSACKOfoSent": {
        readableName: "TCP DSACK Out-of-Order Sent",
        description: "TCP Duplicate SACK blocks sent for out-of-order segments to inform sender of reception status.",
        desired: "lower",
      },
      "TcpExt:TCPDSACKOldSent": {
        readableName: "TCP DSACK Old Sent",
        description: "TCP Duplicate SACK blocks sent for previously acknowledged segments to clarify reception status.",
        desired: "lower",
      },
      "IpExt:InECT0Pkts": {
        readableName: "IP ECT0 Packets Received",
        description:
          "IP packets received with ECT(0) marking indicating ECN-capable transport with no congestion experienced.",
        desired: "depends",
      },
      "IpExt:InOctets": {
        readableName: "Incoming Octets",
        description:
          "Total number of bytes received by the network interface including all protocol headers and payload data.",
        desired: "depends",
      },
      "IpExt:InMcastPkts": {
        readableName: "IP Multicast Packets Received",
        description: "Number of multicast packets received by the network interface for group communication.",
        desired: "depends",
      },
      "TcpExt:TCPKeepAlive": {
        readableName: "TCP Keep Alive",
        description: "TCP keep-alive packets sent to maintain idle connections and detect broken connections.",
        desired: "moderate",
      },
      "TcpExt:TCPACKSkippedTimeWait": {
        readableName: "TCP ACK Skipped Time Wait",
        description: "TCP acknowledgments skipped for connections in TIME-WAIT state to optimize connection cleanup.",
        desired: "lower",
      },
      "TcpExt:TCPFastRetrans": {
        readableName: "TCP Fast Retransmit",
        description: "TCP fast retransmission events triggered by duplicate acknowledgments indicating packet loss.",
        desired: "lower",
      },
      "TcpExt:TCPFromZeroWindowAdv": {
        readableName: "TCP From Zero Window Advertise",
        description:
          "TCP connections recovering from zero window advertisements when receive buffer space becomes available.",
        desired: "higher",
      },
      "MPTcpExt:MPCapableFallbackSYNACK": {
        readableName: "MPTCP Capable Fallback SYN-ACK",
        description:
          "Multipath TCP connections falling back to regular TCP during SYN-ACK phase due to capability negotiation issues.",
        desired: "lower",
      },
      "TcpExt:TCPPartialUndo": {
        readableName: "TCP Partial Undo",
        description:
          "TCP partial undo operations during congestion control recovery to optimize window size adjustments.",
        desired: "higher",
      },
      "MPTcpExt:NoDSSInWindow": {
        readableName: "MPTCP No DSS In Window",
        description:
          "Multipath TCP data sequence signal missing within the receive window causing subflow synchronization issues.",
        desired: "lower",
      },
      "TcpExt:TCPRenoRecoveryFail": {
        readableName: "TCP Reno Recovery Fail",
        description:
          "TCP Reno congestion control recovery failures requiring fallback to alternative recovery mechanisms.",
        desired: "lower",
      },
      "MPTcpExt:OFOMerge": {
        readableName: "MPTCP Out-of-Order Merge",
        description:
          "Multipath TCP out-of-order segments successfully merged across multiple subflows for proper data sequencing.",
        desired: "higher",
      },
      "IpExt:OutBcastOctets": {
        readableName: "IP Output Broadcast Octets",
        description: "Total bytes transmitted in broadcast packets on the network interface.",
        desired: "depends",
      },
      "TcpExt:TCPFastOpenPassiveFail": {
        readableName: "TCP Fast Open Passive Fail",
        description: "TCP Fast Open passive connection attempts that failed during the initial handshake process.",
        desired: "lower",
      },
      "TcpExt:TCPSYNChallenge": {
        readableName: "TCP SYN Challenge",
        description: "TCP SYN challenge responses sent to validate connection requests and prevent SYN flood attacks.",
        desired: "lower",
      },
      "TcpExt:DelayedACKLost": {
        readableName: "Delayed ACK Lost",
        description:
          "TCP delayed acknowledgments that were lost requiring retransmission and potentially impacting performance.",
        desired: "lower",
      },
      "TcpExt:TCPDSACKIgnoredNoUndo": {
        readableName: "TCP DSACK Ignored No Undo",
        description: "TCP Duplicate SACK blocks ignored when undo operations are not available during loss recovery.",
        desired: "lower",
      },
      "TcpExt:TCPSACKReneging": {
        readableName: "TCP SACK Reneging",
        description:
          "TCP Selective Acknowledgment reneging events where previously acknowledged data is later reported as missing.",
        desired: "lower",
      },
      "TcpExt:TCPLossProbes": {
        readableName: "TCP Loss Probes",
        description:
          "TCP loss probe packets sent to detect potential packet loss without waiting for timeout expiration.",
        desired: "lower",
      },
      "TcpExt:TCPZeroWindowDrop": {
        readableName: "TCP Zero Window Drop",
        description: "TCP packets dropped due to zero window conditions when the receiver cannot accept more data.",
        desired: "lower",
      },
      "TcpExt:TCPHystartDelayCwnd": {
        readableName: "TCP HyStart Delay Congestion Window",
        description:
          "TCP HyStart algorithm congestion window adjustments based on delay measurements for optimal throughput.",
        desired: "higher",
      },
      "TcpExt:TCPSackShiftFallback": {
        readableName: "TCP SACK Shift Fallback",
        description:
          "TCP SACK processing fallback when shift operations cannot be performed requiring alternative handling methods.",
        desired: "lower",
      },
      "TcpExt:TCPAbortOnLinger": {
        readableName: "TCP Abort On Linger",
        description:
          "TCP connections aborted during linger timeout when socket close operations cannot complete gracefully.",
        desired: "lower",
      },
      "TcpExt:TCPDSACKIgnoredDubious": {
        readableName: "TCP DSACK Ignored Dubious",
        description: "TCP Duplicate SACK blocks ignored due to dubious or suspicious sequence number information.",
        desired: "lower",
      },
      "TcpExt:TCPBacklogDrop": {
        readableName: "TCP Backlog Drop",
        description:
          "TCP connection requests dropped due to listen backlog queue overflow during high connection load.",
        desired: "lower",
      },
      "TcpExt:EmbryonicRsts": {
        readableName: "Embryonic RSTs",
        description: "TCP reset packets sent for connections in embryonic state before full establishment.",
        desired: "lower",
      },
      "TcpExt:TCPDeferAcceptDrop": {
        readableName: "TCP Defer Accept Drop",
        description:
          "TCP connections dropped due to deferred accept timeout when no data is received within the specified time.",
        desired: "lower",
      },
      "TcpExt:TCPTimeouts": {
        readableName: "TCP Timeouts",
        description:
          "Number of TCP connection timeouts indicating network congestion or connectivity issues that may impact application performance.",
        desired: "lower",
      },
      "TcpExt:TCPDSACKOfoRecv": {
        readableName: "TCP DSACK Out-of-Order Received",
        description:
          "TCP Duplicate SACK blocks received for out-of-order segments indicating network reordering issues.",
        desired: "lower",
      },
      "TcpExt:DelayedACKLocked": {
        readableName: "Delayed ACK Locked",
        description:
          "TCP delayed acknowledgments locked due to socket buffer constraints preventing immediate transmission.",
        desired: "lower",
      },
      "TcpExt:TCPWinProbe": {
        readableName: "TCP Window Probe",
        description:
          "TCP window probe packets sent to detect when the receiver's window opens after zero window conditions.",
        desired: "lower",
      },
      "TcpExt:TCPMemoryPressures": {
        readableName: "TCP Memory Pressures",
        description: "TCP memory pressure events when socket buffer allocation fails due to system memory constraints.",
        desired: "lower",
      },
      "TcpExt:OfoPruned": {
        readableName: "Out-of-Order Pruned",
        description:
          "TCP out-of-order segments pruned from the receive queue due to memory pressure or buffer limitations.",
        desired: "lower",
      },
      "TcpExt:TCPHystartTrainCwnd": {
        readableName: "TCP HyStart Train Congestion Window",
        description: "TCP HyStart algorithm congestion window training adjustments for optimal bandwidth utilization.",
        desired: "higher",
      },
      "TcpExt:TCPTimeWaitOverflow": {
        readableName: "TCP Time Wait Overflow",
        description: "TCP TIME-WAIT state overflow when too many connections are in the time-wait state.",
        desired: "lower",
      },
      "TcpExt:TCPWqueueTooBig": {
        readableName: "TCP Write Queue Too Big",
        description: "TCP write queue overflow when the transmission queue becomes too large for efficient processing.",
        desired: "lower",
      },
      "TcpExt:TCPMTUPFail": {
        readableName: "TCP MTU Probe Fail",
        description: "TCP Maximum Transmission Unit path discovery probe failures indicating network path MTU issues.",
        desired: "lower",
      },
      "TcpExt:TCPACKSkippedFinWait2": {
        readableName: "TCP ACK Skipped FIN-WAIT-2",
        description: "TCP acknowledgments skipped for connections in FIN-WAIT-2 state during connection termination.",
        desired: "lower",
      },
      "IpExt:InMcastOctets": {
        readableName: "IP Input Multicast Octets",
        description: "Total bytes received from multicast packets on the network interface.",
        desired: "depends",
      },
      "TcpExt:TCPMD5Failure": {
        readableName: "TCP MD5 Failure",
        description: "TCP MD5 signature authentication failures indicating security or configuration issues.",
        desired: "lower",
      },
      "TcpExt:TCPRenoRecovery": {
        readableName: "TCP Reno Recovery",
        description: "TCP Reno congestion control recovery operations during packet loss detection and handling.",
        desired: "higher",
      },
      "TcpExt:RcvPruned": {
        readableName: "Receive Pruned",
        description: "TCP receive buffer segments pruned due to memory pressure or buffer overflow conditions.",
        desired: "lower",
      },
      "TcpExt:TCPACKSkippedPAWS": {
        readableName: "TCP ACK Skipped PAWS",
        description:
          "TCP acknowledgments skipped due to Protection Against Wrapped Sequences timestamp validation failures.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinAckHMacFailure": {
        readableName: "MPTCP Join ACK HMAC Failure",
        description: "Multipath TCP join acknowledgment HMAC authentication failures during subflow establishment.",
        desired: "lower",
      },
      "TcpExt:LockDroppedIcmps": {
        readableName: "Lock Dropped ICMPs",
        description: "TCP ICMP messages dropped due to socket lock contention during processing.",
        desired: "lower",
      },
      "TcpExt:TCPSackRecoveryFail": {
        readableName: "TCP SACK Recovery Fail",
        description:
          "TCP Selective Acknowledgment recovery failures requiring fallback to alternative loss recovery mechanisms.",
        desired: "lower",
      },
      "TcpExt:TcpTimeoutRehash": {
        readableName: "TCP Timeout Rehash",
        description: "TCP connection timeout events requiring hash table rehashing for connection management.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynBackupRx": {
        readableName: "MPTCP Join SYN Backup Received",
        description: "MPTCP MP_JOIN SYN packets received on backup subflows for multipath connection establishment.",
        desired: "lower",
      },
      "TcpExt:TCPTSReorder": {
        readableName: "TCP Timestamp Reorder",
        description:
          "TCP packets received with timestamp reordering indicating out-of-order delivery or network issues.",
        desired: "lower",
      },
      "TcpExt:TWKilled": {
        readableName: "TIME-WAIT Killed",
        description: "TCP connections in TIME-WAIT state that were killed due to resource constraints or timeout.",
        desired: "lower",
      },
      "TcpExt:TCPAbortOnTimeout": {
        readableName: "TCP Abort On Timeout",
        description:
          "TCP connections aborted due to timeout events during data transmission or connection establishment.",
        desired: "lower",
      },
      "MPTcpExt:DSSNotMatching": {
        readableName: "MPTCP DSS Not Matching",
        description: "MPTCP Data Sequence Signal (DSS) options that do not match expected sequence numbers.",
        desired: "lower",
      },
      "TcpExt:PruneCalled": {
        readableName: "TCP Prune Called",
        description: "TCP socket buffer pruning operations called to free memory during high memory pressure.",
        desired: "lower",
      },
      "TcpExt:TCPDSACKIgnoredOld": {
        readableName: "TCP DSACK Ignored Old",
        description: "TCP Duplicate SACK blocks ignored because they reference old sequence numbers.",
        desired: "lower",
      },
      "TcpExt:TCPLossUndo": {
        readableName: "TCP Loss Undo",
        description: "TCP loss recovery operations that were undone due to spurious loss detection.",
        desired: "higher",
      },
      "TcpExt:ListenOverflows": {
        readableName: "TCP Listen Overflows",
        description: "TCP listen queue overflows when incoming connection requests exceed the listen backlog.",
        desired: "lower",
      },
      "MPTcpExt:MPCapableSYNRX": {
        readableName: "MPTCP Capable SYN Received",
        description: "MPTCP-capable SYN packets received indicating multipath TCP capability negotiation.",
        desired: "depends",
      },
      "TcpExt:TCPACKSkippedSeq": {
        readableName: "TCP ACK Skipped Sequence",
        description: "TCP ACK packets skipped due to sequence number gaps or out-of-order delivery.",
        desired: "lower",
      },
      "TcpExt:TCPSACKReorder": {
        readableName: "TCP SACK Reorder",
        description: "TCP Selective Acknowledgment packets indicating reordered segments.",
        desired: "lower",
      },
      "TcpExt:TCPAbortOnClose": {
        readableName: "TCP Abort On Close",
        description: "TCP connections aborted during the close process due to errors or timeouts.",
        desired: "lower",
      },
      "TcpExt:TCPRcvQDrop": {
        readableName: "TCP Receive Queue Drop",
        description: "TCP packets dropped from the receive queue due to buffer overflow or resource constraints.",
        desired: "lower",
      },
      "TcpExt:TCPFastOpenListenOverflow": {
        readableName: "TCP Fast Open Listen Overflow",
        description: "TCP Fast Open listen queue overflows when TFO requests exceed the queue capacity.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynAckBackupRx": {
        readableName: "MPTCP Join SYN-ACK Backup Received",
        description: "MPTCP MP_JOIN SYN-ACK packets received on backup subflows during connection establishment.",
        desired: "lower",
      },
      "IpExt:InECT1Pkts": {
        readableName: "IP ECT1 Packets In",
        description: "IP packets received with ECN Capable Transport (ECT1) marking for congestion notification.",
        desired: "depends",
      },
      "MPTcpExt:OFOQueue": {
        readableName: "MPTCP Out-of-Order Queue",
        description: "MPTCP packets queued due to out-of-order arrival across multiple subflows.",
        desired: "lower",
      },
      "TcpExt:TCPSackShifted": {
        readableName: "TCP SACK Shifted",
        description: "TCP SACK blocks that were shifted to optimize acknowledgment processing.",
        desired: "higher",
      },
      "MPTcpExt:DSSCorruptionFallback": {
        readableName: "MPTCP DSS Corruption Fallback",
        description: "MPTCP connections falling back to regular TCP due to Data Sequence Signal corruption.",
        desired: "lower",
      },
      "IpExt:OutOctets": {
        readableName: "Outgoing Octets",
        description:
          "Total number of bytes transmitted by the network interface including all protocol headers and payload data.",
        desired: "depends",
      },
      "TcpExt:TCPFastOpenActiveFail": {
        readableName: "TCP Fast Open Active Fail",
        description: "TCP Fast Open connection attempts that failed during active open.",
        desired: "lower",
      },
      "TcpExt:ListenDrops": {
        readableName: "TCP Listen Drops",
        description: "TCP connection requests dropped at the listen socket due to resource constraints.",
        desired: "lower",
      },
      "TcpExt:TCPFullUndo": {
        readableName: "TCP Full Undo",
        description: "TCP congestion control state fully undone due to spurious loss detection.",
        desired: "higher",
      },
      "MPTcpExt:RcvWndConflictUpdate": {
        readableName: "MPTCP Receive Window Conflict Update",
        description: "MPTCP receive window conflicts requiring updates across subflows.",
        desired: "lower",
      },
      "MPTcpExt:PortAdd": {
        readableName: "MPTCP Port Add",
        description: "MPTCP ADD_ADDR options sent to advertise additional addresses with port numbers.",
        desired: "depends",
      },
      "MPTcpExt:AddAddrDrop": {
        readableName: "MPTCP Add Address Drop",
        description: "MPTCP ADD_ADDR options dropped due to processing errors or resource constraints.",
        desired: "lower",
      },
      "MPTcpExt:MPFailRx": {
        readableName: "MPTCP Fail Received",
        description: "MPTCP MP_FAIL options received indicating subflow failure.",
        desired: "lower",
      },
      "MPTcpExt:RmAddrTx": {
        readableName: "MPTCP Remove Address Transmit",
        description: "MPTCP REMOVE_ADDR options transmitted to remove addresses from the connection.",
        desired: "depends",
      },
      "MPTcpExt:MPRstRx": {
        readableName: "MPTCP Reset Received",
        description: "MPTCP MP_RST options received indicating connection reset.",
        desired: "lower",
      },
      "MPTcpExt:InfiniteMapTx": {
        readableName: "MPTCP Infinite Map Transmit",
        description: "MPTCP infinite data sequence mappings transmitted for fallback scenarios.",
        desired: "lower",
      },
      "TcpExt:TCPAODroppedIcmps": {
        readableName: "TCP AO Dropped ICMPs",
        description: "TCP Authentication Option packets dropped due to ICMP message validation failures.",
        desired: "lower",
      },
      "MPTcpExt:RcvWndShared": {
        readableName: "MPTCP Receive Window Shared",
        description: "MPTCP receive window space shared across multiple subflows.",
        desired: "higher",
      },
      "MPTcpExt:MPFailTx": {
        readableName: "MPTCP Fail Transmit",
        description: "MPTCP MP_FAIL options transmitted to signal subflow failure.",
        desired: "lower",
      },
      "MPTcpExt:RmAddrTxDrop": {
        readableName: "MPTCP Remove Address Transmit Drop",
        description: "MPTCP REMOVE_ADDR options dropped during transmission.",
        desired: "lower",
      },
      "MPTcpExt:EchoAddTxDrop": {
        readableName: "MPTCP Echo Add Transmit Drop",
        description: "MPTCP ECHO ADD_ADDR options dropped during transmission.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinPortSynAckRx": {
        readableName: "MPTCP Join Port SYN-ACK Received",
        description: "MPTCP MP_JOIN SYN-ACK packets received with port information.",
        desired: "lower",
      },
      "MPTcpExt:AddAddrTx": {
        readableName: "MPTCP Add Address Transmit",
        description: "MPTCP ADD_ADDR options transmitted to advertise additional addresses.",
        desired: "depends",
      },
      "TcpExt:TCPAOKeyNotFound": {
        readableName: "TCP AO Key Not Found",
        description: "TCP Authentication Option packets rejected due to missing authentication keys.",
        desired: "lower",
      },
      "TcpExt:TCPAORequired": {
        readableName: "TCP AO Required",
        description: "TCP connections requiring Authentication Option that was not provided.",
        desired: "lower",
      },
      "MPTcpExt:RmAddrDrop": {
        readableName: "MPTCP Remove Address Drop",
        description: "MPTCP REMOVE_ADDR options dropped due to processing errors.",
        desired: "lower",
      },
      "MPTcpExt:MPCapableSYNTX": {
        readableName: "MPTCP Capable SYN Transmit",
        description: "MPTCP-capable SYN packets transmitted to initiate multipath connections.",
        desired: "depends",
      },
      "MPTcpExt:MPCapableSYNACKRX": {
        readableName: "MPTCP Capable SYN-ACK Received",
        description: "MPTCP-capable SYN-ACK packets received confirming multipath capability.",
        desired: "depends",
      },
      "TcpExt:TCPAOBad": {
        readableName: "TCP AO Bad",
        description: "TCP Authentication Option packets with invalid authentication signatures.",
        desired: "lower",
      },
      "MPTcpExt:AddAddrTxDrop": {
        readableName: "MPTCP Add Address Transmit Drop",
        description: "MPTCP ADD_ADDR options dropped during transmission due to errors.",
        desired: "lower",
      },
      "MPTcpExt:MPFallbackTokenInit": {
        readableName: "MPTCP Fallback Token Init",
        description: "MPTCP connections falling back to regular TCP during token initialization.",
        desired: "lower",
      },
      "MPTcpExt:DataCsumErr": {
        readableName: "MPTCP Data Checksum Error",
        description: "MPTCP data packets with checksum validation errors.",
        desired: "lower",
      },
      "MPTcpExt:RcvWndConflict": {
        readableName: "MPTCP Receive Window Conflict",
        description: "MPTCP receive window conflicts between subflows requiring resolution.",
        desired: "lower",
      },
      "MPTcpExt:MPFastcloseTx": {
        readableName: "MPTCP Fast Close Transmit",
        description: "MPTCP MP_FASTCLOSE options transmitted to rapidly close connections.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinPortAckRx": {
        readableName: "MPTCP Join Port ACK Received",
        description: "MPTCP MP_JOIN ACK packets received with port information during subflow establishment.",
        desired: "lower",
      },
      "MPTcpExt:SubflowStale": {
        readableName: "MPTCP Subflow Stale",
        description: "MPTCP subflows marked as stale due to inactivity or failure.",
        desired: "lower",
      },
      "MPTcpExt:RcvPruned": {
        readableName: "MPTCP Receive Pruned",
        description: "MPTCP receive buffer pruning operations to free memory during high pressure.",
        desired: "lower",
      },
      "MPTcpExt:MPCurrEstab": {
        readableName: "MPTCP Current Established",
        description: "Current number of MPTCP connections in established state.",
        desired: "depends",
      },
      "MPTcpExt:MPPrioRx": {
        readableName: "MPTCP Priority Received",
        description: "MPTCP MP_PRIO options received to change subflow priority.",
        desired: "depends",
      },
      "MPTcpExt:EchoAddTx": {
        readableName: "MPTCP Echo Add Transmit",
        description: "MPTCP ECHO ADD_ADDR options transmitted in response to ADD_ADDR.",
        desired: "depends",
      },
      "MPTcpExt:DSSNoMatchTCP": {
        readableName: "MPTCP DSS No Match TCP",
        description: "MPTCP Data Sequence Signal options that do not match TCP sequence numbers.",
        desired: "lower",
      },
      "TcpExt:TCPAOGood": {
        readableName: "TCP AO Good",
        description: "TCP Authentication Option packets with valid authentication signatures.",
        desired: "higher",
      },
      "MPTcpExt:MPRstTx": {
        readableName: "MPTCP Reset Transmit",
        description: "MPTCP MP_RST options transmitted to reset connections.",
        desired: "lower",
      },
      "MPTcpExt:MPCapableEndpAttempt": {
        readableName: "MPTCP Capable Endpoint Attempt",
        description: "MPTCP-capable endpoint connection attempts.",
        desired: "depends",
      },
      "MPTcpExt:SndWndShared": {
        readableName: "MPTCP Send Window Shared",
        description: "MPTCP send window space shared across multiple subflows.",
        desired: "higher",
      },
      "TcpExt:TCPPLBRehash": {
        readableName: "TCP PLB Rehash",
        description: "TCP Packet Load Balancer hash table rehashing operations for connection distribution.",
        desired: "lower",
      },
      "MPTcpExt:MismatchPortSynRx": {
        readableName: "MPTCP Mismatch Port SYN Received",
        description: "MPTCP MP_JOIN SYN packets received with mismatched port information.",
        desired: "lower",
      },
      "MPTcpExt:MPFastcloseRx": {
        readableName: "MPTCP Fast Close Received",
        description: "MPTCP MP_FASTCLOSE options received to rapidly close connections.",
        desired: "lower",
      },
      "MPTcpExt:MPPrioTx": {
        readableName: "MPTCP Priority Transmit",
        description: "MPTCP MP_PRIO options transmitted to change subflow priority.",
        desired: "depends",
      },
      "MPTcpExt:MPJoinPortSynRx": {
        readableName: "MPTCP Join Port SYN Received",
        description: "MPTCP MP_JOIN SYN packets received with port information for subflow establishment.",
        desired: "lower",
      },
      "MPTcpExt:MismatchPortAckRx": {
        readableName: "MPTCP Mismatch Port ACK Received",
        description: "MPTCP MP_JOIN ACK packets received with mismatched port information.",
        desired: "lower",
      },
      "MPTcpExt:SubflowRecover": {
        readableName: "MPTCP Subflow Recover",
        description: "MPTCP subflows recovered from failure or stale state.",
        desired: "higher",
      },
      "MPTcpExt:MPCapableSYNTXDrop": {
        readableName: "MPTCP Capable SYN Transmit Drop",
        description: "MPTCP-capable SYN packets dropped during transmission.",
        desired: "lower",
      },
      "MPTcpExt:MPCapableSYNTXDisabled": {
        readableName: "MPTCP Capable SYN Transmit Disabled",
        description: "MPTCP-capable SYN transmission disabled due to configuration or policy.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynTxBindErr": {
        readableName: "MPTCP Join SYN Transmit Bind Error",
        description: "MPTCP MP_JOIN SYN transmission failures due to socket binding errors.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynTxCreatSkErr": {
        readableName: "MPTCP Join SYN Transmit Create Socket Error",
        description: "MPTCP MP_JOIN SYN transmission failures due to socket creation errors.",
        desired: "lower",
      },
      "MPTcpExt:Blackhole": {
        readableName: "MPTCP Blackhole",
        description: "MPTCP connections experiencing blackhole conditions where packets are silently dropped.",
        desired: "lower",
      },
      "TcpExt:PAWSOldAck": {
        readableName: "PAWS Old ACK",
        description: "TCP Protection Against Wrapped Sequence numbers rejecting old ACK packets.",
        desired: "higher",
      },
      "MPTcpExt:MPJoinSynTxConnectErr": {
        readableName: "MPTCP Join SYN Transmit Connect Error",
        description: "MPTCP MP_JOIN SYN transmission failures due to connection errors.",
        desired: "lower",
      },
      "MPTcpExt:MPJoinSynTx": {
        readableName: "MPTCP Join SYN Transmit",
        description: "MPTCP MP_JOIN SYN packets transmitted to establish subflows.",
        desired: "depends",
      },
    },
  },
  kernel_config: {
    readableName: "Kernel Config",
    summary:
      'Kernel configs contain configuration options used when the running kernel was compiled. The data were collected from /boot/config* file. Value "y" means the module is compiled directly in the kernel, "not set"/"n" means the module is not compiled in the kernel, and "m" means the module is compiled as a loadable module.',
    fieldDescriptions: {},
  },
  sysctl: {
    readableName: "Sysctl Config",
    summary: "Sysctl contains runtime kernel parameters.",
    fieldDescriptions: {},
  },
  flamegraphs: {
    readableName: "Flamegraphs",
    summary:
      "Kernel profiling flamegraphs visualize the call stack hierarchies and the amount of CPU time consumed by different functions. It supports viewing in both the normal (bottom-top) and reverse (top-bottom) order.",
    fieldDescriptions: {},
  },
  perf_profile: {
    readableName: "Top Functions",
    summary:
      "Kernel profiling top functions are the text-based version of the flamegraphs and show the percentage of CPU time spent in each function. It only includes functions with at least 1% of CPU time.",
    fieldDescriptions: {},
  },
  java_profile: {
    readableName: "Java Profiling Heatmaps",
    summary:
      "Java profiling heatmaps show profiled CPU utilization, memory allocations, and wall clocks for JVMs running on the system at every second. For the legacy APerf version, only the flamegraph of CPU utilization across the whole recording period is available.",
    fieldDescriptions: {
      wall: {
        readableName: "Wall Clock Profiling",
        description: "",
      },
      alloc: {
        readableName: "Memory Allocation Profiling",
        description: "",
      },
      cpu: {
        readableName: "CPU Utilization Profiling",
        description: "",
      },
      legacy: {
        readableName: "Flamegraphs (legacy)",
        description: "",
      },
    },
  },
  hotline: {
    readableName: "Hotline",
    summary:
      "Hotline data uses the Statistical Profiling Extension (SPE) of Graviton cores to analyze branch and latency hotspot.",
    fieldDescriptions: {},
  },
  aperf_runlog: {
    readableName: "APerf Logs",
    summary: "APerf logs show the running log of APerf while recording.",
    fieldDescriptions: {},
  },
  aperf_stats: {
    readableName: "APerf Stats",
    summary:
      "APerf stats metrics measure the amount of time APerf spent on recording each data. Every graph contains the time of collecting the data from the system, the time of writing the data to the archive file, and the sum of both as the aggregate. The statistics of a metric graph accounts for the aggregate series.",
    defaultUnit: "Time (us)",
    fieldDescriptions: {
      aperf: {
        readableName: "Total collection time",
        description: "The total time in us for APerf to collect all data during one interval.",
        desired: "lower",
      },
      cpu_utilization: {
        readableName: "CPU utilization collection time",
        description: "The total time in us for APerf to collect the CPU utilization data during one interval.",
        desired: "lower",
      },
      perf_stat: {
        readableName: "PMU events collection time",
        description: "The total time in us for APerf to collect the PMU events data during one interval.",
        desired: "lower",
      },
      meminfo: {
        readableName: "Memory usage collection time",
        description: "The total time in us for APerf to collect the memory usage data during one interval.",
        desired: "lower",
      },
      vmstat: {
        readableName: "Virtual memory stats collection time",
        description: "The total time in us for APerf to collect the virtual memory stats data during one interval.",
        desired: "lower",
      },
      interrupts: {
        readableName: "Interrupts collection time",
        description: "The total time in us for APerf to collect the interrupts data during one interval.",
        desired: "lower",
      },
      diskstats: {
        readableName: "Disk stats collection time",
        description: "The total time in us for APerf to collect the disk stats data during one interval.",
        desired: "lower",
      },
      netstat: {
        readableName: "Network stats collection time",
        description: "The total time in us for APerf to collect the network stats data during one interval.",
        desired: "lower",
      },
      processes: {
        readableName: "Processes collection time",
        description: "The total time in us for APerf to collect the processes data during one interval.",
        desired: "lower",
      },
      flamegraphs: {
        readableName: "Flamegraphs collection time",
        description: "The total time in us for APerf to collect the kernel profiling flamegraphs during one interval.",
        desired: "lower",
      },
      perf_profile: {
        readableName: "Top functions collection time",
        description:
          "The total time in us for APerf to collect the kernel profiling top functions during one interval.",
        desired: "lower",
      },
    },
  },
};
