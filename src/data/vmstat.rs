extern crate ctor;

use anyhow::Result;
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
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

impl GetData for Vmstat {}

#[ctor]
fn init_vmstat() {
    let vmstat = Vmstat::new();
    let file_name = VMSTAT_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::Vmstat(vmstat.clone()),
        file_name.clone(),
        false
    );
    let dv = DataVisualizer::new(
        Data::Vmstat(vmstat.clone()),
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
    use super::Vmstat;
    use crate::data::CollectData;

    #[test]
    fn test_collect_data() {
        let mut vmstat = Vmstat::new();

        assert!(vmstat.collect_data().unwrap() == ());
        assert!(vmstat.vmstat_data.len() != 0);
    }
}
