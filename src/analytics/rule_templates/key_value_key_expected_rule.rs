use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use std::fmt;
use std::fmt::Formatter;

/// This rule checks a key in all groups to match against an expected value. A finding is generated if the value does not match.
pub struct KeyValueKeyExpectedRule {
    pub rule_name: &'static str,
    pub key: &'static str,
    pub expected_value: &'static str,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! key_value_key_expected {
    {
        name: $rule_name:literal,
        key: $key:literal,
        expected_value: $expected_value:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::KeyValueKeyExpectedRule(
            KeyValueKeyExpectedRule{
                rule_name: $rule_name,
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
            "KeyValueKeyExpectedRule {} <checking if the value of key {} is the expected value: {}>",
            self.rule_name, self.key, self.expected_value
        )
    }
}

impl Analyze for KeyValueKeyExpectedRule {
    fn analyze(
        &self,
        report_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        for run_name in processed_data.runs.keys() {
            if let Some(value) =
                processed_data_accessor.key_value_value_by_key(processed_data, run_name, self.key)
            {
                if value != self.expected_value {
                    let finding_description = format!(
                        "The value of {} in {} is \"{}\", instead of \"{}\".",
                        self.key, run_name, value, self.expected_value
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
                    "The key {} in {} is missing, instead of being set to {}",
                    self.key, run_name, self.expected_value
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
