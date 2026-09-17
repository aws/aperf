use crate::analytics::rule_templates::key_value_key_expected_rule::key_value_key_unexpected;
use crate::analytics::rule_templates::key_value_key_run_comparison_rule::key_value_key_run_comparison;
use crate::analytics::{
    AnalyticalRule, KeyValueKeyExpectedRule, KeyValueKeyRunComparisonRule, Score,
};
use crate::data::mem_settings::MemSettings;
use crate::data::AnalyzeData;

impl AnalyzeData for MemSettings {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            key_value_key_unexpected! {
                name: "Transparent Huge Pages Disabled",
                key: "transparent_hugepage/enabled",
                unexpected_value: "never",
                score: Score::Concerning,
                message: "Transparent Huge Page is turned off. Using hugepages should reduce TLB pressure and generally improve performance on all EC2 instance types, but using exclusively hugepages may sometime lead to performance degradation. Fully test your application after enabling and/or allocating huge-pages.",
            },
            key_value_key_run_comparison! {
                name: "Inconsistent Transparent Huge Page Config",
                key: "transparent_hugepage/enabled",
                score: Score::Poor,
                message: "Transparent Huge Page is configured differently, which affects both TLB pressure and memory footprint. Make sure the difference is intended.",
            },
            key_value_key_run_comparison! {
                name: "Inconsistent Huge Page Allocation Effort",
                key: "transparent_hugepage/defrag",
                score: Score::Poor,
                message: "Huge page allocation effort is configured differently, which affects how long an application stalls when no huge page is available. Make sure the difference is intended.",
            },
        ]
    }
}
