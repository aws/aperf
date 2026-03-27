use crate::analytics;
use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::computations::{formatted_number_string, Comparator};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use std::fmt;
use std::fmt::Formatter;

/// This rule runs for the specified metric in every run and compares every data point in each metric
/// against the threshold.
pub struct TimeSeriesDataPointThresholdRule {
    pub rule_name: &'static str,
    pub metric_name: &'static str,
    pub regex_matching: bool,
    pub comparator: Comparator,
    pub threshold: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_data_point_threshold {
    {
        name: $rule_name:literal,
        metric: $metric_name:literal,
        comparator: $comparator:path,
        threshold: $threshold:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesDataPointThresholdRule(
            TimeSeriesDataPointThresholdRule{
                rule_name: $rule_name,
                metric_name: $metric_name,
                regex_matching: false,
                comparator: $comparator,
                threshold: $threshold,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_data_point_threshold;

macro_rules! time_series_data_point_threshold_multi_metric {
    {
        name: $rule_name:literal,
        pattern: $pattern:literal,
        comparator: $comparator:path,
        threshold: $threshold:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesDataPointThresholdRule(
            TimeSeriesDataPointThresholdRule{
                rule_name: $rule_name,
                metric_name: $pattern,
                regex_matching: true,
                comparator: $comparator,
                threshold: $threshold,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_data_point_threshold_multi_metric;

impl fmt::Display for TimeSeriesDataPointThresholdRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeSeriesDataPointThresholdRule {} <checking if any data points of {} is {} {}>",
            self.rule_name, self.metric_name, self.comparator, self.threshold
        )
    }
}

impl Analyze for TimeSeriesDataPointThresholdRule {
    fn analyze(
        &self,
        data_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        for run_name in processed_data.runs.keys() {
            let matched_metric_names = if self.regex_matching {
                processed_data_accessor.time_series_matched_metric_names(
                    processed_data,
                    run_name,
                    self.metric_name,
                )
            } else {
                vec![self.metric_name]
            };

            for metric_name in matched_metric_names {
                // Produce one finding per metric for the data point with the largest absolute score
                let mut max_abs_finding_score: f64 = 0.0;
                let mut max_score_value: f64 = 0.0;
                let mut rule_matched = false;
                for &value in processed_data_accessor.time_series_metric_values_iterator(
                    processed_data,
                    run_name,
                    metric_name,
                ) {
                    if self.comparator.compare(value, self.threshold) {
                        rule_matched = true;
                        let abs_finding_score =
                            analytics::compute_finding_score(value, self.threshold, self.score)
                                .abs();
                        if abs_finding_score >= max_abs_finding_score {
                            max_score_value = value;
                            max_abs_finding_score = abs_finding_score;
                        }
                    }
                }

                if rule_matched {
                    let finding_score = if self.score > 0.0 {
                        max_abs_finding_score
                    } else {
                        max_abs_finding_score * -1.0
                    };
                    let finding_description = format!(
                        "At least one data point in {} is {} ({} the threshold of {}).",
                        run_name,
                        formatted_number_string(max_score_value),
                        self.comparator,
                        self.threshold
                    );

                    data_findings.insert_finding(
                        run_name,
                        metric_name,
                        AnalyticalFinding::new(
                            self.rule_name.to_string(),
                            finding_score,
                            finding_description,
                            self.message.to_string(),
                        ),
                    );
                }
            }
        }
    }
}
