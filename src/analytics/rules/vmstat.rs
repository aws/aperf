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
                name: "Out of Memory Kill Detected",
                metric: "oom_kill",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Critical,
                message: "One or more processes were killed by the kernel's Out-of-Memory killer. Check the kernel log to see what was killed and how much memory it held at the time.",
            ),
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
            time_series_stat_run_comparison!(
                name: "Inconsistent Dirty Page Cache Utilization",
                metric: "dirty_utilization",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: true,
                delta_absolute: 10.0,
                score: Score::Poor,
                message: "The runs used different proportions of the kernel's dirty page buffer, which can affect throughput on write-heavy workloads.",
            ),
            time_series_stat_threshold!(
                name: "Failing Memory Compaction",
                metric: "compact_fail",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                threshold: 2.0,
                score: Score::Poor,
                message: "The workload repeatedly stopped to compact memory in order to get a physically contiguous block, and the compaction did not produce one, so the time spent waiting achieved nothing.",
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
            time_series_data_point_threshold!(
                name: "Memory Swapped Out",
                metric: "pswpout",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Concerning,
                message: "The kernel ran out of memory to reclaim and started writing application memory to swap. This normally means the workload needs more memory than the system has, unless the system is configured to use compressed swap such as zram or zswap.",
            ),
            time_series_data_point_threshold!(
                name: "Memory Swapped In",
                metric: "pswpin",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Poor,
                message: "The workload accessed memory that had been swapped to disk, and had to wait for the disk to read it back. This is as slow as a major page fault.",
            ),
            time_series_stat_threshold!(
                name: "Application Stalled Reclaiming Memory",
                metric: "pgsteal_direct",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                threshold: 1000.0,
                score: Score::Concerning,
                message: "The workload had to free memory itself before its allocations could proceed, which stops it from running until enough memory is freed.",
            ),
        ]
    }
}
