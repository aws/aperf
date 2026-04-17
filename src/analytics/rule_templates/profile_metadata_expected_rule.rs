use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::data::common::data_formats::{AperfData, ProcessedData};
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use log::debug;
use regex::Regex;
use std::fmt;
use std::fmt::Formatter;

/// This rule checks if expected metadata fields exists and also checks the expected value in profile data. It generates
/// a finding if the value does not match the expected_value regex. A finding will also be generated if the value cannot
/// be found and should_exist is true.
pub struct ProfileMetadataExpectedRule {
    pub rule_name: &'static str,
    pub group: &'static str,
    pub key: &'static str,
    pub expected_value: &'static str,
    pub should_exist: bool,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! profile_metadata_expected {
    {
        name: $rule_name:literal,
        group: $group:literal,
        key: $key:literal,
        expected_value: $expected:literal,
        should_exist: $should_exist:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::ProfileMetadataExpectedRule(
            ProfileMetadataExpectedRule {
                rule_name: $rule_name,
                group: $group,
                key: $key,
                expected_value: $expected,
                should_exist: $should_exist,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use profile_metadata_expected;

impl fmt::Display for ProfileMetadataExpectedRule {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl Analyze for ProfileMetadataExpectedRule {
    fn analyze(
        &self,
        report_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        _processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        let regex = match Regex::new(self.expected_value) {
            Ok(re) => Some(re),
            Err(e) => {
                debug!(
                    "Error: Failed to compile regex '{}' in rule '{}': {}",
                    self.expected_value, self.rule_name, e
                );
                None
            }
        };

        for (run_name, run_data) in &processed_data.runs {
            let AperfData::Graph(graph_data) = run_data else {
                continue;
            };
            for (key, profiler_data) in &graph_data.profiler_data_map {
                let metadata_value = profiler_data
                    .metadata
                    .key_value_groups
                    .get(self.group)
                    .and_then(|group| group.key_values.get(self.key))
                    .cloned();

                let field_exists = metadata_value.is_some();
                let value_matches = matches!((&metadata_value, &regex), (Some(value), Some(re)) if re.is_match(value));

                // Report finding if value doesn't match, or if it should exist but doesnt
                if (field_exists && !value_matches) || (self.should_exist && !field_exists) {
                    let finding_description = if field_exists {
                        format!(
                            "Event type '{}' field '{}' in {} should match pattern '{}', found: {:?}",
                            self.group,
                            self.key,
                            key,
                            self.expected_value,
                            metadata_value.unwrap_or_default()
                        )
                    } else {
                        format!(
                            "Event type '{}' field '{}' not found in profile data",
                            self.group, self.key
                        )
                    };

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
