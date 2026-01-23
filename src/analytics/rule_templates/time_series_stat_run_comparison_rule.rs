use crate::analytics;
use crate::analytics::{AnalyticalFinding, Analyze, DataFindings};
use crate::computations::{
    delta_ratio_to_percentage_string, formatted_number_string, Comparator, Stat,
};
use crate::data::data_formats::ProcessedData;
use log::error;
use std::fmt;
use std::fmt::Formatter;

/// This rule computes the delta between the specified metric stat of every run and the base run,
/// and compares it against the threshold delta_ratio. If abs is true, the magnitude of the ratio is used.
pub struct TimeSeriesStatRunComparisonRule {
    pub rule_name: &'static str,
    pub metric_name: &'static str,
    pub stat: Stat,
    pub comparator: Comparator,
    pub abs: bool,
    pub delta_ratio: f64,
    pub score: f64,
    pub message: &'static str,
}

macro_rules! time_series_stat_run_comparison {
    {
        name: $rule_name:literal,
        metric: $metric_name:literal,
        stat: $stat:path,
        comparator: $comparator:path,
        abs: $abs:literal,
        delta_ratio: $delta_ratio:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesStatRunComparisonRule(
            TimeSeriesStatRunComparisonRule{
                rule_name: $rule_name,
                metric_name: $metric_name,
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
pub(crate) use time_series_stat_run_comparison;

impl fmt::Display for TimeSeriesStatRunComparisonRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeSeriesStatRunComparisonRule {} <checking if the delta_ratio {} of {} is {} {} of the base run >",
            self.rule_name, self.stat, self.metric_name, self.comparator, self.delta_ratio
        )
    }
}

impl Analyze for TimeSeriesStatRunComparisonRule {
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

            let original_delta_ratio = if base_stat == 0.0 {
                // When the base stat is zero, the delta computation cannot be performed,
                // so treat current stat as 100% larger than the base stat
                1.0
            } else {
                (cur_stat - base_stat) / base_stat
            };
            let comparison_delta_ratio = if self.abs {
                original_delta_ratio.abs()
            } else {
                original_delta_ratio
            };

            if self
                .comparator
                .compare(comparison_delta_ratio, self.delta_ratio)
            {
                let finding_score = analytics::compute_finding_score(
                    comparison_delta_ratio,
                    self.delta_ratio,
                    self.score,
                );

                let finding_description = format!(
                    "The {} in {} ({}) is {} {} ({}).",
                    self.stat,
                    run_name,
                    formatted_number_string(cur_stat),
                    delta_ratio_to_percentage_string(original_delta_ratio),
                    base_run_name,
                    formatted_number_string(base_stat),
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
