use crate::analytics::rule_templates::{
    time_series_run_stat_comparison_rule::time_series_run_stat_comparison,
    time_series_single_metric_data_point_rule::time_series_single_data_point,
};
use crate::analytics::{
    AnalyticalRule, TimeSeriesRunStatComparisonRule, TimeSeriesSingleMetricDataPointRule,
};
use crate::computations::{Comparator, Stat};
use crate::data::perf_stat::PerfStat;
use crate::data::AnalyzeData;

impl AnalyzeData for PerfStat {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_run_stat_comparison! {
                metric_name: "ipc",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                ratio: 1.1,
                score: -1.0,
                message: "",
            },
            time_series_single_data_point!(
                metric_name: "data-l1-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 20.0,
                score: -2.0,
                message: "A large number of L1 cache miss means code locality can be improved.",
            ),
            time_series_single_data_point!(
                metric_name: "data-l2-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 10.0,
                score: -2.0,
                message: "A large number of L2 cache miss means code locality can be improved.",
            ),
            time_series_single_data_point!(
                metric_name: "data-l3-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 2.0,
                score: -2.0,
                message: "A large number of L3 cache miss means code locality can be improved.",
            ),
        ]
    }
}
