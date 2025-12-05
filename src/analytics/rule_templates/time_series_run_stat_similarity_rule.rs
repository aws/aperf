use crate::analytics;
use crate::analytics::{Analyze, DataFindings};
use crate::computations::{ratio_to_percentage_string, Comparator, Stat};
use crate::data::data_formats::ProcessedData;
use log::error;
use std::fmt;
use std::fmt::Formatter;

/// This rule checks the specified metric stat of every run with the base run, compute the ratio,
/// and compares it against the threshold ratio.
pub struct TimeSeriesRunStatSimilarityRule {
    pub metric_name: &'static str,
    pub stat: Stat,
    pub delta_ratio: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_run_stat_similarity {
    {
        metric_name: $metric_name:literal,
        stat: $stat:path,
        delta_ratio: $delta_ratio:literal,
        score: $score:literal,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesRunStatSimilarityRule(
            TimeSeriesRunStatSimilarityRule{
                metric_name: $metric_name,
                stat: $stat,
                delta_ratio: $delta_ratio,
                score: $score,
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_run_stat_similarity;

impl fmt::Display for TimeSeriesRunStatSimilarityRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "TimeSeriesRunStatSimilarityRule <checking if the {} of {} is less than {} different from the base run >", self.stat, self.metric_name, self.delta_ratio)
    }
}

impl Analyze for TimeSeriesRunStatSimilarityRule {
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

            let abs_delta = (cur_stat - base_stat).abs() / base_stat;

            if Comparator::LessEqual.compare(abs_delta, self.delta_ratio) {
                let mut finding_description = format!(
                    "The {} in {} ({}) is {} different from {} ({}).",
                    self.stat,
                    run_name,
                    cur_stat,
                    ratio_to_percentage_string(abs_delta),
                    base_run_name,
                    base_stat
                );
                if !self.message.is_empty() {
                    finding_description.push(' ');
                    finding_description.push_str(self.message);
                }
                // Use the rule's score directly since we usually do not need to care how similar the run is
                data_findings.insert_finding(
                    run_name,
                    self.metric_name,
                    self.score,
                    finding_description,
                );
            }
        }
    }
}
