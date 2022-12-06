extern crate ctor;

use anyhow::{Result, bail};
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA, PDError};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sysctl::Sysctl;

pub static SYSCTL_FILE_NAME: &str = "sysctl";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SysctlData {
   pub time: TimeEnum,
   pub sysctl_data: BTreeMap<String, String>,
}

impl SysctlData {
    fn new() -> Self {
        SysctlData {
            time: TimeEnum::DateTime(Utc::now()),
            sysctl_data: BTreeMap::new(),
        }
    }

    fn add_ctl(&mut self, name: String, value: String) {
        self.sysctl_data.insert(name, value);
    }
}

const DONT_COLLECT: &[&str] = &[
    "rss_key",
];

fn can_collect(name: String) -> bool {
    for item in DONT_COLLECT {
        if name.contains(item) {
            return false;
        }
    }
    return true;
}

impl CollectData for SysctlData {
    fn collect_data(&mut self) -> Result<()> {
        let ctls = sysctl::CtlIter::root().filter_map(Result::ok);
        for ctl in ctls {
            let flags = match ctl.flags() {
                Ok(f) => f,
                Err(_) => continue,
            };
            if !flags.contains(sysctl::CtlFlags::SKIP) && can_collect(ctl.name()?) {
                self.add_ctl(ctl.name()?, ctl.value_string()?);
            }
        }
        debug!("{:#?}", self.sysctl_data);
        Ok(())
    }
}

fn get_sysctl_data(value: SysctlData) -> Result<String> {
    Ok(serde_json::to_string(&value.sysctl_data)?)
}

impl GetData for SysctlData {
    fn get_data(&mut self, buffer: Vec<Data>, query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                Data::SysctlData(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        if param.len() < 2 {
            return Ok("Not enough parameters".to_string());
        }
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "values" => return get_sysctl_data(values[0].clone()),
            _ => bail!(PDError::VisualizerUnsupportedAPI),
        }
    }
}

#[ctor]
fn init_sysctl() {
    let sysctl_data = SysctlData::new();
    let file_name = SYSCTL_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::SysctlData(sysctl_data.clone()),
        file_name.clone(),
        true
    );
    let js_file_name = file_name.clone() + &".js".to_string();
    let dv = DataVisualizer::new(
        Data::SysctlData(sysctl_data),
        file_name.clone(),
        js_file_name,
        include_str!("../bin/html_files/js/sysctl.js").to_string(),
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
    use super::{SysctlData, DONT_COLLECT};
    use crate::data::{CollectData, Data};
    use crate::visualizer::GetData;
    use std::collections::BTreeMap;

    #[test]
    fn test_collect_data() {
        let mut sysctl = SysctlData::new();

        assert!(sysctl.collect_data().unwrap() == ());
        assert!(sysctl.sysctl_data.len() != 0);
    }

    #[test]
    fn test_dont_collect() {
        let mut sysctl = SysctlData::new();

        sysctl.collect_data().unwrap();
        let keys: Vec<String> = sysctl.sysctl_data.keys().cloned().collect();
        for key in keys {
            for item in DONT_COLLECT {
                if key.contains(item) {
                    assert!(false, "Should not collect: {}", key);
                }
            }
        }
    }

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut sysctl = SysctlData::new();

        sysctl.collect_data().unwrap();
        buffer.push(Data::SysctlData(sysctl));
        let json = SysctlData::new().get_data(buffer, "run=test&get=values".to_string()).unwrap();
        let values: BTreeMap<String, String> = serde_json::from_str(&json).unwrap();
        assert!(values.len() != 0);
    }
}
