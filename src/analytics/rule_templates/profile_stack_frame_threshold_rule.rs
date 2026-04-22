use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::data::common::data_formats::{AperfData, ProcessedData};
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use crate::profiling::{FrameType, ThreadState};
use std::fmt;
use std::fmt::Formatter;

/// This rule compares the % samples in a stack frame against a threshold, and generates a finding if samples > threshold.
/// stack_frame          - list of stack frame patterns; each pattern is a list of frames (root is index 0, leaf frame is last index)
/// frame_type           - For JFR, specify the frame type to match against the leaf frame. For other stacks, use Native or Any
/// thread_states        - For JFR, specify the thread states to include samples for. For other stacks, select None or leave empty.
/// total_samples        - Use total samples in function if true, otherwise use self samples
/// aggregate_occurences - Use the sum of all stack frame patterns if true, otherwise check threshold for patterns individually
/// threshold            - % samples to generate finding
pub struct ProfileStackFrameThresholdRule {
    pub rule_name: &'static str,
    pub profile_type: &'static str,
    pub stack_frame: &'static [&'static [&'static str]],
    pub frame_type: Option<FrameType>,
    pub thread_states: &'static [ThreadState],
    pub aggregate_occurences: bool,
    pub total_samples: bool,
    pub threshold: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! profile_stack_frame_threshold {
    {
        name: $rule_name:literal,
        profile_type: $profile_type:literal,
        stack_frame: [$([$($frame:literal),+]),+],
        frame_type: $frame_type:expr,
        $(thread_states: [$($state:expr),*],)?
        aggregate_occurences: $aggregate_occurences:literal,
        total_samples: $total_samples:literal,
        threshold: $threshold:expr,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::ProfileStackFrameThresholdRule(
            ProfileStackFrameThresholdRule {
                rule_name: $rule_name,
                profile_type: $profile_type,
                stack_frame: &[$(&[$($frame),+]),+],
                frame_type: $frame_type,
                thread_states: &[$($($state),*)?],
                aggregate_occurences: $aggregate_occurences,
                total_samples: $total_samples,
                threshold: $threshold,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use profile_stack_frame_threshold;

impl fmt::Display for ProfileStackFrameThresholdRule {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl Analyze for ProfileStackFrameThresholdRule {
    fn analyze(
        &self,
        report_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        _processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        for (run_name, run_data) in &processed_data.runs {
            let AperfData::Profile(profiling_data) = run_data else {
                continue;
            };

            for (key, profiler) in &profiling_data.profilers {
                if !profiler.profiles.contains_key(self.profile_type) {
                    continue;
                }
                let total_samples =
                    profiler.get_total_samples(self.profile_type, self.thread_states);

                let (sample_count, matched_pattern) =
                    self.stack_frame
                        .iter()
                        .fold((0u64, None), |(acc, acc_pat), pattern| {
                            let count = profiler.get_samples(
                                self.profile_type,
                                pattern,
                                self.frame_type,
                                self.thread_states,
                                self.total_samples,
                            );
                            if self.aggregate_occurences {
                                (acc + count, None)
                            } else if count > acc {
                                (count, Some(pattern))
                            } else {
                                (acc, acc_pat)
                            }
                        });

                let percentage = if total_samples > 0 {
                    (sample_count as f64 / total_samples as f64) * 100.0
                } else {
                    0.0
                };

                if percentage >= self.threshold {
                    let pattern_str = if let Some(pat) = matched_pattern {
                        format!("[{}]", pat.join(", "))
                    } else {
                        let all: Vec<String> = self
                            .stack_frame
                            .iter()
                            .map(|p| format!("[{}]", p.join(", ")))
                            .collect();
                        all.join(", ")
                    };
                    let finding_description = format!(
                        "Stack pattern {} accounts for {:.2}% of samples in {} (threshold: {:.2}%)",
                        pattern_str, percentage, self.profile_type, self.threshold
                    );

                    report_findings.insert_finding(
                        run_name,
                        key,
                        AnalyticalFinding::new(
                            self.rule_name.to_string(),
                            self.score,
                            finding_description,
                            self.message.to_string(),
                        ),
                    );
                }
            }
        }
    }
}
