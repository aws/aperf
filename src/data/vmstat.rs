extern crate ctor;

use crate::data::{CollectData, Data, DataType};
use crate::PDResult;
use crate::PERFORMANCE_DATA;
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use procfs::vmstat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vmstat {
    pub time: DateTime<Utc>,
    pub vmstat_data: HashMap<String, i64>,
}

impl Vmstat {
    fn new() -> Self {
        Vmstat {
            time: Utc::now(),
            vmstat_data: HashMap::new(),
        }
    }

    fn set_time(&mut self, time: DateTime<Utc>) {
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

        self.set_time(time_now);
        self.set_data(vmstat_data);
        debug!("Vmstat data: {:#?}", self);
        Ok(())
    }
}

#[ctor]
fn init_vmstat() {
    let vmstat = Vmstat::new();

    let dt = DataType {
        data: Data::Vmstat(vmstat),
        file_handle: None,
        file_name: "vmstat".to_string(),
        dir_name: String::new(),
        full_path: String::new(),
    };
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
