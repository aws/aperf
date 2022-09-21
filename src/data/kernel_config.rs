extern crate ctor;

use anyhow::Result;
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use procfs::{ConfigSetting, kernel_config};
use std::collections::HashMap;
use std::fmt::Debug;
use serde::{Deserialize, Serialize};

pub static KERNEL_CONFIG_FILE_NAME: &str = "kernel_config";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelConfig {
    pub time: TimeEnum,
    pub kernel_config_data: HashMap<String, String>,
}

impl KernelConfig {
    fn new() -> Self {
        KernelConfig {
            time: TimeEnum::DateTime(Utc::now()),
            kernel_config_data: HashMap::new(),
        }
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }

    fn set_data(&mut self, data: HashMap<String, String>) {
        self.kernel_config_data = data;
    }
}

impl CollectData for KernelConfig {
    fn collect_data(&mut self) -> Result<()> {
        let time_now = Utc::now();
        let kernel_config_data = kernel_config().unwrap();
        let mut kernel_data_processed: HashMap<String, String> = HashMap::new();

        for (key, key_value) in &kernel_config_data {
            let output;

            match key_value {
                ConfigSetting::Yes => output = "y",
                ConfigSetting::Module => output = "m",
                ConfigSetting::Value(s) => output = s,
            }

            kernel_data_processed.insert(key.to_string(), output.parse().unwrap());
        }

        self.set_time(TimeEnum::DateTime(time_now));
        self.set_data(kernel_data_processed);
        debug!("KernelConfig data: {:#?}", self);
        Ok(())
    }
}

impl GetData for KernelConfig {}

#[ctor]
fn init_kernel_config() {
    let kernel_config = KernelConfig::new();
    let file_name = KERNEL_CONFIG_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::KernelConfig(kernel_config.clone()),
        file_name.clone(),
        true
    );
    let dv = DataVisualizer::new(
        Data::KernelConfig(kernel_config),
        file_name.clone(),
        String::new(),
        String::new(),
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
    use super::KernelConfig;
    use crate::data::CollectData;

    #[test]
    fn test_collect_data() {
        let mut kernel_config = KernelConfig::new();

        assert!(kernel_config.collect_data().unwrap() == ());
        assert!(kernel_config.kernel_config_data.len() != 0);
    }
}
