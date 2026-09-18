import { DataType } from "./types";
import {
  CPU_BACKEND_STALLS_INVESTIGATION,
  CPU_FRONTEND_STALLS_INVESTIGATION,
  CPU_UTILIZATION_OPTIMIZATION,
  DATA_FOOTPRINT_OPTIMIZATION,
  EC2_NETWORK_BANDWIDTH_ALLOWANCE_RECOMMENDATIONS,
  EC2_NETWORK_LINK_LOCAL_ALLOWANCE_RECOMMENDATIONS,
  EC2_NETWORK_PPS_ALLOWANCE_RECOMMENDATIONS,
  EC2_NETWORK_TRACKED_CONNECTIONS_ALLOWANCE_RECOMMENDATIONS,
  INSTRUCTION_FOOTPRINT_OPTIMIZATION,
  IOWAIT_TIME_OPTIMIZATION,
  LOW_IPC_INVESTIGATION,
  LSE_OPTIMIZATION,
  NETWORK_USAGE_INVESTIGATION,
  TLB_MISS_OPTIMIZATION,
} from "./data-descriptions/optimization-guides";
import { KERNEL_CONFIG_DATA_DESCRIPTION } from "./data-descriptions/kernel-config";
import { SYSCTL_DATA_DESCRIPTION } from "./data-descriptions/sysctl";
import { MEM_SETTINGS_DATA_DESCRIPTION } from "./data-descriptions/mem-settings";
import { MEMINFO_DATA_DESCRIPTION } from "./data-descriptions/meminfo";
import { VMSTAT_DATA_DESCRIPTION } from "./data-descriptions/vmstat";

export type DesiredValue = "higher" | "lower" | "moderate" | "fixed" | "depends";

export interface DataDescription {
  /**
   * Human-readable name of the data type - used as the data page's title
   */
  readonly readableName: string;
  /**
   * Short description of the data type
   */
  readonly summary: string;
  /**
   * (For time-series data only) the default unit to be used in all metric graphs
   */
  readonly defaultUnit?: string;
  /**
   * The default helpful links to be shown in all help panels
   */
  readonly defaultHelpfulLinks?: string[];
  /**
   * Configuration of every included data
   */
  readonly fieldDescriptions: {
    [key in string]: {
      /**
       * Human-readable name of the data - used as the help panel's title
       */
      readonly readableName: string;
      /**
       * The content of the help panel
       */
      readonly description: string;
      /**
       * (For time-series data only) The unit to be used in the metric graph (overrides the data type's default unit)
       */
      readonly unit?: string;
      /**
       * (For time-series data only) Whether the metric should be higher or lower for better performance
       */
      readonly desired?: DesiredValue;
      /**
       * Optimization guides in markdown source - all guides will be concatenated with an empty line. Look for
       * the appropriate optimization guides in data-descriptions/optimization-guides.ts
       */
      readonly optimization?: string[];
      /**
       * The list of helpful links to be shown in the help panel (extends the data type's default helpful links)
       */
      readonly helpfulLinks?: string[];
    };
  };
}

/**
 * Descriptions of the per-size HugeTLB pool counters that the meminfo collector appends from
 * /sys/kernel/mm/hugepages/hugepages-<size>kB.
 */
/**
 * Descriptions of the per-size transparent huge page counters that the vmstat collector appends from
 * /sys/kernel/mm/transparent_hugepage/hugepages-<size>kB/stats.
 */
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
    defaultHelpfulLinks: [
      "https://aws.github.io/graviton/perfrunbook/system-load-and-compute-headroom.html",
      "https://aws.github.io/graviton/perfrunbook/debug_system_perf.html#check-cpu-usage",
      "https://man7.org/linux/man-pages/man5/proc_stat.5.html",
    ],
    fieldDescriptions: {
      aggregate: {
        readableName: "Total CPU Utilization",
        description:
          "Percentage of CPU time spent on all activities (across all CPUs for each type). The aggregate series represents the total CPU utilization. Note that the performance of Graviton instances increases near-linearly with CPU utilization.",
        desired: "higher",
        optimization: [CPU_UTILIZATION_OPTIMIZATION],
      },
      idle: {
        readableName: "CPU Idle Time",
        description: "Percentage of CPU time spent idle.",
        desired: "lower",
        optimization: [CPU_UTILIZATION_OPTIMIZATION],
      },
      iowait: {
        readableName: "CPU I/O Wait Time",
        description: "Percentage of CPU time spent waiting for I/O operations to complete.",
        desired: "lower",
        optimization: [IOWAIT_TIME_OPTIMIZATION],
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
        optimization: [CPU_UTILIZATION_OPTIMIZATION],
      },
    },
  },
  processes: {
    readableName: "Processes",
    summary:
      "Processes metrics monitor usage of various resources for processes running on the system during APerf collection. The data were collected and computed from the system pseudo-files /proc/<pid>/stat. Every metric graph contains the top 16 processes in the highest average usage of the corresponding resource. The stats of a metric graph accounts for the process with the highest average.",
    defaultUnit: "Count",
    defaultHelpfulLinks: ["https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html"],
    fieldDescriptions: {
      user_space_time: {
        readableName: "User Space Time (utime)",
        description:
          "The aggregate CPU time spent executing application code for each process. The values are represented as the equivalent number of cores consumed by the process.",
        desired: "higher",
        unit: "Number of cores",
      },
      kernel_space_time: {
        readableName: "Kernel Space Time (stime)",
        description:
          "The aggregate CPU time spent executing in kernel mode (system calls). The values are represented as the equivalent number of cores consumed by the process.",
        desired: "lower",
        unit: "Number of cores",
      },
      number_threads: {
        readableName: "Number of Threads (num_threads)",
        description: "The number of threads spawned by a process.",
        desired: "depends",
      },
      virtual_memory_size: {
        readableName: "Virtual Memory Size (vsize)",
        description: "Total virtual memory used by a process.",
        desired: "lower",
        unit: "Bytes",
      },
      resident_set_size_bytes: {
        readableName: "Resident Set Size Bytes (rss)",
        description: "Physical memory in bytes used by a process. Converted from pages using sysconf(_SC_PAGESIZE).",
        desired: "lower",
        unit: "Bytes",
      },
      resident_set_size: {
        readableName: "Resident Set Size (rss)",
        description: "Physical memory in number of pages used by a process. Multiply by page size to convert to bytes.",
        desired: "lower",
        unit: "Pages",
      },
      number_processes: {
        readableName: "Number of Processes",
        description: "System-wide count of processes with a readable /proc/<pid>/stat entry at each collection sample.",
        desired: "depends",
        unit: "Count",
      },
    },
  },
  perf_stat: {
    readableName: "PMU Events",
    summary:
      "PMU metrics collect and compute the PMU (Performance Monitoring Unit) counters, which track hardware-level events, across all CPUs. Every graph corresponds to a metric computed using one or more PMU counters for every CPU, as well as the aggregate (average) of all CPUs. The statistics of a metric graph accounts for its aggregate series.",
    defaultUnit: "Counts",
    defaultHelpfulLinks: ["https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html"],
    fieldDescriptions: {
      "data-tlb-mpki": {
        readableName: "Data TLB Misses per Thousand Instructions",
        description:
          "Translation Lookaside Buffer (TLB) misses for data accesses per thousand instructions, indicating how often the CPU has to perform extra stalls to translate a virtual address into physical address before issuing a load/store to the memory system.",
        desired: "lower",
        optimization: [TLB_MISS_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/DTLB-Effectiveness-metrics-for-Neoverse-N3?lang=en#md544-dtlb_effectiveness_mg__l1d_tlb_mpki_m_DTLB_Effectiveness",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-back-end-stalls",
        ],
      },
      "data-tlb-tw-pki": {
        readableName: "Data TLB Table Walk per Thousand Instructions",
        description:
          "Translation Lookaside Buffer (TLB) table walks for data accesses per thousand instructions. It is triggered upon a cache miss in the data TLB to translate virtual addresses in a load/store instruction into physical ones, and multiple traversals in the OS-build page table were required to find the correct physical address. It can lead to much higher latency for some memory accesses.",
        desired: "lower",
        optimization: [TLB_MISS_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/MPKI-metrics-for-Neoverse-N3?lang=en#md407-mpki_mg__dtlb_mpki_m_MPKI",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-back-end-stalls",
        ],
      },
      "l3-mpki": {
        readableName: "L3 Cache Misses per Thousand Instructions",
        description:
          "Level 3 cache misses per thousand instructions executed, indicating how often the CPU has to access DRAM. A higher number means more DRAM bandwidth will be consumed.",
        desired: "lower",
        optimization: [DATA_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/MPKI-metrics-for-Neoverse-N3?lang=en#md407-mpki_mg__ll_cache_read_mpki_m_MPKI",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html?highlight=l3-mpki#drill-down-back-end-stalls",
        ],
      },
      "branch-mpki": {
        readableName: "Branch Misses per Thousand Instructions",
        description:
          "Number of branch prediction misses per thousand instructions indicating CPU pipeline efficiency and code predictability.",
        desired: "lower",
        optimization: [INSTRUCTION_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/MPKI-metrics-for-Neoverse-N3?lang=en#md407-mpki_mg__branch_mpki_m_MPKI",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-front-end-stalls",
        ],
      },
      "inst-tlb-tw-pki": {
        readableName: "Instruction TLB Table Walk per Thousand Instructions",
        description:
          "Translation Lookaside Buffer (TLB) table walks for instruction fetches per thousand instructions. It is triggered upon a cache miss in the instruction TLB to translate virtual instruction addresses into physical ones, and multiple traversals in the OS-built page table were required to find the correct physical address. It indicates code size issues and poor code locality.",
        desired: "lower",
        optimization: [INSTRUCTION_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/MPKI-metrics-for-Neoverse-N3?lang=en#md407-mpki_mg__itlb_mpki_m_MPKI",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-front-end-stalls",
        ],
      },
      "inst-tlb-mpki": {
        readableName: "Instruction TLB Misses per Thousand Instructions",
        description:
          "Translation Lookaside Buffer (TLB) misses for code instructions per thousand instructions. When the instruction footprint is too large, the TLB is filled, and the CPU needs to query the OS-built page table in memory to translate the virtual instruction addresses to physical ones.",
        desired: "lower",
        optimization: [INSTRUCTION_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/ITLB-Effectiveness-metrics-for-Neoverse-N3?lang=en#md549-itlb_effectiveness_mg__l1i_tlb_mpki_m_ITLB_Effectiveness",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-front-end-stalls",
        ],
      },
      "stall-frontend-pkc": {
        readableName: "Frontend Stall per Thousand Cycles",
        description:
          "Cycle count that were stalled due to resource constraints in the frontend unit of the processor per thousand cycles. CPU frontend is responsible for fetching, predicting, and decoding instructions into micro-operations, before they are sent to the backend for execution.",
        desired: "lower",
        optimization: [CPU_FRONTEND_STALLS_INVESTIGATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/Cycle-Accounting-metrics-for-Neoverse-N3?lang=en#md523-cycle_accounting_mg__frontend_stalled_cycles_m_Cycle_Accounting",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#top-down-method-to-debug-hardware-performance",
        ],
      },
      "data-l1-mpki": {
        readableName: "Data L1 Cache Misses per Thousand Instructions",
        description:
          "Level 1 data cache misses per thousand instructions, indicating data access patterns and cache efficiency for frequently accessed data.",
        desired: "lower",
        optimization: [DATA_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/L1D-Cache-Effectiveness-metrics-for-Neoverse-N3?lang=en#md553-l1d_cache_effectiveness_mg__l1d_cache_mpki_m_L1D_Cache_Effectiveness",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-back-end-stalls",
        ],
      },
      "data-rd-tlb-tw-pki": {
        readableName: "Data Read TLB Table Walk per Thousand Instructions",
        description:
          "Translation Lookaside Buffer table walks for data read operations per thousand instructions. It is triggered upon a cache miss in the data TLB to translate virtual addresses in a load instruction into physical ones, and multiple traversals in the OS-build page table were required to find the correct physical address. Higher values indicate memory management overhead for read accesses.",
        desired: "lower",
        optimization: [TLB_MISS_OPTIMIZATION],
      },
      "data-st-tlb-tw-pki": {
        readableName: "Data Store TLB Table Walk per Thousand Instructions",
        description:
          "Translation Lookaside Buffer table walks for data store operations per thousand instructions. It is triggered upon a cache miss in the data TLB to translate virtual addresses in a store instruction into physical ones, and multiple traversals in the OS-build page table were required to find the correct physical address. Higher values indicate memory management overhead for write accesses.",
        desired: "lower",
        optimization: [TLB_MISS_OPTIMIZATION],
      },
      "inst-l1-mpki": {
        readableName: "Instruction L1 Cache Misses per Thousand Instructions",
        description:
          "Level 1 instruction cache misses per thousand instructions indicating instruction fetch efficiency and code locality patterns.",
        desired: "lower",
        optimization: [INSTRUCTION_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/L1I-Cache-Effectiveness-metrics-for-Neoverse-N3?lang=en#md558-l1i_cache_effectiveness_mg__l1i_cache_mpki_m_L1I_Cache_Effectiveness",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-front-end-stalls",
        ],
      },
      ipc: {
        readableName: "Instructions Per Cycle",
        description:
          "Average number of instructions executed per CPU clock cycle indicating overall CPU utilization efficiency and performance.",
        desired: "higher",
        optimization: [LOW_IPC_INVESTIGATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/General-metrics-for-Neoverse-N3?lang=en#md420-general_mg__ipc_m_General",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#top-down-method-to-debug-hardware-performance",
        ],
      },
      "turbo-ratio": {
        readableName: "Turbo Ratio",
        description:
          "Ratio of the core clock cycles actually elapsed to the cycles that would have elapsed at the processor's nominal (base) frequency, for the time the core was not halted. A value of 1.0 means the core ran at its nominal frequency, and x86 cores commonly boost up to about 1.5. Multiply by the nominal frequency of the instance type to obtain the absolute frequency in GHz. This matters when comparing metrics such as IPC across instance types, since a core that boosts higher completes more work per unit of time even at the same IPC.",
        desired: "depends",
        unit: "Ratio",
        helpfulLinks: ["https://aws.amazon.com/ec2/instance-types/"],
      },
      "data-rd-tlb-mpki": {
        readableName: "Data Read TLB Misses per Thousand Instructions",
        description:
          "Translation Lookaside Buffer misses for data read operations per thousand instructions indicating memory access patterns for read operations.",
        desired: "lower",
        optimization: [TLB_MISS_OPTIMIZATION],
      },
      "l2-mpki": {
        readableName: "L2 Cache Misses per Thousand Instructions",
        description: "Number of level 2 cache accesses missed per thousand instructions executed.",
        desired: "lower",
        optimization: [DATA_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/L2-Cache-Effectiveness-metrics-for-Neoverse-N3?lang=en#md550-l2_cache_effectiveness_mg__l2_cache_mpki_m_L2_Cache_Effectiveness",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-back-end-stalls",
        ],
      },
      "data-st-tlb-mpki": {
        readableName: "Data Store TLB Misses per Thousand Instructions",
        description:
          "Number of Translation Lookaside Buffer misses for data store operations per thousand instructions.",
        desired: "lower",
        optimization: [TLB_MISS_OPTIMIZATION],
      },
      "stall-backend-pkc": {
        readableName: "Backend Stall per Thousand Cycles",
        description:
          "Cycle count that were stalled due to resource constraints in the backend unit of the processor per thousand cycles. CPU backend is responsible for executing the micro-operations decoded from instructions by the frontend, and producing memory side effects.",
        desired: "lower",
        optimization: [CPU_BACKEND_STALLS_INVESTIGATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109530/0100/Metrics-by-metric-group-in-Neoverse-N3/Cycle-Accounting-metrics-for-Neoverse-N3?lang=en#md523-cycle_accounting_mg__backend_stalled_cycles_m_Cycle_Accounting",
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#top-down-method-to-debug-hardware-performance",
        ],
      },
      "code-sparsity": {
        readableName: "Code Sparsity",
        description:
          "Code sparsity is a measure of how compact the instruction code is packed and how closely related code is placed. Lower sparsity helps branch prediction and the cache subsystem.",
        desired: "lower",
        optimization: [INSTRUCTION_FOOTPRINT_OPTIMIZATION],
        helpfulLinks: [
          "https://aws.github.io/graviton/perfrunbook/debug_hw_perf.html#drill-down-front-end-stalls",
          "https://aws.github.io/graviton/perfrunbook/optimization_recommendation.html#optimizing-for-large-instruction-footprint",
        ],
      },
      "strex-spec-pki": {
        readableName: "Store Exclusive per Thousand Instructions",
        description:
          "The number of store exclusive operations that have been speculatively executed per thousand instructions. STREX is an old-style atomic instruction and part of the load-store pair. It is less efficient than the newer LSE instructions, which perform atomic operations with a single instruction and hardware managed atomicity. For workloads that involve heavy lock contentions, switching to LSE instructions could lead to significant performance improvement.",
        desired: "lower",
        optimization: [LSE_OPTIMIZATION],
        helpfulLinks: [
          "https://developer.arm.com/documentation/109528/0200/PMU-events-by-functional-group-in-Neoverse-V2/Spec-Operation--SPEC-OPERATION--events-for-Neoverse-V2?lang=en#md761-spec_operation_fg__STREX_SPEC_e",
          "https://aws.github.io/graviton/c-c++.html#large-system-extensions-lse",
        ],
      },
      mux_counter_schedule_rate: {
        readableName: "Average Counter Collection Schedule Rate",
        description:
          "The percentage of collection time when a counter was actually scheduled on a core. The value shows the level of multiplexing during collection, which impacts the accuracy of the PMU data and consumes additional CPU time. 100% means there were no multiplexing and all PMU counters to be collected fitted in the available PMU registers.",
        unit: "Average Counter Schedule Rate (%)",
        desired: "higher",
        helpfulLinks: [
          "https://developer.arm.com/community/arm-community-blogs/b/architectures-and-processors-blog/posts/p2-perf-pmu-feature-armv8-cpus",
        ],
      },
    },
  },
  memalloc: {
    readableName: "Memory Allocation",
    summary:
      "Memory allocation metrics show buddy allocator free blocks, page type information, pageblock counts, and slab allocator statistics. BuddyInfo shows total free blocks at each order. PageType shows free blocks by migration type. PageBlocks shows aggregated pageblock counts. SlabInfo shows kernel slab allocator metrics for each slab cache. Data collected from /proc/buddyinfo, /proc/pagetypeinfo, and /proc/slabinfo.",
    defaultUnit: "Count",
    defaultHelpfulLinks: [
      "https://www.kernel.org/doc/html/latest/admin-guide/mm/concepts.html#buddy-allocator",
      "https://www.kernel.org/doc/html/latest/admin-guide/mm/concepts.html#migrate-types",
      "https://www.kernel.org/doc/html/latest/vm/slub.html",
    ],
    fieldDescriptions: {
      "BuddyInfo order_0 (4KB)": {
        readableName: "Free 4KB Blocks",
        description:
          "Number of free 4KB (2^0 pages) memory blocks available in the buddy allocator. These are the smallest allocation units and most commonly used for single-page allocations.",
        desired: "higher",
      },
      "BuddyInfo order_1 (8KB)": {
        readableName: "Free 8KB Blocks",
        description:
          "Number of free 8KB (2^1 pages) contiguous memory blocks. Used for small multi-page allocations requiring physically contiguous memory.",
        desired: "higher",
      },
      "BuddyInfo order_2 (16KB)": {
        readableName: "Free 16KB Blocks",
        description:
          "Number of free 16KB (2^2 pages) contiguous memory blocks. Common for kernel data structures and small DMA buffers.",
        desired: "higher",
      },
      "BuddyInfo order_3 (32KB)": {
        readableName: "Free 32KB Blocks",
        description:
          "Number of free 32KB (2^3 pages) contiguous memory blocks. Used for medium-sized kernel allocations and device drivers.",
        desired: "higher",
      },
      "BuddyInfo order_4 (64KB)": {
        readableName: "Free 64KB Blocks",
        description:
          "Number of free 64KB (2^4 pages) contiguous memory blocks. Important for larger DMA operations and kernel subsystems.",
        desired: "higher",
      },
      "BuddyInfo order_5 (128KB)": {
        readableName: "Free 128KB Blocks",
        description:
          "Number of free 128KB (2^5 pages) contiguous memory blocks. Critical for large kernel allocations and high-performance I/O.",
        desired: "higher",
      },
      "BuddyInfo order_6 (256KB)": {
        readableName: "Free 256KB Blocks",
        description:
          "Number of free 256KB (2^6 pages) contiguous memory blocks. Essential for large contiguous allocations and memory-intensive operations.",
        desired: "higher",
      },
      "BuddyInfo order_7 (512KB)": {
        readableName: "Free 512KB Blocks",
        description:
          "Number of free 512KB (2^7 pages) contiguous memory blocks. Important indicator of memory fragmentation at medium scales.",
        desired: "higher",
      },
      "BuddyInfo order_8 (1MB)": {
        readableName: "Free 1MB Blocks",
        description:
          "Number of free 1MB (2^8 pages) contiguous memory blocks. Key metric for large contiguous allocations and huge page support.",
        desired: "higher",
      },
      "BuddyInfo order_9 (2MB)": {
        readableName: "Free 2MB Blocks",
        description:
          "Number of free 2MB (2^9 pages) contiguous memory blocks. Critical for transparent huge pages and large memory allocations.",
        desired: "higher",
      },
      "BuddyInfo order_10 (4MB)": {
        readableName: "Free 4MB Blocks",
        description:
          "Number of free 4MB (2^10 pages) contiguous memory blocks. Largest buddy allocator order, indicates excellent memory contiguity and low fragmentation.",
        desired: "higher",
      },
      "PageType Unmovable - order_0 (4KB)": {
        readableName: "Free 4KB Unmovable Blocks",
        description:
          "Free 4KB blocks reserved for unmovable pages that cannot be migrated or reclaimed, typically used for kernel data structures.",
        desired: "moderate",
      },
      "PageType Unmovable - order_1 (8KB)": {
        readableName: "Free 8KB Unmovable Blocks",
        description: "Free 8KB unmovable blocks for kernel allocations that must remain at fixed physical addresses.",
        desired: "moderate",
      },
      "PageType Unmovable - order_2 (16KB)": {
        readableName: "Free 16KB Unmovable Blocks",
        description: "Free 16KB unmovable blocks for larger kernel structures requiring fixed physical memory.",
        desired: "moderate",
      },
      "PageType Unmovable - order_3 (32KB)": {
        readableName: "Free 32KB Unmovable Blocks",
        description: "Free 32KB unmovable blocks for substantial kernel allocations that cannot be relocated.",
        desired: "moderate",
      },
      "PageType Unmovable - order_4 (64KB)": {
        readableName: "Free 64KB Unmovable Blocks",
        description: "Free 64KB unmovable blocks for large kernel data structures and device drivers.",
        desired: "moderate",
      },
      "PageType Unmovable - order_5 (128KB)": {
        readableName: "Free 128KB Unmovable Blocks",
        description:
          "Free 128KB unmovable blocks for very large kernel allocations requiring physical address stability.",
        desired: "moderate",
      },
      "PageType Unmovable - order_6 (256KB)": {
        readableName: "Free 256KB Unmovable Blocks",
        description: "Free 256KB unmovable blocks for substantial kernel subsystem allocations.",
        desired: "moderate",
      },
      "PageType Unmovable - order_7 (512KB)": {
        readableName: "Free 512KB Unmovable Blocks",
        description: "Free 512KB unmovable blocks indicating availability of large contiguous kernel memory.",
        desired: "moderate",
      },
      "PageType Unmovable - order_8 (1MB)": {
        readableName: "Free 1MB Unmovable Blocks",
        description: "Free 1MB unmovable blocks for very large kernel allocations and specialized subsystems.",
        desired: "moderate",
      },
      "PageType Unmovable - order_9 (2MB)": {
        readableName: "Free 2MB Unmovable Blocks",
        description: "Free 2MB unmovable blocks for huge kernel allocations requiring fixed physical addresses.",
        desired: "moderate",
      },
      "PageType Unmovable - order_10 (4MB)": {
        readableName: "Free 4MB Unmovable Blocks",
        description: "Free 4MB unmovable blocks, the largest unmovable allocation size available.",
        desired: "moderate",
      },
      "PageType Movable - order_0 (4KB)": {
        readableName: "Free 4KB Movable Blocks",
        description:
          "Free 4KB blocks for movable pages that can be migrated to reduce fragmentation, typically used for user-space allocations.",
        desired: "higher",
      },
      "PageType Movable - order_1 (8KB)": {
        readableName: "Free 8KB Movable Blocks",
        description: "Free 8KB movable blocks that can be relocated during memory compaction.",
        desired: "higher",
      },
      "PageType Movable - order_2 (16KB)": {
        readableName: "Free 16KB Movable Blocks",
        description: "Free 16KB movable blocks available for user-space allocations and page migration.",
        desired: "higher",
      },
      "PageType Movable - order_3 (32KB)": {
        readableName: "Free 32KB Movable Blocks",
        description: "Free 32KB movable blocks that support memory defragmentation through migration.",
        desired: "higher",
      },
      "PageType Movable - order_4 (64KB)": {
        readableName: "Free 64KB Movable Blocks",
        description: "Free 64KB movable blocks for larger user allocations that can be compacted.",
        desired: "higher",
      },
      "PageType Movable - order_5 (128KB)": {
        readableName: "Free 128KB Movable Blocks",
        description: "Free 128KB movable blocks critical for transparent huge page allocations.",
        desired: "higher",
      },
      "PageType Movable - order_6 (256KB)": {
        readableName: "Free 256KB Movable Blocks",
        description: "Free 256KB movable blocks supporting large contiguous user-space allocations.",
        desired: "higher",
      },
      "PageType Movable - order_7 (512KB)": {
        readableName: "Free 512KB Movable Blocks",
        description: "Free 512KB movable blocks indicating good memory contiguity for large allocations.",
        desired: "higher",
      },
      "PageType Movable - order_8 (1MB)": {
        readableName: "Free 1MB Movable Blocks",
        description: "Free 1MB movable blocks essential for huge page support and large memory mappings.",
        desired: "higher",
      },
      "PageType Movable - order_9 (2MB)": {
        readableName: "Free 2MB Movable Blocks",
        description: "Free 2MB movable blocks critical for transparent huge pages and low memory fragmentation.",
        desired: "higher",
      },
      "PageType Movable - order_10 (4MB)": {
        readableName: "Free 4MB Movable Blocks",
        description:
          "Free 4MB movable blocks, the largest movable allocation size, indicating excellent memory health.",
        desired: "higher",
      },
      "PageType Reclaimable - order_0 (4KB)": {
        readableName: "Free 4KB Reclaimable Blocks",
        description:
          "Free 4KB blocks for reclaimable pages that can be freed under memory pressure, such as file caches and slab caches.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_1 (8KB)": {
        readableName: "Free 8KB Reclaimable Blocks",
        description: "Free 8KB reclaimable blocks that can be reclaimed when memory is needed.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_2 (16KB)": {
        readableName: "Free 16KB Reclaimable Blocks",
        description: "Free 16KB reclaimable blocks for kernel caches that can be freed during memory pressure.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_3 (32KB)": {
        readableName: "Free 32KB Reclaimable Blocks",
        description: "Free 32KB reclaimable blocks available for cache allocations that can be reclaimed.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_4 (64KB)": {
        readableName: "Free 64KB Reclaimable Blocks",
        description: "Free 64KB reclaimable blocks for larger cache structures that can be freed when needed.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_5 (128KB)": {
        readableName: "Free 128KB Reclaimable Blocks",
        description: "Free 128KB reclaimable blocks for substantial cache allocations.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_6 (256KB)": {
        readableName: "Free 256KB Reclaimable Blocks",
        description: "Free 256KB reclaimable blocks for large kernel cache structures.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_7 (512KB)": {
        readableName: "Free 512KB Reclaimable Blocks",
        description: "Free 512KB reclaimable blocks indicating substantial reclaimable memory available.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_8 (1MB)": {
        readableName: "Free 1MB Reclaimable Blocks",
        description: "Free 1MB reclaimable blocks for very large cache allocations.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_9 (2MB)": {
        readableName: "Free 2MB Reclaimable Blocks",
        description: "Free 2MB reclaimable blocks for huge cache structures that can be freed.",
        desired: "moderate",
      },
      "PageType Reclaimable - order_10 (4MB)": {
        readableName: "Free 4MB Reclaimable Blocks",
        description: "Free 4MB reclaimable blocks, the largest reclaimable allocation size.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_0 (4KB)": {
        readableName: "Free 4KB HighAtomic Blocks",
        description:
          "Free 4KB blocks reserved for high-priority atomic allocations that cannot fail or sleep, used in interrupt contexts.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_1 (8KB)": {
        readableName: "Free 8KB HighAtomic Blocks",
        description: "Free 8KB high-priority atomic blocks for critical interrupt-context allocations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_2 (16KB)": {
        readableName: "Free 16KB HighAtomic Blocks",
        description: "Free 16KB high-priority atomic blocks for larger atomic allocations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_3 (32KB)": {
        readableName: "Free 32KB HighAtomic Blocks",
        description: "Free 32KB high-priority atomic blocks for substantial atomic allocations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_4 (64KB)": {
        readableName: "Free 64KB HighAtomic Blocks",
        description: "Free 64KB high-priority atomic blocks for large atomic operations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_5 (128KB)": {
        readableName: "Free 128KB HighAtomic Blocks",
        description: "Free 128KB high-priority atomic blocks for very large atomic allocations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_6 (256KB)": {
        readableName: "Free 256KB HighAtomic Blocks",
        description: "Free 256KB high-priority atomic blocks reserved for critical operations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_7 (512KB)": {
        readableName: "Free 512KB HighAtomic Blocks",
        description: "Free 512KB high-priority atomic blocks for substantial atomic operations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_8 (1MB)": {
        readableName: "Free 1MB HighAtomic Blocks",
        description: "Free 1MB high-priority atomic blocks for very large atomic allocations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_9 (2MB)": {
        readableName: "Free 2MB HighAtomic Blocks",
        description: "Free 2MB high-priority atomic blocks for huge atomic operations.",
        desired: "moderate",
      },
      "PageType HighAtomic - order_10 (4MB)": {
        readableName: "Free 4MB HighAtomic Blocks",
        description: "Free 4MB high-priority atomic blocks, the largest atomic allocation size.",
        desired: "moderate",
      },
      "PageType CMA - order_0 (4KB)": {
        readableName: "Free 4KB CMA Blocks",
        description:
          "Free 4KB blocks in Contiguous Memory Allocator reserved for devices requiring physically contiguous memory.",
        desired: "higher",
      },
      "PageType CMA - order_1 (8KB)": {
        readableName: "Free 8KB CMA Blocks",
        description: "Free 8KB CMA blocks for small contiguous device allocations.",
        desired: "higher",
      },
      "PageType CMA - order_2 (16KB)": {
        readableName: "Free 16KB CMA Blocks",
        description: "Free 16KB CMA blocks for device DMA operations.",
        desired: "higher",
      },
      "PageType CMA - order_3 (32KB)": {
        readableName: "Free 32KB CMA Blocks",
        description: "Free 32KB CMA blocks for medium-sized contiguous device memory.",
        desired: "higher",
      },
      "PageType CMA - order_4 (64KB)": {
        readableName: "Free 64KB CMA Blocks",
        description: "Free 64KB CMA blocks for larger device DMA buffers.",
        desired: "higher",
      },
      "PageType CMA - order_5 (128KB)": {
        readableName: "Free 128KB CMA Blocks",
        description: "Free 128KB CMA blocks for substantial device memory requirements.",
        desired: "higher",
      },
      "PageType CMA - order_6 (256KB)": {
        readableName: "Free 256KB CMA Blocks",
        description: "Free 256KB CMA blocks for large contiguous device allocations.",
        desired: "higher",
      },
      "PageType CMA - order_7 (512KB)": {
        readableName: "Free 512KB CMA Blocks",
        description: "Free 512KB CMA blocks for very large device memory requirements.",
        desired: "higher",
      },
      "PageType CMA - order_8 (1MB)": {
        readableName: "Free 1MB CMA Blocks",
        description: "Free 1MB CMA blocks for huge device DMA operations.",
        desired: "higher",
      },
      "PageType CMA - order_9 (2MB)": {
        readableName: "Free 2MB CMA Blocks",
        description: "Free 2MB CMA blocks for massive contiguous device allocations.",
        desired: "higher",
      },
      "PageType CMA - order_10 (4MB)": {
        readableName: "Free 4MB CMA Blocks",
        description: "Free 4MB CMA blocks, the largest CMA allocation size for device memory.",
        desired: "higher",
      },
      "PageType Isolate - order_0 (4KB)": {
        readableName: "Free 4KB Isolate Blocks",
        description:
          "Free 4KB blocks in isolated pageblocks used during memory compaction and page migration operations.",
        desired: "lower",
      },
      "PageType Isolate - order_1 (8KB)": {
        readableName: "Free 8KB Isolate Blocks",
        description: "Free 8KB isolated blocks temporarily removed from normal allocation pools.",
        desired: "lower",
      },
      "PageType Isolate - order_2 (16KB)": {
        readableName: "Free 16KB Isolate Blocks",
        description: "Free 16KB isolated blocks used during memory management operations.",
        desired: "lower",
      },
      "PageType Isolate - order_3 (32KB)": {
        readableName: "Free 32KB Isolate Blocks",
        description: "Free 32KB isolated blocks for page migration and compaction.",
        desired: "lower",
      },
      "PageType Isolate - order_4 (64KB)": {
        readableName: "Free 64KB Isolate Blocks",
        description: "Free 64KB isolated blocks temporarily unavailable for allocation.",
        desired: "lower",
      },
      "PageType Isolate - order_5 (128KB)": {
        readableName: "Free 128KB Isolate Blocks",
        description: "Free 128KB isolated blocks used in memory compaction processes.",
        desired: "lower",
      },
      "PageType Isolate - order_6 (256KB)": {
        readableName: "Free 256KB Isolate Blocks",
        description: "Free 256KB isolated blocks for large-scale memory operations.",
        desired: "lower",
      },
      "PageType Isolate - order_7 (512KB)": {
        readableName: "Free 512KB Isolate Blocks",
        description: "Free 512KB isolated blocks during memory management operations.",
        desired: "lower",
      },
      "PageType Isolate - order_8 (1MB)": {
        readableName: "Free 1MB Isolate Blocks",
        description: "Free 1MB isolated blocks for very large memory operations.",
        desired: "lower",
      },
      "PageType Isolate - order_9 (2MB)": {
        readableName: "Free 2MB Isolate Blocks",
        description: "Free 2MB isolated blocks during huge page operations.",
        desired: "lower",
      },
      "PageType Isolate - order_10 (4MB)": {
        readableName: "Free 4MB Isolate Blocks",
        description: "Free 4MB isolated blocks, the largest isolated allocation size.",
        desired: "lower",
      },
      "PageBlocks - Unmovable": {
        readableName: "Unmovable PageBlocks Count",
        description:
          "Total number of pageblocks designated for unmovable pages. Each pageblock is 2MB (order-9). High counts indicate significant kernel memory allocation.",
        desired: "moderate",
      },
      "PageBlocks - Movable": {
        readableName: "Movable PageBlocks Count",
        description:
          "Total number of pageblocks designated for movable pages. Each pageblock is 2MB (order-9). High counts indicate good memory available for user-space and defragmentation.",
        desired: "higher",
      },
      "PageBlocks - Reclaimable": {
        readableName: "Reclaimable PageBlocks Count",
        description:
          "Total number of pageblocks designated for reclaimable pages. Each pageblock is 2MB (order-9). These can be freed under memory pressure.",
        desired: "moderate",
      },
      "PageBlocks - HighAtomic": {
        readableName: "HighAtomic PageBlocks Count",
        description:
          "Total number of pageblocks reserved for high-priority atomic allocations. Each pageblock is 2MB (order-9). Should typically be zero or very low.",
        desired: "lower",
      },
      "PageBlocks - CMA": {
        readableName: "CMA PageBlocks Count",
        description:
          "Total number of pageblocks in Contiguous Memory Allocator. Each pageblock is 2MB (order-9). Reserved for devices requiring large contiguous memory.",
        desired: "depends",
      },
      "PageBlocks - Isolate": {
        readableName: "Isolate PageBlocks Count",
        description:
          "Total number of pageblocks currently isolated during memory operations. Each pageblock is 2MB (order-9). Should typically be zero.",
        desired: "lower",
      },
      "SlabInfo active objs": {
        readableName: "Active Objects",
        description:
          "Number of objects currently in use (allocated) in this slab cache. Indicates active memory consumption by this slab type.",
        desired: "depends",
      },
      "SlabInfo num objs": {
        readableName: "Total Objects",
        description:
          "Total number of objects available in this slab cache, including both active and free objects. Shows total capacity.",
        desired: "depends",
      },
      "SlabInfo objsize": {
        readableName: "Object Size",
        description: "Size of each object in bytes in this slab cache. Fixed size for all objects in this cache type.",
        desired: "fixed",
        unit: "Bytes",
      },
      "SlabInfo objperslab": {
        readableName: "Objects Per Slab",
        description:
          "Number of objects that fit in each slab. Determines slab packing efficiency for this object size.",
        desired: "fixed",
      },
      "SlabInfo pagesperslab": {
        readableName: "Pages Per Slab",
        description:
          "Number of memory pages (typically 4KB each) used by each slab. Indicates memory granularity for this cache.",
        desired: "fixed",
      },
      "SlabInfo limit": {
        readableName: "Per-CPU Limit",
        description:
          "Maximum number of free objects that can be cached per CPU. Value of 0 means per-CPU caching is disabled.",
        desired: "fixed",
      },
      "SlabInfo batchcount": {
        readableName: "Batch Count",
        description:
          "Number of objects to transfer at once between per-CPU cache and shared cache. Value of 0 means batching is disabled.",
        desired: "fixed",
      },
      "SlabInfo sharedfactor": {
        readableName: "Shared Factor",
        description:
          "Multiplier for shared cache size. Value of 0 means no shared cache between CPUs for this slab type.",
        desired: "fixed",
      },
      "SlabInfo active slabs": {
        readableName: "Active Slabs",
        description:
          "Number of slabs currently in use (containing at least one allocated object). Indicates active slab memory usage.",
        desired: "depends",
      },
      "SlabInfo num slabs": {
        readableName: "Total Slabs",
        description:
          "Total number of slabs allocated for this cache, including both active and empty slabs. Shows total slab capacity.",
        desired: "depends",
      },
      "SlabInfo sharedavail": {
        readableName: "Shared Available",
        description:
          "Number of objects available in the shared cache between CPUs. Indicates cross-CPU cache efficiency.",
        desired: "depends",
      },
    },
  },
  meminfo: MEMINFO_DATA_DESCRIPTION,
  vmstat: VMSTAT_DATA_DESCRIPTION,
  interrupts: {
    readableName: "Interrupts",
    summary:
      "Interrupt metrics measure that number of interrupts handled by each CPU. The data were collected from the system pseudo-file /proc/interrupts. Every metric graph show the number of times a specific interrupt was handled by each CPU, as well as the aggregate (average) of all CPUs. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for its aggregate series.",
    defaultUnit: "Counts",
    defaultHelpfulLinks: [
      "https://man7.org/linux/man-pages/man5/proc_interrupts.5.html",
      "https://developer.arm.com/documentation/198123/0302/Handling-interrupts",
    ],
    fieldDescriptions: {
      "CAL (Function call interrupts)": {
        readableName: "Function Call Interrupts",
        description:
          "Inter-processor interrupts used to execute functions on remote CPUs. These are essential for SMP coordination and workload distribution across cores.",
        desired: "depends",
      },
      "DFR (Deferred Error APIC interrupts)": {
        readableName: "Deferred Error APIC Interrupts",
        description:
          "APIC interrupts for handling deferred machine check errors that don't require immediate attention. These allow the system to process non-critical hardware errors without blocking normal operation.",
        desired: "lower",
      },
      ERR: {
        readableName: "Error Count",
        description:
          "Counter for various interrupt controller errors and spurious interrupts. This tracks interrupt delivery failures and hardware anomalies in the interrupt subsystem.",
        desired: "lower",
      },
      Err: {
        readableName: "Error Count",
        description:
          "Alternative error counter used on some architectures for interrupt-related errors. Similar to ERR but may track different types of interrupt subsystem failures.",
        desired: "lower",
      },
      "HYP (Hypervisor callback interrupts)": {
        readableName: "Hypervisor Callback Interrupts",
        description:
          "Interrupts generated by hypervisor for communication with guest operating systems. These facilitate virtualization operations and host-guest coordination in virtualized environments.",
        desired: "depends",
      },
      "IPI0 (Rescheduling interrupts)": {
        readableName: "Rescheduling Interrupts",
        description:
          "Inter-processor interrupts that trigger CPU rescheduling on remote cores. These ensure proper load balancing and task migration across multiple processors.",
        desired: "depends",
      },
      "IPI1 (Function call interrupts)": {
        readableName: "Function Call Interrupts",
        description:
          "Inter-processor interrupts for executing specific functions on target CPUs. These enable cross-CPU synchronization and distributed processing operations.",
        desired: "depends",
      },
      "IPI2 (CPU stop interrupts)": {
        readableName: "CPU Stop Interrupts",
        description:
          "Inter-processor interrupts used to halt specific CPUs during system shutdown or maintenance. These provide controlled CPU shutdown for system management operations.",
        desired: "lower",
      },
      "IPI3 (CPU stop (for crash dump) interrupts)": {
        readableName: "CPU Stop for Crash Dump Interrupts",
        description:
          "Inter-processor interrupts that stop CPUs to enable crash dump collection. These ensure system stability during kernel panic and crash analysis procedures.",
        desired: "lower",
      },
      "IPI3 (CPU stop NMIs)": {
        readableName: "CPU Stop NMIs",
        description:
          "Non-maskable inter-processor interrupts for emergency CPU halt operations. These provide the highest priority mechanism to stop CPUs during critical system failures.",
        desired: "lower",
      },
      "IPI4 (Timer broadcast interrupts)": {
        readableName: "Timer Broadcast Interrupts",
        description:
          "Inter-processor interrupts for broadcasting timer events across CPUs. These maintain time synchronization and coordinate timer-based operations in multi-processor systems.",
        desired: "depends",
      },
      "IPI5 (IRQ work interrupts)": {
        readableName: "IRQ Work Interrupts",
        description:
          "Inter-processor interrupts for scheduling work items on remote CPUs. These enable deferred work processing and cross-CPU task delegation.",
        desired: "depends",
      },
      "IPI6 (CPU backtrace interrupts)": {
        readableName: "CPU Backtrace Interrupts",
        description:
          "Inter-processor interrupts that trigger stack trace collection from remote CPUs. These are used for debugging and system analysis during development or troubleshooting.",
        desired: "lower",
      },
      "IPI6 (CPU wake-up interrupts)": {
        readableName: "CPU Wake-up Interrupts",
        description:
          "Inter-processor interrupts used to wake up idle or sleeping CPUs. These enable dynamic CPU power management and on-demand processor activation.",
        desired: "depends",
      },
      "IPI7 (KGDB roundup interrupts)": {
        readableName: "KGDB Roundup Interrupts",
        description:
          "Inter-processor interrupts used by kernel debugger to synchronize all CPUs. These facilitate kernel debugging by ensuring all processors are in a known state.",
        desired: "lower",
      },
      "IWI (IRQ work interrupts)": {
        readableName: "IRQ Work Interrupts",
        description:
          "Interrupts for processing deferred IRQ work items on the current CPU. These handle interrupt-related tasks that cannot be completed in interrupt context.",
        desired: "depends",
      },
      "LOC (Local timer interrupts)": {
        readableName: "Local Timer Interrupts",
        description:
          "Timer interrupts generated by each CPU's local APIC timer. These drive the scheduler tick and maintain per-CPU timing for process scheduling and system timekeeping.",
        desired: "depends",
      },
      "MCE (Machine check exceptions)": {
        readableName: "Machine Check Exceptions",
        description:
          "Hardware-generated interrupts for reporting serious CPU and system errors. These indicate potential hardware failures that require immediate attention or system shutdown.",
        desired: "lower",
      },
      "MCP (Machine check polls)": {
        readableName: "Machine Check Polls",
        description:
          "Periodic interrupts for polling machine check status registers. These proactively monitor hardware health and detect errors before they become critical.",
        desired: "lower",
      },
      MIS: {
        readableName: "Miscellaneous Interrupts",
        description:
          "Counter for various uncategorized interrupt events. This tracks interrupt activity that doesn't fit into other specific categories.",
        desired: "lower",
      },
      "NMI (Non-maskable interrupts)": {
        readableName: "Non-Maskable Interrupts",
        description:
          "High-priority interrupts that cannot be disabled by software. These handle critical system events like hardware failures and debugging requests.",
        desired: "lower",
      },
      "NPI (Nested posted-interrupt event)": {
        readableName: "Nested Posted-Interrupt Event",
        description:
          "Virtualization-specific interrupts for handling nested interrupt posting. These optimize interrupt delivery in nested virtualization environments.",
        desired: "depends",
      },
      "PIN (Posted-interrupt notification event)": {
        readableName: "Posted-Interrupt Notification Event",
        description:
          "Virtualization interrupts for notifying about posted interrupt availability. These improve interrupt handling efficiency in virtual machine environments.",
        desired: "depends",
      },
      "PIW (Posted-interrupt wakeup event)": {
        readableName: "Posted-Interrupt Wakeup Event",
        description:
          "Virtualization interrupts for waking up CPUs to handle posted interrupts. These optimize interrupt processing in virtualized systems with sleeping CPUs.",
        desired: "depends",
      },
      "PMI (Performance monitoring interrupts)": {
        readableName: "Performance Monitoring Interrupts",
        description:
          "Interrupts generated by CPU performance monitoring units. These enable profiling and performance analysis by sampling CPU events and counters.",
        desired: "depends",
      },
      "RES (Rescheduling interrupts)": {
        readableName: "Rescheduling Interrupts",
        description:
          "Interrupts that trigger process rescheduling on the current CPU. These maintain fair scheduling and respond to priority changes in the system.",
        desired: "depends",
      },
      "RTR (APIC ICR read retries)": {
        readableName: "APIC ICR Read Retries",
        description:
          "Counter for APIC Interrupt Command Register read retry operations. This tracks communication reliability between APIC controllers in multi-processor systems.",
        desired: "lower",
      },
      "SPU (Spurious interrupts)": {
        readableName: "Spurious Interrupts",
        description:
          "False interrupts generated due to electrical noise or timing issues. These represent interrupt controller glitches that don't correspond to actual interrupt sources.",
        desired: "lower",
      },
      "THR (Threshold APIC interrupts)": {
        readableName: "Threshold APIC Interrupts",
        description:
          "APIC interrupts triggered when error counters exceed predefined thresholds. These provide early warning for accumulating hardware errors before they become critical.",
        desired: "lower",
      },
      "TLB (TLB shootdowns)": {
        readableName: "TLB Shootdowns",
        description:
          "Inter-processor interrupts for invalidating Translation Lookaside Buffer entries. These maintain memory coherency by ensuring stale page translations are removed across all CPUs.",
        desired: "depends",
      },
      "TRM (Thermal event interrupts)": {
        readableName: "Thermal Event Interrupts",
        description:
          "Interrupts generated when CPU temperature exceeds safe operating limits. These trigger thermal throttling and cooling measures to prevent hardware damage.",
        desired: "lower",
      },
    },
  },
  diskstats: {
    readableName: "Disk Stats",
    summary:
      "Disk stats metrics measure the I/O stats for each disk device and partition of the system. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for the device series with the highest average.",
    defaultUnit: "Counts",
    defaultHelpfulLinks: ["https://docs.kernel.org/admin-guide/iostats.html"],
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
    readableName: "TCP/IP Stats",
    summary:
      "Network stats metrics measure various networking stats at the TCP/IP layer. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero.",
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
        unit: "Bytes",
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
        unit: "Bytes",
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
        description: "Total number of bytes received by the network interface at the IP layer.",
        unit: "Bytes",
        desired: "depends",
        optimization: [NETWORK_USAGE_INVESTIGATION],
        helpfulLinks: ["https://aws.github.io/graviton/perfrunbook/debug_system_perf.html#check-network-usage"],
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
        unit: "Bytes",
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
        unit: "Bytes",
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
        description: "Total number of bytes transmitted by the network interface at the IP layer.",
        unit: "Bytes",
        desired: "depends",
        optimization: [NETWORK_USAGE_INVESTIGATION],
        helpfulLinks: ["https://aws.github.io/graviton/perfrunbook/debug_system_perf.html#check-network-usage"],
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
  ena_stat: {
    readableName: "ENA Stats",
    summary:
      "ENA (Elastic Network Adapter) stats metrics measure network performance and health for AWS EC2 instances using ENA drivers. The data is collected from ethtool statistics and provides insights into packet processing, errors, and resource utilization. Per-queue metrics (queue_N_tx_* and queue_N_rx_*) are aggregated into their base metric name with per-queue series.",
    defaultUnit: "Count",
    defaultHelpfulLinks: [
      "https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/monitoring-network-performance-ena.html",
      "https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ena-nitro-perf.html",
    ],
    fieldDescriptions: {
      bw_in_allowance_exceeded: {
        readableName: "Inbound Bandwidth Allowance Exceeded",
        description:
          "Number of packets queued or dropped because the inbound aggregate bandwidth exceeded the maximum for the instance.",
        desired: "lower",
        optimization: [EC2_NETWORK_BANDWIDTH_ALLOWANCE_RECOMMENDATIONS],
      },
      bw_out_allowance_exceeded: {
        readableName: "Outbound Bandwidth Allowance Exceeded",
        description:
          "Number of packets queued or dropped because the outbound aggregate bandwidth exceeded the maximum for the instance.",
        desired: "lower",
        optimization: [EC2_NETWORK_BANDWIDTH_ALLOWANCE_RECOMMENDATIONS],
      },
      conntrack_allowance_exceeded: {
        readableName: "Connection Tracking Allowance Exceeded",
        description:
          "Number of packets dropped because the number of tracked connection exceeded the maximum for the instance and new connections could not be established. This can result in packet loss for traffic to or from the instance. The security group tracks each connection established to ensure that return packets are delivered as expected. There is a maximum number of connections that can be tracked per instance.",
        desired: "lower",
        optimization: [EC2_NETWORK_TRACKED_CONNECTIONS_ALLOWANCE_RECOMMENDATIONS],
      },
      linklocal_allowance_exceeded: {
        readableName: "Link Local Service Allowance Exceeded",
        description:
          "Number of packets dropped because the PPS of the traffic to local proxy services exceeded the maximum for the network interface. This impacts traffic to the DNS service, the Instance Metadata Service, and the Amazon Time Sync Service, but does not impact traffic to custom DNS resolvers.",
        desired: "lower",
        optimization: [EC2_NETWORK_LINK_LOCAL_ALLOWANCE_RECOMMENDATIONS],
      },
      pps_allowance_exceeded: {
        readableName: "Packet-per-second Allowance Exceeded",
        description:
          "Number of packets queued or dropped because the bidirectional Packet-per-second (PPS) exceeded the maximum for the instance. PPS allowance is enforced separately to the overall bandwidth allowance and, while the instance may still be under overall bandwidth allowance, the PPS allowance may exceed if the mean packet size is small",
        desired: "lower",
        optimization: [EC2_NETWORK_PPS_ALLOWANCE_RECOMMENDATIONS],
      },
      conntrack_allowance_available: {
        readableName: "Connection Tracking Allowance Available",
        description:
          "The number of tracked connections that can be established by the instance before hitting the Connections Tracked allowance of that instance type. The security group tracks certain connections established to ensure that return packets are delivered as expected. There is a maximum number of connections that can be tracked per instance.",
        desired: "higher",
        optimization: [EC2_NETWORK_TRACKED_CONNECTIONS_ALLOWANCE_RECOMMENDATIONS],
      },
      ena_srd_mode: {
        readableName: "ENA Express Mode",
        description:
          "Indicates whether ENA Express is configured. 0 = ENA Express off, UDP off; 1 = ENA Express on, UDP off; 2 = ENA Express off, UDP on (only happens after disabling the previously enabled ENA Express with UDP); 3 = ENA Express on, UDP on.",
        desired: "fixed",
      },
      ena_srd_eligible_tx_pkts: {
        readableName: "ENA Express Eligible SRD TX Packets",
        description:
          "Number of transmitted packets that were eligible for SRD. If the metric's value is much larger than ena_srd_tx_pkts (the number of packages actually transmitted with SRD), eligible packets are not able to transmit via SRD and falling back to standard ENA transmission. This could happen when the network card attached to the instance has used up its maximum resources, or if packets are over the MTU limit. Packets can also fall into this gap during live migrations or live server updates. Additional troubleshooting is required to determine the root cause.",
        desired: "higher",
      },
      ena_srd_tx_pkts: {
        readableName: "ENA Express SRD TX Packets",
        description:
          "The number of SRD packets transmitted within a given time period. ENA Express is powered by AWS Scalable Reliable Datagram (SRD) technology, a high performance network transport protocol that uses dynamic routing to increase throughput and minimize tail latency.",
        desired: "higher",
      },
      ena_srd_rx_pkts: {
        readableName: "ENA Express SRD RX Packets",
        description:
          "The number of SRD packets received within a given time period. ENA Express is powered by AWS Scalable Reliable Datagram (SRD) technology, a high performance network transport protocol that uses dynamic routing to increase throughput and minimize tail latency.",
        desired: "higher",
      },
      ena_srd_resource_utilization: {
        readableName: "ENA Express SRD Resource Utilization",
        description:
          "The percentage of the maximum allowed memory utilization for concurrent SRD connections that the instance has consumed. As utilization approaches 100%, you can expect to see performance issues. ENA Express falls back from SRD to standard ENA transmission, and the possibility of dropped packets increases. High resource utilization is a sign that it’s time to scale the instance out to improve network performance.",
        desired: "lower",
        unit: "Utilization (%)",
      },
      total_resets: {
        readableName: "Total Resets",
        description:
          "Total number of ENA driver resets. A reset reinitializes the driver and temporarily disrupts network connectivity. Frequent resets indicate driver or hardware instability that should be investigated.",
        desired: "lower",
      },
      reset_fail: {
        readableName: "Reset Failures",
        description:
          "Number of ENA driver reset attempts that failed. A failed reset means the driver could not recover from an error state, potentially leaving the network interface in a degraded or non-functional state.",
        desired: "lower",
      },
      tx_timeout: {
        readableName: "TX Timeout",
        description:
          "Number of transmit timeout events where the kernel's TX watchdog detected that a transmit queue has not made progress within the expected time. This typically triggers a driver reset to recover.",
        desired: "lower",
      },
      wd_expired: {
        readableName: "Watchdog Expired",
        description:
          "Number of ENA driver watchdog timer expirations. The watchdog monitors driver health and triggers a reset when the device becomes unresponsive. Frequent expirations suggest device or driver issues.",
        desired: "lower",
      },
      admin_q_pause: {
        readableName: "Admin Queue Pause",
        description:
          "Number of times the ENA admin queue was paused. The admin queue handles control-plane operations such as queue creation and configuration. Pauses indicate the device is temporarily unable to process admin commands.",
        desired: "lower",
      },
      bad_tx_req_id: {
        readableName: "Bad TX Request ID",
        description:
          "Number of transmit completions received with an invalid request ID. This indicates a mismatch between the driver and device state, typically a sign of a firmware or driver bug.",
        desired: "lower",
      },
      bad_rx_req_id: {
        readableName: "Bad RX Request ID",
        description:
          "Number of receive completions with an invalid request ID. This indicates a mismatch between the driver and device state for receive operations.",
        desired: "lower",
      },
      bad_rx_desc_num: {
        readableName: "Bad RX Descriptor Number",
        description:
          "Number of receive completions referencing an invalid descriptor number. This indicates corruption or a bug in the driver-device communication for the receive path.",
        desired: "lower",
      },
      missing_intr: {
        readableName: "Missing Interrupt",
        description:
          "Number of expected hardware interrupts that were not received. The driver uses a watchdog mechanism to detect and recover from missed interrupts, which can cause temporary latency spikes.",
        desired: "lower",
      },
      suspected_poll_starvation: {
        readableName: "Suspected Poll Starvation",
        description:
          "Number of times the driver suspected that NAPI polling was not being scheduled frequently enough to keep up with incoming traffic. This can occur when CPU resources are contended or interrupt affinity is misconfigured.",
        desired: "lower",
      },
      missing_tx_cmpl: {
        readableName: "Missing TX Completion",
        description:
          "Number of transmit completions that were expected but not received within the timeout period. Missing completions can cause transmit queue stalls and may trigger a driver reset.",
        desired: "lower",
      },
      rx_desc_malformed: {
        readableName: "RX Descriptor Malformed",
        description:
          "Number of receive descriptors that were malformed or contained invalid data. This indicates a device firmware issue or memory corruption in the receive descriptor ring.",
        desired: "lower",
      },
      tx_desc_malformed: {
        readableName: "TX Descriptor Malformed",
        description:
          "Number of transmit descriptors that were malformed or contained invalid data. This indicates a device firmware issue or memory corruption in the transmit descriptor ring.",
        desired: "lower",
      },
      invalid_state: {
        readableName: "Invalid State",
        description:
          "Number of times the ENA driver detected an invalid internal state. This is a catch-all error counter for unexpected conditions that may require a driver reset to recover.",
        desired: "lower",
      },
      os_netdev_wd: {
        readableName: "OS Network Device Watchdog",
        description:
          "Number of times the Linux kernel's network device watchdog fired for this interface. The kernel watchdog monitors transmit queue activity and triggers when a queue appears hung.",
        desired: "lower",
      },
      missing_admin_interrupt: {
        readableName: "Missing Admin Interrupt",
        description:
          "Number of expected admin queue completion interrupts that were not received. The admin queue handles control-plane operations, and missing interrupts can delay queue management.",
        desired: "lower",
      },
      admin_to: {
        readableName: "Admin Timeout",
        description:
          "Number of admin queue command timeouts. An admin command timeout means the device did not respond to a control-plane request within the expected time, which may trigger a driver reset.",
        desired: "lower",
      },
      device_request_reset: {
        readableName: "Device Request Reset",
        description:
          "Number of reset requests initiated by the ENA device itself. The device may request a reset when it detects an internal error that requires reinitialization to recover.",
        desired: "lower",
      },
      missing_first_intr: {
        readableName: "Missing First Interrupt",
        description:
          "Number of times the first interrupt after queue creation was not received. This can indicate an interrupt configuration issue during queue initialization.",
        desired: "lower",
      },
      suspend: {
        readableName: "Suspend Events",
        description:
          "Number of times the ENA driver entered a suspended state, typically during system power management transitions such as hibernation or live migration.",
        desired: "lower",
      },
      resume: {
        readableName: "Resume Events",
        description:
          "Number of times the ENA driver resumed from a suspended state. Each resume should correspond to a prior suspend event.",
        desired: "lower",
      },
      interface_down: {
        readableName: "Interface Down",
        description:
          "Number of times the network interface was administratively brought down (e.g., via ifconfig down or ip link set down).",
        desired: "lower",
      },
      interface_up: {
        readableName: "Interface Up",
        description:
          "Number of times the network interface was administratively brought up (e.g., via ifconfig up or ip link set up).",
        desired: "lower",
      },
      ena_admin_q_aborted_cmd: {
        readableName: "Admin Queue Aborted Commands",
        description:
          "Number of admin queue commands that were aborted before completion. Aborted commands indicate the driver cancelled a pending control-plane operation, possibly due to a timeout or reset.",
        desired: "lower",
      },
      ena_admin_q_submitted_cmd: {
        readableName: "Admin Queue Submitted Commands",
        description:
          "Total number of commands submitted to the ENA admin queue. This reflects control-plane activity such as queue creation, destruction, and configuration changes.",
        desired: "depends",
      },
      ena_admin_q_completed_cmd: {
        readableName: "Admin Queue Completed Commands",
        description:
          "Total number of admin queue commands that completed successfully. Comparing this with submitted commands can reveal how many commands were lost or aborted.",
        desired: "depends",
      },
      ena_admin_q_out_of_space: {
        readableName: "Admin Queue Out of Space",
        description:
          "Number of times the admin queue ran out of space for new commands. This means the driver attempted to submit a control-plane command but the queue was full.",
        desired: "lower",
      },
      ena_admin_q_no_completion: {
        readableName: "Admin Queue No Completion",
        description:
          "Number of admin queue commands that received no completion from the device. This typically indicates a device hang or communication failure on the control plane.",
        desired: "lower",
      },
      num_of_active_io_queues: {
        readableName: "Number of Active I/O Queues",
        description:
          "Current number of active I/O queue pairs (TX + RX). This reflects the level of parallelism available for packet processing and is typically equal to the number of vCPUs or a configured maximum.",
        desired: "higher",
      },
      num_of_xdp_tx_queues: {
        readableName: "Number of XDP TX Queues",
        description:
          "Current number of dedicated XDP (eXpress Data Path) transmit queues. XDP queues are used for high-performance packet processing that bypasses the normal kernel networking stack.",
        desired: "higher",
      },
      // =====================================================================
      // ENA per-queue TX metrics (from ethtool -S, queue_N_tx_* transformed)
      // =====================================================================
      tx_bytes: {
        readableName: "TX Bytes",
        description:
          "Number of bytes transmitted per queue. This is the raw byte count from the ENA device, reflecting the total data volume sent through each transmit queue.",
        desired: "depends",
        unit: "Bytes",
      },
      tx_cnt: {
        readableName: "TX Packet Count",
        description:
          "Number of packets transmitted per queue. This is the raw packet count from the ENA device, reflecting transmit throughput at the queue level.",
        desired: "depends",
      },
      tx_tx_poll: {
        readableName: "TX Poll",
        description:
          "Number of times the transmit completion routine was invoked per queue. Each poll processes completed transmit descriptors and frees the associated buffers. A high poll count relative to packet count may indicate small batch sizes.",
        desired: "depends",
      },
      tx_doorbells: {
        readableName: "TX Doorbells",
        description:
          "Number of doorbell writes to the ENA device per transmit queue. A doorbell notifies the device that new transmit descriptors are ready for processing. Fewer doorbells relative to packets indicates efficient batching.",
        desired: "lower",
      },
      tx_unmask_interrupt: {
        readableName: "TX Unmask Interrupt",
        description:
          "Number of times the transmit interrupt was unmasked per queue. After NAPI polling completes, the driver unmasks the interrupt to receive the next completion notification.",
        desired: "depends",
      },
      tx_lost_interrupt: {
        readableName: "TX Lost Interrupt",
        description:
          "Number of times a software workaround was triggered to handle a lost hardware interrupt during transmission per queue. The driver detects this via a watchdog and re-polls for completions.",
        desired: "lower",
      },
      tx_queue_stop: {
        readableName: "TX Queue Stop",
        description:
          "Number of times a transmit queue was stopped because it ran out of descriptors. When stopped, the kernel queues packets in the qdisc layer until descriptors are freed by completions. Frequent stops indicate the queue depth is insufficient for the workload.",
        desired: "lower",
      },
      tx_queue_wakeup: {
        readableName: "TX Queue Wakeup",
        description:
          "Number of times a previously stopped transmit queue was restarted after descriptors became available. Each wakeup corresponds to a prior queue stop.",
        desired: "lower",
      },
      tx_dma_mapping_err: {
        readableName: "TX DMA Mapping Error",
        description:
          "Number of DMA mapping failures for transmit buffers per queue. A DMA mapping error means the driver could not map a packet buffer for device access, causing the packet to be dropped.",
        desired: "lower",
      },
      tx_linearize: {
        readableName: "TX Linearize",
        description:
          "Number of transmit packets that required linearization (copying scattered data into a single contiguous buffer) per queue. Linearization adds CPU overhead and indicates the packet layout was incompatible with the device's scatter-gather capabilities.",
        desired: "lower",
      },
      tx_linearize_failed: {
        readableName: "TX Linearize Failed",
        description:
          "Number of transmit packet linearization attempts that failed per queue, causing the packet to be dropped. This typically occurs due to memory allocation failure during the copy.",
        desired: "lower",
      },
      tx_napi_comp: {
        readableName: "TX NAPI Completions",
        description:
          "Number of times NAPI polling completed for a transmit queue, meaning all pending completions were processed and the driver returned to interrupt-driven mode.",
        desired: "depends",
      },
      tx_prepare_ctx_err: {
        readableName: "TX Prepare Context Error",
        description:
          "Number of transmit context preparation errors per queue. The driver prepares a context descriptor for each packet with offload information; a failure here drops the packet.",
        desired: "lower",
      },
      tx_bad_req_id: {
        readableName: "TX Bad Request ID",
        description:
          "Number of transmit completions with an invalid request ID per queue. This indicates a mismatch between the driver's descriptor tracking and the device's completion reports.",
        desired: "lower",
      },
      tx_llq_buffer_copy: {
        readableName: "TX LLQ Buffer Copy",
        description:
          "Number of packets copied into the Low Latency Queue (LLQ) push buffer per transmit queue. LLQ mode copies packet headers directly to device memory to reduce transmit latency.",
        desired: "depends",
      },
      tx_missed_tx: {
        readableName: "TX Missed TX",
        description:
          "Number of missed transmit opportunities per queue, where the driver had packets ready but could not submit them to the device in time.",
        desired: "lower",
      },
      tx_pending_timedout_pkt: {
        readableName: "TX Pending Timed Out Packets",
        description:
          "Number of transmit packets that timed out while pending in the queue. These packets were submitted to the device but never completed within the expected time.",
        desired: "lower",
      },
      tx_pending_timedout_pk: {
        readableName: "TX Pending Timed Out Packets (Legacy)",
        description:
          "Legacy counter for transmit packets that timed out while pending. This is an older variant of tx_pending_timedout_pkt found in some ENA driver versions.",
        desired: "lower",
      },
      tx_xdp_frags_exceeded: {
        readableName: "TX XDP Fragments Exceeded",
        description:
          "Number of XDP transmit operations that failed because the packet had more fragments than the device supports. The packet is dropped when this limit is exceeded.",
        desired: "lower",
      },
      tx_xdp_short_linear_par: {
        readableName: "TX XDP Short Linear Part",
        description:
          "Number of XDP transmit operations where the linear (non-fragmented) portion of the packet was too short for the device to process. This can occur when XDP modifies packet headers.",
        desired: "lower",
      },
      tx_xdp_short_linear_pa: {
        readableName: "TX XDP Short Linear Part (Legacy)",
        description:
          "Legacy counter for XDP transmit operations with a short linear part. This is an older variant of tx_xdp_short_linear_par found in some ENA driver versions.",
        desired: "lower",
      },
      tx_xsk_cnt: {
        readableName: "TX XSK Packet Count",
        description:
          "Number of packets transmitted through XDP sockets (AF_XDP) per queue. XDP sockets provide a high-performance zero-copy path for user-space packet processing.",
        desired: "depends",
      },
      tx_xsk_bytes: {
        readableName: "TX XSK Bytes",
        description: "Number of bytes transmitted through XDP sockets (AF_XDP) per queue.",
        desired: "depends",
        unit: "Bytes",
      },
      tx_xsk_need_wakeup_set: {
        readableName: "TX XSK Need Wakeup Set",
        description:
          "Number of times the XDP socket need-wakeup flag was set for a transmit queue, indicating the kernel needs to be woken up to process pending XSK transmit requests.",
        desired: "depends",
      },
      tx_xsk_wakeup_request: {
        readableName: "TX XSK Wakeup Request",
        description:
          "Number of wakeup requests issued for XDP socket transmit processing per queue. User-space sends a wakeup when it has new packets to transmit via AF_XDP.",
        desired: "depends",
      },
      // =====================================================================
      // ENA per-queue RX metrics (from ethtool -S, queue_N_rx_* transformed)
      // =====================================================================
      rx_bytes: {
        readableName: "RX Bytes",
        description:
          "Number of bytes received per queue. This is the raw byte count from the ENA device, reflecting the total data volume received through each receive queue.",
        desired: "depends",
        unit: "Bytes",
      },
      rx_cnt: {
        readableName: "RX Packet Count",
        description:
          "Number of packets received per queue. This is the raw packet count from the ENA device, reflecting receive throughput at the queue level.",
        desired: "depends",
      },
      rx_rx_copybreak_pkt: {
        readableName: "RX Copybreak Packets",
        description:
          "Number of received packets that were smaller than the copybreak threshold and were copied into a new smaller buffer per queue. This trades a memory copy for reduced memory usage on small packets.",
        desired: "depends",
      },
      rx_csum_good: {
        readableName: "RX Checksum Good",
        description:
          "Number of received packets where the hardware verified the checksum as correct per queue. Hardware checksum offload avoids redundant CPU-based checksum computation.",
        desired: "higher",
      },
      rx_refil_partial: {
        readableName: "RX Refill Partial",
        description:
          "Number of times the receive buffer ring could not be fully refilled per queue. Partial refills mean fewer buffers are available for incoming packets, which can lead to drops under high load.",
        desired: "lower",
      },
      rx_csum_bad: {
        readableName: "RX Checksum Bad",
        description:
          "Number of received packets where the hardware detected an invalid checksum per queue. These packets are passed to the kernel with a bad checksum flag and are typically dropped by the network stack.",
        desired: "lower",
      },
      rx_page_alloc_fail: {
        readableName: "RX Page Allocation Failure",
        description:
          "Number of memory page allocation failures when refilling the receive buffer ring per queue. Allocation failures reduce the number of available receive buffers and can cause packet drops under memory pressure.",
        desired: "lower",
      },
      rx_skb_alloc_fail: {
        readableName: "RX SKB Allocation Failure",
        description:
          "Number of socket buffer (sk_buff) allocation failures during receive processing per queue. An SKB allocation failure means the driver could not create the kernel data structure needed to pass the packet up the network stack.",
        desired: "lower",
      },
      rx_bad_desc_num: {
        readableName: "RX Bad Descriptor Number",
        description:
          "Number of receive completions referencing an invalid descriptor number per queue. This indicates corruption or a bug in the driver-device communication for the receive path.",
        desired: "lower",
      },
      rx_bad_req_id: {
        readableName: "RX Bad Request ID",
        description:
          "Number of receive completions with an invalid request ID per queue. This indicates a mismatch between the driver's descriptor tracking and the device's completion reports.",
        desired: "lower",
      },
      rx_empty_rx_ring: {
        readableName: "RX Empty Ring",
        description:
          "Number of times the receive ring was found completely empty per queue. An empty ring means no buffers were available for incoming packets, causing all arriving packets to be dropped by the device.",
        desired: "lower",
      },
      rx_csum_unchecked: {
        readableName: "RX Checksum Unchecked",
        description:
          "Number of received packets where the hardware did not perform checksum verification per queue. The kernel's software checksum path handles these packets instead.",
        desired: "depends",
      },
      rx_dma_mapping_err: {
        readableName: "RX DMA Mapping Error",
        description:
          "Number of DMA mapping failures for receive buffers per queue. A DMA mapping error means the driver could not map a buffer for device access, reducing the number of available receive slots.",
        desired: "lower",
      },
      rx_xdp_aborted: {
        readableName: "RX XDP Aborted",
        description:
          "Number of packets for which the XDP program returned XDP_ABORTED per queue. This typically indicates a bug in the XDP program and the packet is dropped with a trace event.",
        desired: "lower",
      },
      rx_xdp_drop: {
        readableName: "RX XDP Drop",
        description:
          "Number of packets intentionally dropped by the XDP program (XDP_DROP verdict) per queue. This is the expected fast-path drop mechanism for XDP-based filtering.",
        desired: "depends",
      },
      rx_xdp_pass: {
        readableName: "RX XDP Pass",
        description:
          "Number of packets passed by the XDP program to the normal kernel networking stack (XDP_PASS verdict) per queue.",
        desired: "depends",
      },
      rx_xdp_tx: {
        readableName: "RX XDP TX",
        description:
          "Number of packets transmitted back out the same interface by the XDP program (XDP_TX verdict) per queue. This enables hairpin forwarding without going through the kernel stack.",
        desired: "depends",
      },
      rx_xdp_invalid: {
        readableName: "RX XDP Invalid",
        description:
          "Number of packets for which the XDP program returned an unrecognized verdict per queue. Invalid verdicts are treated as errors and the packet is dropped.",
        desired: "lower",
      },
      rx_xdp_redirect: {
        readableName: "RX XDP Redirect",
        description:
          "Number of packets redirected by the XDP program to another interface, CPU, or XDP socket (XDP_REDIRECT verdict) per queue.",
        desired: "depends",
      },
      rx_lpc_warm_up: {
        readableName: "RX LPC Warm Up",
        description:
          "Number of Local Page Cache (LPC) warm-up operations per receive queue. The LPC caches recently freed pages for reuse in the same NUMA node, and warm-up populates this cache.",
        desired: "depends",
      },
      rx_lpc_full: {
        readableName: "RX LPC Full",
        description:
          "Number of times the Local Page Cache was full when the driver attempted to return a page per receive queue. A full LPC means the page must be freed back to the kernel allocator instead of being cached for reuse.",
        desired: "lower",
      },
      rx_lpc_wrong_numa: {
        readableName: "RX LPC Wrong NUMA",
        description:
          "Number of times a page from the Local Page Cache belonged to a different NUMA node than the one processing the receive queue. Cross-NUMA memory access adds latency to packet processing.",
        desired: "lower",
      },
      rx_zc_queue_pkt_copy: {
        readableName: "RX Zero Copy Queue Packet Copy",
        description:
          "Number of packets that required a copy in zero-copy (AF_XDP) receive mode per queue. Ideally zero-copy mode avoids copies, so this counter indicates fallback to the copy path.",
        desired: "lower",
      },
      rx_xsk_need_wakeup_set: {
        readableName: "RX XSK Need Wakeup Set",
        description:
          "Number of times the XDP socket need-wakeup flag was set for a receive queue, indicating the kernel needs to be woken up to process pending XSK receive completions.",
        desired: "depends",
      },
    },
  },
  efa_stat: {
    readableName: "EFA Stats",
    summary:
      "EFA (Elastic Fabric Adapter) stats metrics measure network performance for high-performance computing workloads using RDMA. The data is collected from EFA driver hardware counters and provides cumulative counts of packets, bytes, and operations since instance launch or driver reset. Metrics are collected per EFA device.",
    defaultUnit: "Count",
    defaultHelpfulLinks: ["https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/efa-working-monitor.html"],
    fieldDescriptions: {
      tx_bytes: {
        readableName: "Transmitted Bytes",
        description: "The number of bytes transmitted from the EFA driver.",
        desired: "depends",
        unit: "Bytes",
      },
      rx_bytes: {
        readableName: "Received Bytes",
        description: "The number of bytes received by the EFA driver.",
        desired: "depends",
        unit: "Bytes",
      },
      tx_pkts: {
        readableName: "Transmitted Packets",
        description: "The number of packets transmitted from the EFA driver.",
        desired: "depends",
      },
      rx_pkts: {
        readableName: "Received Packets",
        description: "The number of packets received by the EFA driver.",
        desired: "depends",
      },
      rx_drops: {
        readableName: "Received Packets Dropped",
        description: "The number of packets that were received by the EFA driver and then dropped.",
        desired: "lower",
      },
      send_bytes: {
        readableName: "Send Operation Bytes",
        description: "The number of bytes sent using send operations from the EFA driver.",
        desired: "depends",
        unit: "Bytes",
      },
      recv_bytes: {
        readableName: "Receive Operation Bytes",
        description: "The number of bytes received by send operations by the EFA driver.",
        desired: "depends",
        unit: "Bytes",
      },
      send_wrs: {
        readableName: "Send Working Requests",
        description: "The number of packets sent using send working requests from the EFA driver.",
        desired: "depends",
      },
      recv_wrs: {
        readableName: "Receive Working Requests",
        description: "The number of packets received by send working requests by the EFA driver.",
        desired: "depends",
      },
      avg_bytes_per_send_wr: {
        readableName: "Average Bytes Sent per Send Working Requests",
        description: "The average number of bytes sent from the EFA driver per send working request.",
        unit: "Bytes",
        desired: "depends",
      },
      avg_bytes_per_recv_wr: {
        readableName: "Average Bytes Received per Send Working Requests",
        description: "The average number of bytes received by the EFA driver per send working request.",
        unit: "Bytes",
        desired: "depends",
      },
      rdma_write_wrs: {
        readableName: "RDMA Write Working Requests",
        description: "The number of completed RDMA write working requests.",
        desired: "depends",
      },
      rdma_read_wrs: {
        readableName: "RDMA Read Working Requests",
        description: "The number of completed RDMA read working requests.",
        desired: "depends",
      },
      rdma_write_bytes: {
        readableName: "RDMA Write Bytes",
        description: "The number of bytes written to it by other instances using RDMA write working requests.",
        desired: "depends",
        unit: "Bytes",
      },
      rdma_read_bytes: {
        readableName: "RDMA Read Bytes",
        description: "The number of bytes received using RDMA read working requests.",
        desired: "depends",
        unit: "Bytes",
      },
      avg_bytes_per_rdma_write_wr: {
        readableName: "Average Bytes per RDMA Write Working Requests",
        description: "The average number of bytes per RDMA write working request.",
        unit: "Bytes",
        desired: "depends",
      },
      avg_bytes_per_rdma_read_wr: {
        readableName: "Average Bytes per RDMA Read Working Requests",
        description: "The average number of bytes per RDMA read working request.",
        unit: "Bytes",
        desired: "depends",
      },
      rdma_write_wr_err: {
        readableName: "RDMA Write Errors",
        description: "The number of RDMA write operations that had local or remote errors.",
        desired: "lower",
      },
      rdma_read_wr_err: {
        readableName: "RDMA Read Errors",
        description: "The number of RDMA read operations that had local or remote errors.",
        desired: "lower",
      },
      rdma_read_resp_bytes: {
        readableName: "RDMA Read Response Bytes",
        description: "The number of bytes sent in response to RDMA read operations.",
        desired: "depends",
        unit: "Bytes",
      },
      rdma_write_recv_bytes: {
        readableName: "RDMA Write Received Bytes",
        description: "The number of bytes received by RDMA write operations.",
        desired: "depends",
        unit: "Bytes",
      },
      retrans_bytes: {
        readableName: "Retransmitted Bytes",
        description: "The number of EFA SRD bytes retransmitted. Available on Nitro v4 and later instances.",
        desired: "lower",
        unit: "Bytes",
      },
      retrans_pkts: {
        readableName: "Retransmitted Packets",
        description: "The number of EFA SRD packets retransmitted. Available on Nitro v4 and later instances.",
        desired: "lower",
      },
      retrans_timeout_events: {
        readableName: "Retransmission Timeout Events",
        description:
          "The number of times EFA SRD traffic timed out and resulted in a network path change. Available on Nitro v4 and later instances.",
        desired: "lower",
      },
      impaired_remote_conn_events: {
        readableName: "Impaired Remote Connection Events",
        description:
          "The number of times EFA SRD connections entered an impaired state, resulting in a reduced throughput rate limit. Available on Nitro v4 and later instances.",
        desired: "lower",
      },
      unresponsive_remote_events: {
        readableName: "Unresponsive Remote Events",
        description:
          "The number of times an EFA SRD remote connection was unresponsive. Available on Nitro v4 and later instances.",
        desired: "lower",
      },
      lifespan: {
        readableName: "Device Lifespan",
        description: "The time in seconds since the EFA device was initialized or last reset.",
        desired: "depends",
        unit: "Seconds",
      },
      submitted_cmds: {
        readableName: "Submitted Commands",
        description:
          "Total number of admin commands submitted to the EFA device. Admin commands handle control-plane operations such as queue pair creation, memory registration, and device configuration.",
        desired: "depends",
      },
      completed_cmds: {
        readableName: "Completed Commands",
        description:
          "Total number of admin commands that completed successfully on the EFA device. Comparing this with submitted_cmds reveals how many commands were lost or timed out.",
        desired: "depends",
      },
      no_completion_cmds: {
        readableName: "No Completion Commands",
        description:
          "Number of admin commands that were submitted but never received a completion from the EFA device. This typically indicates a device hang or communication failure on the control plane.",
        desired: "lower",
      },
      cmds_err: {
        readableName: "Command Errors",
        description:
          "Number of admin commands that completed with an error status. Command errors indicate the EFA device rejected or failed to execute a control-plane operation such as resource allocation or configuration.",
        desired: "lower",
      },
      keep_alive_rcvd: {
        readableName: "Keep Alive Received",
        description:
          "Number of keep-alive messages received from the EFA device. The driver uses keep-alive messages to monitor device health; a gap in these messages triggers a device reset.",
        desired: "depends",
      },
      alloc_pd_err: {
        readableName: "Allocate Protection Domain Errors",
        description:
          "Number of failed protection domain (PD) allocation attempts. A PD groups related RDMA resources for access control; allocation failures prevent creation of new queue pairs and memory regions.",
        desired: "lower",
      },
      alloc_mr_err: {
        readableName: "Allocate Memory Region Errors",
        description:
          "Number of failed memory region (MR) allocation attempts. Memory regions must be registered with the EFA device before they can be used for RDMA operations; failures here block data transfer setup.",
        desired: "lower",
      },
      reg_mr_err: {
        readableName: "Register Memory Region Errors",
        description:
          "Number of failed memory region registration attempts. Registration maps user-space memory for direct device access; failures prevent the application from using that memory buffer for RDMA.",
        desired: "lower",
      },
      get_dma_mr_err: {
        readableName: "Get DMA Memory Region Errors",
        description:
          "Number of failed DMA memory region acquisition attempts. DMA memory regions are used for kernel-level RDMA operations; failures indicate the device could not set up the required DMA mappings.",
        desired: "lower",
      },
      alloc_ucontext_err: {
        readableName: "Allocate User Context Errors",
        description:
          "Number of failed user context allocation attempts. A user context is created when an application opens the EFA device; failures prevent the application from accessing EFA resources.",
        desired: "lower",
      },
      create_qp_err: {
        readableName: "Create Queue Pair Errors",
        description:
          "Number of failed queue pair (QP) creation attempts. Queue pairs are the fundamental communication endpoints for RDMA; creation failures prevent establishing new RDMA connections.",
        desired: "lower",
      },
      create_cq_err: {
        readableName: "Create Completion Queue Errors",
        description:
          "Number of failed completion queue (CQ) creation attempts. Completion queues receive notifications when RDMA operations finish; creation failures prevent setting up the completion notification path.",
        desired: "lower",
      },
      create_ah_err: {
        readableName: "Create Address Handle Errors",
        description:
          "Number of failed address handle (AH) creation attempts. Address handles store the routing information needed to reach a remote EFA endpoint; failures prevent establishing communication with that peer.",
        desired: "lower",
      },
      mmap_err: {
        readableName: "Memory Map Errors",
        description:
          "Number of failed mmap operations on the EFA device. The driver uses mmap to map device registers and queues into user-space; failures prevent the application from directly accessing EFA hardware resources.",
        desired: "lower",
      },
    },
  },
  numastat: {
    readableName: "NUMA Stats",
    summary:
      "NUMA Stats displays per-node NUMA hit and miss system statistics from the kernel memory allocator. Optimal performance is indicated by high numa_hit values and low numa_miss values. To debug, cross-reference per-node values with each CPU to verify process threads are running on the same node where their memory is allocated.",
    defaultUnit: "Count",
    defaultHelpfulLinks: ["https://docs.kernel.org/admin-guide/numastat.html"],
    fieldDescriptions: {
      numa_hit: {
        readableName: "NUMA Hit",
        description: "Number of pages successfully allocated on this node as intended.",
        desired: "depends",
      },
      numa_miss: {
        readableName: "NUMA Miss",
        description:
          "Number of pages allocated on this node despite the process preferring some different node. Each numa_miss has a numa_foreign on another node.",
        desired: "lower",
      },
      numa_foreign: {
        readableName: "NUMA Foreign",
        description:
          "Number of pages intended for this node, but actually allocated on some different node. Each numa_foreign has a numa_miss on another node.",
        desired: "lower",
      },
      local_node: {
        readableName: "Local Node",
        description: "Number of pages allocated on this node while a process was running on it.",
        desired: "depends",
      },
      other_node: {
        readableName: "Other Node",
        description: "Number of pages allocated on this node while a process was running on some other node.",
        desired: "depends",
      },
      interleave_hit: {
        readableName: "Interleave Hit",
        description: "Interleaved memory successfully allocated on this node as intended.",
        desired: "depends",
      },
    },
  },
  kernel_config: KERNEL_CONFIG_DATA_DESCRIPTION,
  sysctl: SYSCTL_DATA_DESCRIPTION,
  mem_settings: MEM_SETTINGS_DATA_DESCRIPTION,
  perf_profile: {
    readableName: "Perf Profiling",
    summary: "Perf profiling is system-wide CPU profiling performed through Linux's Perf tool.",
    defaultHelpfulLinks: ["https://perfwiki.github.io/main/"],
    fieldDescriptions: {},
  },
  // TODO: Move content of profiling analytical findings to help panel
  java_profile: {
    readableName: "Java Profiling",
    summary:
      "Java profiling shows profiled CPU utilization, memory allocations, and wall clocks for JVMs running on the system visualized as heatmaps. JVM and async-profiler metadata are also displayed. For the legacy APerf version, only the flamegraph of CPU utilization across the whole recording period is available.",
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
      process_user_space_time: {
        readableName: "APerf Process User Space Time (utime)",
        description:
          "The aggregate CPU time consumed by the APerf process executing application code. The values are represented as the equivalent number of cores consumed by the process.",
        desired: "lower",
        unit: "Number of Cores",
      },
      process_kernel_space_time: {
        readableName: "APerf Process Kernel Space Time (stime)",
        description:
          "The aggregate CPU time consumed by the APerf process in kernel mode (system calls). The values are represented as the equivalent number of cores consumed by the process.",
        desired: "lower",
        unit: "Number of Cores",
      },
      process_number_threads: {
        readableName: "APerf Process Number of Threads (num_threads)",
        description: "The number of threads spwaned by the APerf process.",
        desired: "fixed",
      },
      process_virtual_memory_size: {
        readableName: "APerf Process Virtual Memory Size (vsize)",
        description: "Total virtual memory used by the APerf process.",
        desired: "lower",
        unit: "Bytes",
      },
      process_resident_set_size_bytes: {
        readableName: "APerf Process Resident Set Size Bytes (rss)",
        description:
          "Physical memory in bytes used by the APerf process. Converted from pages using sysconf(_SC_PAGESIZE).",
        desired: "lower",
        unit: "Bytes",
      },
      process_resident_set_size: {
        readableName: "APerf Process Resident Set Size (rss)",
        description:
          "Physical memory in number of pages used by the APerf process. Multiply by page size to convert to bytes.",
        desired: "lower",
        unit: "Pages",
      },
      aperf: {
        readableName: "Total collection time",
        description: "The total time in us for APerf to collect all data during one interval.",
        desired: "lower",
      },
      prepare: {
        readableName: "Data preparation time",
        description: "The total time in us spent in each data's pre-collection preparation stage.",
        desired: "lower",
      },
      finish: {
        readableName: "Data finish time",
        description: "The total time in us spent in each data's post-collection finish stage.",
        desired: "lower",
      },
      systeminfo: {
        readableName: "System Info collection time",
        description: "The total time in us for APerf to collect the System Info data.",
        desired: "lower",
      },
      kernel_config: {
        readableName: "Kernel Config collection time",
        description: "The total time in us for APerf to collect the Kernel Config data.",
        desired: "lower",
      },
      sysctl: {
        readableName: "Sysctl Config collection time",
        description: "The total time in us for APerf to collect the Sysctl Config data.",
        desired: "lower",
      },
      mem_settings: {
        readableName: "Memory Settings collection time",
        description: "The total time in us for APerf to collect the Memory Settings data.",
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
      memalloc: {
        readableName: "Memory allocation stats collection time",
        description: "The total time in us for APerf to collect the memory allocation stats data during one interval.",
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
      ena_stat: {
        readableName: "ENA stats collection time",
        description: "The total time in us for APerf to collect the ENA stats during one interval.",
        desired: "lower",
      },
      efa_stat: {
        readableName: "EFA stats collection time",
        description: "The total time in us for APerf to collect the EFA stats data during one interval.",
        desired: "lower",
      },
      numastat: {
        readableName: "NUMA stats collection time",
        description: "The total time in us for APerf to collect the NUMA stats data during one interval.",
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
