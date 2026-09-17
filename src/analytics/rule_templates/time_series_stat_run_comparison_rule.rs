use crate::analytics::{
    compute_finding_score, get_base_run_name, AnalyticalFinding, Analyze, DataFindings,
};
use crate::computations::{
    delta_ratio_to_percentage_string, delta_to_difference_string, formatted_number_string,
    Comparator, Stat,
};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use log::debug;
use std::fmt;
use std::fmt::Formatter;

/// How the delta between a run's metric stat and the base run's is measured.
pub enum DeltaThreshold {
    /// A fraction of the base run's stat, so 0.2 fires when the stat differs by 20%.
    Ratio(f64),
    /// A difference in the metric's absolute value.
    Absolute(f64),
}

impl DeltaThreshold {
    fn threshold(&self) -> f64 {
        match self {
            Self::Ratio(threshold) | Self::Absolute(threshold) => *threshold,
        }
    }
}

/// This rule computes the delta between the specified metric stat of every run and the base run,
/// and compares it against the delta threshold. If abs is true, the magnitude of the delta is used.
pub struct TimeSeriesStatRunComparisonRule {
    pub rule_name: &'static str,
    pub metric_name: &'static str,
    pub stat: Stat,
    pub comparator: Comparator,
    pub abs: bool,
    pub delta: DeltaThreshold,
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
                delta: $crate::analytics::rule_templates::time_series_stat_run_comparison_rule::DeltaThreshold::Ratio($delta_ratio),
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
    {
        name: $rule_name:literal,
        metric: $metric_name:literal,
        stat: $stat:path,
        comparator: $comparator:path,
        abs: $abs:literal,
        delta_absolute: $delta_absolute:literal,
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
                delta: $crate::analytics::rule_templates::time_series_stat_run_comparison_rule::DeltaThreshold::Absolute($delta_absolute),
                score: $score.as_f64(),
                message: $message,
            }
        )
    };
}
pub(crate) use time_series_stat_run_comparison;

impl fmt::Display for TimeSeriesStatRunComparisonRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let delta_description = match self.delta {
            DeltaThreshold::Ratio(threshold) => format!("delta_ratio {}", threshold),
            DeltaThreshold::Absolute(threshold) => format!("absolute delta {}", threshold),
        };
        write!(
            f,
            "TimeSeriesStatRunComparisonRule {} <checking if the {} of {} is {} {} of the base run >",
            self.rule_name, delta_description, self.metric_name, self.comparator, self.stat
        )
    }
}

impl Analyze for TimeSeriesStatRunComparisonRule {
    fn analyze(
        &self,
        data_findings: &mut DataFindings,
        processed_data: &mut ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        let base_run_name = &get_base_run_name();

        let base_stat = match processed_data_accessor.time_series_metric_stats(
            processed_data,
            base_run_name,
            self.metric_name,
        ) {
            Some(base_stats) => self.stat.get_stat(&base_stats),
            None => {
                debug!("{self} failed to analyze: cannot find the base time series metric or its stats");
                return;
            }
        };

        for run_name in processed_data.runs.keys() {
            if base_run_name == run_name {
                continue;
            }

            let cur_stat = match processed_data_accessor.time_series_metric_stats(
                processed_data,
                run_name,
                self.metric_name,
            ) {
                Some(cur_stats) => self.stat.get_stat(&cur_stats),
                None => continue,
            };

            let original_delta = match self.delta {
                DeltaThreshold::Ratio(_) => {
                    if cur_stat == base_stat {
                        0.0
                    } else if base_stat == 0.0 {
                        // When the base stat is zero, the delta computation cannot be performed,
                        // so treat current stat as 100% larger than the base stat
                        1.0
                    } else {
                        (cur_stat - base_stat) / base_stat
                    }
                }
                DeltaThreshold::Absolute(_) => cur_stat - base_stat,
            };

            let comparison_delta = if self.abs {
                original_delta.abs()
            } else {
                original_delta
            };

            if self
                .comparator
                .compare(comparison_delta, self.delta.threshold())
            {
                let finding_score =
                    compute_finding_score(comparison_delta, self.delta.threshold(), self.score);

                let delta_description = match self.delta {
                    DeltaThreshold::Ratio(_) => delta_ratio_to_percentage_string(original_delta),
                    DeltaThreshold::Absolute(_) => delta_to_difference_string(original_delta),
                };

                let finding_description = format!(
                    "The {} in {} ({}) is {} {} ({}).",
                    self.stat,
                    run_name,
                    formatted_number_string(cur_stat),
                    delta_description,
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
