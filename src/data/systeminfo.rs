extern crate ctor;

use std::collections::HashMap;
use sysinfo::{System, SystemExt};
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::PDResult;
use crate::PERFORMANCE_DATA;
use chrono::prelude::*;
use ctor::ctor;
use log::{debug};
use serde::{Deserialize, Serialize};
use cmd_lib::run_fun;

pub static SYSTEMINFO_FILE_NAME: &str = "system_info";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemInfo {
    pub time: TimeEnum,
    pub system_name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub host_name: String,
    pub total_cpus: usize,
    pub instance_metadata: HashMap<String, String>
}

impl SystemInfo {
    fn new() -> Self {
        SystemInfo {
            time: TimeEnum::DateTime(Utc::now()),
            system_name: String::new(),
            kernel_version: String::new(),
            os_version: String::new(),
            host_name: String::new(),
            total_cpus: 0,
            instance_metadata: HashMap::new()
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

    fn set_instance_metadata(&mut self, instance_metadata: HashMap<String, String>) {
        self.instance_metadata = instance_metadata;
    }
}


fn get_instance_metadata() -> HashMap<String, String>{
    let metadata_collection_list = ["ami-id".to_string(), "instance-id".to_string(), "local-hostname".to_string(),
        "instance-type".to_string(), "placement/region".to_string()];
    let mut metadata = HashMap::new();
    let instance_token = run_fun!(curl -X PUT "http://169.254.169.254/latest/api/token" -H "X-aws-ec2-metadata-token-ttl-seconds: 21600").unwrap();
    for item in metadata_collection_list.iter() {
        let item_value = run_fun!(curl -H "X-aws-ec2-metadata-token: $instance_token" -v "http://169.254.169.254/latest/meta-data/$item").unwrap();
        metadata.insert(item.to_string(), item_value.to_string());
    }
    return metadata;
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
        self.set_instance_metadata(get_instance_metadata());

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
        assert!(systeminfo.system_name != String::new());
        assert!(systeminfo.kernel_version != String::new());
        assert!(systeminfo.os_version != String::new());
        assert!(systeminfo.host_name != String::new());
    }
}
