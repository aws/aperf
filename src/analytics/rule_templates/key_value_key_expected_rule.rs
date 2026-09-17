use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use std::fmt;
use std::fmt::Formatter;

/// This rule checks a key in all groups against a value. Whether report_on_match is
/// true decides if the finding is generated when the value matches the target_value
/// or not.
pub struct KeyValueKeyExpectedRule {
    pub rule_name: &'static str,
    pub key: &'static str,
    pub target_value: &'static str,
    pub report_on_match: bool,
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
                target_value: $expected_value,
                report_on_match: false,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use key_value_key_expected;

macro_rules! key_value_key_unexpected {
    {
        name: $rule_name:literal,
        key: $key:literal,
        unexpected_value: $unexpected_value:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::KeyValueKeyExpectedRule(
            KeyValueKeyExpectedRule{
                rule_name: $rule_name,
                key: $key,
                target_value: $unexpected_value,
                report_on_match: true,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use key_value_key_unexpected;

impl fmt::Display for KeyValueKeyExpectedRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let relation = if self.report_on_match { "is" } else { "is not" };
        write!(
            f,
            "KeyValueKeyExpectedRule {} <checking if the value of key {} {} {}>",
            self.rule_name, self.key, relation, self.target_value
        )
    }
}

impl Analyze for KeyValueKeyExpectedRule {
    fn analyze(
        &self,
        report_findings: &mut DataFindings,
        processed_data: &mut ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        for run_name in processed_data.runs.keys() {
            if let Some(value) =
                processed_data_accessor.key_value_value_by_key(processed_data, run_name, self.key)
            {
                if (value == self.target_value) == self.report_on_match {
                    let finding_description = if self.report_on_match {
                        format!(
                            "The value of {} in {} is \"{}\".",
                            self.key, run_name, value
                        )
                    } else {
                        format!(
                            "The value of {} in {} is \"{}\", instead of \"{}\".",
                            self.key, run_name, value, self.target_value
                        )
                    };
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
            } else if !self.report_on_match {
                // A key that is absent cannot hold the value being called out.
                let finding_description = format!(
                    "The key {} in {} is missing, instead of being set to {}",
                    self.key, run_name, self.target_value
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
