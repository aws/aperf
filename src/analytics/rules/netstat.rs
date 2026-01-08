use crate::analytics::rule_templates::time_series_stat_run_comparison_rule::time_series_stat_run_comparison;
use crate::analytics::{AnalyticalRule, Score, TimeSeriesStatRunComparisonRule};
use crate::computations::{Comparator, Stat};
use crate::data::netstat::Netstat;
use crate::data::AnalyzeData;

impl AnalyzeData for Netstat {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_run_comparison! (
                name: "Inconsistent Inbound Network Traffic",
                metric: "IpExt:InOctets",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: true,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "The average number of bytes received by the network interface is different. Verify that the load generator is providing the expected traffics, if the system is under tests." ,
                reference: "https://aws.github.io/graviton/perfrunbook/debug_system_perf.html#check-network-usage",
            ),
            time_series_stat_run_comparison! (
                name: "Inconsistent Outbound Network Traffic",
                metric: "IpExt:OutOctets",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: true,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "The average number of bytes transmitted by the network interface is different. Look for heavily used connections on the system through \"watch netstat -t\".",
                reference: "https://aws.github.io/graviton/perfrunbook/debug_system_perf.html#check-network-usage",
            ),
        ]
    }
}
