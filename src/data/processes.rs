use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_max_series_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::data_processing::ReportParams;
use crate::ProcessMetric;
use anyhow::Result;
use core::f64;
use log::warn;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;
#[cfg(target_os = "linux")]
use {
    crate::data::common::utils::read_virtual_file, crate::data::CollectData,
    crate::data_collection::InitParams, chrono::Utc, std::fs,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessesRaw {
    pub time: TimeEnum,
    pub ticks_per_second: u64,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl ProcessesRaw {
    pub fn new() -> Self {
        ProcessesRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
            ticks_per_second: 0,
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for ProcessesRaw {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl CollectData for ProcessesRaw {
    fn prepare_data_collector(&mut self, _init_params: &InitParams) -> Result<()> {
        self.ticks_per_second = procfs::ticks_per_second()? as u64;
        Ok(())
    }

    fn collect_data(&mut self, _init_params: &InitParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            let file_name = entry.file_name().to_str().unwrap().to_string();
            if file_name.chars().all(char::is_numeric) {
                let mut path = entry.path();
                path.push("stat");
                if let Ok(v) = read_virtual_file(path) {
                    self.data.push_str(&v)
                }
            }
        }
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

/// The number of `/proc/<pid>/stat` fields after the command name that are needed; the last
/// one read is ResidentSetSize at index 21.
const STAT_FIELD_COUNT: usize = 22;

fn get_process_metric_value(
    process_metric: ProcessMetric,
    values: &[&str],
    page_size: u64,
) -> Option<f64> {
    if values.len() < STAT_FIELD_COUNT {
        return None;
    }
    let field = |index: usize| values[index].parse::<u64>().ok();
    let result = match process_metric {
        ProcessMetric::UserSpaceTime => field(11)?,
        ProcessMetric::KernelSpaceTime => field(12)?,
        ProcessMetric::NumberThreads => field(17)?,
        ProcessMetric::VirtualMemorySize => field(20)?,
        ProcessMetric::ResidentSetSize => field(21)?,
        ProcessMetric::ResidentSetSizeBytes => {
            if page_size == 0 {
                return None;
            }
            field(21)? * page_size
        }
        ProcessMetric::NumberProcesses => return None,
    };
    Some(result as f64)
}

/// A `/proc/<pid>/stat` line borrowed from the raw data, with its fields left unsplit.
struct ProcessStatLine<'a> {
    pid: u64,
    name: &'a str,
    fields: &'a str,
}

impl<'a> ProcessStatLine<'a> {
    fn parse(line: &'a str) -> Option<Self> {
        let open_pos = line.find('(')?;
        let close_pos = line.rfind(')')?;
        if close_pos < open_pos {
            return None;
        }
        Some(ProcessStatLine {
            pid: line.get(..open_pos.checked_sub(1)?)?.parse().ok()?,
            name: line.get(open_pos + 1..close_pos)?,
            fields: line.get(close_pos + 2..)?,
        })
    }

    /// Every stat line in `data`, skipping lines that are not properly formatted stat entries.
    fn all(data: &'a str) -> impl Iterator<Item = Self> + 'a {
        data.lines().filter_map(Self::parse)
    }

    fn values(&self) -> Vec<&'a str> {
        self.fields.split_whitespace().collect()
    }

    fn key(&self) -> String {
        format!("{}_{}", self.pid, self.name)
    }
}

/// Lines without both user and kernel CPU ticks are treated as if the process were absent.
fn cpu_ticks(values: &[&str], page_size: u64) -> Option<(f64, f64)> {
    Some((
        get_process_metric_value(ProcessMetric::UserSpaceTime, values, page_size)?,
        get_process_metric_value(ProcessMetric::KernelSpaceTime, values, page_size)?,
    ))
}

impl ProcessData for Processes {
    fn process_raw_data(
        &mut self,
        report_params: &ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor =
            time_series_data_processor_with_max_series_aggregate!(report_params.collection_start);

        let raw_values: Vec<&ProcessesRaw> = raw_data
            .iter()
            .map(|buffer| match buffer {
                Data::ProcessesRaw(value) => value,
                _ => panic!("Invalid Data type in raw file"),
            })
            .collect();

        // Pass 1 ranks processes by CPU time; pass 2 re-parses only the lines of the top
        // ones, so memory does not grow with the run length times the number of processes.

        // The samples to keep, with their number of processes, and the max cpu time of each
        // process in the format of Map<pid_name, (utime, stime)>.
        let mut kept_samples: Vec<(&ProcessesRaw, usize)> = Vec::new();
        let mut per_process_cpu_time: HashMap<String, (f64, f64)> = HashMap::new();
        let mut ticks_per_second_option: Option<f64> = None;

        for raw_value in raw_values {
            // If multiple data were added at the same time diff, only keep the last one
            // Since processes data is collected once again at the end of collection,
            // this could happen if the finish stage completed fast.
            if let Some(&(last_raw_value, _)) = kept_samples.last() {
                if raw_value.time - last_raw_value.time == TimeEnum::TimeDiff(0) {
                    kept_samples.pop();
                }
            }

            ticks_per_second_option.get_or_insert(raw_value.ticks_per_second as f64);

            let mut number_processes = 0;
            for line in raw_value.data.lines() {
                let Some(stat_line) = ProcessStatLine::parse(line) else {
                    warn!("Malformed proc/<PID>/stat entry found, skipping...");
                    continue;
                };
                let values = stat_line.values();
                let Some((utime, stime)) = cpu_ticks(&values, report_params.page_size) else {
                    if values.len() < STAT_FIELD_COUNT {
                        warn!("Incomplete proc/<PID>/stat entry found, skipping...");
                    }
                    continue;
                };

                let key = stat_line.key();
                if let Some((max_utime, max_stime)) = per_process_cpu_time.get_mut(&key) {
                    *max_utime = max_utime.max(utime);
                    *max_stime = max_stime.max(stime);
                } else {
                    per_process_cpu_time.insert(key, (utime, stime));
                }
                number_processes += 1;
            }

            kept_samples.push((raw_value, number_processes));
        }

        // If the raw data is empty default ticks per second to 1, in which case it should never
        // be used to compute any series values
        let ticks_per_second = ticks_per_second_option.unwrap_or(1.0);

        let mut ranking: Vec<(String, f64)> = per_process_cpu_time
            .iter()
            .map(|(k, v)| (k.clone(), v.0 + v.1))
            .collect();
        ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        // Only retain the top 16 processes of cpu utilization.
        let mut processes_to_include: HashSet<String> =
            ranking.into_iter().take(16).map(|(name, _)| name).collect();

        for pid in &report_params.aperf_process_pids {
            let pid_prefix = format!("{pid}_");
            if let Some(aperf_process) = per_process_cpu_time
                .keys()
                .find(|name| name.starts_with(&pid_prefix))
            {
                processes_to_include.insert(aperf_process.clone());
            }
        }

        let number_processes_str = ProcessMetric::NumberProcesses.to_string();

        for (raw_value, number_processes) in kept_samples {
            time_series_data_processor.proceed_to_time(raw_value.time);
            time_series_data_processor.add_data_point(
                &number_processes_str,
                &number_processes_str,
                number_processes as f64,
            );

            for stat_line in ProcessStatLine::all(&raw_value.data) {
                let Some(process) = processes_to_include.get(stat_line.key().as_str()) else {
                    continue;
                };
                let values = stat_line.values();

                for process_metric in ProcessMetric::iter() {
                    let Some(value) =
                        get_process_metric_value(process_metric, &values, report_params.page_size)
                    else {
                        continue;
                    };
                    match process_metric {
                        ProcessMetric::UserSpaceTime | ProcessMetric::KernelSpaceTime => {
                            time_series_data_processor.add_accumulative_data_point(
                                &process_metric.to_string(),
                                process,
                                value / ticks_per_second,
                            )
                        }
                        _ => time_series_data_processor.add_data_point(
                            &process_metric.to_string(),
                            process,
                            value,
                        ),
                    };
                }
            }
        }

        let metric_order: Vec<String> = ProcessMetric::iter()
            .map(|process_metric| process_metric.to_string())
            .collect();
        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(
                metric_order.iter().map(String::as_str).collect(),
            );

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod process_test {
    #[cfg(target_os = "linux")]
    use {super::ProcessesRaw, crate::data::CollectData, crate::data_collection::InitParams};

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut processes = ProcessesRaw::new();
        let params = InitParams::default();
        processes.prepare_data_collector(&params).unwrap();
        processes.collect_data(&params).unwrap();
        assert!(!processes.data.is_empty());
    }
}
