mod rule_templates;
mod rules;

use crate::data::common::data_formats::ProcessedData;
use rule_templates::{
    key_value_key_expected_rule::KeyValueKeyExpectedRule,
    key_value_key_run_comparison_rule::KeyValueKeyRunComparisonRule,
    time_series_data_point_threshold_rule::TimeSeriesDataPointThresholdRule,
    time_series_stat_intra_run_comparison_rule::TimeSeriesStatIntraRunComparisonRule,
    time_series_stat_run_comparison_rule::TimeSeriesStatRunComparisonRule,
    time_series_stat_threshold_rule::TimeSeriesStatThresholdRule,
};
// Re-export rule types for testing
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
pub use rule_templates::{
    key_value_key_expected_rule, key_value_key_run_comparison_rule,
    time_series_data_point_threshold_rule, time_series_stat_intra_run_comparison_rule,
    time_series_stat_run_comparison_rule, time_series_stat_threshold_rule,
};
use rules::multi_data_rules::get_multi_data_rules;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};

lazy_static! {
    pub static ref BASE_RUN_NAME: Mutex<String> = Mutex::new(String::from(""));
}

fn get_base_run_name() -> String {
    BASE_RUN_NAME.lock().unwrap().to_string()
}

pub struct AnalyticalEngine<'a> {
    // Map from a data name to its processed data across all runs
    all_processed_data: HashMap<String, &'a ProcessedData>,
    // Map from a data name to its defined rules
    per_data_rules: HashMap<String, Vec<AnalyticalRule>>,
    // All rules that require multiple data types
    multi_data_rules: Vec<MultiDataAnalyticalRule>,
    // Map from a data name to its all analytical findings
    pub findings: HashMap<String, DataFindings>,
}

impl Default for AnalyticalEngine<'_> {
    fn default() -> Self {
        AnalyticalEngine {
            all_processed_data: Default::default(),
            per_data_rules: Default::default(),
            multi_data_rules: get_multi_data_rules(),
            findings: Default::default(),
        }
    }
}

impl<'a> AnalyticalEngine<'a> {
    pub fn add_processed_data(&mut self, data_name: String, processed_data: &'a ProcessedData) {
        self.all_processed_data.insert(data_name, processed_data);
    }

    pub fn add_data_rules(&mut self, data_name: String, rules: Vec<AnalyticalRule>) {
        self.per_data_rules.insert(data_name, rules);
    }

    pub fn run(&mut self, processed_data_accessor: &mut ProcessedDataAccessor) {
        for (data_name, data_rules) in &self.per_data_rules {
            if let Some(&processed_data) = self.all_processed_data.get(data_name) {
                let data_findings = self
                    .findings
                    .entry(data_name.clone())
                    .or_insert(DataFindings::default());
                for data_rule in data_rules {
                    data_rule.analyze(data_findings, processed_data, processed_data_accessor);
                }
            }
        }

        for multi_data_rule in &self.multi_data_rules {
            multi_data_rule.analyze(
                &mut self.findings,
                &self.all_processed_data,
                processed_data_accessor,
            );
        }
    }

    pub fn get_data_findings(&mut self, data_name: String) -> &DataFindings {
        self.findings
            .entry(data_name)
            .or_insert(DataFindings::default())
    }
}

/// Stores all analytical findings for a data type grouped by runs
#[derive(Serialize, Deserialize, Default)]
pub struct DataFindings {
    per_run_findings: HashMap<String, RunFindings>,
}

impl DataFindings {
    pub fn insert_finding(&mut self, run_name: &String, key: &str, finding: AnalyticalFinding) {
        let run_findings = self
            .per_run_findings
            .entry(run_name.clone())
            .or_insert(RunFindings::default());
        let key_findings = run_findings
            .findings
            .entry(key.to_string())
            .or_insert(Vec::new());
        key_findings.push(finding);
    }
}

/// Stores all analytical findings of a data type within current run. All findings are
/// grouped by the data key (such metric name or key-value key)
#[derive(Serialize, Deserialize, Default)]
pub struct RunFindings {
    findings: HashMap<String, Vec<AnalyticalFinding>>,
}

/// All information about an analytical finding. This data is passed to the report
/// frontend and rendered as UI component.
#[derive(Serialize, Deserialize)]
pub struct AnalyticalFinding {
    rule_name: String,
    score: f64,
    description: String,
    message: String,
}

impl AnalyticalFinding {
    pub fn new(rule_name: String, score: f64, description: String, message: String) -> Self {
        AnalyticalFinding {
            rule_name,
            score,
            description,
            message,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Score {
    Critical = -256,
    Poor = -16,
    Bad = -2,
    Concerning = -1,
    Neutral = 0,
    Acceptable = 1,
    Good = 2,
    Great = 16,
    Optimal = 256,
}

impl Score {
    pub fn as_f64(self) -> f64 {
        self as i32 as f64
    }
}

/// Compute the score of a finding with the base score and how much the actual value is
/// different from the threshold
fn compute_finding_score(value: f64, threshold: f64, rule_score: f64) -> f64 {
    // When the threshold is zero, raise it to 1 for computation of the final score
    // TODO: come up with a better mechanism to handle the zero case
    if threshold == 0.0 {
        return if value < 1.0 {
            rule_score
        } else {
            (value - 1.0) * rule_score
        };
    }

    // Value being 100% less than the threshold should be treated the same as
    // being 100% greater than the threshold
    if value == 0.0 {
        return 2.0 * rule_score;
    }

    let mut delta = value / threshold;
    if delta < 1.0 {
        delta = delta.recip();
    }
    delta * rule_score
}

/// The trait to be implemented by every single-data rule. It runs the rule against the
/// processed data of the corresponding type. If a rule matches, it should produce one or
/// more findings and store them in the data_findings struct.
pub trait Analyze {
    fn analyze(
        &self,
        data_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    );
}

/// The trait to be implemented by every multi-data rule. It runs the rule against multiple types
/// of processed data. If a rule matches, it should produce one or more findings and store them
/// in the relative data's findings.
pub trait MultiDataAnalyze {
    fn analyze(
        &self,
        findings: &HashMap<String, DataFindings>,
        all_processed_data: &HashMap<String, &ProcessedData>,
        processed_data_accessor: &mut ProcessedDataAccessor,
    );
}

macro_rules! analytical_rules {
    ($( $analytical_rule:ident ), *) => {
        pub enum AnalyticalRule {
            $(
                $analytical_rule($analytical_rule),
            )*
        }

        impl AnalyticalRule {
            pub fn analyze(&self, data_findings: &mut DataFindings, processed_data: &ProcessedData, processed_data_accessor: &mut ProcessedDataAccessor) {
                match self {
                    $(
                        AnalyticalRule::$analytical_rule(ref analytical_rule) => analytical_rule.analyze(data_findings, processed_data, processed_data_accessor),
                    )*
                }
            }
        }
    };
}

// Register all single-data-type rule templates here
analytical_rules!(
    TimeSeriesStatRunComparisonRule,
    TimeSeriesStatThresholdRule,
    TimeSeriesDataPointThresholdRule,
    KeyValueKeyRunComparisonRule,
    KeyValueKeyExpectedRule,
    TimeSeriesStatIntraRunComparisonRule
);

macro_rules! multi_data_analytical_rules {
    ($( $multi_data_analytical_rule:ident ), *) => {
        pub enum MultiDataAnalyticalRule {
            $(
                $multi_data_analytical_rule($multi_data_analytical_rule),
            )*
        }

        impl MultiDataAnalyticalRule {
            pub fn analyze(&self, _findings: &mut HashMap<String, DataFindings>, _all_processed_data: &HashMap<String, &ProcessedData>, _processed_data_accessor: &mut ProcessedDataAccessor) {
                match self {
                    $(
                        MultiDataAnalyticalRule::$multi_data_analytical_rule(ref multi_data_analytical_rule) => multi_data_analytical_rule.analyze(findings, all_processed_data, processed_data_accessor),
                    )*
                    _ => todo!(),
                }
            }
        }
    };
}

// Register all multi-data-type rule templates here
multi_data_analytical_rules!();

#[cfg(test)]
mod tests {
    use super::*;

    // --- value == 0.0 cases ---

    #[test]
    fn test_value_zero_positive_score() {
        // value=0 with non-zero threshold should return 2.0 * rule_score
        let result = compute_finding_score(0.0, 10.0, 5.0);
        assert_eq!(result, 10.0); // 2.0 * 5.0
    }

    #[test]
    fn test_value_zero_negative_score() {
        let result = compute_finding_score(0.0, 50.0, -2.0);
        assert_eq!(result, -4.0); // 2.0 * -2.0
    }

    #[test]
    fn test_value_zero_zero_score() {
        let result = compute_finding_score(0.0, 100.0, 0.0);
        assert_eq!(result, 0.0); // 2.0 * 0.0
    }

    // --- threshold == 0.0 cases ---

    #[test]
    fn test_threshold_zero_value_less_than_one() {
        // threshold=0, value<1 => rule_score
        let result = compute_finding_score(0.5, 0.0, 3.0);
        assert_eq!(result, 3.0);
    }

    #[test]
    fn test_threshold_zero_value_greater_than_one() {
        // threshold=0, value>=1 => (value - 1.0) * rule_score
        let result = compute_finding_score(5.0, 0.0, 3.0);
        assert_eq!(result, 12.0); // (5.0 - 1.0) * 3.0
    }

    #[test]
    fn test_threshold_zero_value_exactly_one() {
        // threshold=0, value==1.0 => (1.0 - 1.0) * rule_score = 0
        let result = compute_finding_score(1.0, 0.0, 3.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_both_value_and_threshold_zero() {
        // threshold==0 branch is checked first; value=0.0 < 1.0 => rule_score
        let result = compute_finding_score(0.0, 0.0, 7.0);
        assert_eq!(result, 7.0);
    }

    // --- normal delta cases ---

    #[test]
    fn test_value_above_threshold() {
        // delta = 10/5 = 2.0, >= 1 so no recip => 2.0 * 3.0 = 6.0
        let result = compute_finding_score(10.0, 5.0, 3.0);
        assert_eq!(result, 6.0);
    }

    #[test]
    fn test_value_below_threshold() {
        // delta = 2/10 = 0.2, < 1 so recip => 5.0 * 3.0 = 15.0
        let result = compute_finding_score(2.0, 10.0, 3.0);
        assert_eq!(result, 15.0);
    }

    #[test]
    fn test_value_equals_threshold() {
        // delta = 1.0, not < 1 => 1.0 * rule_score
        let result = compute_finding_score(5.0, 5.0, 4.0);
        assert_eq!(result, 4.0);
    }
}
