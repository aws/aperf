use crate::analytics::time_series_stat_threshold_rule::time_series_stat_threshold_for_series;
use crate::analytics::{AnalyticalRule, Score, TimeSeriesStatThresholdRule};
use crate::computations::{Comparator, Stat};
use crate::data::{aperf_stats::AperfStats, AnalyzeData};

impl AnalyzeData for AperfStats {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_threshold_for_series! {
                name: "High APerf Userspace CPU Time",
                metric: "process_user_space_time",
                series: "aperf",
                stat: Stat::Average,
                comparator: Comparator::Greater,
                threshold: 0.01,
                score: Score::Concerning,
                message: "APerf consumed higher-than-expected userspace CPU time during its collection, and it might impacted the performance of the application undertest.",
            },
            time_series_stat_threshold_for_series! {
                name: "High APerf Kernelspace CPU Time",
                metric: "process_kernel_space_time",
                series: "aperf",
                stat: Stat::Average,
                comparator: Comparator::Greater,
                threshold: 0.04,
                score: Score::Concerning,
                message: "APerf consumed higher-than-expected kernelspace CPU time during its collection, and it might impacted the performance of the application undertest.",
            },
        ]
    }
}
