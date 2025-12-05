use crate::analytics::rule_templates::{
    time_series_run_stat_comparison_rule::time_series_run_stat_comparison,
    time_series_run_stat_similarity_rule::time_series_run_stat_similarity,
    time_series_single_metric_stat_rule::time_series_single_metric_stat,
};
use crate::analytics::{
    AnalyticalRule, TimeSeriesRunStatComparisonRule, TimeSeriesRunStatSimilarityRule,
    TimeSeriesSingleMetricStatRule,
};
use crate::computations::{Comparator, Stat};
use crate::data::{cpu_utilization::CpuUtilization, AnalyzeData};

impl AnalyzeData for CpuUtilization {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_run_stat_similarity! {
                metric_name: "aggregate",
                stat: Stat::Average,
                delta_ratio: 0.01,
                score: 1.0,
                message: "The CPU utilization between two runs are similar.",
            },
            time_series_single_metric_stat! {
                metric_name: "idle",
                stat: Stat::Average,
                comparator: Comparator::Greater,
                threshold: 50.0,
                score: -1.0,
                message: "CPU utilization might not be maximized.",
            },
            time_series_run_stat_comparison! {
                metric_name: "user",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                ratio: 1.1,
                score: 0.5,
                message: "",
            },
        ]
    }
}
