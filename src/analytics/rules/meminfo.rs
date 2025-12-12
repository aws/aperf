use crate::analytics::rule_templates::time_series_stat_run_comparison_rule::time_series_stat_run_comparison;
use crate::analytics::{AnalyticalRule, Score, TimeSeriesStatRunComparisonRule};
use crate::computations::{Comparator, Stat};
use crate::data::meminfo::MeminfoData;
use crate::data::AnalyzeData;

impl AnalyzeData for MeminfoData {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_run_comparison! (
                metric_name: "mem_total",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: true,
                delta_ratio: 0.1,
                score: Score::VeryBad,
                message: "Total memory between runs is different, which may result in significantly different performance between runs.",
            ),
            time_series_stat_run_comparison! (
                metric_name: "mem_available",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_system_perf.md#check-system-memory-usage",
            ),
        ]
    }
}
