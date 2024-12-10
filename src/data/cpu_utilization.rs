extern crate ctor;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData, TimeEnum};
use crate::utils::DataMetrics;
use crate::visualizer::{DataVisualizer, GetData};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::trace;
use procfs::{CpuTime, KernelStats};
use serde::{Deserialize, Serialize};
use std::ops::Sub;

pub static CPU_UTILIZATION_FILE_NAME: &str = "cpu_utilization";

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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct UtilValues {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub irq: u64,
    pub softirq: u64,
    pub idle: u64,
    pub iowait: u64,
    pub steal: u64,
}

impl UtilValues {
    fn new() -> Self {
        UtilValues {
            user: 0,
            nice: 0,
            system: 0,
            irq: 0,
            softirq: 0,
            idle: 0,
            iowait: 0,
            steal: 0,
        }
    }

    fn is_less_than(self, other: UtilValues) -> bool {
        !(self.user >= other.user
            && self.nice >= other.nice
            && self.system >= other.system
            && self.irq >= other.irq
            && self.softirq >= other.softirq
            && self.idle >= other.idle
            && self.iowait >= other.iowait
            && self.steal >= other.steal)
    }
}

impl Sub for UtilValues {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            user: self.user - other.user,
            nice: self.nice - other.nice,
            system: self.system - other.system,
            irq: self.irq - other.irq,
            softirq: self.softirq - other.softirq,
            idle: self.idle - other.idle,
            iowait: self.iowait - other.iowait,
            steal: self.steal - other.steal,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CpuData {
    pub time: TimeEnum,
    pub cpu: i64,
    pub values: UtilValues,
}

impl CpuData {
    fn new() -> Self {
        CpuData {
            time: TimeEnum::DateTime(Utc::now()),
            cpu: 0,
            values: UtilValues::new(),
        }
    }

    fn set_data(&mut self, cpu: i64, cpu_time: &CpuTime) {
        self.cpu = cpu;
        self.values.user = cpu_time.user;
        self.values.nice = cpu_time.nice;
        self.values.system = cpu_time.system;
        self.values.irq = cpu_time.irq.unwrap_or_default();
        self.values.softirq = cpu_time.softirq.unwrap_or_default();
        self.values.idle = cpu_time.idle;
        self.values.iowait = cpu_time.iowait.unwrap_or_default();
        self.values.steal = cpu_time.steal.unwrap_or_default();
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpuUtilization {
    pub total: CpuData,
    pub per_cpu: Vec<CpuData>,
}

impl Default for CpuUtilization {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuUtilization {
    pub fn new() -> Self {
        CpuUtilization {
            total: CpuData::new(),
            per_cpu: Vec::<CpuData>::new(),
        }
    }

    fn set_total(&mut self, cpu: i64, total: CpuTime) {
        self.total.set_data(cpu, &total);
    }

    fn set_total_time(&mut self, time: DateTime<Utc>) {
        self.total.set_time(TimeEnum::DateTime(time));
    }

    fn add_per_cpu_data(&mut self, cpu_data: CpuData) {
        self.per_cpu.push(cpu_data);
    }
}

/// Process gathered raw data during visualization.
fn process_gathered_raw_data(buffer: Data) -> Result<ProcessedData> {
    let raw_value = match buffer {
        Data::CpuUtilizationRaw(ref value) => value,
        _ => panic!("Invalid Data type in raw file"),
    };
    let stat = KernelStats::from_reader(raw_value.data.as_bytes()).unwrap();
    let mut cpu_utilization = CpuUtilization::new();
    let time_now = match raw_value.time {
        TimeEnum::DateTime(value) => value,
        _ => panic!("Has to be datetime"),
    };

    /* Get total numbers */
    cpu_utilization.set_total(-1, stat.total);
    cpu_utilization.set_total_time(time_now);

    /* Get per_cpu numbers */
    for (i, cpu) in stat.cpu_time.iter().enumerate() {
        let mut current_cpu_data = CpuData::new();

        /* Set this CPU's data */
        current_cpu_data.set_data(i as i64, cpu);
        current_cpu_data.set_time(TimeEnum::DateTime(time_now));

        /* Push to Vec of per_cpu data */
        cpu_utilization.add_per_cpu_data(current_cpu_data);
    }
    let data = ProcessedData::CpuUtilization(cpu_utilization.clone());
    Ok(data)
}

/// Visualize processed data.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimeData {
    pub time: TimeEnum,
    pub value: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct UtilData {
    pub cpu: i64,
    pub data: Vec<TimeData>,
}

fn percentage(value: u64, total: u64) -> u64 {
    if total > 0 {
        return ((value as f64 / total as f64) * 100.0) as u64;
    }
    0
}

fn set_as_percent(value: UtilValues) -> UtilValues {
    let total = value.user
        + value.nice
        + value.system
        + value.irq
        + value.softirq
        + value.idle
        + value.iowait
        + value.steal;

    let mut new_values = UtilValues::new();
    new_values.user = percentage(value.user, total);
    new_values.nice = percentage(value.nice, total);
    new_values.system = percentage(value.system, total);
    new_values.irq = percentage(value.irq, total);
    new_values.softirq = percentage(value.softirq, total);
    new_values.idle = percentage(value.idle, total);
    new_values.iowait = percentage(value.iowait, total);
    new_values.steal = percentage(value.steal, total);
    new_values
}

pub fn get_aggregate_data(values: Vec<CpuData>) -> Result<String> {
    let mut end_values = Vec::new();
    let mut prev_cpu_data = values[0].values;
    let time_zero = values[0].time;
    for value in values {
        let mut end_value = CpuData::new();
        let current_cpu_data = value.values;
        let current_time = value.time;

        if current_cpu_data.is_less_than(prev_cpu_data) {
            prev_cpu_data = current_cpu_data;
            continue;
        }
        end_value.cpu = value.cpu;
        end_value.values = set_as_percent(current_cpu_data - prev_cpu_data);
        end_value.set_time(current_time - time_zero);
        end_values.push(end_value);
        prev_cpu_data = current_cpu_data;
    }
    Ok(serde_json::to_string(&end_values)?)
}

pub fn get_cpu_values(cpu: i64, values: Vec<CpuUtilization>) -> Vec<CpuData> {
    let mut end_values = Vec::new();
    for value in values {
        for per_cpu_value in value.per_cpu {
            if cpu == per_cpu_value.cpu {
                end_values.push(per_cpu_value);
                break;
            }
        }
    }
    end_values
}

fn get_type(count: u64, values: Vec<CpuUtilization>, util_type: &str) -> Result<String> {
    let mut end_values = Vec::new();
    for i in 0..count {
        let mut util_data = UtilData {
            cpu: (i as i64),
            data: Vec::new(),
        };

        /* Get cpu 'i' values */
        let cpu_values = get_cpu_values(i as i64, values.clone());
        let time_zero = cpu_values[0].time;
        let mut prev_cpu_data = cpu_values[0].values;
        for value in cpu_values {
            let mut end_value = CpuData::new();
            let current_time = value.time;
            let current_cpu_data = value.values;

            if current_cpu_data.is_less_than(prev_cpu_data) {
                prev_cpu_data = current_cpu_data;
                continue;
            }
            end_value.cpu = value.cpu;
            end_value.values = set_as_percent(current_cpu_data - prev_cpu_data);
            end_value.set_time(current_time - time_zero);

            let value = match util_type {
                "user" => end_value.values.user,
                "nice" => end_value.values.nice,
                "system" => end_value.values.system,
                "irq" => end_value.values.irq,
                "softirq" => end_value.values.softirq,
                "idle" => end_value.values.idle,
                "iowait" => end_value.values.iowait,
                "steal" => end_value.values.steal,
                _ => panic!("Invalid util type"),
            };
            let time_data = TimeData {
                time: end_value.time,
                value,
            };
            util_data.data.push(time_data);
            prev_cpu_data = current_cpu_data;
        }
        end_values.push(util_data);
    }
    Ok(serde_json::to_string(&end_values)?)
}

impl GetData for CpuUtilization {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        process_gathered_raw_data(buffer)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["keys".to_string(), "values".to_string()])
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
                ProcessedData::CpuUtilization(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => {
                let end_values = [
                    "aggregate",
                    "user",
                    "nice",
                    "system",
                    "irq",
                    "softirq",
                    "idle",
                    "iowait",
                    "steal",
                ];
                Ok(serde_json::to_string(&end_values)?)
            }
            "values" => {
                let (_, key) = &param[2];
                if key == "aggregate" {
                    let mut temp_values = Vec::new();
                    for value in values {
                        temp_values.push(value.total);
                    }
                    get_aggregate_data(temp_values)
                } else {
                    get_type(values[0].per_cpu.len() as u64, values, key)
                }
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_cpu_utilization() {
    let cpu_utilization_raw = CpuUtilizationRaw::new();
    let file_name = CPU_UTILIZATION_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::CpuUtilizationRaw(cpu_utilization_raw.clone()),
        file_name.clone(),
        false,
    );
    let js_file_name = file_name.clone() + ".js";
    let cpu_utilization = CpuUtilization::new();
    let dv = DataVisualizer::new(
        ProcessedData::CpuUtilization(cpu_utilization),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/cpu_utilization.js")).to_string(),
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
mod cpu_tests {
    use super::{CpuData, CpuUtilization, CpuUtilizationRaw, UtilData};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut cpu_utilization = CpuUtilizationRaw::new();
        let params = CollectorParams::new();

        cpu_utilization.collect_data(&params).unwrap();
        assert!(!cpu_utilization.data.is_empty());
    }

    #[test]
    fn test_get_data_aggregate_cpu() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut cpu_util_zero = CpuUtilizationRaw::new();
        let mut cpu_util_one = CpuUtilizationRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        cpu_util_zero.collect_data(&params).unwrap();
        cpu_util_one.collect_data(&params).unwrap();

        buffer.push(Data::CpuUtilizationRaw(cpu_util_zero));
        buffer.push(Data::CpuUtilizationRaw(cpu_util_one));
        for buf in buffer {
            processed_buffer.push(CpuUtilization::new().process_raw_data(buf).unwrap());
        }
        let json = CpuUtilization::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&=aggregate".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<CpuData> = serde_json::from_str(&json).unwrap();
        assert!(values[0].cpu == -1);
    }

    #[test]
    fn test_get_util_types() {
        let types = CpuUtilization::new()
            .get_data(
                Vec::new(),
                "run=test&get=keys".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<&str> = serde_json::from_str(&types).unwrap();
        for type_str in values {
            match type_str {
                "aggregate" | "user" | "nice" | "system" | "irq" | "softirq" | "idle"
                | "iowait" | "steal" => {}
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_get_user() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut cpu_util_zero = CpuUtilizationRaw::new();
        let mut cpu_util_one = CpuUtilizationRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        cpu_util_zero.collect_data(&params).unwrap();
        cpu_util_one.collect_data(&params).unwrap();

        buffer.push(Data::CpuUtilizationRaw(cpu_util_zero));
        buffer.push(Data::CpuUtilizationRaw(cpu_util_one));
        for buf in buffer {
            processed_buffer.push(CpuUtilization::new().process_raw_data(buf).unwrap());
        }
        let json = CpuUtilization::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&key=user".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<UtilData> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }
}
