use crate::analytics::rule_templates::profile_stack_frame_threshold_rule::profile_stack_frame_threshold;
use crate::analytics::{AnalyticalRule, ProfileStackFrameThresholdRule, Score};
use crate::data::flamegraphs::Flamegraph;
use crate::data::AnalyzeData;

impl AnalyzeData for Flamegraph {
    fn get_analytical_rules(&self) -> Vec<crate::analytics::AnalyticalRule> {
        vec![profile_stack_frame_threshold! {
                name: "Place Holder",
                graph_group: "default",
                stack_frame: [["place_holder_frame"]],
                frame_type: None,
                thread_states: [],
                aggregate_occurences: true,
                total_samples: true,
                threshold: 100.0,
                score: Score::Poor,
                message: "Resolution",
        }]
    }
}
