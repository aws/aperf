use crate::data::{CollectData, Data, ProcessedData, TimeEnum};
use crate::utils::DataMetrics;
use crate::visualizer::GetData;
use crate::PDError;
use anyhow::Result;
use chrono::prelude::*;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use super::CollectorParams;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Entry {
    ConfigEntry(KernelConfigEntry),
    ConfigGroup(KernelConfigEntryGroup),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelConfigEntry {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelConfigEntryGroup {
    pub name: String,
    pub entries: Vec<Entry>,
}

impl KernelConfigEntryGroup {
    fn new() -> Self {
        KernelConfigEntryGroup {
            name: String::new(),
            entries: Vec::new(),
        }
    }

    fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelConfig {
    pub time: TimeEnum,
    pub kernel_config_data: Vec<KernelConfigEntryGroup>,
}

impl KernelConfig {
    pub fn new() -> Self {
        KernelConfig {
            time: TimeEnum::DateTime(Utc::now()),
            kernel_config_data: Vec::new(),
        }
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }

    fn set_data(&mut self, data: Vec<KernelConfigEntryGroup>) {
        self.kernel_config_data = data;
    }
}

fn get_kernel_config_data() -> Result<Box<dyn BufRead>> {
    /* This is the same as procfs crate. We need access to the commented out CONFIGs and
     * headings in the Config file.
     */
    let mut conf = format!(
        "/boot/config-{}",
        rustix::system::uname().release().to_string_lossy()
    );
    let reader: Box<dyn BufRead> = {
        if !Path::new(&conf).exists() {
            conf = "/boot/config".to_string();
        }
        match OpenOptions::new().read(true).open(&conf) {
            Ok(file) => Box::new(BufReader::new(file)),
            Err(e) => {
                debug!("Error: {} when opening {}", e, conf);
                Box::new(io::Cursor::new(b"KERNEL_CONFIG=NOT FOUND"))
            }
        }
    };
    Ok(reader)
}

impl CollectData for KernelConfig {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        let time_now = Utc::now();
        let mut kernel_data_processed: Vec<KernelConfigEntryGroup> = Vec::new();
        let mut comments = Vec::new();

        /* Get kernel config data from file */
        let kernel_data = get_kernel_config_data()?;

        let mut first_group = KernelConfigEntryGroup::new();
        first_group.name = "".to_string();
        kernel_data_processed.push(first_group);

        for line in kernel_data.lines() {
            let line = line?;
            if line.starts_with('#')
                && !line.contains("is not set")
                && !line.contains("NOTE")
                && !line.contains("also be needed")
                && !line.contains("end of")
            {
                comments.push(line);
                continue;
            } else {
                if comments.len() == 3 {
                    let mut group = KernelConfigEntryGroup::new();
                    group.name = comments[1].clone()[2..].to_string();
                    kernel_data_processed.push(group.clone());
                }
                comments.clear();
            }
            if line.contains('=') {
                let mut s = line.splitn(2, '=');
                let name = s.next().ok_or(PDError::CollectorLineNameError)?.to_owned();
                let value = s.next().ok_or(PDError::CollectorLineValueError)?;
                let entry = KernelConfigEntry {
                    name: name.clone(),
                    value: value.to_string(),
                };
                kernel_data_processed
                    .last_mut()
                    .unwrap()
                    .add_entry(Entry::ConfigEntry(entry));
                comments.clear();
            }
            if line.contains("is not set") {
                let mut s = line.splitn(3, ' ');
                s.next();
                let name = s.next().ok_or(PDError::CollectorLineNameError)?.to_owned();
                let value = "not set";
                let entry = KernelConfigEntry {
                    name: name.clone(),
                    value: value.to_string(),
                };
                kernel_data_processed
                    .last_mut()
                    .unwrap()
                    .add_entry(Entry::ConfigEntry(entry));
                comments.clear();
            }
            if line.contains("end of") {
                let s = line.splitn(4, ' ');
                let name = s.last().ok_or(PDError::CollectorLineNameError)?.to_owned();
                if name == kernel_data_processed.last_mut().unwrap().name {
                    continue;
                }
                let mut group_to_add_index = 0;
                let mut start_appending: bool = false;
                for (i, group) in kernel_data_processed.clone().iter().enumerate() {
                    if group.name == name {
                        group_to_add_index = i;
                        start_appending = true;
                        continue;
                    }
                    if start_appending {
                        kernel_data_processed[group_to_add_index]
                            .add_entry(Entry::ConfigGroup(group.clone()));
                    }
                }
                if start_appending {
                    kernel_data_processed =
                        kernel_data_processed[..group_to_add_index + 1].to_vec();
                }
            }
        }
        self.set_time(TimeEnum::DateTime(time_now));
        self.set_data(kernel_data_processed);
        trace!("KernelConfig data: {:#?}", self);
        Ok(())
    }

    fn is_static() -> bool {
        true
    }
}

fn get_kernel_config(value: KernelConfig) -> Result<String> {
    Ok(serde_json::to_string(&value.kernel_config_data)?)
}

impl GetData for KernelConfig {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::KernelConfig(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let processed_data = ProcessedData::KernelConfig((*raw_value).clone());
        Ok(processed_data)
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
                ProcessedData::KernelConfig(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        if param.len() < 2 {
            panic!("Not enough arguments");
        }
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "values" => get_kernel_config(values[0].clone()),
            _ => panic!("Unsupported API"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{KernelConfig, KernelConfigEntryGroup};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut kernel_config = KernelConfig::new();
        let params = CollectorParams::new();

        kernel_config.collect_data(&params).unwrap();
        assert!(!kernel_config.kernel_config_data.is_empty());
    }

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut kernel_config = KernelConfig::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        kernel_config.collect_data(&params).unwrap();
        buffer.push(Data::KernelConfig(kernel_config));
        processed_buffer.push(
            KernelConfig::new()
                .process_raw_data(buffer[0].clone())
                .unwrap(),
        );
        let json = KernelConfig::new()
            .get_data(
                processed_buffer,
                "run=test&get=values".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<KernelConfigEntryGroup> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }
}
