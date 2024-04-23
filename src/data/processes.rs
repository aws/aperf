extern crate ctor;
extern crate lazy_static;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData, TimeEnum};
use crate::visualizer::{DataVisualizer, GetData};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::sync::Mutex;

pub static PROCESS_FILE_NAME: &str = "processes";
pub static PROC_PID_STAT_USERSPACE_TIME_POS: usize = 11;
pub static PROC_PID_STAT_KERNELSPACE_TIME_POS: usize = 12;

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
    fn prepare_data_collector(&mut self, _params: CollectorParams) -> Result<()> {
        *TICKS_PER_SECOND.lock().unwrap() = procfs::ticks_per_second()? as u64;
        Ok(())
    }

    fn collect_data(&mut self) -> Result<()> {
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
    fn new() -> Self {
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

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(&mut self, buffer: Vec<ProcessedData>, query: String) -> Result<String> {
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

#[ctor]
fn init_system_processes() {
    let processes_raw = ProcessesRaw::new();
    let file_name = PROCESS_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::ProcessesRaw(processes_raw.clone()),
        file_name.clone(),
        false,
    );
    let js_file_name = file_name.clone() + ".js";
    let processes = Processes::new();
    let dv = DataVisualizer::new(
        ProcessedData::Processes(processes),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/processes.js")).to_string(),
        file_name.clone(),
    );

    PERFORMANCE_DATA
        .lock()
        .unwrap()
        .add_datatype(file_name.clone(), dt);

    VISUALIZATION_DATA
        .lock()
        .unwrap()
        .add_visualizer(file_name.clone(), dv);
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
        processes.prepare_data_collector(params).unwrap();
        processes.collect_data().unwrap();
        assert!(!processes.data.is_empty());
    }

    #[test]
    fn test_process_raw_data() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut processes_zero = ProcessesRaw::new();
        let mut processes_one = ProcessesRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        processes_zero
            .prepare_data_collector(params.clone())
            .unwrap();
        processes_one.prepare_data_collector(params).unwrap();
        processes_zero.collect_data().unwrap();
        processes_one.collect_data().unwrap();

        buffer.push(Data::ProcessesRaw(processes_zero));
        buffer.push(Data::ProcessesRaw(processes_one));
        for buf in buffer {
            processed_buffer.push(Processes::new().process_raw_data(buf).unwrap());
        }
        assert!(!processed_buffer.is_empty(), "{:#?}", processed_buffer);
    }
}
