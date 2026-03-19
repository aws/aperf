use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_custom_aggregate;
use crate::data::common::utils::{get_aggregate_series_name, get_cpu_series_name};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::prelude::*,
};

/// Gather CPU Utilization raw data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpuUtilizationRaw {
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl CpuUtilizationRaw {
    pub fn new() -> Self {
        CpuUtilizationRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for CpuUtilizationRaw {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl CollectData for CpuUtilizationRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/stat")?;
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

/// A helper struct that parses and holds one snapshot of /proc/stat data
pub struct ProcKernelStat {
    total: Vec<u64>,
    per_cpu: Vec<Vec<u64>>,
}

impl ProcKernelStat {
    pub fn from_raw_data(raw_data: &String) -> Self {
        let mut kernel_stats = ProcKernelStat {
            total: Vec::new(),
            per_cpu: Vec::new(),
        };

        for line in raw_data.lines() {
            let split: Vec<&str> = line.split_whitespace().collect();

            if split.len() < 9 {
                continue;
            }

            // For aggregate the label will just be "cpu"; for individual core the label
            // will be "cpu<number>"
            let cpu_label = split[0];

            let cpu_stat: Vec<u64> = split[1..]
                .iter()
                .map(|&s| s.parse::<u64>().unwrap_or_default())
                .collect();

            if cpu_label == "cpu" {
                kernel_stats.total = cpu_stat;
            } else if cpu_label.starts_with("cpu") {
                kernel_stats.per_cpu.push(cpu_stat);
            }
        }

        kernel_stats
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

fn get_cpu_time(cpu_state: &CpuState, cpu_time: &Vec<u64>) -> u64 {
    let index = match cpu_state {
        CpuState::USER => 0,
        CpuState::NICE => 1,
        CpuState::SYSTEM => 2,
        CpuState::IDLE => 3,
        CpuState::IOWAIT => 4,
        CpuState::IRQ => 5,
        CpuState::SOFTIRQ => 6,
        CpuState::STEAL => 7,
    };

    cpu_time.get(index).copied().unwrap_or_default()
}

impl ProcessData for CpuUtilization {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_custom_aggregate!();
        // Override the value ranges - we want every metric graph to show from 0 to 100
        time_series_data_processor.set_fixed_value_range((0, 100));
        // CPU utils have a dedicated aggregate metric to hold the aggregate of every CPU-state metric, as well as
        // a series for total CPU utilization
        let aggregate_metric_name = "aggregate";
        let total_series_name = "total";

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::CpuUtilizationRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            let kernel_stats = ProcKernelStat::from_raw_data(&raw_value.data);
            let aggregate_cpu_time = vec![kernel_stats.total];
            let all_cpu_time = kernel_stats.per_cpu;
            let num_cpus = all_cpu_time.len();

            for (cpu, cpu_time) in all_cpu_time
                .iter()
                .chain(aggregate_cpu_time.iter())
                .enumerate()
            {
                // Compute the cpu time delta for every CPU state as the numerator, and the sum of
                // all deltas as the denominator
                let mut per_cpu_state_time_delta: HashMap<CpuState, f64> = HashMap::new();
                let mut cpu_time_delta_sum = 0.0;
                for cpu_state in CpuState::iter() {
                    let cur_cpu_time = get_cpu_time(&cpu_state, cpu_time);
                    if let Some(cpu_time_delta) = time_series_data_processor
                        .get_delta_and_set_previous_value(
                            &cpu_state.to_string(),
                            &get_cpu_series_name(cpu),
                            cur_cpu_time as f64,
                        )
                    {
                        per_cpu_state_time_delta.insert(cpu_state, cpu_time_delta);
                        cpu_time_delta_sum += cpu_time_delta;
                    }
                }

                // Compute CPU utilization by dividing every time delta by the delta sum (times 100 to get percentage)
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
                        // Processing one of the CPUs - add the data point to the CPU series in the
                        // state metric
                        time_series_data_processor.add_data_point(
                            &cpu_state.to_string(),
                            &get_cpu_series_name(cpu),
                            cpu_util,
                        );
                    } else {
                        // Processing the aggregate of all CPUs - add the data point to the aggregate
                        // series of the state metric, as well as the state series of the aggregate metric
                        time_series_data_processor.add_aggregate_data_point(
                            &cpu_state.to_string(),
                            &get_aggregate_series_name(),
                            cpu_util,
                        );
                        time_series_data_processor.add_data_point(
                            aggregate_metric_name,
                            &cpu_state.to_string(),
                            cpu_util,
                        );
                    }
                }

                // If processing the aggregate of all CPUs, also compute the total CPU utilization
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
                    time_series_data_processor.add_aggregate_data_point(
                        aggregate_metric_name,
                        total_series_name,
                        total_cpu_util,
                    );
                }
            }
        }

        let mut sorted_metric_names = vec![aggregate_metric_name.to_string()];
        for cpu_state in CpuState::iter() {
            sorted_metric_names.push(cpu_state.to_string());
        }
        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(
                sorted_metric_names.iter().map(AsRef::as_ref).collect(),
            );

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod cpu_tests {
    #[cfg(target_os = "linux")]
    use {
        super::CpuUtilizationRaw,
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut cpu_utilization = CpuUtilizationRaw::new();
        let params = CollectorParams::new();

        cpu_utilization.collect_data(&params).unwrap();
        assert!(!cpu_utilization.data.is_empty());
    }
}
