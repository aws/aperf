import { DataType } from "./types";

type DesiredValue = "higher" | "lower" | "moderate" | "fixed" | "depends";

interface DataDescription {
  readonly readableName: string;
  readonly summary: string;
  readonly defaultUnit?: string;
  readonly yDomain?: number[];
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
    readableName: "Report",
    summary:
      "The APerf report homepage provides overviews of each recording run. In this page, you can view every run's system information and analytical findings. For more details, use the side navigation panel to open a specific data's report, which includes everything that APerf collected.",
    fieldDescriptions: {},
  },
  cpu_utilization: {
    readableName: "CPU Utilization",
    summary:
      "CPU utilization metrics measure the percentage of CPU time spent in various CPU state. The data were collected and computed from the system pseudo-file /proc/stat. Every metric graph shows the percentage of time spent in the corresponding state for each CPU, as well as the aggregate of all CPUs. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for its aggregate series.",
    defaultUnit: "Utilization (%)",
    yDomain: [0, 100],
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
    },
  },
  processes: {
    readableName: "Processes",
    summary:
      "Processes metrics monitor usage of various resources for processes running on the system during APerf collection. The data were collected and computed from the system pseudo-files /proc/<pid>/stat. Every metric graph contains the top 16 processes in the highest average usage of the corresponding resource. The stats of a metric graph accounts for the process with the highest average.",
    defaultUnit: "Count",
    fieldDescriptions: {},
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
      },
    },
  },
  meminfo: {
    readableName: "Memory Usage",
    summary:
      "Memory usage metrics measure the usage of the system's physical memory. The data were collected from the system pseudo-file /proc/meminfo.",
    defaultUnit: "GB",
    fieldDescriptions: {},
  },
  vmstat: {
    readableName: "Virtual Memory Stats",
    summary:
      "Virtual memory metrics measure the usage of the system's virtual memory. The data were collected from the system pseudo-file /proc/vmstat. Note that for some metrics, the values were computed using the delta of two snapshots, so that first value is always zero.",
    defaultUnit: "Pages",
    fieldDescriptions: {},
  },
  interrupts: {
    readableName: "Interrupts",
    summary:
      "Interrupt metrics measure that number of interrupts handled by each CPU. The data were collected from the system pseudo-file /proc/interrupts. Every metric graph show the number of times a specific interrupt was handled by each CPU, as well as the aggregate (average) of all CPUs. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for its aggregate series.",
    defaultUnit: "Counts",
    fieldDescriptions: {},
  },
  diskstats: {
    readableName: "Disk Stats",
    summary:
      "Disk stats metrics measure the I/O stats for each disk device and partition of the system. Note that since the metric values were computed using the delta between two snapshots, the first value is always zero. The statistics of a metric graph accounts for the device series with the highest average.",
    defaultUnit: "Counts",
    fieldDescriptions: {},
  },
  netstat: {
    readableName: "Network Stats",
    summary:
      "Network stats metrics measure various networking stats for different protocols (TCP, IP, etc.). Note that since the metric values were computed using the delta between two snapshots, the first value is always zero.",
    defaultUnit: "Counts",
    fieldDescriptions: {},
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
    readableName: "Kernel Profiling Flamegraphs",
    summary:
      "Kernel profiling flamegraphs visualize the call stack hierarchies and the amount of CPU time consumed by different functions. It supports viewing in both the normal (bottom-top) and reverse (top-bottom) order.",
    fieldDescriptions: {},
  },
  perf_profile: {
    readableName: "Kernel Profiling Top Functions",
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
    fieldDescriptions: {},
  },
};
