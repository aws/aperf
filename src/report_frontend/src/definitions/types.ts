export const ALL_DATA_TYPES = [
  "systeminfo",
  "cpu_utilization",
  "processes",
  "perf_stat",
  "meminfo",
  "vmstat",
  "interrupts",
  "diskstats",
  "netstat",
  "kernel_config",
  "sysctl",
  "flamegraphs",
  "perf_profile",
  "java_profile",
  "hotline",
  "aperf_runlog",
  "aperf_stats",
] as const;
export type DataType = (typeof ALL_DATA_TYPES)[number];

export type DataFormat = "time_series" | "key_value" | "text" | "graph" | "unknown";

// See src/data/data_formats.rs
export type AperfData = TimeSeriesData | KeyValueData | TextData | GraphData;

export interface ReportData {
  readonly data_name: DataType;
  readonly data_format: DataFormat;
  readonly runs: { [key in string]: AperfData };
}

export interface TimeSeriesData {
  readonly metrics: { [key in string]: TimeSeriesMetric };
  readonly sorted_metric_names: string[];
}

export interface TimeSeriesMetric {
  readonly metric_name: string;
  readonly series: Series[];
  readonly value_range: number[];
  readonly stats: Statistics;
}

export interface Series {
  readonly series_name?: string;
  readonly time_diff: number[];
  readonly values: number[];
  readonly is_aggregate: boolean;
}

export interface KeyValueData {
  readonly key_value_groups: { [key in string]: KeyValueGroup };
}

export interface KeyValueGroup {
  readonly key_values: { [key in string]: string };
}

export interface TextData {
  readonly lines: string[];
}

export interface GraphData {
  readonly graph_groups: GraphGroup[];
}

export interface GraphGroup {
  readonly group_name: string;
  readonly graphs: { [key in string]: GraphInfo };
}

export interface GraphInfo {
  readonly graph_name: string;
  readonly graph_path: string;
  readonly graph_size?: number;
}

export const ALL_STATS = ["avg", "std", "min", "max", "p50", "p90", "p99", "p99_9"] as const;
export type Stat = (typeof ALL_STATS)[number];
export type Statistics = { [key in Stat]: number };

// See src/analytics/mod.rs
export interface DataFindings {
  readonly per_run_findings: { [key in string]: RunFindings };
}

export interface RunFindings {
  readonly findings: { [key in string]: AnalyticalFinding[] };
}

export interface AnalyticalFinding {
  readonly description: string;
  readonly score: number;
}

export interface DataPageProps {
  readonly dataType: DataType;
}

export interface TimeSeriesMetricProps {
  readonly dataType: DataType;
  readonly runName: string;
  readonly metricName: string;
}

export type NumCpusPerRun = { [key in string]: number };
export type SelectedCpusPerRun = { [key in string]: { aggregate: boolean; cpus: boolean[] } };

export const ALL_FINDING_TYPES = ["negative", "zero", "positive"] as const;
export type FindingType = (typeof ALL_FINDING_TYPES)[number];

export type SplitPanelType = "analytical" | "statistical";
