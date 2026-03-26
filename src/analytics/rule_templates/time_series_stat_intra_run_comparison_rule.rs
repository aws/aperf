use crate::analytics::{compute_finding_score, AnalyticalFinding, Analyze, DataFindings};
use crate::computations::{
    delta_ratio_to_percentage_string, formatted_number_string, Comparator, Stat,
};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use std::fmt;
use std::fmt::Formatter;

/// This rule computes the delta of the specified metric stat between two runs,
/// and compares it (or the magnitude of it with abs=True) with the threshold delta_ratio.
pub struct TimeSeriesStatIntraRunComparisonRule {
    pub rule_name: &'static str,
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
        name: $rule_name:literal,
        baseline_metric: $baseline_metric_name:literal,
        comparison_metric: $comparison_metric_name:literal,
        stat: $stat:path,
        comparator: $comparator:path,
        abs: $abs:literal,
        delta_ratio: $delta_ratio:literal,
        score: $score:expr,
        message: $message:literal,
    } => {
        AnalyticalRule::TimeSeriesStatIntraRunComparisonRule(
            TimeSeriesStatIntraRunComparisonRule{
                rule_name: $rule_name,
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
            "TimeSeriesStatIntraRunComparisonRule {} <checking if the delta_ratio of {} of {} between {} is {} than {}>",
            self.rule_name, self.stat, self.baseline_metric_name, self.comparison_metric_name, self.comparator, self.delta_ratio
        )
    }
}

impl Analyze for TimeSeriesStatIntraRunComparisonRule {
    fn analyze(
        &self,
        data_findings: &mut DataFindings,
        processed_data: &ProcessedData,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        for run_name in processed_data.runs.keys() {
            let baseline_metric_stat = match processed_data_accessor.time_series_metric_stats(
                processed_data,
                run_name,
                self.baseline_metric_name,
            ) {
                Some(baseline_metric_stats) => self.stat.get_stat(&baseline_metric_stats),
                None => continue,
            };

            let comparison_metric_stat = match processed_data_accessor.time_series_metric_stats(
                processed_data,
                run_name,
                self.comparison_metric_name,
            ) {
                Some(comparison_metric_stats) => self.stat.get_stat(&comparison_metric_stats),
                None => continue,
            };

            let cur_ratio = if comparison_metric_stat == baseline_metric_stat {
                0.0
            } else if baseline_metric_stat == 0.0 {
                // When the baseline metric stat is zero, the delta computation cannot be performed,
                // so treat the comparison metric stat as 100% larger than the base stat
                1.0
            } else {
                (comparison_metric_stat - baseline_metric_stat) / baseline_metric_stat
            };

            let rule_matched = self.comparator.compare(
                if self.abs { cur_ratio.abs() } else { cur_ratio },
                self.delta_ratio,
            );

            if rule_matched {
                let finding_score = compute_finding_score(cur_ratio, self.delta_ratio, self.score);

                let finding_description = format!(
                    "The {} in {} ({}) is {} {} ({}).",
                    self.stat,
                    self.comparison_metric_name,
                    formatted_number_string(comparison_metric_stat),
                    delta_ratio_to_percentage_string(cur_ratio),
                    self.baseline_metric_name,
                    formatted_number_string(baseline_metric_stat),
                );

                data_findings.insert_finding(
                    run_name,
                    self.comparison_metric_name,
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
