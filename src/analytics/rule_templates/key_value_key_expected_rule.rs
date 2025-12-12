use crate::analytics::{Analyze, DataFindings};
use crate::data::data_formats::ProcessedData;
use std::fmt;
use std::fmt::Formatter;

/// This rule checks a key in all groups to match against an expected value. A finding is generated if the value does not match.
pub struct KeyValueKeyExpectedRule {
    pub key: &'static str,
    pub expected_value: &'static str,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! key_value_key_expected {
    {
        key: $key:literal,
        expected_value: $expected_value:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::KeyValueKeyExpectedRule(
            KeyValueKeyExpectedRule{
                key: $key,
                expected_value: $expected_value,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}

pub(crate) use key_value_key_expected;

impl fmt::Display for KeyValueKeyExpectedRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyValueKeyExpectedRule <checking if the value of key {} is the expected value: {}>",
            self.key, self.expected_value
        )
    }
}

impl Analyze for KeyValueKeyExpectedRule {
    fn analyze(&self, report_findings: &mut DataFindings, processed_data: &ProcessedData) {
        for run_name in processed_data.runs.keys() {
            let key_value_data = match processed_data.get_key_value_data(&run_name) {
                Some(key_value_data) => key_value_data,
                None => continue,
            };
            for key_value_group in key_value_data.key_value_groups.values() {
                if let Some(value) = key_value_group.key_values.get(self.key) {
                    if value != self.expected_value {
                        let mut finding_description = format!(
                            "The value of {} in {} is \"{}\", instead of \"{}\".",
                            self.key, run_name, value, self.expected_value
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
