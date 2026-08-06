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
use std::collections::HashMap;
use strum::IntoEnumIterator;
#[cfg(target_os = "linux")]
use {crate::data::CollectData, crate::data_collection::InitParams, chrono::Utc, std::fs};

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
                if let Ok(v) = fs::read_to_string(path) {
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

fn get_process_metric_value(
    process_metric: ProcessMetric,
    values: &[String],
    page_size: u64,
) -> Option<f64> {
    // The last element we access is the 22nd element in a values vector (ResidentSetSize), make sure the index 21 exists
    if values.len() < 21 + 1 {
        warn!("Incomplete proc/<PID>/stat entry found, skipping...");
        return None;
    }
    let result = match process_metric {
        ProcessMetric::UserSpaceTime => values[11].parse::<u64>().ok()?,
        ProcessMetric::KernelSpaceTime => values[12].parse::<u64>().ok()?,
        ProcessMetric::NumberThreads => values[17].parse::<u64>().ok()?,
        ProcessMetric::VirtualMemorySize => values[20].parse::<u64>().ok()?,
        ProcessMetric::ResidentSetSize => values[21].parse::<u64>().ok()?,
        ProcessMetric::ResidentSetSizeBytes => {
            if page_size == 0 {
                return None;
            }
            values[21].parse::<u64>().ok()? * page_size
        }
        ProcessMetric::NumberProcesses => return None,
    };
    Some(result as f64)
}

impl ProcessData for Processes {
    fn process_raw_data(
        &mut self,
        report_params: &ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor =
            time_series_data_processor_with_max_series_aggregate!(report_params.collection_start);

        // For each timestamp, it stores all parsed processes data in the format of
        // Map<pid_name, parsed_data>.
        let mut parsed_data: Vec<(TimeEnum, HashMap<String, Vec<String>>)> = Vec::new();
        // Track per process cpu time to filter out the top ones to retain, in the
        // format of Map<pid_name, (utime, stime)>.
        let mut per_process_cpu_time: HashMap<String, (f64, f64)> = HashMap::new();

        let mut ticks_per_second_option: Option<f64> = None;

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::ProcessesRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            // If multiple data were added at the same time diff, only keep the last one
            // Since processes data is collected once again at the end of collection,
            // this could happen if the finish stage completed fast.
            if let Some((last_parsed_time, _)) = parsed_data.last() {
                if raw_value.time - *last_parsed_time == TimeEnum::TimeDiff(0) {
                    parsed_data.pop();
                }
            }

            ticks_per_second_option.get_or_insert(raw_value.ticks_per_second as f64);

            let mut cur_parsed_data: HashMap<String, Vec<String>> = HashMap::new();

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
                let values: Vec<String> = line[close_pos + 2..]
                    .split_whitespace()
                    .map(String::from)
                    .collect();

                let process_pid_name = format!("{}_{}", pid, name);

                let (utime, stime) = match (
                    get_process_metric_value(
                        ProcessMetric::UserSpaceTime,
                        &values,
                        report_params.page_size,
                    ),
                    get_process_metric_value(
                        ProcessMetric::KernelSpaceTime,
                        &values,
                        report_params.page_size,
                    ),
                ) {
                    (Some(utime), Some(stime)) => (utime, stime),
                    _ => continue,
                };

                if let Some((max_utime, max_stime)) =
                    per_process_cpu_time.get_mut(&process_pid_name)
                {
                    *max_utime = max_utime.max(utime);
                    *max_stime = max_stime.max(stime);
                } else {
                    per_process_cpu_time.insert(process_pid_name.clone(), (utime, stime));
                }

                cur_parsed_data.insert(process_pid_name, values);
            }

            parsed_data.push((raw_value.time.clone(), cur_parsed_data));
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
        let mut processes_to_include: Vec<String> =
            ranking.into_iter().take(16).map(|(name, _)| name).collect();

        for pid in &report_params.aperf_process_pids {
            let pid_prefix = format!("{pid}_");
            if let Some(aperf_process) = per_process_cpu_time
                .keys()
                .find(|name| name.starts_with(&pid_prefix))
            {
                if !processes_to_include.contains(aperf_process) {
                    processes_to_include.push(aperf_process.clone());
                }
            }
        }

        for (time, data) in parsed_data {
            time_series_data_processor.proceed_to_time(time);

            let number_processes_str = ProcessMetric::NumberProcesses.to_string();
            time_series_data_processor.add_data_point(
                &number_processes_str,
                &number_processes_str,
                data.len() as f64,
            );

            for process in &processes_to_include {
                let values = match data.get(process) {
                    Some(values) => values,
                    None => continue,
                };
                for process_metric in ProcessMetric::iter() {
                    let value = match get_process_metric_value(
                        process_metric,
                        values,
                        report_params.page_size,
                    ) {
                        Some(value) => value,
                        None => continue,
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
