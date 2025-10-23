extern crate lazy_static;

use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::DataMetrics;
use crate::visualizer::GetData;
use anyhow::Result;
use chrono::prelude::*;
use core::f64;
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
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
pub struct ProcessTime {
    pub time: TimeEnum,
    pub cpu_time: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Processes {
    pub time: TimeEnum,
    pub entries: Vec<SampleEntry>,
}

impl Processes {
    pub fn new() -> Self {
        Processes {
            time: TimeEnum::DateTime(Utc::now()),
            entries: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SampleEntry {
    pub name: String,
    pub pid: u64,
    pub cpu_time: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessEntry {
    pub name: String,
    pub total_cpu_time: u64,
    pub samples: HashMap<TimeEnum, u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndEntry {
    pub name: String,
    pub total_cpu_time: f64,
    pub entries: Vec<Sample>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndEntries {
    pub collection_time: TimeEnum,
    pub end_entries: Vec<EndEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sample {
    pub cpu_time: f64,
    pub time: TimeEnum,
}

pub fn get_values(values: Vec<Processes>) -> Result<String> {
    let value_zero = values[0].clone();
    let time_zero = value_zero.time;
    let ticks_per_second: u64 = *TICKS_PER_SECOND.lock().unwrap();
    let mut process_map: HashMap<String, ProcessEntry> = HashMap::new();
    let mut total_time: u64 = 1;
    if let TimeEnum::TimeDiff(v) = values.last().unwrap().time - values[0].time {
        if v > 0 {
            total_time = v;
        }
    }

    for value in values {
        for entry in value.entries {
            let time = value.time - time_zero;
            match process_map.get_mut(&entry.name) {
                Some(pe) => {
                    let mut sample_cpu_time: u64 = entry.cpu_time;
                    if let Some(v) = pe.samples.get(&time) {
                        sample_cpu_time += v;
                    }
                    pe.samples.insert(time, sample_cpu_time);
                }
                None => {
                    let mut process_entry = ProcessEntry {
                        name: entry.name.clone(),
                        total_cpu_time: 0,
                        samples: HashMap::new(),
                    };
                    process_entry.samples.insert(time, entry.cpu_time);
                    process_map.insert(entry.name, process_entry);
                }
            }
        }
    }
    let mut end_values: EndEntries = EndEntries {
        collection_time: TimeEnum::TimeDiff(total_time),
        end_entries: Vec::new(),
    };

    for (_, process) in process_map.iter_mut() {
        let mut end_entry = EndEntry {
            name: process.name.clone(),
            total_cpu_time: 0.0,
            entries: Vec::new(),
        };
        let mut entries: Vec<(TimeEnum, u64)> = process.samples.clone().into_iter().collect();
        entries.sort_by(|(a, _), (c, _)| a.cmp(c));
        let entry_zero: (TimeEnum, u64) = entries[0];
        let mut prev_sample = Sample {
            time: entry_zero.0,
            cpu_time: entry_zero.1 as f64,
        };
        let mut prev_time: u64 = 0;
        let mut time_now;
        if let TimeEnum::TimeDiff(v) = prev_sample.time {
            prev_time = v;
        }
        for (time, cpu_time) in &entries {
            let sample = Sample {
                time: *time,
                cpu_time: *cpu_time as f64,
            };
            /* End sample */
            let mut end_sample = sample.clone();

            if end_sample.cpu_time as i64 - prev_sample.cpu_time as i64 >= 0 {
                /* Update sample based on previous sample */
                end_sample.cpu_time -= prev_sample.cpu_time;
            } else {
                end_sample.cpu_time = 0.0;
            }
            /* Add to total_cpu_time */
            end_entry.total_cpu_time += end_sample.cpu_time;

            match *time {
                TimeEnum::TimeDiff(v) => {
                    time_now = v;
                    if time_now - prev_time == 0 {
                        continue;
                    }
                }
                _ => continue,
            }

            /* Percentage utilization */
            end_sample.cpu_time /= (ticks_per_second * (time_now - prev_time)) as f64;
            end_sample.cpu_time *= 100.0;

            prev_time = time_now;
            end_entry.entries.push(end_sample);

            /* Copy to prev_sample */
            prev_sample = sample.clone();
        }
        end_values.end_entries.push(end_entry);
    }
    /* Order the processes by Total CPU Time per collection time */
    end_values
        .end_entries
        .sort_by(|a, b| (b.total_cpu_time).total_cmp(&(a.total_cpu_time)));

    if end_values.end_entries.len() > 16 {
        end_values.end_entries = end_values.end_entries[0..15].to_vec();
    }

    Ok(serde_json::to_string(&end_values)?)
}

// TODO: ------------------------------------------------------------------------------------------
//       Below are the new implementation to process processes into uniform data format. Remove
//       the original for the migration.
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
// TODO: ------------------------------------------------------------------------------------------

impl GetData for Processes {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let mut processes = Processes::new();
        let raw_value = match buffer {
            Data::ProcessesRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        *TICKS_PER_SECOND.lock().unwrap() = raw_value.ticks_per_second;
        let reader = BufReader::new(raw_value.data.as_bytes());
        processes.time = raw_value.time;
        for line in reader.lines() {
            let line = line?;

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
            let pid = line[..open_pos - 1].parse::<u64>()?;
            let name = line[open_pos + 1..close_pos].to_string();
            let values: Vec<&str> = line[close_pos + 2..].split_whitespace().collect();

            if values.len() < PROC_PID_STAT_KERNELSPACE_TIME_POS + 1 {
                continue;
            }
            let user_time = values[PROC_PID_STAT_USERSPACE_TIME_POS].parse::<u64>()?;
            let kernel_time = values[PROC_PID_STAT_KERNELSPACE_TIME_POS].parse::<u64>()?;
            let cpu_time = user_time + kernel_time;

            processes.entries.push(SampleEntry {
                name,
                pid,
                cpu_time,
            });
        }
        let processed_data = ProcessedData::Processes(processes);
        Ok(processed_data)
    }

    fn process_raw_data_new(&mut self, raw_data: Vec<Data>) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();

        // map process field -> process -> series
        let mut per_field_per_process_series: HashMap<ProcessKey, HashMap<String, Series>> =
            HashMap::new();

        let ticks_per_second: u64 = *TICKS_PER_SECOND.lock().unwrap();
        let time_zero = if let Some(first_buffer) = raw_data.first() {
            match first_buffer {
                Data::ProcessesRaw(ref value) => value.time,
                _ => panic!("Invalid Data type in raw file"),
            }
        } else {
            return Ok(AperfData::TimeSeries(time_series_data));
        };

        // Get data into Series format
        for buffer in raw_data {
            let raw_value = match buffer {
                Data::ProcessesRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

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

        // Track totals for filtering top processes
        let mut process_totals_map: HashMap<ProcessKey, HashMap<String, f64>> = HashMap::new();

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
                        ProcessKey::VirtualMemorySize => {
                            // Virtual memory size: snapshot convert to KB
                            current_value / 1000.0
                        }
                        _ => {
                            // Other metrics: snapshot value
                            current_value
                        }
                    };

                    // Accumulate totals for all ProcessKey types
                    *process_totals_map
                        .entry(*process_key)
                        .or_insert_with(HashMap::new)
                        .entry(process_pid_name.clone())
                        .or_insert(0.0) += stat;

                    series.values[i] = stat;
                    prev_value = current_value;
                    prev_time = current_time;
                }
            }
        }

        // Add top processes to time_series_data
        for (process_key, process_map) in per_field_per_process_series {
            // Get top 16 processes for this ProcessKey
            let top_processes: Vec<String> =
                if let Some(totals) = process_totals_map.get(&process_key) {
                    let mut ranking: Vec<(String, f64)> =
                        totals.iter().map(|(k, v)| (k.clone(), *v)).collect();
                    ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
                    ranking.into_iter().take(16).map(|(name, _)| name).collect()
                } else {
                    Vec::new()
                };

            let mut series_vec = Vec::new();

            for process_name in &top_processes {
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

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(
        &mut self,
        buffer: Vec<ProcessedData>,
        query: String,
        _metrics: &mut DataMetrics,
    ) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::Processes(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        if param.len() < 2 {
            panic!("Not enough arguments");
        }
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "values" => get_values(values.clone()),
            _ => panic!("Unsupported API"),
        }
    }
}

#[cfg(test)]
mod process_test {
    use super::{Processes, ProcessesRaw};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut processes = ProcessesRaw::new();
        let params = CollectorParams::new();
        processes.prepare_data_collector(&params).unwrap();
        processes.collect_data(&params).unwrap();
        assert!(!processes.data.is_empty());
    }

    #[test]
    fn test_process_raw_data() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut processes_zero = ProcessesRaw::new();
        let mut processes_one = ProcessesRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        processes_zero.prepare_data_collector(&params).unwrap();
        processes_one.prepare_data_collector(&params).unwrap();
        processes_zero.collect_data(&params).unwrap();
        processes_one.collect_data(&params).unwrap();

        buffer.push(Data::ProcessesRaw(processes_zero));
        buffer.push(Data::ProcessesRaw(processes_one));
        for buf in buffer {
            processed_buffer.push(Processes::new().process_raw_data(buf).unwrap());
        }
        assert!(!processed_buffer.is_empty(), "{:#?}", processed_buffer);
    }
}
