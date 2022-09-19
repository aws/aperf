extern crate ctor;

use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::PDResult;
use crate::PERFORMANCE_DATA;
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
    fn collect_data(&mut self) -> PDResult {
        let time_now = Utc::now();
        let vmstat_data = vmstat().unwrap();

        self.set_time(TimeEnum::DateTime(time_now));
        self.set_data(vmstat_data);
        debug!("Vmstat data: {:#?}", self);
        Ok(())
    }
}

#[ctor]
fn init_vmstat() {
    let dt = DataType::new(
        Data::Vmstat(Vmstat::new()),
        VMSTAT_FILE_NAME.to_string(),
        false
    );

    PERFORMANCE_DATA
        .lock()
        .unwrap()
        .add_datatype("Vmstat".to_string(), dt);
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
