use crate::analytics::{compute_finding_score, AnalyticalFinding, Analyze, DataFindings};
use crate::computations::{formatted_number_string, Comparator, Stat};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use std::fmt;
use std::fmt::Formatter;

/// This rule runs for the specified metric in every run and compares each metric's specified stat
/// against the threshold.
pub struct TimeSeriesStatThresholdRule {
    pub rule_name: &'static str,
    pub metric_name: &'static str,
    pub stat: Stat,
    pub comparator: Comparator,
    pub threshold: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_stat_threshold {
    {
        name: $rule_name:literal,
        metric: $metric_name:literal,
        stat: $stat:path,
        comparator: $comparator:path,
        threshold: $threshold:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesStatThresholdRule(
            TimeSeriesStatThresholdRule{
                rule_name: $rule_name,
                metric_name: $metric_name,
                stat: $stat,
                comparator: $comparator,
                threshold: $threshold,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_stat_threshold;

impl fmt::Display for TimeSeriesStatThresholdRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeSeriesStatThresholdRule {} <checking if the {} of {} is {} {}>",
            self.rule_name, self.stat, self.metric_name, self.comparator, self.threshold
        )
    }
}

impl Analyze for TimeSeriesStatThresholdRule {
    fn analyze(
        &self,
        data_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        for run_name in processed_data.runs.keys() {
            let metric_stat = match processed_data_accessor.time_series_metric_stats(
                processed_data,
                run_name,
                &self.metric_name,
            ) {
                Some(metric_stats) => self.stat.get_stat(&metric_stats),
                None => continue,
            };

            if self.comparator.compare(metric_stat, self.threshold) {
                let finding_score = compute_finding_score(metric_stat, self.threshold, self.score);
                let finding_description = format!(
                    "The {} in {} is {}.",
                    self.stat,
                    run_name,
                    formatted_number_string(metric_stat),
                );

                data_findings.insert_finding(
                    run_name,
                    self.metric_name,
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
