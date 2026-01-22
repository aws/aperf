extern crate lazy_static;

use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use core::f64;
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

pub const PROC_PID_STAT_USERSPACE_TIME_POS: usize = 11;
pub const PROC_PID_STAT_KERNELSPACE_TIME_POS: usize = 12;

lazy_static! {
    pub static ref TICKS_PER_SECOND: Mutex<u64> = Mutex::new(0);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessesRaw {
    pub time: TimeEnum,
    pub ticks_per_second: u64,
    pub data: String,
}

impl ProcessesRaw {
    pub fn new() -> Self {
        ProcessesRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
            ticks_per_second: 0,
        }
    }
}

impl Default for ProcessesRaw {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectData for ProcessesRaw {
    fn prepare_data_collector(&mut self, _params: &CollectorParams) -> Result<()> {
        *TICKS_PER_SECOND.lock().unwrap() = procfs::ticks_per_second()? as u64;
        Ok(())
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        let ticks_per_second: u64 = *TICKS_PER_SECOND.lock().unwrap();
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            let file_name = entry.file_name().to_str().unwrap().to_string();
            if file_name.chars().all(char::is_numeric) {
                let mut path = entry.path();
                path.push("stat");
                if let Ok(v) = fs::read_to_string(path) {
                    self.data.push_str(&v)
                }
            }
        }
        self.ticks_per_second = ticks_per_second;
        trace!("{:#?}", self.data);
        trace!("{:#?}", self.ticks_per_second);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Processes;

impl Processes {
    pub fn new() -> Self {
        Processes
    }
}

#[derive(EnumIter, Display, Clone, Copy, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum ProcessKey {
    UserSpaceTime,
    KernelSpaceTime,
    NumberThreads,
    VirtualMemorySize,
    ResidentSetSize,
}

fn get_process_key_stat(process_key: ProcessKey, values: Vec<&str>) -> Option<u64> {
    // The last element we access is the 22nd element in a values vector (ResidentSetSize), make sure the index 21 exists
    if values.len() < 21 + 1 {
        warn!("Incomplete proc/<PID>/stat entry found, skipping...");
        return None;
    }
    let result = match process_key {
        ProcessKey::UserSpaceTime => values[11].parse::<u64>().ok()?,
        ProcessKey::KernelSpaceTime => values[12].parse::<u64>().ok()?,
        ProcessKey::NumberThreads => values[17].parse::<u64>().ok()?,
        ProcessKey::VirtualMemorySize => values[20].parse::<u64>().ok()?,
        ProcessKey::ResidentSetSize => values[21].parse::<u64>().ok()?,
    };
    Some(result)
}

impl ProcessData for Processes {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();

        // map process field -> process -> series
        let mut per_field_per_process_series: HashMap<ProcessKey, HashMap<String, Series>> =
            HashMap::new();

        let time_zero = if let Some(first_buffer) = raw_data.first() {
            match first_buffer {
                Data::ProcessesRaw(ref value) => value.time,
                _ => panic!("Invalid Data type in raw file"),
            }
        } else {
            return Ok(AperfData::TimeSeries(time_series_data));
        };

        let mut ticks_per_second_option: Option<f64> = None;
        // Get data into Series format
        for buffer in raw_data {
            let raw_value = match buffer {
                Data::ProcessesRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            ticks_per_second_option.get_or_insert(raw_value.ticks_per_second as f64);

            let time: u64 = match raw_value.time - time_zero {
                TimeEnum::TimeDiff(v) => v,
                _ => continue,
            };

            for line in raw_value.data.lines() {
                let open_parenthesis = line.find('(');
                let open_pos = match open_parenthesis {
                    Some(v) => v,
                    None => continue,
                };
                let close_parenthesis = line.find(')');
                let close_pos = match close_parenthesis {
                    Some(v) => v,
                    None => continue,
                };
                let pid = line[..open_pos - 1]
                    .parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("Failed to parse PID"))?;
                let name = line[open_pos + 1..close_pos].to_string();
                let values: Vec<&str> = line[close_pos + 2..].split_whitespace().collect();

                for process_key in ProcessKey::iter() {
                    let Some(process_key_stat) = get_process_key_stat(process_key, values.clone())
                    else {
                        continue;
                    };

                    let process_pid_name = format!("{}_{}", pid, name);
                    let per_process_series = per_field_per_process_series
                        .entry(process_key)
                        .or_insert(HashMap::new());
                    let process_series = per_process_series
                        .entry(process_pid_name.clone())
                        .or_insert(Series::new(Some(process_pid_name.clone())));
                    process_series.time_diff.push(time);
                    process_series.values.push(process_key_stat as f64);
                }
            }
        }

        // If the raw data is empty default ticks per second to 1, in which case it should never
        // be used to compute any series values
        let ticks_per_second = ticks_per_second_option.unwrap_or_else(|| 1.0);
        // Track total cpu time for filtering top processes
        let mut process_cpu_time_map: HashMap<String, f64> = HashMap::new();

        // Convert to useful data from stats and calculate total time
        for (process_key, process_map) in per_field_per_process_series.iter_mut() {
            for (process_pid_name, series) in process_map.iter_mut() {
                let mut prev_value = series.values.get(0).copied().unwrap_or(0.0);
                let mut prev_time = series.time_diff.get(0).copied().unwrap_or(0);

                for i in 0..series.values.len() {
                    let current_value = series.values[i];
                    let current_time = series.time_diff[i];

                    let stat = match process_key {
                        ProcessKey::UserSpaceTime | ProcessKey::KernelSpaceTime => {
                            // CPU time: cumulative value, convert to aggregate percentage
                            let value_diff = current_value - prev_value;
                            let time_diff = current_time - prev_time;
                            let s = if time_diff > 0 {
                                (value_diff / (ticks_per_second as f64 * time_diff as f64)) * 100.0
                            } else {
                                0.0
                            };
                            s
                        }
                        _ => {
                            // Other metrics: snapshot value
                            current_value
                        }
                    };

                    if process_key == &ProcessKey::UserSpaceTime
                        || process_key == &ProcessKey::KernelSpaceTime
                    {
                        *process_cpu_time_map
                            .entry(process_pid_name.clone())
                            .or_insert(0.0) += stat;
                    }

                    series.values[i] = stat;
                    prev_value = current_value;
                    prev_time = current_time;
                }
            }
        }

        let mut ranking: Vec<(String, f64)> = process_cpu_time_map
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        let top_cpu_time_processes: Vec<String> =
            ranking.into_iter().take(16).map(|(name, _)| name).collect();

        // Add top processes for each metric to time_series_data
        for (process_key, process_map) in per_field_per_process_series {
            let mut series_vec = Vec::new();

            for process_name in &top_cpu_time_processes {
                if let Some(series) = process_map.get(process_name) {
                    series_vec.push(series.clone());
                }
            }

            if !series_vec.is_empty() {
                let mut max_avg = 0.0;
                let mut final_stats = Statistics::default();
                let mut metric_min: f64 = f64::MAX;
                let mut metric_max: f64 = 0.0;

                for series in &series_vec {
                    let skip = matches!(
                        process_key,
                        ProcessKey::UserSpaceTime | ProcessKey::KernelSpaceTime
                    ) as usize;
                    let stats = Statistics::from_values(&series.values[skip..].to_vec());
                    metric_min = metric_min.min(stats.min);
                    metric_max = metric_max.max(stats.max);
                    if stats.avg > max_avg {
                        max_avg = stats.avg;
                        final_stats = stats;
                    }
                }

                let value_range = (metric_min.floor() as u64, metric_max.ceil() as u64);

                let metric = TimeSeriesMetric {
                    metric_name: process_key.to_string(),
                    series: series_vec,
                    value_range,
                    stats: final_stats,
                };
                let metric_name = process_key.to_string();
                time_series_data.metrics.insert(metric_name.clone(), metric);
            }
        }

        time_series_data.sorted_metric_names = ProcessKey::iter()
            .map(|process_key| process_key.to_string())
            .collect();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod process_test {
    use super::ProcessesRaw;
    use crate::data::{CollectData, CollectorParams};

    #[test]
    fn test_collect_data() {
        let mut processes = ProcessesRaw::new();
        let params = CollectorParams::new();
        processes.prepare_data_collector(&params).unwrap();
        processes.collect_data(&params).unwrap();
        assert!(!processes.data.is_empty());
    }
}
