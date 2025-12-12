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
                key_group: "",
                key: "Kernel Version",
                score: Score::Bad,
                message: "Kernel Versions between runs is different, make sure this is intended.",
            },
            key_value_key_run_comparison! {
                key_group: "",
                key: "CPUs",
                score: Score::Bad,
                message: "The number of CPUs between runs is different, which may result in differing performance.",
            },
        ]
    }
}
