import { DataType, ReportData } from "./types";

declare let runs_raw;
declare let processed_systeminfo_data;
declare let processed_cpu_utilization_data;
declare let processed_vmstat_data;
declare let processed_kernel_config_data;
declare let processed_sysctl_data;
declare let processed_interrupts_data;
declare let processed_diskstats_data;
declare let processed_perf_stat_data;
declare let processed_processes_data;
declare let processed_meminfo_data;
declare let processed_netstat_data;
declare let processed_perf_profile_data;
declare let processed_flamegraphs_data;
declare let processed_aperf_stats_data;
declare let processed_java_profile_data;
declare let processed_aperf_runlog_data;
declare let processed_hotline_data;

export const PROCESSED_DATA: { [key in DataType]: ReportData } = {
  systeminfo: processed_systeminfo_data,
  cpu_utilization: processed_cpu_utilization_data,
  processes: processed_processes_data,
  perf_stat: processed_perf_stat_data,
  meminfo: processed_meminfo_data,
  vmstat: processed_vmstat_data,
  interrupts: processed_interrupts_data,
  diskstats: processed_diskstats_data,
  netstat: processed_netstat_data,
  kernel_config: processed_kernel_config_data,
  sysctl: processed_sysctl_data,
  flamegraphs: processed_flamegraphs_data,
  perf_profile: processed_perf_profile_data,
  java_profile: processed_java_profile_data,
  hotline: processed_hotline_data,
  aperf_runlog: processed_aperf_runlog_data,
  aperf_stats: processed_aperf_stats_data,
};

export const RUNS: string[] = Array.from(runs_raw);

export const CPU_DATA_TYPES: DataType[] = ["cpu_utilization", "perf_stat", "interrupts"];

interface NavigationConfig {
  readonly sectionName: string;
  readonly items: DataType[];
}

export const NAVIGATION_CONFIGS: NavigationConfig[] = [
  {
    sectionName: "Performance Data",
    items: ["cpu_utilization", "processes", "perf_stat", "meminfo", "vmstat", "interrupts", "diskstats", "netstat"],
  },
  {
    sectionName: "System Configurations",
    items: ["kernel_config", "sysctl"],
  },
  {
    sectionName: "Profiling",
    items: ["flamegraphs", "perf_profile", "java_profile", "hotline"],
  },
  {
    sectionName: "Aperf Execution",
    items: ["aperf_stats", "aperf_runlog"],
  },
];
