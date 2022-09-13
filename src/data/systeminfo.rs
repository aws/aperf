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
    pub system_name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub host_name: Option<String>,
    pub total_cpus: usize,
}

impl SystemInfo {
    fn new() -> Self {
        SystemInfo {
            time: TimeEnum::DateTime(Utc::now()),
            system_name: None,
            kernel_version: None,
            os_version: None,
            host_name: None,
            total_cpus: 0
        }
    }

    fn set_system_name(&mut self, system_name: Option<String>) {
        self.system_name = system_name;
    }

    fn set_kernel_version(&mut self, kernel_version: Option<String>) {
        self.kernel_version = kernel_version;
    }

    fn set_os_version(&mut self, os_version: Option<String>) {
        self.os_version = os_version;
    }

    fn set_host_name(&mut self, host_name: Option<String>) {
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

        self.set_system_name(sys.name());
        self.set_kernel_version(sys.kernel_version());
        self.set_os_version(sys.os_version());
        self.set_host_name(sys.host_name());
        self.set_total_cpus(sys.cpus().len());
        debug!("SysInfo:\n{:#?}", self);
        Ok(())
    }
}

#[ctor]
fn init_systeminfo() {
    let system_info = SystemInfo::new();

    let dt = DataType {
        data: Data::SystemInfo(system_info),
        file_handle: None,
        file_name: SYSTEMINFO_FILE_NAME.to_string(),
        dir_name: String::new(),
        full_path: String::new(),
        collect_once: true
    };
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
        assert!(systeminfo.system_name != None);
        assert!(systeminfo.kernel_version != None);
        assert!(systeminfo.os_version != None);
        assert!(systeminfo.host_name != None);
    }
}