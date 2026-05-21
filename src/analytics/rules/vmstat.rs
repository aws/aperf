use crate::analytics::rule_templates::time_series_data_point_threshold_rule::time_series_data_point_threshold;
use crate::analytics::rule_templates::time_series_stat_run_comparison_rule::time_series_stat_run_comparison;
use crate::analytics::rule_templates::time_series_stat_threshold_rule::time_series_stat_threshold;
use crate::analytics::{
    AnalyticalRule, Score, TimeSeriesDataPointThresholdRule, TimeSeriesStatRunComparisonRule,
    TimeSeriesStatThresholdRule,
};
use crate::computations::{Comparator, Stat};
use crate::data::vmstat::Vmstat;
use crate::data::AnalyzeData;

impl AnalyzeData for Vmstat {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_data_point_threshold!(
                name: "Major Page Faults Detected",
                metric: "pgmajfault",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Poor,
                message: "Major page faults require disk I/O to resolve and can severely impact latency. Investigate memory pressure or insufficient page cache.",
            ),
            time_series_stat_run_comparison!(
                name: "Increased Minor Page Faults",
                metric: "pgminorfault",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.2,
                score: Score::Concerning,
                message: "A significant increase in minor page faults may indicate changes in memory access patterns or working set size.",
            ),
            time_series_stat_run_comparison!(
                name: "Increased Major Page Faults",
                metric: "pgmajfault",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.2,
                score: Score::Poor,
                message: "A significant increase in major page faults indicates growing disk-backed paging activity, which degrades performance.",
            ),
            time_series_stat_threshold!(
                name: "Excessive Page Fault Rate",
                metric: "pgfault",
                stat: Stat::P99,
                comparator: Comparator::Greater,
                threshold: 100000.0,
                score: Score::Concerning,
                message: "Very high page fault rate (>100k/s at P99) may indicate memory thrashing or an application allocating memory excessively.",
            ),
        ]
    }
}
