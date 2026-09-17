use crate::analytics::rule_templates::time_series_stat_intra_run_comparison_rule::time_series_stat_intra_run_comparison;
use crate::analytics::rule_templates::time_series_stat_run_comparison_rule::time_series_stat_run_comparison;
use crate::analytics::rule_templates::time_series_stat_threshold_rule::time_series_stat_threshold;
use crate::analytics::{
    AnalyticalRule, Score, TimeSeriesStatIntraRunComparisonRule, TimeSeriesStatRunComparisonRule,
    TimeSeriesStatThresholdRule,
};
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
            time_series_stat_intra_run_comparison!(
                name: "Low Memory Availability",
                baseline_metric: "MemTotal",
                comparison_metric: "MemAvailable",
                stat: Stat::Min,
                comparator: Comparator::LessEqual,
                abs: false,
                delta_ratio: -0.9,
                score: Score::Concerning,
                message: "High memory usage leaves little headroom for allocation spikes. Check the Resident Set Size metric in the Processes data to see if the processes holding the memories are expected.",
            ),
            time_series_stat_threshold!(
                name: "Underused Reserved Huge Pages",
                metric: "Hugetlb_Unused_Memory_Percent",
                stat: Stat::Min,
                comparator: Comparator::GreaterEqual,
                threshold: 5.0,
                score: Score::Concerning,
                message: "A meaningful share of memory is reserved in the HugeTLB pools but never allocated or committed during the run. Check the huge page pool size settings in the Memory Settings data.",
            ),
            time_series_stat_threshold!(
                name: "High Page Table Memory",
                metric: "PageTables_Memory_Percent",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                threshold: 20.0,
                score: Score::Poor,
                message: "A large share of memory is spent on page tables rather than on the workload itself. Backing the mapped memory with huge pages, or reducing the number of processes that map it, would return most of it.",
            ),
        ]
    }
}
