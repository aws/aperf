use crate::analytics::{Analyze, DataFindings};
use crate::computations::{f64_to_fixed_2, ratio_to_percentage_delta_string, Comparator, Stat};
use crate::data::data_formats::ProcessedData;
use std::fmt;
use std::fmt::Formatter;

/// This rule computes the delta_ratio between two metric stats of every run,
/// and compares it (or the magnitude of it with abs=True) with the threshold delta_ratio.
pub struct TimeSeriesStatIntraRunComparisonRule {
    pub baseline_metric_name: &'static str,
    pub comparison_metric_name: &'static str,
    pub stat: Stat,
    pub comparator: Comparator,
    pub abs: bool,
    pub delta_ratio: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_stat_intra_run_comparison {
    {
        baseline_metric_name: $baseline_metric_name:literal,
        comparison_metric_name: $comparison_metric_name:literal,
        stat: $stat:path,
        comparator: $comparator:path,
        abs: $abs:literal,
        delta_ratio: $delta_ratio:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesStatIntraRunComparisonRule(
            TimeSeriesStatIntraRunComparisonRule{
                baseline_metric_name: $baseline_metric_name,
                comparison_metric_name: $comparison_metric_name,
                stat: $stat,
                comparator: $comparator,
                abs: $abs,
                delta_ratio: $delta_ratio,
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_stat_intra_run_comparison;

impl fmt::Display for TimeSeriesStatIntraRunComparisonRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeSeriesStatIntraRunComparisonRule <checking if the delta_ratio of {} of {} between {} is {} than {}>",
            self.stat, self.baseline_metric_name, self.comparison_metric_name, self.comparator, self.delta_ratio
        )
    }
}

impl Analyze for TimeSeriesStatIntraRunComparisonRule {
    fn analyze(&self, data_findings: &mut DataFindings, processed_data: &ProcessedData) {
        for run_name in processed_data.runs.keys() {
            let time_series_data = match processed_data.get_time_series_data(run_name) {
                Some(time_series_data) => time_series_data,
                None => continue,
            };

            let baseline_metric = match time_series_data.metrics.get(self.baseline_metric_name) {
                Some(time_series_metric) => time_series_metric,
                None => continue,
            };
            let baseline_metric_stat = self.stat.get_stat(&baseline_metric.stats);

            let comparison_metric = match time_series_data.metrics.get(self.comparison_metric_name)
            {
                Some(time_series_metric) => time_series_metric,
                None => continue,
            };
            let comparison_metric_stat = self.stat.get_stat(&comparison_metric.stats);

            let mut delta_ratio =
                (comparison_metric_stat - baseline_metric_stat) / baseline_metric_stat;
            if self.abs {
                delta_ratio = delta_ratio.abs();
            }

            if self.comparator.compare(delta_ratio, self.delta_ratio) {
                let mut finding_description = format!(
                    "The delta_ratio of {} between {} ({}) and {} ({}) in {} is {} which is {} than threshold of {}.",
                    self.stat,
                    self.comparison_metric_name,
                    f64_to_fixed_2(comparison_metric_stat),
                    self.baseline_metric_name,
                    f64_to_fixed_2(baseline_metric_stat),
                    run_name,
                    ratio_to_percentage_delta_string(delta_ratio),
                    self.comparator,
                    ratio_to_percentage_delta_string(self.delta_ratio)
                );
                if !self.message.is_empty() {
                    finding_description.push(' ');
                    finding_description.push_str(self.message);
                }
                data_findings.insert_finding(
                    run_name,
                    self.baseline_metric_name,
                    self.score,
                    finding_description,
                );
            }
        }
    }
}
