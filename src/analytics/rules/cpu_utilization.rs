use crate::analytics::rule_templates::{
    time_series_stat_run_comparison_rule::time_series_stat_run_comparison,
    time_series_stat_threshold_rule::time_series_stat_threshold,
};
use crate::analytics::{
    AnalyticalRule, Score, TimeSeriesStatRunComparisonRule, TimeSeriesStatThresholdRule,
};
use crate::computations::{Comparator, Stat};
use crate::data::{cpu_utilization::CpuUtilization, AnalyzeData};

impl AnalyzeData for CpuUtilization {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_run_comparison! {
                name: "Consistent CPU Utilization",
                metric: "aggregate",
                stat: Stat::Average,
                comparator: Comparator::LessEqual,
                abs: true,
                delta_ratio: 0.01,
                score: Score::Good,
                message: "Similar amount of CPU resources are being utilized between two runs.",
            },
            time_series_stat_threshold! {
                name: "Underutilized CPU",
                metric: "idle",
                stat: Stat::Average,
                comparator: Comparator::Greater,
                threshold: 50.0,
                score: Score::Critical,
                message: "CPU utilization might not be maximized.",
            },
            time_series_stat_run_comparison! {
                name: "Increased User-Space CPU Utilization",
                metric: "user",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Poor,
                message: "The code is consuming more CPU time. The results of CPU profiling could help identify hot functions.",
                reference: "https://aws.github.io/graviton/perfrunbook/debug_code_perf.html#on-cpu-profiling",
            },
            time_series_stat_run_comparison! {
                name: "Decreased User-Space CPU Utilization",
                metric: "user",
                stat: Stat::Average,
                comparator: Comparator::LessEqual,
                abs: false,
                delta_ratio: -0.1,
                score: Score::Poor,
                message: "The code is consuming less CPU time. Some functions may be putting threads to sleep and causing the CPU to go idle more.",
                reference: "https://aws.github.io/graviton/perfrunbook/debug_code_perf.html#off-cpu-profiling",
            },
            time_series_stat_run_comparison! {
                name: "Increased I/O Wait Time",
                metric: "iowait",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Poor,
                message: "Higher iowait time indicates a bottleneck in disk operations. If the system uses an NFS, consider expanding the provision or switching to local storage.",
                reference: "https://aws.github.io/graviton/perfrunbook/debug_system_perf.html#check-cpu-usage",
            },
        ]
    }
}
