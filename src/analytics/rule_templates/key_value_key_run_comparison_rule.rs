use crate::analytics;
use crate::analytics::{Analyze, DataFindings};
use crate::data::data_formats::ProcessedData;
use log::error;
use std::fmt;
use std::fmt::Formatter;

/// This rule compares the value of the specified key in every run against the base run and produces
/// a finding if the value is different from the base run.
pub struct KeyValueKeyRunComparisonRule {
    pub key_group: &'static str,
    pub key: &'static str,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! key_value_key_run_comparison {
    {
        key_group: $key_group:literal,
        key: $key:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::KeyValueKeyRunComparisonRule(
            KeyValueKeyRunComparisonRule{
                key_group: $key_group,
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
            "KeyValueKeyRunComparisonRule <checking different values of key {} in group {}>",
            self.key, self.key_group
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
        if let Some(key_value_group) = base_key_value_data.key_value_groups.get(self.key_group) {
            base_value = key_value_group.key_values.get(self.key);
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
            for key_value_group in key_value_data.key_value_groups.values() {
                if let Some(value) = key_value_group.key_values.get(self.key) {
                    if value != base_value {
                        let mut finding_description = format!(
                            "The value of {} in {} is {}, different from {} in {}.",
                            self.key, run_name, value, base_value, base_run_name
                        );
                        if !self.message.is_empty() {
                            finding_description.push(' ');
                            finding_description.push_str(self.message);
                        }
                        report_findings.insert_finding(
                            run_name,
                            self.key,
                            self.score,
                            finding_description,
                        );
                    }
                    break;
                }
            }
        }
    }
}
