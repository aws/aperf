use crate::analytics;
use crate::analytics::{Analyze, DataFindings};
use crate::computations::{f64_to_fixed_2, ratio_to_percentage_delta_string, Comparator, Stat};
use crate::data::data_formats::ProcessedData;
use log::error;
use std::fmt;
use std::fmt::Formatter;

/// This rule computes the ratio between the specified metric stat of every run and the base run,
/// and compares it against the threshold ratio.
pub struct TimeSeriesRunStatComparisonRule {
    pub metric_name: &'static str,
    pub stat: Stat,
    pub comparator: Comparator,
    pub ratio: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_run_stat_comparison {
    {
        metric_name: $metric_name:literal,
        stat: $stat:path,
        comparator: $comparator:path,
        ratio: $ratio:literal,
        score: $score:literal,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesRunStatComparisonRule(
            TimeSeriesRunStatComparisonRule{
                metric_name: $metric_name,
                stat: $stat,
                comparator: $comparator,
                ratio: $ratio,
                score: $score,
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_run_stat_comparison;

impl fmt::Display for TimeSeriesRunStatComparisonRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeSeriesRunStatComparisonRule <checking if the {} of {} is {} {} of the base run >",
            self.stat, self.metric_name, self.comparator, self.ratio
        )
    }
}

impl Analyze for TimeSeriesRunStatComparisonRule {
    fn analyze(&self, data_findings: &mut DataFindings, processed_data: &ProcessedData) {
        let base_run_name = &analytics::get_base_run_name();

        let base_time_series_data = match processed_data.get_time_series_data(base_run_name) {
            Some(time_series_data) => time_series_data,
            None => {
                error!("{self} failed to analyze: the base time series data does not exist");
                return;
            }
        };
        let base_metric = match base_time_series_data.metrics.get(self.metric_name) {
            Some(time_series_metric) => time_series_metric,
            None => {
                error!("{self} failed to analyze: the base time series metric does not exist");
                return;
            }
        };
        let base_stat = self.stat.get_stat(&base_metric.stats);

        for run_name in processed_data.runs.keys() {
            if base_run_name == run_name {
                continue;
            }

            let cur_time_series_data = match processed_data.get_time_series_data(run_name) {
                Some(time_series_data) => time_series_data,
                None => continue,
            };
            let cur_metric = match cur_time_series_data.metrics.get(self.metric_name) {
                Some(time_series_metric) => time_series_metric,
                None => continue,
            };
            let cur_stat = self.stat.get_stat(&cur_metric.stats);

            let cur_ratio = cur_stat / base_stat;

            if self.comparator.compare(cur_ratio, self.ratio) {
                let finding_score =
                    analytics::compute_finding_score(cur_ratio, self.ratio, self.score);
                let mut finding_description = format!(
                    "The {} in {} ({}) is {} {} ({}).",
                    self.stat,
                    run_name,
                    f64_to_fixed_2(cur_stat),
                    ratio_to_percentage_delta_string(cur_ratio),
                    base_run_name,
                    f64_to_fixed_2(base_stat)
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
