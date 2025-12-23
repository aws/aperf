use crate::analytics::rule_templates::key_value_key_run_comparison_rule::key_value_key_run_comparison;
use crate::analytics::AnalyticalRule;
use crate::analytics::KeyValueKeyRunComparisonRule;
use crate::analytics::Score;
use crate::data::systeminfo::SystemInfo;
use crate::data::AnalyzeData;

impl AnalyzeData for SystemInfo {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            key_value_key_run_comparison! {
                name: "Kernel Version Mismatch",
                key: "Kernel Version",
                score: Score::Critical,
                message: "Kernel Versions between runs are different. Make sure the difference is intended.",
            },
            key_value_key_run_comparison! {
                name: "Inconsistent Number of CPUs",
                key: "CPUs",
                score: Score::Critical,
                message: "Different amount of CPU resources could result in significant performance discrepancy.",
            },
        ]
    }
}
