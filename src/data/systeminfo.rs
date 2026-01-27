use crate::data::data_formats::{AperfData, KeyValueData, KeyValueGroup};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    log::warn,
    sysinfo::{System, SystemExt},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemInfo {
    pub time: TimeEnum,
    pub system_name: String,
    pub kernel_version: String,
    pub os_version: String,
    pub host_name: String,
    pub total_cpus: usize,
    pub instance_metadata: EC2Metadata,
}

impl SystemInfo {
    pub fn new() -> Self {
        SystemInfo {
            time: TimeEnum::DateTime(Utc::now()),
            system_name: String::new(),
            kernel_version: String::new(),
            os_version: String::new(),
            host_name: String::new(),
            total_cpus: 0,
            instance_metadata: EC2Metadata::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl SystemInfo {
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

#[cfg(target_os = "linux")]
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
            instance_type: String::new(),
        }
    }

    #[cfg(target_os = "linux")]
    async fn get_instance_metadata() -> Result<EC2Metadata, BoxError> {
        use aws_config::imds;

        let imds_client = imds::Client::builder().build();

        let ami_id = imds_client.get("/latest/meta-data/ami-id").await?;
        let instance_id = imds_client.get("/latest/meta-data/instance-id").await?;
        let local_hostname = imds_client.get("/latest/meta-data/local-hostname").await?;
        let instance_type = imds_client.get("/latest/meta-data/instance-type").await?;
        let region = imds_client
            .get("/latest/meta-data/placement/region")
            .await?;

        Ok(EC2Metadata {
            instance_id: instance_id.into(),
            local_hostname: local_hostname.into(),
            ami_id: ami_id.into(),
            region: region.into(),
            instance_type: instance_type.into(),
        })
    }
}

#[cfg(target_os = "linux")]
impl CollectData for SystemInfo {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        let mut sys = System::new();
        sys.refresh_system();

        self.set_system_name(sys.name().unwrap());
        self.set_kernel_version(sys.kernel_version().unwrap());
        self.set_os_version(sys.os_version().unwrap());
        self.set_host_name(sys.host_name().unwrap());
        self.set_total_cpus(sys.cpus().len());

        let rt = tokio::runtime::Runtime::new().unwrap();

        match rt.block_on(EC2Metadata::get_instance_metadata()) {
            Ok(s) => self.set_instance_metadata(s),
            Err(e) => {
                warn!("Unable to get instance metadata: {}", e);
                let s = EC2Metadata {
                    instance_id: "N/A".to_string(),
                    local_hostname: "N/A".to_string(),
                    ami_id: "N/A".to_string(),
                    region: "N/A".to_string(),
                    instance_type: "N/A".to_string(),
                };
                self.set_instance_metadata(s);
            }
        };

        Ok(())
    }

    fn is_static() -> bool {
        true
    }
}

impl ProcessData for SystemInfo {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["system_info"]
    }

    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut key_value_data = KeyValueData::default();

        let mut key_values: HashMap<String, String> = HashMap::new();
        // The raw_data should contain a single data. Processing it in a loop to follow the generic
        // pattern
        for buffer in raw_data {
            let raw_value = match buffer {
                Data::SystemInfo(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            key_values.insert("System Name".to_string(), raw_value.system_name.clone());
            key_values.insert("OS Version".to_string(), raw_value.os_version.clone());
            key_values.insert(
                "Kernel Version".to_string(),
                raw_value.kernel_version.clone(),
            );
            key_values.insert("Hostname".to_string(), raw_value.host_name.clone());
            key_values.insert("CPUs".to_string(), raw_value.total_cpus.to_string());
            key_values.insert(
                "Instance ID".to_string(),
                raw_value.instance_metadata.instance_id.clone(),
            );
            key_values.insert(
                "Region".to_string(),
                raw_value.instance_metadata.region.clone(),
            );
            key_values.insert(
                "Instance Type".to_string(),
                raw_value.instance_metadata.instance_type.clone(),
            );
            key_values.insert(
                "AMI ID".to_string(),
                raw_value.instance_metadata.ami_id.clone(),
            );
            key_values.insert(
                "Local Hostname".to_string(),
                raw_value.instance_metadata.local_hostname.clone(),
            );
        }

        let mut key_value_group = KeyValueGroup::default();
        key_value_group.key_values = key_values;
        key_value_data
            .key_value_groups
            .insert(String::new(), key_value_group);

        Ok(AperfData::KeyValue(key_value_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::SystemInfo,
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut systeminfo = SystemInfo::new();
        let params = CollectorParams::new();

        systeminfo.collect_data(&params).unwrap();
        assert_ne!(systeminfo.total_cpus, 0);
        assert_ne!(systeminfo.system_name, String::new());
        assert_ne!(systeminfo.kernel_version, String::new());
        assert_ne!(systeminfo.os_version, String::new());
        assert_ne!(systeminfo.host_name, String::new());
    }
}
