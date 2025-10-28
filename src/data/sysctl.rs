use crate::data::data_formats::{AperfData, KeyValueData, KeyValueGroup};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::DataMetrics;
use crate::visualizer::{GetData, ReportParams};
use anyhow::Result;
use chrono::prelude::*;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sysctl::Sysctl;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SysctlData {
    pub time: TimeEnum,
    pub sysctl_data: BTreeMap<String, String>,
}

impl SysctlData {
    pub fn new() -> Self {
        SysctlData {
            time: TimeEnum::DateTime(Utc::now()),
            sysctl_data: BTreeMap::new(),
        }
    }

    fn add_ctl(&mut self, name: String, value: String) {
        self.sysctl_data.insert(name, value);
    }
}

const DONT_COLLECT: &[&str] = &["rss_key"];

fn can_collect(name: String) -> bool {
    for item in DONT_COLLECT {
        if name.contains(item) {
            return false;
        }
    }
    true
}

impl CollectData for SysctlData {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        let ctls = sysctl::CtlIter::root().filter_map(Result::ok);
        for ctl in ctls {
            let flags = match ctl.flags() {
                Ok(f) => f,
                Err(_) => continue,
            };
            if !flags.contains(sysctl::CtlFlags::SKIP) && can_collect(ctl.name()?) {
                let name = match ctl.name() {
                    Ok(s) => s,
                    _ => continue,
                };
                let value = match ctl.value_string() {
                    Ok(s) => s,
                    _ => continue,
                };
                self.add_ctl(name, value);
            }
        }
        trace!("{:#?}", self.sysctl_data);
        Ok(())
    }

    fn is_static() -> bool {
        true
    }
}

fn get_sysctl_data(value: SysctlData) -> Result<String> {
    Ok(serde_json::to_string(&value.sysctl_data)?)
}

impl GetData for SysctlData {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::SysctlData(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let processed_data = ProcessedData::SysctlData((*raw_value).clone());
        Ok(processed_data)
    }

    fn process_raw_data_new(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut key_value_data = KeyValueData::default();

        key_value_data
            .key_value_groups
            .insert("".to_string(), KeyValueGroup::default());
        let key_value_map = &mut key_value_data
            .key_value_groups
            .get_mut("")
            .unwrap()
            .key_values;

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::SysctlData(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            //add to key value data into default group
            for (key, value) in &raw_value.sysctl_data {
                key_value_map.insert(key.clone(), value.clone());
            }
        }

        Ok(AperfData::KeyValue(key_value_data))
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
                ProcessedData::SysctlData(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        if param.len() < 2 {
            return Ok("Not enough parameters".to_string());
        }
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "values" => get_sysctl_data(values[0].clone()),
            _ => panic!("Unsupported API"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SysctlData, DONT_COLLECT};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;
    use std::collections::BTreeMap;

    #[test]
    fn test_collect_data() {
        let mut sysctl = SysctlData::new();
        let params = CollectorParams::new();

        sysctl.collect_data(&params).unwrap();
        assert!(!sysctl.sysctl_data.is_empty());
    }

    #[test]
    fn test_dont_collect() {
        let mut sysctl = SysctlData::new();
        let params = CollectorParams::new();

        sysctl.collect_data(&params).unwrap();
        let keys: Vec<String> = sysctl.sysctl_data.keys().cloned().collect();
        for key in keys {
            for item in DONT_COLLECT {
                if key.contains(item) {
                    unreachable!("Should not collect: {}", key);
                }
            }
        }
    }

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut sysctl = SysctlData::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        sysctl.collect_data(&params).unwrap();
        buffer.push(Data::SysctlData(sysctl));
        processed_buffer.push(
            SysctlData::new()
                .process_raw_data(buffer[0].clone())
                .unwrap(),
        );
        let json = SysctlData::new()
            .get_data(
                processed_buffer,
                "run=test&get=values".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: BTreeMap<String, String> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }
}
