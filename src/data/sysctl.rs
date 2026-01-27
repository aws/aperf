use crate::data::data_formats::{AperfData, KeyValueData, KeyValueGroup};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    sysctl::Sysctl,
};

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

    #[cfg(target_os = "linux")]
    fn add_ctl(&mut self, name: String, value: String) {
        self.sysctl_data.insert(name, value);
    }
}

#[cfg(target_os = "linux")]
const DONT_COLLECT: &[&str] = &["rss_key"];

#[cfg(target_os = "linux")]
fn can_collect(name: String) -> bool {
    for item in DONT_COLLECT {
        if name.contains(item) {
            return false;
        }
    }
    true
}

#[cfg(target_os = "linux")]
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
        Ok(())
    }

    fn is_static() -> bool {
        true
    }
}

impl ProcessData for SysctlData {
    fn process_raw_data(
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
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::{SysctlData, DONT_COLLECT},
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut sysctl = SysctlData::new();
        let params = CollectorParams::new();

        sysctl.collect_data(&params).unwrap();
        assert!(!sysctl.sysctl_data.is_empty());
    }

    #[cfg(target_os = "linux")]
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
}
