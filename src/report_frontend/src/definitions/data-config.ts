import {DataFindings, DataType, ReportData} from "./types";

declare let runs_raw;
declare let version_info;
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
declare let systeminfo_findings;
declare let cpu_utilization_findings;
declare let vmstat_findings;
declare let kernel_config_findings;
declare let sysctl_findings;
declare let interrupts_findings;
declare let diskstats_findings;
declare let perf_stat_findings;
declare let processes_findings;
declare let meminfo_findings;
declare let netstat_findings;
declare let perf_profile_findings;
declare let flamegraphs_findings;
declare let aperf_stats_findings;
declare let java_profile_findings;
declare let aperf_runlog_findings;
declare let hotline_findings;

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

export const ANALYTICAL_FINDINGS: { [key in DataType]: DataFindings } = {
  systeminfo: systeminfo_findings,
  cpu_utilization: cpu_utilization_findings,
  processes: processes_findings,
  perf_stat: perf_stat_findings,
  meminfo: meminfo_findings,
  vmstat: vmstat_findings,
  interrupts: interrupts_findings,
  diskstats: diskstats_findings,
  netstat: netstat_findings,
  kernel_config: kernel_config_findings,
  sysctl: sysctl_findings,
  flamegraphs: flamegraphs_findings,
  perf_profile: perf_profile_findings,
  java_profile: java_profile_findings,
  hotline: hotline_findings,
  aperf_runlog: aperf_runlog_findings,
  aperf_stats: aperf_stats_findings,
};

export const RUNS: string[] = Array.from(runs_raw);

export const VERSION_INFO = version_info;

export const CPU_DATA_TYPES: DataType[] = ["cpu_utilization", "perf_stat", "interrupts"];

interface NavigationConfig {
  readonly sectionName: string;
  readonly items: DataType[];
}

export const NAVIGATION_CONFIGS: NavigationConfig[] = [
  {
    sectionName: "Performance Data",
    items: ["cpu_utilization", "perf_stat", "meminfo", "vmstat", "interrupts", "diskstats", "netstat", "processes"],
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
    sectionName: "APerf Execution",
    items: ["aperf_stats", "aperf_runlog"],
  },
];
