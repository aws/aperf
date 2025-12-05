use crate::analytics::rule_templates::key_value_run_comparison_rule::key_value_comparison;
use crate::analytics::AnalyticalRule;
use crate::analytics::KeyValueRunComparisonRule;
use crate::data::systeminfo::SystemInfo;
use crate::data::AnalyzeData;

impl AnalyzeData for SystemInfo {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            key_value_comparison! {
                key_group: "",
                key: "Kernel Version",
                score: -1.0,
                message: "Kernel Versions between runs is different, make sure this is intended.",
            },
            key_value_comparison! {
                key_group: "",
                key: "CPUs",
                score: -1.0,
                message: "",
            },
        ]
    }
}
