use crate::analytics;
use crate::analytics::{Analyze, DataFindings};
use crate::computations::{f64_to_fixed_2, Comparator, Stat};
use crate::data::data_formats::ProcessedData;
use std::fmt;
use std::fmt::Formatter;

/// This rule runs for the specified metric in every run and compares each metric's specified stat
/// against the threshold.
pub struct TimeSeriesSingleMetricStatRule {
    pub metric_name: &'static str,
    pub stat: Stat,
    pub comparator: Comparator,
    pub threshold: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_single_metric_stat {
    {
        metric_name: $metric_name:literal,
        stat: $stat:path,
        comparator: $comparator:path,
        threshold: $threshold:literal,
        score: $score:literal,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesSingleMetricStatRule(
            TimeSeriesSingleMetricStatRule{
                metric_name: $metric_name,
                stat: $stat,
                comparator: $comparator,
                threshold: $threshold,
                score: $score,
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_single_metric_stat;

impl fmt::Display for TimeSeriesSingleMetricStatRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeSeriesSingleMetricStatRule <checking if the {} of {} is {} {}>",
            self.stat, self.metric_name, self.comparator, self.threshold
        )
    }
}

impl Analyze for TimeSeriesSingleMetricStatRule {
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
            let metric_stat = self.stat.get_stat(&metric.stats);

            if self.comparator.compare(metric_stat, self.threshold) {
                let finding_score =
                    analytics::compute_finding_score(metric_stat, self.threshold, self.score);
                let mut finding_description = format!(
                    "The {} in {} is {} ({} the threshold of {}).",
                    self.stat, run_name, f64_to_fixed_2(metric_stat), self.comparator, self.threshold
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
