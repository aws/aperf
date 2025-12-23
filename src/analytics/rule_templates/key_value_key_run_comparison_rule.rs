use crate::analytics;
use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::data::data_formats::ProcessedData;
use log::error;
use std::fmt;
use std::fmt::Formatter;

/// This rule compares the value of the specified key in every run against the base run and produces
/// a finding if the value is different from the base run.
pub struct KeyValueKeyRunComparisonRule {
    pub rule_name: &'static str,
    pub key: &'static str,
    pub score: f64,
    pub message: &'static str,
    pub reference: &'static str,
}

macro_rules! key_value_key_run_comparison {
    {
        name: $rule_name:literal,
        key: $key:literal,
        score: $score:expr,
        message: $message:literal,
        reference: $reference:literal,
    } => {
        AnalyticalRule::KeyValueKeyRunComparisonRule(
            KeyValueKeyRunComparisonRule{
                rule_name: $rule_name,
                key: $key,
                score: $score.as_f64(),
                message: $message,
                reference: $reference,
            }
        )
    };
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
                reference: "",
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
    fn analyze(&self, report_findings: &mut DataFindings, processed_data: &ProcessedData) {
        let base_run_name = analytics::get_base_run_name();

        let base_key_value_data = match processed_data.get_key_value_data(&base_run_name) {
            Some(key_value_data) => key_value_data,
            None => {
                error!("{self} failed to analyze: the base key value data does not exist");
                return;
            }
        };
        let mut base_value: Option<&String> = None;
        for key_value_group in base_key_value_data.key_value_groups.values() {
            if let Some(value) = key_value_group.key_values.get(self.key) {
                base_value = Some(value);
                break;
            }
        }

        if base_value.is_none() {
            error!("{self} failed to analyze: the base value does not exist");
            return;
        }
        let base_value = base_value.unwrap();

        for run_name in processed_data.runs.keys() {
            if base_run_name == *run_name {
                continue;
            }

            let key_value_data = match processed_data.get_key_value_data(&run_name) {
                Some(key_value_data) => key_value_data,
                None => continue,
            };

            let mut found_key = false;
            for key_value_group in key_value_data.key_value_groups.values() {
                if let Some(value) = key_value_group.key_values.get(self.key) {
                    found_key = true;
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
                                self.reference.to_string(),
                            ),
                        );
                    }
                    break;
                }
            }

            if !found_key {
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
                        self.reference.to_string(),
                    ),
                );
            }
        }
    }
}
