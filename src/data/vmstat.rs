extern crate ctor;

use crate::data::{CollectData, Data, ProcessedData, DataType, TimeEnum};
use crate::visualizer::{DataVisualizer, GetData};
use crate::{PDError, PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

pub static VMSTAT_FILE_NAME: &str = "vmstat";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VmstatRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl VmstatRaw {
    fn new() -> Self {
        VmstatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for VmstatRaw {
    fn collect_data(&mut self) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/vmstat")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vmstat {
    pub time: TimeEnum,
    pub vmstat_data: HashMap<String, i64>,
}

impl Vmstat {
    fn new() -> Self {
        Vmstat {
            time: TimeEnum::DateTime(Utc::now()),
            vmstat_data: HashMap::new(),
        }
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }

    fn set_data(&mut self, data: HashMap<String, i64>) {
        self.vmstat_data = data;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VmstatEntry {
    pub time: TimeEnum,
    pub value: i64,
}

fn get_entry(values: Vec<Vmstat>, key: String) -> Result<String> {
    let mut end_values = Vec::new();
    let time_zero = values[0].time;
    let mut prev_vmstat = values[0].clone();
    for value in values {
        let current_vmstat = value.clone();
        let current_time = current_vmstat.time;

        let curr_data = current_vmstat.vmstat_data.clone();
        let curr_value = curr_data
            .get(&key)
            .ok_or(PDError::VisualizerVmstatValueGetError(key.to_string()))?;
        let prev_data = prev_vmstat.vmstat_data.clone();
        let prev_value = prev_data
            .get(&key)
            .ok_or(PDError::VisualizerVmstatValueGetError(key.to_string()))?;

        let mut v = *curr_value;
        if !key.contains("nr_") {
            v = *curr_value - *prev_value;
        }
        let vmstat_entry = VmstatEntry {
            time: (current_time - time_zero),
            value: v,
        };
        end_values.push(vmstat_entry);
        prev_vmstat = value.clone();
    }
    Ok(serde_json::to_string(&end_values)?)
}

fn get_entries(value: Vmstat) -> Result<String> {
    let mut keys: Vec<String> = value.vmstat_data.into_keys().collect();
    keys.sort();
    Ok(serde_json::to_string(&keys)?)
}

impl GetData for Vmstat {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::VmstatRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let mut vmstat = Vmstat::new();
	let reader = BufReader::new(raw_value.data.as_bytes());
	let mut map: HashMap<String, i64> = HashMap::new();
	for line in reader.lines() {
            let line = line?;
            let mut split = line.split_whitespace();
            let name = split.next().ok_or(PDError::ProcessorOptionExtractError)?;
            let val = split.next().ok_or(PDError::ProcessorOptionExtractError)?;
	    map.insert(name.to_owned(), val.parse::<i64>()?);
	}
        vmstat.set_time(raw_value.time);
        vmstat.set_data(map);
        let processed_data = ProcessedData::Vmstat(vmstat);
        Ok(processed_data)
    }

    fn get_data(&mut self, buffer: Vec<ProcessedData>, query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::Vmstat(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "entries" => return get_entries(values[0].clone()),
            "values" => {
                let (_, key) = &param[2];
                return get_entry(values, key.to_string());
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_vmstat() {
    let vmstat_raw = VmstatRaw::new();
    let file_name = VMSTAT_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::VmstatRaw(vmstat_raw.clone()),
        file_name.clone(),
        false
    );
    let js_file_name = file_name.clone() + &".js".to_string();
    let vmstat = Vmstat::new();
    let dv = DataVisualizer::new(
        ProcessedData::Vmstat(vmstat.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/vmstat.js")).to_string(),
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
mod tests {
    use super::{Vmstat, VmstatEntry, VmstatRaw};
    use crate::data::{CollectData, Data, ProcessedData, TimeEnum};
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut vmstat = VmstatRaw::new();

        assert!(vmstat.collect_data().unwrap() == ());
        assert!(!vmstat.data.is_empty());
    }

    #[test]
    fn test_get_entries() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut vmstat = VmstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();

        assert!(vmstat.collect_data().unwrap() == ());
        buffer.push(Data::VmstatRaw(vmstat));
        processed_buffer.push(Vmstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Vmstat::new().get_data(processed_buffer, "run=test&get=entries".to_string()).unwrap();
        let values: Vec<&str> = serde_json::from_str(&json).unwrap();
        assert!(values.len() > 0);
    }

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut vmstat = VmstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();

        assert!(vmstat.collect_data().unwrap() == ());
        buffer.push(Data::VmstatRaw(vmstat));
        processed_buffer.push(Vmstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Vmstat::new().get_data(processed_buffer, "run=test&get=values&key=nr_dirty".to_string()).unwrap();
        let values: Vec<VmstatEntry> = serde_json::from_str(&json).unwrap();
        assert!(values.len() > 0);
        match values[0].time {
            TimeEnum::TimeDiff(value) => assert!(value == 0),
            _ => assert!(false),
        }
    }
}
