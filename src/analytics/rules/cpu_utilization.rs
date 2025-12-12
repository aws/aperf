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
                metric_name: "aggregate",
                stat: Stat::Average,
                comparator: Comparator::LessEqual,
                abs: true,
                delta_ratio: 0.01,
                score: Score::Good,
                message: "The CPU utilization between two runs are similar.",
            },
            time_series_stat_threshold! {
                metric_name: "idle",
                stat: Stat::Average,
                comparator: Comparator::Greater,
                threshold: 50.0,
                score: Score::Bad,
                message: "CPU utilization might not be maximized.",
            },
            time_series_stat_run_comparison! {
                metric_name: "user",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "CPU usage is higher, proceed to profile for hot-functions. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_code_perf.md",
            },
            time_series_stat_run_comparison! {
                metric_name: "user",
                stat: Stat::Average,
                comparator: Comparator::LessEqual,
                abs: false,
                delta_ratio: -0.1,
                score: Score::Bad,
                message: "CPU usage is lower, some functions may be putting threads to sleep and causing the CPU to go idle more. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_code_perf.md",
            },
            time_series_stat_run_comparison! {
                metric_name: "iowait",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "Higher cpu-iowait indicates a bottleneck in disk operations. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_system_perf.md#check-cpu-usage",
            },
        ]
    }
}
