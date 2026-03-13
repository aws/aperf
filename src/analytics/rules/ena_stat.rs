use crate::analytics::rule_templates::time_series_data_point_threshold_rule::time_series_data_point_threshold;
use crate::analytics::rule_templates::time_series_stat_intra_run_comparison_rule::time_series_stat_intra_run_comparison;
use crate::analytics::rule_templates::time_series_stat_threshold_rule::time_series_stat_threshold;
use crate::analytics::{
    AnalyticalRule, Score, TimeSeriesDataPointThresholdRule, TimeSeriesStatIntraRunComparisonRule,
    TimeSeriesStatThresholdRule,
};
use crate::computations::{Comparator, Stat};
use crate::data::ena_stat::EnaStat;
use crate::data::AnalyzeData;

impl AnalyzeData for EnaStat {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_data_point_threshold!(
                name: "Inbound Bandwidth Exceeded Instance Allowance",
                metric: "bw_in_allowance_exceeded",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Critical,
                message: "EFA is queuing or dropping packets due to exceeding the instance's inbound bandwidth allowance.",
            ),
            time_series_data_point_threshold!(
                name: "Outbound Bandwidth Exceeded Instance Allowance",
                metric: "bw_out_allowance_exceeded",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Critical,
                message: "EFA is queuing or dropping packets due to exceeding the instance's outbound bandwidth allowance.",
            ),
            time_series_data_point_threshold!(
                name: "PPS Exceeded Instance Allowance",
                metric: "pps_allowance_exceeded",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Critical,
                message: "EFA is queuing or dropping packets due to exceeding the instance's bidirectional PPS maximum.",
            ),
            time_series_data_point_threshold!(
                name: "Connection Tracking Exceeded Instance Allowance",
                metric: "conntrack_allowance_exceeded",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Critical,
                message: "EFA is dropping packets because connection tracking exceeded the maximum for the instance and new connections could not be established.",
            ),
            time_series_data_point_threshold!(
                name: "Link-Local Service Exceeded Instance Allowance",
                metric: "linklocal_allowance_exceeded",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Critical,
                message: "EFA is dropping packets because the PPS of the traffic to local proxy services exceeded the maximum for the network interface. Consider caching DNS responses locally or reducing the rate of metadata queries.",
            ),
            time_series_data_point_threshold!(
                name: "Disabled ENA Express",
                metric: "ena_srd_mode",
                comparator: Comparator::Equal,
                threshold: 0.0,
                score: Score::Concerning,
                message: "ENA Express is disabled. Verify if this is expected.",
            ),
            time_series_data_point_threshold!(
                name: "Enabled ENA Express without UDP",
                metric: "ena_srd_mode",
                comparator: Comparator::Equal,
                threshold: 1.0,
                score: Score::Concerning,
                message: "ENA Express is enabled but only for TCP traffics. Verify if ENA Express with UDP needs to be enabled too.",
            ),
            time_series_data_point_threshold!(
                name: "Disabled ENA Express with Previously Enabled UDP",
                metric: "ena_srd_mode",
                comparator: Comparator::Equal,
                threshold: 2.0,
                score: Score::Bad,
                message: "ENA Express is disabled, but it was previously enabled with UDP support. Verify if this is expected.",
            ),
            time_series_data_point_threshold!(
                name: "ENA Express Enabled with UDP",
                metric: "ena_srd_mode",
                comparator: Comparator::Equal,
                threshold: 3.0,
                score: Score::Good,
                message: "ENA Express is enabled for both TCP and UDP.",
            ),
            time_series_stat_intra_run_comparison!(
                name: "Low SRD Usage Rate",
                baseline_metric: "ena_srd_tx_pkts",
                comparison_metric: "ena_srd_eligible_tx_pkts",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.3,
                score: Score::Poor,
                message: "A large number of SRD-eligible packets are not transmitted using SRD, potentially caused by resource utilization issues.",
            ),
            time_series_stat_threshold!(
                name: "High SRD Resource Utilization",
                metric: "ena_srd_resource_utilization",
                stat: Stat::P99,
                comparator: Comparator::Greater,
                threshold: 95.0,
                score: Score::Poor,
                message: "Concurrent SRD connections are having a high resource utilization, which can lead to fallback from SRD to standard ENA transmission and packet drops.",
            ),
        ]
    }
}
