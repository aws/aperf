extern crate ctor;

use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::visualizer::{DataVisualizer, GetData};
use crate::{PDError, PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use procfs::vmstat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub static VMSTAT_FILE_NAME: &str = "vmstat";

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

impl CollectData for Vmstat {
    fn collect_data(&mut self) -> Result<()> {
        let time_now = Utc::now();
        let vmstat_data = vmstat().unwrap();

        self.set_time(TimeEnum::DateTime(time_now));
        self.set_data(vmstat_data);
        debug!("Vmstat data: {:#?}", self);
        Ok(())
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
    for value in values {
        let current_vmstat = value;
        let current_time = current_vmstat.time;

        let curr_data = current_vmstat.vmstat_data.clone();
        let curr_value = curr_data
            .get(&key)
            .ok_or(PDError::VisualizerVmstatValueGetError(key.to_string()))?;

        let vmstat_entry = VmstatEntry {
            time: (current_time - time_zero),
            value: *curr_value,
        };
        end_values.push(vmstat_entry);
    }
    Ok(serde_json::to_string(&end_values)?)
}

fn get_entries(value: Vmstat) -> Result<String> {
    let mut keys: Vec<String> = value.vmstat_data.into_keys().collect();
    keys.sort();
    Ok(serde_json::to_string(&keys)?)
}

impl GetData for Vmstat {
    fn get_data(&mut self, buffer: Vec<Data>, query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                Data::Vmstat(ref value) => values.push(value.clone()),
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
    let vmstat = Vmstat::new();
    let file_name = VMSTAT_FILE_NAME.to_string();
    let dt = DataType::new(Data::Vmstat(vmstat.clone()), file_name.clone(), false);
    let dv = DataVisualizer::new(
        Data::Vmstat(vmstat.clone()),
        file_name.clone(),
        file_name.clone() + &".js".to_string(),
        include_str!("../bin/html_files/js/vmstat.js").to_string(),
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
    use super::{Vmstat, VmstatEntry};
    use crate::data::{CollectData, Data, TimeEnum};
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut vmstat = Vmstat::new();

        assert!(vmstat.collect_data().unwrap() == ());
        assert!(vmstat.vmstat_data.len() != 0);
    }

    #[test]
    fn test_get_entries() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut vmstat = Vmstat::new();

        assert!(vmstat.collect_data().unwrap() == ());
        buffer.push(Data::Vmstat(vmstat));
        let json = Vmstat::new().get_data(buffer, "run=test&get=entries".to_string()).unwrap();
        let values: Vec<&str> = serde_json::from_str(&json).unwrap();
        assert!(values.len() > 0);
    }

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut vmstat = Vmstat::new();

        assert!(vmstat.collect_data().unwrap() == ());
        buffer.push(Data::Vmstat(vmstat));
        let json = Vmstat::new().get_data(buffer, "run=test&get=values&key=nr_dirty".to_string()).unwrap();
        let values: Vec<VmstatEntry> = serde_json::from_str(&json).unwrap();
        assert!(values.len() > 0);
        match values[0].time {
            TimeEnum::TimeDiff(value) => assert!(value == 0),
            _ => assert!(false),
        }
    }
}
