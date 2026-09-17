use crate::analytics::rule_templates::key_value_key_expected_rule::key_value_key_expected;
use crate::analytics::rule_templates::key_value_key_run_comparison_rule::key_value_key_run_comparison;
use crate::analytics::{
    AnalyticalRule, KeyValueKeyExpectedRule, KeyValueKeyRunComparisonRule, Score,
};
use crate::data::kernel_config::KernelConfig;
use crate::data::AnalyzeData;

impl AnalyzeData for KernelConfig {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            key_value_key_expected! {
                name: "Transparent Huge Page Unsupported",
                key: "CONFIG_TRANSPARENT_HUGEPAGE",
                expected_value: "y",
                score: Score::Poor,
                message: "The Kernel was not built with Transparent Huge Page support. Using hugepages should reduce TLB pressure and generally improve performance on all EC2 instance types, but using exclusively hugepages may sometime lead to performance degradation. Fully test your application after enabling and/or allocating huge-pages.",
            },
            key_value_key_expected! {
                name: "Block Writeback Throttling Disabled",
                key: "CONFIG_BLK_WBT",
                expected_value: "y",
                score: Score::Poor,
                message: "Without writeback throttling, I/O submitted by the application can be queued behind bulk background writeback and take longer to complete than expected. If this system runs a write-heavy workload that is sensitive to I/O latency, consider using a kernel built with this option enabled. Most distribution kernels enable it, so a kernel without it is often a custom or older build.",
            },
            key_value_key_expected! {
                name: "Multi-Queue Block Writeback Throttling Disabled",
                key: "CONFIG_BLK_WBT_MQ",
                expected_value: "y",
                score: Score::Poor,
                message: "While this option is disabled, writeback throttling has no effect on multi-queue devices such as NVMe and virtio storage, even if CONFIG_BLK_WBT is enabled.",
            },
            key_value_key_run_comparison! {
                name: "Inconsistent Kernel Timer Frequency",
                key: "CONFIG_HZ",
                score: Score::Poor,
                message: "The runs were built with different kernel timer tick frequencies, which can affect both throughput and latency independently of the workload. Make sure the difference is intended before attributing a performance difference between these runs to other causes.",
            },
        ]
    }
}
