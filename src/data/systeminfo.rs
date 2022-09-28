extern crate ctor;

use anyhow::Result;
use sysinfo::{System, SystemExt};
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::{debug, info};
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
    pub instance_metadata: EC2Metadata
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
            instance_metadata: EC2Metadata::new()
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

    fn set_instance_metadata(&mut self, instance_metadata: EC2Metadata) {
        self.instance_metadata = instance_metadata;
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EC2Metadata {
    pub instance_id: String,
    pub local_hostname: String,
    pub ami_id: String,
    pub region: String,
    pub instance_type: String,
}

impl EC2Metadata {
    fn new() -> Self {
        EC2Metadata {
            instance_id: String::new(),
            local_hostname: String::new(),
            ami_id: String::new(),
            region: String::new(),
            instance_type: String::new()
        }
    }

    async fn get_instance_metadata() -> Result<EC2Metadata, BoxError> {
        use aws_config::imds;

        let imds_client = imds::Client::builder().build().await?;

        let ami_id = imds_client.get("/latest/meta-data/ami-id").await?;
        let instance_id = imds_client.get("/latest/meta-data/instance-id").await?;
        let local_hostname = imds_client.get("/latest/meta-data/local-hostname").await?;
        let instance_type = imds_client.get("/latest/meta-data/instance-type").await?;
        let region = imds_client
            .get("/latest/meta-data/placement/region")
            .await?;

        Ok(EC2Metadata {
            instance_id,
            local_hostname,
            ami_id,
            region,
            instance_type
        })
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

        let rt = tokio::runtime::Runtime::new().unwrap();

        match rt.block_on(EC2Metadata::get_instance_metadata()) {
            Ok(s) => self.set_instance_metadata(s),
            Err(e) => info!("An error occurred: {}", e),
        };

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
