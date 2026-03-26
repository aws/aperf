use crate::analytics;
use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use log::debug;
use std::fmt;
use std::fmt::Formatter;

/// This rule compares the value of the specified key in every run against the base run and produces
/// a finding if the value is different from the base run.
pub struct KeyValueKeyRunComparisonRule {
    pub rule_name: &'static str,
    pub key: &'static str,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! key_value_key_run_comparison {
    {
        name: $rule_name:literal,
        key: $key:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::KeyValueKeyRunComparisonRule(
            KeyValueKeyRunComparisonRule{
                rule_name: $rule_name,
                key: $key,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use key_value_key_run_comparison;

impl fmt::Display for KeyValueKeyRunComparisonRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyValueKeyRunComparisonRule {} <checking if values of key {} are different>",
            self.rule_name, self.key,
        )
    }
}

impl Analyze for KeyValueKeyRunComparisonRule {
    fn analyze(
        &self,
        report_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        let base_run_name = analytics::get_base_run_name();

        let base_value = match processed_data_accessor.key_value_value_by_key(
            processed_data,
            &base_run_name,
            self.key,
        ) {
            Some(base_value) => base_value,
            None => {
                debug!("{self} failed to analyze: the base value does not exist");
                return;
            }
        };

        for run_name in processed_data.runs.keys() {
            if base_run_name == *run_name {
                continue;
            }

            if let Some(value) =
                processed_data_accessor.key_value_value_by_key(processed_data, run_name, self.key)
            {
                if value != base_value {
                    let finding_description = format!(
                        "The value of {} in {} (\"{}\") is different from {} (\"{}\").",
                        self.key, run_name, value, base_run_name, base_value,
                    );

                    report_findings.insert_finding(
                        run_name,
                        self.key,
                        AnalyticalFinding::new(
                            self.rule_name.to_string(),
                            self.score,
                            finding_description,
                            self.message.to_string(),
                        ),
                    );
                }
            } else {
                let finding_description = format!(
                    "The key {} does not exist in {}, while its value in {} is \"{}\".",
                    self.key, run_name, base_run_name, base_value,
                );

                report_findings.insert_finding(
                    run_name,
                    self.key,
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
