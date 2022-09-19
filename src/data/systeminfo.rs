extern crate ctor;

use sysinfo::{System, SystemExt};
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::PDResult;
use crate::PERFORMANCE_DATA;
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
            system_name: "".to_string(),
            kernel_version: "".to_string(),
            os_version: "".to_string(),
            host_name: "".to_string(),
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
    fn collect_data(&mut self) -> PDResult {
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

#[ctor]
fn init_systeminfo() {
    let dt = DataType::new(
        Data::SystemInfo(SystemInfo::new()),
        SYSTEMINFO_FILE_NAME.to_string(),
        true
    );

    PERFORMANCE_DATA
        .lock()
        .unwrap()
        .add_datatype("SystemInfo".to_string(), dt);
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
        assert!(systeminfo.system_name != "".to_string());
        assert!(systeminfo.kernel_version != "".to_string());
        assert!(systeminfo.os_version != "".to_string());
        assert!(systeminfo.host_name != "".to_string());
    }
}
