extern crate ctor;

use anyhow::Result;
use sysinfo::{System, SystemExt};
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use serde::{Deserialize, Serialize};

pub static SYSTEMINFO_FILE_NAME: &str = "system_info";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemInfo {
    pub time: TimeEnum,
    pub system_name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub host_name: String,
    pub total_cpus: usize,
}

impl SystemInfo {
    fn new() -> Self {
        SystemInfo {
            time: TimeEnum::DateTime(Utc::now()),
            system_name: String::new(),
            kernel_version: String::new(),
            os_version: String::new(),
            host_name: String::new(),
            total_cpus: 0
        }
    }

    fn set_system_name(&mut self, system_name: String) {
        self.system_name = system_name;
    }

    fn set_kernel_version(&mut self, kernel_version: String) {
        self.kernel_version = kernel_version;
    }

    fn set_os_version(&mut self, os_version: String) {
        self.os_version = os_version;
    }

    fn set_host_name(&mut self, host_name: String) {
        self.host_name = host_name;
    }

    fn set_total_cpus(&mut self, total_cpus: usize) {
        self.total_cpus = total_cpus;
    }
}

impl CollectData for SystemInfo {
    fn collect_data(&mut self) -> Result<()> {
        let mut sys = System::new_all();
        sys.refresh_all();

        self.set_system_name(sys.name().unwrap());
        self.set_kernel_version(sys.kernel_version().unwrap());
        self.set_os_version(sys.os_version().unwrap());
        self.set_host_name(sys.host_name().unwrap());
        self.set_total_cpus(sys.cpus().len());
        debug!("SysInfo:\n{:#?}", self);
        Ok(())
    }
}

impl GetData for SystemInfo {}

#[ctor]
fn init_systeminfo() {
    let system_info = SystemInfo::new();
    let file_name = SYSTEMINFO_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::SystemInfo(system_info.clone()),
        file_name.clone(),
        true
    );
    let dv = DataVisualizer::new(
        Data::SystemInfo(system_info.clone()),
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
    use super::SystemInfo;
    use crate::data::CollectData;

    #[test]
    fn test_collect_data() {
        let mut systeminfo = SystemInfo::new();

        assert!(systeminfo.collect_data().unwrap() == ());
        assert!(systeminfo.total_cpus != 0);
        assert!(systeminfo.system_name != String::new());
        assert!(systeminfo.kernel_version != String::new());
        assert!(systeminfo.os_version != String::new());
        assert!(systeminfo.host_name != String::new());
    }
}
