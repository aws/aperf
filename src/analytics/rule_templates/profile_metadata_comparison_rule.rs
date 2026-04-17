use crate::analytics;
use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::data::common::data_formats::{AperfData, ProcessedData};
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use log::debug;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// This rule compares metadata field values between runs. Generates a finding if the values of the metadata between a run and
/// the base run are different. It will also generate a finding if a field is expected (should_exist = true) to exist but doesn't.
/// Metadata has the same structure as KeyValueData.
pub struct ProfileMetadataComparisonRule {
    pub rule_name: &'static str,
    pub group: &'static str,
    pub key: &'static str,
    pub should_exist: bool,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! profile_metadata_comparison {
    {
        name: $rule_name:literal,
        group: $group:literal,
        key: $key:literal,
        should_exist: $should_exist:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::ProfileMetadataComparisonRule(
            ProfileMetadataComparisonRule {
                rule_name: $rule_name,
                group: $group,
                key: $key,
                should_exist: $should_exist,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use profile_metadata_comparison;

impl fmt::Display for ProfileMetadataComparisonRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProfileMetadataComparisonRule {} <checking if {} field {} differs between runs>",
            self.rule_name, self.group, self.key,
        )
    }
}

impl Analyze for ProfileMetadataComparisonRule {
    fn analyze(
        &self,
        report_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        _processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        let base_run_name = analytics::get_base_run_name();

        let base_run_data = match processed_data.runs.get(&base_run_name) {
            Some(data) => data,
            None => {
                if processed_data.runs.keys().len() > 0 {
                    debug!("{self} failed to analyze: base run does not exist");
                }
                return;
            }
        };

        // Map base values from all profiles in the first record
        let base_values: HashMap<String, String> =
            if let AperfData::Graph(graph_data) = base_run_data {
                graph_data
                    .profiler_data_map
                    .iter()
                    .filter_map(|(key, profiler_data)| {
                        profiler_data
                            .metadata
                            .key_value_groups
                            .get(self.group)
                            .and_then(|group| group.key_values.get(self.key))
                            .map(|value| (key.clone(), value.clone()))
                    })
                    .collect()
            } else {
                HashMap::new()
            };

        if base_values.is_empty() {
            if self.should_exist {
                report_findings.insert_finding(
                    &base_run_name,
                    self.rule_name,
                    AnalyticalFinding::new(
                        self.rule_name.to_string(),
                        self.score,
                        format!(
                            "Event type '{}' field '{}' not found in run {}.",
                            self.group, self.key, base_run_name
                        ),
                        self.message.to_string(),
                    ),
                );
            }
            return;
        }

        for (run_name, run_data) in &processed_data.runs {
            if *run_name == base_run_name {
                continue;
            }

            let AperfData::Graph(graph_data) = run_data else {
                continue;
            };

            let comparison_values: HashMap<String, String> = graph_data
                .profiler_data_map
                .iter()
                .filter_map(|(key, profiler_data)| {
                    profiler_data
                        .metadata
                        .key_value_groups
                        .get(self.group)
                        .and_then(|group| group.key_values.get(self.key))
                        .map(|value| (key.clone(), value.clone()))
                })
                .collect();

            // If both runs have exactly one key-value pair, compare values regardless of key match
            if base_values.len() == 1 && comparison_values.len() == 1 {
                let (base_key, base_value) = base_values.iter().next().unwrap();
                let (comp_key, comp_value) = comparison_values.iter().next().unwrap();

                if comp_value != base_value {
                    report_findings.insert_finding(
                        run_name,
                        comp_key,
                        AnalyticalFinding::new(
                            self.rule_name.to_string(),
                            self.score,
                            format!(
                                "Event type '{}' field '{}' in {} for '{}' (\"{}\") differs from {} for '{}' (\"{}\")",
                                self.group, self.key, run_name, comp_key, comp_value, base_run_name, base_key, base_value
                            ),
                            self.message.to_string(),
                        ),
                    );
                }
                continue;
            }

            // Otherwise try to match and compare keys
            for (key, base_value) in &base_values {
                let value = comparison_values.get(key);

                let finding_description = match value {
                    Some(v) if v != base_value => Some(format!(
                        "Event type '{}' field '{}' in {} for '{}' (\"{}\") differs from {} (\"{}\")",
                        self.group, self.key, run_name, key, v, base_run_name, base_value
                    )),
                    None if self.should_exist => Some(format!(
                        "Event type '{}' field '{}' not found in {} for '{}', while its value in {} is \"{}\"",
                        self.group, self.key, run_name, key, base_run_name, base_value
                    )),
                    _ => None,
                };

                if let Some(description) = finding_description {
                    report_findings.insert_finding(
                        run_name,
                        key,
                        AnalyticalFinding::new(
                            self.rule_name.to_string(),
                            self.score,
                            description,
                            self.message.to_string(),
                        ),
                    );
                }
            }
        }
    }
}
