use crate::analytics::rule_templates::time_series_stat_run_comparison_rule::time_series_stat_run_comparison;
use crate::analytics::{AnalyticalRule, Score, TimeSeriesStatRunComparisonRule};
use crate::computations::{Comparator, Stat};
use crate::data::netstat::Netstat;
use crate::data::AnalyzeData;

impl AnalyzeData for Netstat {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_run_comparison! (
                metric_name: "IpExt:InOctets",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: true,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "The average number of bytes received by the network interface is different between runs. Verify expected behavior for the load generator. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_system_perf.md#check-network-usage",
            ),
            time_series_stat_run_comparison! (
                metric_name: "IpExt:OutOctets",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: true,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "The average number of bytes transmitted by the network interface is different between runs. Verify expected behavior for the SUT. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_system_perf.md#check-network-usage",
            ),
        ]
    }
}
