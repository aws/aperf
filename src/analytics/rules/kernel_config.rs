use crate::analytics::rule_templates::key_value_key_expected_rule::key_value_key_expected;
use crate::analytics::{AnalyticalRule, KeyValueKeyExpectedRule, Score};
use crate::data::kernel_config::KernelConfig;
use crate::data::AnalyzeData;

impl AnalyzeData for KernelConfig {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![key_value_key_expected! {
            name: "Huge Page Disabled",
            key: "CONFIG_TRANSPARENT_HUGEPAGE",
            expected_value: "y",
            score: Score::Poor,
            message: "Using huge-pages should reduce TLB pressure and generally improve performance on all EC2 instance types, but using exclusively huge-pages may sometime lead to performance degradation. Fully test your application after enabling and/or allocating huge-pages.",
        }]
    }
}
