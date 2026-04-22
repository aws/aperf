use crate::analytics::rule_templates::profile_metadata_comparison_rule::profile_metadata_comparison;
use crate::analytics::rule_templates::profile_metadata_expected_rule::profile_metadata_expected;
use crate::analytics::rule_templates::profile_stack_frame_threshold_rule::profile_stack_frame_threshold;
use crate::analytics::{
    AnalyticalRule, ProfileMetadataComparisonRule, ProfileMetadataExpectedRule,
    ProfileStackFrameThresholdRule, Score,
};
use crate::data::java_profile::JavaProfile;
use crate::data::AnalyzeData;
use crate::profiling::ThreadState;

impl AnalyzeData for JavaProfile {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            // Anti-Pattern Rules
            // If rule for cpu or alloc group, use only ThreadState::AsyncDefault
            profile_stack_frame_threshold! {
                name: "Excessive Exceptions",
                profile_type: "cpu",
                stack_frame: [[r"java\.lang\.(Throwable\.(fillInStackTrace|getStackTrace|getOurStackTrace)|StackTraceElement\.<init>)|\w+(\.\w+)*\.((\w*Exception|\w*Error|Throwable)\.<init>)"]],
                frame_type: None,
                thread_states: [ThreadState::AsyncDefault],
                aggregate_occurences: true,
                total_samples: true,
                threshold: 5.0,
                score: Score::Critical,
                message: "We recommend not to use Java exceptions as control flow, and to remove exceptions when they appear in the hot-code path. Overhead can be mitigated some by using the -XX:+OmitStackTraceInFastThrow JVM flag to allow the Java runtime to optimize the exception flow for some hot paths. The best solution is to avoid the exceptions as much as possible.",
            },
            // Other Rules
            profile_metadata_expected! {
                name: "Tiered Compilation Check",
                group: "jdk.JVMInformation",
                key: "jvmArguments",
                expected_value: ".*-XX:(-|\\+)TieredCompilation.*",
                should_exist: true,
                score: Score::Poor,
                message: "If your code has a large instruction footprint, try setting this argument. This feature enables the runtime to more adaptively use the just-in-time (JIT) compiler to achieve better performance.",
            },
            profile_metadata_comparison! {
                name: "JVM Version Comparison",
                group: "jdk.JVMInformation",
                key: "jvmVersion",
                should_exist: true,
                score: Score::Critical,
                message: "JVM versions are different across runs, which may significantly affect performance. Ensure that this is intentional, or run comparison using consistent versions.",
            },
        ]
    }
}
