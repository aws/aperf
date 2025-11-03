use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::utils::{get_aggregate_cpu_series_name, get_cpu_series_name};
use crate::data::{CollectData, CollectorParams, Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use log::trace;
use procfs::{CpuTime, KernelStats};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

/// Gather CPU Utilization raw data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpuUtilizationRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl CpuUtilizationRaw {
    pub fn new() -> Self {
        CpuUtilizationRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl Default for CpuUtilizationRaw {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectData for CpuUtilizationRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/stat")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpuUtilization;

impl CpuUtilization {
    pub fn new() -> Self {
        CpuUtilization
    }
}

#[derive(EnumIter, Display, Clone, Copy, Eq, Hash, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum CpuState {
    USER,
    NICE,
    SYSTEM,
    IDLE,
    IOWAIT,
    IRQ,
    SOFTIRQ,
    STEAL,
}

fn get_cpu_time(cpu_state: &CpuState, cpu_time: &CpuTime) -> u64 {
    match cpu_state {
        CpuState::USER => cpu_time.user,
        CpuState::NICE => cpu_time.nice,
        CpuState::SYSTEM => cpu_time.system,
        CpuState::IDLE => cpu_time.idle,
        CpuState::IOWAIT => cpu_time.iowait.unwrap_or_default(),
        CpuState::IRQ => cpu_time.irq.unwrap_or_default(),
        CpuState::SOFTIRQ => cpu_time.softirq.unwrap_or_default(),
        CpuState::STEAL => cpu_time.steal.unwrap_or_default(),
    }
}

impl ProcessData for CpuUtilization {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();
        // the aggregate series of all CPU state's metrics to be included in the aggregate metric
        let mut per_cpu_state_aggregate_series: HashMap<CpuState, Series> = HashMap::new();
        // the aggregate series of total CPU utilization
        let mut aggregate_total_util_series = Series::new(Some("total".to_string()));
        aggregate_total_util_series.is_aggregate = true;

        // memorize the previous CPU time to compute delta (the raw /proc/stat file contains
        // accumulated CPU jiffies since boot time)
        let mut prev_cpu_time: Vec<CpuTime> = Vec::new();
        // initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::CpuUtilizationRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let kernel_stats = KernelStats::from_reader(raw_value.data.as_bytes())?;
            let aggregate_cpu_time = vec![kernel_stats.total];
            let all_cpu_time = kernel_stats.cpu_time;
            let num_cpus = all_cpu_time.len();

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            for (cpu, cpu_time) in all_cpu_time
                .iter()
                .chain(aggregate_cpu_time.iter())
                .enumerate()
            {
                // in the case where the current raw data is the first data point, use the current
                // CPU time as the prev time, to produce a dummy delta of 0
                if cpu >= prev_cpu_time.len() {
                    prev_cpu_time.push(cpu_time.clone());
                }

                // compute the cpu time delta for every CPU state as the numerator, and the sum of
                // all deltas as the denominator
                let mut per_cpu_state_time_delta: HashMap<CpuState, f64> = HashMap::new();
                let mut cpu_time_delta_sum = 0.0;
                for cpu_state in CpuState::iter() {
                    let time_delta = (get_cpu_time(&cpu_state, cpu_time)
                        - get_cpu_time(&cpu_state, &prev_cpu_time[cpu]))
                        as f64;
                    per_cpu_state_time_delta.insert(cpu_state, time_delta);
                    cpu_time_delta_sum += time_delta;
                }
                prev_cpu_time[cpu] = cpu_time.clone();

                // compute CPU utilization by dividing every time delta by the delta sum (times 100 to get percentage)
                // and store the result in the series values of the corresponding CPU state metric
                for cpu_state in CpuState::iter() {
                    let cpu_util = if cpu_time_delta_sum > 0.0 {
                        per_cpu_state_time_delta
                            .get(&cpu_state)
                            .copied()
                            .unwrap_or_default()
                            / cpu_time_delta_sum
                            * 100.0
                    } else {
                        0.0
                    };

                    if cpu < num_cpus {
                        // processing one of the CPUs - put the util data point in the corresponding series
                        // in the CPU state metric
                        let cpu_state_metric = time_series_data
                            .metrics
                            .entry(cpu_state.to_string())
                            .or_insert_with(|| {
                                let mut _cpu_state_metric = TimeSeriesMetric::default();
                                _cpu_state_metric.metric_name = cpu_state.to_string();
                                _cpu_state_metric.value_range = (0, 100);
                                _cpu_state_metric
                            });

                        if cpu >= cpu_state_metric.series.len() {
                            cpu_state_metric
                                .series
                                .push(Series::new(get_cpu_series_name(cpu)));
                        }
                        let cpu_series = &mut cpu_state_metric.series[cpu];
                        cpu_series.time_diff.push(time_diff);
                        cpu_series.values.push(cpu_util);
                    } else {
                        // processing the aggregate of all CPUs - put the util data point in the CPU state
                        // series to be included the aggregate metric
                        let aggregate_cpu_state_series = per_cpu_state_aggregate_series
                            .entry(cpu_state)
                            .or_insert(Series::new(Some(cpu_state.to_string())));
                        aggregate_cpu_state_series.time_diff.push(time_diff);
                        aggregate_cpu_state_series.values.push(cpu_util);
                    }
                }

                // if processing the aggregate of all CPUs, also compute the total CPU utilization
                // which is sum of per-state time minus idle time
                if cpu >= num_cpus {
                    let total_cpu_util = if cpu_time_delta_sum > 0.0 {
                        (cpu_time_delta_sum
                            - per_cpu_state_time_delta
                                .get(&CpuState::IDLE)
                                .copied()
                                .unwrap_or_default())
                            / cpu_time_delta_sum
                            * 100.0
                    } else {
                        0.0
                    };
                    aggregate_total_util_series.time_diff.push(time_diff);
                    aggregate_total_util_series.values.push(total_cpu_util);
                }
            }
        }

        // add every aggregate CPU state series to the corresponding CPU state metric and set the
        // metric's stats as computed from the aggregate series
        for (cpu_state, aggregate_cpu_state_series) in &per_cpu_state_aggregate_series {
            if let Some(cpu_state_metric) = time_series_data.metrics.get_mut(&cpu_state.to_string())
            {
                let mut cur_cpu_state_aggregate_series = aggregate_cpu_state_series.clone();
                cur_cpu_state_aggregate_series.series_name = get_aggregate_cpu_series_name();
                cur_cpu_state_aggregate_series.is_aggregate = true;
                cpu_state_metric.series.push(cur_cpu_state_aggregate_series);
                cpu_state_metric.stats =
                    Statistics::from_values(&aggregate_cpu_state_series.values);
            }
        }

        let aggregate_metric_name = "aggregate";
        if !per_cpu_state_aggregate_series.is_empty() {
            // create the aggregate metric to include the aggregate series of all CPU states as well
            // as the total CPU utilization
            let mut aggregate_metric = TimeSeriesMetric::default();
            aggregate_metric.metric_name = aggregate_metric_name.to_string();
            aggregate_metric.value_range = (0, 100);
            for cpu_state in CpuState::iter() {
                if let Some(cpu_state_aggregate_series) =
                    per_cpu_state_aggregate_series.remove(&cpu_state)
                {
                    aggregate_metric.series.push(cpu_state_aggregate_series);
                }
            }
            aggregate_metric.stats = Statistics::from_values(&aggregate_total_util_series.values);
            aggregate_metric.series.push(aggregate_total_util_series);
            time_series_data
                .metrics
                .insert(aggregate_metric_name.to_string(), aggregate_metric);
        }

        let mut sorted_metric_names = vec![aggregate_metric_name.to_string()];
        for cpu_state in CpuState::iter() {
            sorted_metric_names.push(cpu_state.to_string());
        }
        time_series_data.sorted_metric_names = sorted_metric_names;

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod cpu_tests {
    use super::CpuUtilizationRaw;
    use crate::data::{CollectData, CollectorParams};

    #[test]
    fn test_collect_data() {
        let mut cpu_utilization = CpuUtilizationRaw::new();
        let params = CollectorParams::new();

        cpu_utilization.collect_data(&params).unwrap();
        assert!(!cpu_utilization.data.is_empty());
    }
}
