extern crate ctor;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData, TimeEnum};
use crate::utils::DataMetrics;
use crate::visualizer::{DataVisualizer, GetData, GraphLimitType, GraphMetadata};
use crate::{PDError, PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

pub static NETSTAT_FILE_NAME: &str = "netstat";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetstatRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl NetstatRaw {
    fn new() -> Self {
        NetstatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for NetstatRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/net/netstat")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Netstat {
    pub time: TimeEnum,
    pub netstat_data: HashMap<String, u64>,
}

impl Netstat {
    fn new() -> Self {
        Netstat {
            time: TimeEnum::DateTime(Utc::now()),
            netstat_data: HashMap::new(),
        }
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }

    fn set_data(&mut self, data: HashMap<String, u64>) {
        self.netstat_data = data;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NetstatEntry {
    pub time: TimeEnum,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EndNetData {
    pub data: Vec<NetstatEntry>,
    pub metadata: GraphMetadata,
}

fn get_entry(values: Vec<Netstat>, key: String) -> Result<String> {
    let mut end_values = Vec::new();
    let mut metadata = GraphMetadata::new();
    let time_zero = values[0].time;
    let mut prev_netstat = values[0].clone();
    for value in values {
        let current_netstat = value.clone();
        let current_time = current_netstat.time;

        let curr_data = current_netstat.netstat_data.clone();
        let curr_value = curr_data
            .get(&key)
            .ok_or(PDError::VisualizerNetstatValueGetError(key.to_string()))?;
        let prev_data = prev_netstat.netstat_data.clone();
        let prev_value = prev_data
            .get(&key)
            .ok_or(PDError::VisualizerNetstatValueGetError(key.to_string()))?;

        let netstat_entry = NetstatEntry {
            time: (current_time - time_zero),
            value: *curr_value - *prev_value,
        };
        metadata.update_limits(GraphLimitType::UInt64(netstat_entry.value));
        end_values.push(netstat_entry);
        prev_netstat = value.clone();
    }
    let netdata = EndNetData {
        data: end_values,
        metadata,
    };
    Ok(serde_json::to_string(&netdata)?)
}

fn get_entries(value: Netstat) -> Result<String> {
    let mut keys: Vec<String> = value.netstat_data.into_keys().collect();
    keys.sort();
    Ok(serde_json::to_string(&keys)?)
}

impl GetData for Netstat {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::NetstatRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let mut netstat = Netstat::new();
        let reader = BufReader::new(raw_value.data.as_bytes());
        let mut map: HashMap<String, u64> = HashMap::new();
        let mut lines = reader.lines();

        while let (Some(line1), Some(line2)) = (lines.next(), lines.next()) {
            let binding = line1.unwrap();
            let params: Vec<&str> = binding.split_whitespace().collect();

            let binding = line2.unwrap();
            let values: Vec<&str> = binding.split_whitespace().collect();

            if params.len() != values.len() {
                panic!("Parameter count should match value count!")
            }

            let mut param_itr = params.iter();
            let mut val_itr = values.iter();

            let tag = param_itr.next().unwrap().to_owned();
            val_itr.next();

            for param in param_itr {
                let val = val_itr.next().ok_or(PDError::ProcessorOptionExtractError)?;
                map.insert(tag.to_owned() + " " + param.to_owned(), val.parse::<u64>()?);
            }
        }

        netstat.set_time(raw_value.time);
        netstat.set_data(map);
        let processed_data = ProcessedData::Netstat(netstat);
        Ok(processed_data)
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
                ProcessedData::Netstat(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => get_entries(values[0].clone()),
            "values" => {
                let (_, key) = &param[2];
                get_entry(values, key.to_string())
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_netstat() {
    let netstat_raw = NetstatRaw::new();
    let file_name = NETSTAT_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::NetstatRaw(netstat_raw.clone()),
        file_name.clone(),
        false,
    );
    let js_file_name = file_name.clone() + ".js";
    let netstat = Netstat::new();
    let dv = DataVisualizer::new(
        ProcessedData::Netstat(netstat.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/netstat.js")).to_string(),
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
    use super::{EndNetData, Netstat, NetstatRaw};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut netstat = NetstatRaw::new();
        let params = CollectorParams::new();

        netstat.collect_data(&params).unwrap();
        assert!(!netstat.data.is_empty());
    }

    #[test]
    fn test_get_entries() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut netstat = NetstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        netstat.collect_data(&params).unwrap();
        buffer.push(Data::NetstatRaw(netstat));
        processed_buffer.push(Netstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Netstat::new()
            .get_data(
                processed_buffer,
                "run=test&get=keys".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<&str> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut netstat = NetstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        netstat.collect_data(&params).unwrap();
        buffer.push(Data::NetstatRaw(netstat));
        processed_buffer.push(Netstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Netstat::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&key=TcpExt: TCPDSACKRecv".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let data: EndNetData = serde_json::from_str(&json).unwrap();
        assert!(!data.data.is_empty());
        match data.data[0].time {
            TimeEnum::TimeDiff(value) => assert!(value == 0),
            _ => unreachable!(),
        }
    }
}
