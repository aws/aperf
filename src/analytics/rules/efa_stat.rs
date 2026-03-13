use crate::analytics::rule_templates::time_series_stat_run_comparison_rule::time_series_stat_run_comparison;
use crate::analytics::rule_templates::time_series_stat_threshold_rule::time_series_stat_threshold;
use crate::analytics::{
    AnalyticalRule, Score, TimeSeriesStatRunComparisonRule, TimeSeriesStatThresholdRule,
};
use crate::computations::{Comparator, Stat};
use crate::data::efa_stat::EfaStat;
use crate::data::AnalyzeData;

impl AnalyzeData for EfaStat {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_threshold!(
                name: "Unused RDMA Read",
                metric: "rdma_read_wrs",
                stat: Stat::Max,
                comparator: Comparator::Equal,
                threshold: 0.0,
                score: Score::Concerning,
                message: "No RDMA read operations were completed. Verify that this is the expected behavior.",
            ),
            time_series_stat_threshold!(
                name: "Unused RDMA Write",
                metric: "rdma_write_wrs",
                stat: Stat::Max,
                comparator: Comparator::Equal,
                threshold: 0.0,
                score: Score::Concerning,
                message: "No RDMA write operations were completed. Verify that this is the expected behavior.",
            ),
            time_series_stat_run_comparison!(
                name: "Inconsistent Inbound EFA Traffics",
                metric: "rx_bytes",
                stat: Stat::Average,
                comparator: Comparator::Greater,
                abs: true,
                delta_ratio: 0.1,
                score: Score::Critical,
                message: "The average numbers of bytes received by EFA are different. Verify if this is the expected behavior.",
            ),
            time_series_stat_run_comparison!(
                name: "Inconsistent Outbound EFA Traffics",
                metric: "tx_bytes",
                stat: Stat::Average,
                comparator: Comparator::Greater,
                abs: true,
                delta_ratio: 0.1,
                score: Score::Critical,
                message: "The average numbers of bytes transmitted by EFA are different. Verify if this is the expected behavior.",
            ),
        ]
    }
}
