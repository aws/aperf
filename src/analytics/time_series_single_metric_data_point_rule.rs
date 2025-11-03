use crate::analytics;
use crate::analytics::{Analyze, DataFindings};
use crate::computations::{f64_to_fixed_2, Comparator};
use crate::data::data_formats::ProcessedData;
use std::fmt;
use std::fmt::Formatter;

/// This rule runs for the specified metric in every run and compares every data point in each metric
/// against the threshold.
pub struct TimeSeriesSingleMetricDataPointRule {
    pub metric_name: &'static str,
    pub comparator: Comparator,
    pub threshold: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_single_data_point {
    {
        metric_name: $metric_name:literal,
        comparator: $comparator:path,
        threshold: $threshold:literal,
        score: $score:literal,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesSingleMetricDataPointRule(
            TimeSeriesSingleMetricDataPointRule{
                metric_name: $metric_name,
                comparator: $comparator,
                threshold: $threshold,
                score: $score,
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_single_data_point;

impl fmt::Display for TimeSeriesSingleMetricDataPointRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeSeriesSingleMetricDataPointRule <checking if any data points of {} is {} {}>",
            self.metric_name, self.comparator, self.threshold
        )
    }
}

impl Analyze for TimeSeriesSingleMetricDataPointRule {
    fn analyze(&self, data_findings: &mut DataFindings, processed_data: &ProcessedData) {
        for run_name in processed_data.runs.keys() {
            let time_series_data = match processed_data.get_time_series_data(run_name) {
                Some(time_series_data) => time_series_data,
                None => continue,
            };
            let metric = match time_series_data.metrics.get(self.metric_name) {
                Some(time_series_metric) => time_series_metric,
                None => continue,
            };

            // Produce one finding per metric for the data point with the largest absolute score
            let mut max_abs_finding_score: f64 = 0.0;
            let mut max_score_value: f64 = 0.0;
            let mut rule_matched = false;
            for series in &metric.series {
                for &value in &series.values {
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
            }

            if rule_matched {
                let finding_score = if self.score > 0.0 {
                    max_abs_finding_score
                } else {
                    max_abs_finding_score * -1.0
                };
                let mut finding_description = format!(
                    "At least one data point in {} is {} ({} the threshold of {}).",
                    run_name, f64_to_fixed_2(max_score_value), self.comparator, self.threshold
                );
                if !self.message.is_empty() {
                    finding_description.push(' ');
                    finding_description.push_str(self.message);
                }
                data_findings.insert_finding(
                    run_name,
                    self.metric_name,
                    finding_score,
                    finding_description,
                );
            }
        }
    }
}
