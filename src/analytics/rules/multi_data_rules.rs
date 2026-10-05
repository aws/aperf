use crate::analytics::{
    AnalyticalFinding, DataFindings, MultiDataAnalyticalRule, MultiDataAnalyze, Score,
};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use crate::data::common::utils::is_metal_instance_type;
use crate::data::kernel_config::KernelConfig;
use crate::data::perf_stat::PerfStat;
use crate::data::systeminfo::SystemInfo;
use crate::get_data_name_from_type;
use std::collections::HashMap;
use versions::Versioning;

// TODO: implement key-value data
const PREEMPT_LAZY_MESSAGE: &str = "Linux 7.0 changed the default preemption model to PREEMPT_LAZY. If you observe unexpected performance differences, consider setting the preemption model to PREEMPT_NONE via kernel boot parameter (preempt=none) or kernel config (CONFIG_PREEMPT_NONE=y), or implementing rseq (restartable sequences) in the application's hot functions. See: https://lore.kernel.org/all/20260403191942.21410-1-dipiets@amazon.it/T/#t";

/// Fires when kernel >= 7 and CONFIG_PREEMPT_LAZY=y.
/// Based on: https://lore.kernel.org/all/20260403191942.21410-1-dipiets@amazon.it/T/#t
pub struct PreemptLazyDetectedRule;

impl MultiDataAnalyze for PreemptLazyDetectedRule {
    fn analyze(
        &self,
        findings: &mut HashMap<String, DataFindings>,
        all_processed_data: &HashMap<String, &ProcessedData>,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        let systeminfo = match all_processed_data.get(get_data_name_from_type::<SystemInfo>()) {
            Some(d) => *d,
            None => return,
        };
        let kconfig = match all_processed_data.get(get_data_name_from_type::<KernelConfig>()) {
            Some(d) => *d,
            None => return,
        };

        for run_name in systeminfo.runs.keys() {
            let version = match processed_data_accessor.key_value_value_by_key(
                systeminfo,
                run_name,
                "Kernel Version",
            ) {
                Some(v) => v,
                None => continue,
            };

            if Versioning::new(version).is_none_or(|v| v < Versioning::new("7").unwrap()) {
                continue;
            }

            let is_lazy = processed_data_accessor
                .key_value_value_by_key(kconfig, run_name, "CONFIG_PREEMPT_LAZY")
                .map(|v| v == "y")
                .unwrap_or(false);
            if !is_lazy {
                continue;
            }

            let desc = format!(
                "Kernel {} in {} is running with PREEMPT_LAZY preemption model.",
                version, run_name
            );
            findings
                .entry(get_data_name_from_type::<KernelConfig>().to_string())
                .or_insert(DataFindings::default())
                .insert_finding(
                    run_name,
                    "CONFIG_PREEMPT_LAZY",
                    AnalyticalFinding::new(
                        "PREEMPT_LAZY Detected".to_string(),
                        Score::Neutral.as_f64(),
                        desc,
                        PREEMPT_LAZY_MESSAGE.to_string(),
                    ),
                );
        }
    }
}

const LOW_PERF_EVENT_MUX_INTERVAL_MESSAGE: &str = "Frequent context rotation can cause increased CPU overhead per core. Consider setting it to 100ms before recording: echo 100 | sudo tee /sys/bus/event_source/devices/*/perf_event_mux_interval_ms";

/// Fires when the PMU counters were multiplexed (a counter schedule rate below 100%) while
/// perf_event_mux_interval_ms was below the recommended 100ms, except on metal instances
/// running kernel v6.2 or newer, where the context rotation overhead is very low.
pub struct LowPerfEventMuxIntervalRule;

impl MultiDataAnalyze for LowPerfEventMuxIntervalRule {
    fn analyze(
        &self,
        findings: &mut HashMap<String, DataFindings>,
        all_processed_data: &HashMap<String, &ProcessedData>,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        let systeminfo = match all_processed_data.get(get_data_name_from_type::<SystemInfo>()) {
            Some(d) => *d,
            None => return,
        };
        let perf_stat = match all_processed_data.get(get_data_name_from_type::<PerfStat>()) {
            Some(d) => *d,
            None => return,
        };

        for run_name in systeminfo.runs.keys() {
            let mux_interval_ms = match processed_data_accessor
                .key_value_value_by_key(systeminfo, run_name, "Perf Event Mux Interval")
                .and_then(|v| v.split_whitespace().next())
                .and_then(|v| v.parse::<f64>().ok())
            {
                Some(v) => v,
                None => continue,
            };
            if mux_interval_ms >= 100.0 {
                continue;
            }

            let is_metal = processed_data_accessor
                .key_value_value_by_key(systeminfo, run_name, "Instance Type")
                .is_some_and(is_metal_instance_type);
            let is_kernel_6_2_or_newer = processed_data_accessor
                .key_value_value_by_key(systeminfo, run_name, "Kernel Version")
                .and_then(Versioning::new)
                .is_some_and(|v| v >= Versioning::new("6.2").unwrap());
            if is_metal && is_kernel_6_2_or_newer {
                continue;
            }

            let Some(schedule_rate_stats) = processed_data_accessor.time_series_metric_stats(
                perf_stat,
                run_name,
                "mux_counter_schedule_rate",
            ) else {
                continue;
            };
            if schedule_rate_stats.min >= 100.0 {
                continue;
            }

            let desc = format!(
                "The PMU counters in {} were multiplexed with a perf_event_mux_interval_ms of {}ms.",
                run_name,
                mux_interval_ms
            );
            findings
                .entry(get_data_name_from_type::<PerfStat>().to_string())
                .or_insert(DataFindings::default())
                .insert_finding(
                    run_name,
                    "mux_counter_schedule_rate",
                    AnalyticalFinding::new(
                        "Low Perf Event Mux Interval".to_string(),
                        Score::Concerning.as_f64(),
                        desc,
                        LOW_PERF_EVENT_MUX_INTERVAL_MESSAGE.to_string(),
                    ),
                );
        }
    }
}

pub fn get_multi_data_rules() -> Vec<MultiDataAnalyticalRule> {
    vec![
        MultiDataAnalyticalRule::PreemptLazyDetectedRule(PreemptLazyDetectedRule),
        MultiDataAnalyticalRule::LowPerfEventMuxIntervalRule(LowPerfEventMuxIntervalRule),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::common::data_formats::{
        AperfData, DataFormat, KeyValueData, KeyValueGroup, Series, TimeSeriesData,
        TimeSeriesMetric,
    };
    use crate::data::kernel_config::KernelConfig;
    use crate::data::perf_stat::PerfStat;
    use crate::data::systeminfo::SystemInfo;

    fn kv_data(runs: Vec<(&str, Vec<(&str, &str)>)>) -> ProcessedData {
        let mut pd = ProcessedData::new("test".to_string());
        pd.data_format = DataFormat::KeyValue;
        for (run, pairs) in runs {
            let mut kv = KeyValueData::default();
            let mut group = KeyValueGroup::default();
            for (k, v) in pairs {
                group.key_values.insert(k.to_string(), v.to_string());
            }
            kv.key_value_groups.insert(String::new(), group);
            pd.runs.insert(run.to_string(), AperfData::KeyValue(kv));
        }
        pd
    }

    fn run_rule(sysinfo: &ProcessedData, kconfig: &ProcessedData) -> HashMap<String, DataFindings> {
        let mut all: HashMap<String, &ProcessedData> = HashMap::new();
        all.insert(get_data_name_from_type::<SystemInfo>().to_string(), sysinfo);
        all.insert(
            get_data_name_from_type::<KernelConfig>().to_string(),
            kconfig,
        );
        let mut findings = HashMap::new();
        let mut acc = ProcessedDataAccessor::new();
        PreemptLazyDetectedRule.analyze(&mut findings, &all, &mut acc);
        findings
    }

    #[test]
    fn triggers_k7_preempt_lazy() {
        let si = kv_data(vec![("r1", vec![("Kernel Version", "7.0.1")])]);
        let kc = kv_data(vec![("r1", vec![("CONFIG_PREEMPT_LAZY", "y")])]);
        assert!(run_rule(&si, &kc).contains_key(get_data_name_from_type::<KernelConfig>()));
    }

    #[test]
    fn skips_k6() {
        let si = kv_data(vec![("r1", vec![("Kernel Version", "6.12.0")])]);
        let kc = kv_data(vec![("r1", vec![("CONFIG_PREEMPT_LAZY", "y")])]);
        assert!(!run_rule(&si, &kc).contains_key(get_data_name_from_type::<KernelConfig>()));
    }

    #[test]
    fn skips_no_preempt_lazy() {
        let si = kv_data(vec![("r1", vec![("Kernel Version", "7.0.1")])]);
        let kc = kv_data(vec![("r1", vec![])]);
        assert!(!run_rule(&si, &kc).contains_key(get_data_name_from_type::<KernelConfig>()));
    }

    fn schedule_rate_data(runs: Vec<(&str, Vec<f64>)>) -> ProcessedData {
        let mut pd = ProcessedData::new(get_data_name_from_type::<PerfStat>().to_string());
        pd.data_format = DataFormat::TimeSeries;
        for (run, values) in runs {
            let mut metric = TimeSeriesMetric::new("mux_counter_schedule_rate".to_string());
            metric.series.push(Series {
                series_name: "Aggregate".to_string(),
                time_diff: (0..values.len() as u64).collect(),
                values,
                is_aggregate: true,
            });
            let mut ts = TimeSeriesData::default();
            ts.metrics
                .insert("mux_counter_schedule_rate".to_string(), metric);
            pd.runs.insert(run.to_string(), AperfData::TimeSeries(ts));
        }
        pd
    }

    fn run_mux_interval_rule(
        sysinfo: &ProcessedData,
        perf_stat: &ProcessedData,
    ) -> HashMap<String, DataFindings> {
        let mut all: HashMap<String, &ProcessedData> = HashMap::new();
        all.insert(get_data_name_from_type::<SystemInfo>().to_string(), sysinfo);
        all.insert(get_data_name_from_type::<PerfStat>().to_string(), perf_stat);
        let mut findings = HashMap::new();
        let mut acc = ProcessedDataAccessor::new();
        LowPerfEventMuxIntervalRule.analyze(&mut findings, &all, &mut acc);
        findings
    }

    #[test]
    fn triggers_low_mux_interval_with_multiplexing() {
        let si = kv_data(vec![("r1", vec![("Perf Event Mux Interval", "10 ms")])]);
        let ps = schedule_rate_data(vec![("r1", vec![100.0, 62.5, 100.0])]);
        assert!(run_mux_interval_rule(&si, &ps).contains_key(get_data_name_from_type::<PerfStat>()));
    }

    #[test]
    fn skips_recommended_mux_interval_with_multiplexing() {
        let si = kv_data(vec![("r1", vec![("Perf Event Mux Interval", "100 ms")])]);
        let ps = schedule_rate_data(vec![("r1", vec![50.0])]);
        assert!(
            !run_mux_interval_rule(&si, &ps).contains_key(get_data_name_from_type::<PerfStat>())
        );
    }

    #[test]
    fn skips_metal_on_new_kernel() {
        let ps = schedule_rate_data(vec![("r1", vec![50.0])]);
        for (instance_type, kernel_version) in [
            ("c7g.metal", "6.12.103-127.188.amzn2023.aarch64"),
            ("m7i.metal-24xl", "6.2.0"),
        ] {
            let si = kv_data(vec![(
                "r1",
                vec![
                    ("Perf Event Mux Interval", "10 ms"),
                    ("Instance Type", instance_type),
                    ("Kernel Version", kernel_version),
                ],
            )]);
            assert!(!run_mux_interval_rule(&si, &ps)
                .contains_key(get_data_name_from_type::<PerfStat>()));
        }
    }
}
