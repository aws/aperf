use crate::analytics::rule_templates::time_series_stat_run_comparison_rule::time_series_stat_run_comparison;
use crate::analytics::{AnalyticalRule, Score, TimeSeriesStatRunComparisonRule};
use crate::computations::{Comparator, Stat};
use crate::data::meminfo::MeminfoData;
use crate::data::AnalyzeData;

impl AnalyzeData for MeminfoData {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_run_comparison! (
                name: "Inconsistent Physical Memory",
                metric: "MemTotal",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: true,
                delta_ratio: 0.1,
                score: Score::Critical,
                message: "Different amount of total physical memory could result in significant performance discrepancy.",
            ),
            time_series_stat_run_comparison! (
                name: "Reduced Memory Availability",
                metric: "MemAvailable",
                stat: Stat::Average,
                comparator: Comparator::LessEqual,
                abs: false,
                delta_ratio: -0.1,
                score: Score::Poor,
                message: "The system is under a higher memory pressure (if the total memory is consistent between runs).",
            ),
        ]
    }
}
