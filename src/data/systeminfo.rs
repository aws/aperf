extern crate ctor;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData, TimeEnum};
use crate::utils::{DataMetrics, ValueType};
use crate::visualizer::{DataVisualizer, GetData};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{System, SystemExt};

pub static SYSTEMINFO_FILE_NAME: &str = "system_info";

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
    fn new() -> Self {
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
            instance_type: String::new(),
        }
    }

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

        trace!("SysInfo:\n{:#?}", self);

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SUTConfigEntry {
    pub name: String,
    pub value: String,
}

fn get_values(buffer: SystemInfo, metrics: &mut DataMetrics) -> Result<String> {
    let mut end_values = Vec::new();
    let mut my_metrics = HashMap::new();

    let system_name = SUTConfigEntry {
        name: "System Name".to_string(),
        value: buffer.system_name.clone(),
    };
    my_metrics.insert(
        "System Name".to_string(),
        ValueType::String(buffer.system_name),
    );
    end_values.push(system_name);

    let os_version = SUTConfigEntry {
        name: "OS Version".to_string(),
        value: buffer.os_version.clone(),
    };
    my_metrics.insert(
        "OS Version".to_string(),
        ValueType::String(buffer.os_version),
    );
    end_values.push(os_version);

    let kernel_version = SUTConfigEntry {
        name: "Kernel Version".to_string(),
        value: buffer.kernel_version.clone(),
    };
    my_metrics.insert(
        "Kernel Version".to_string(),
        ValueType::String(buffer.kernel_version),
    );
    end_values.push(kernel_version);

    let region = SUTConfigEntry {
        name: "Region".to_string(),
        value: buffer.instance_metadata.region.clone(),
    };
    my_metrics.insert(
        "Region".to_string(),
        ValueType::String(buffer.instance_metadata.region),
    );
    end_values.push(region);

    let instance_type = SUTConfigEntry {
        name: "Instance Type".to_string(),
        value: buffer.instance_metadata.instance_type.clone(),
    };
    my_metrics.insert(
        "Instance Type".to_string(),
        ValueType::String(buffer.instance_metadata.instance_type),
    );
    end_values.push(instance_type);

    let total_cpus = SUTConfigEntry {
        name: "Total CPUs".to_string(),
        value: buffer.total_cpus.to_string(),
    };
    my_metrics.insert(
        "Total CPUs".to_string(),
        ValueType::UInt64(buffer.total_cpus as u64),
    );
    end_values.push(total_cpus);

    let instance_id = SUTConfigEntry {
        name: "Instance ID".to_string(),
        value: buffer.instance_metadata.instance_id.clone(),
    };
    my_metrics.insert(
        "Instance ID".to_string(),
        ValueType::String(buffer.instance_metadata.instance_id),
    );
    end_values.push(instance_id);

    let ami_id = SUTConfigEntry {
        name: "AMI ID".to_string(),
        value: buffer.instance_metadata.ami_id.clone(),
    };
    my_metrics.insert(
        "AMI ID".to_string(),
        ValueType::String(buffer.instance_metadata.ami_id),
    );
    end_values.push(ami_id);

    let host_name = SUTConfigEntry {
        name: "Host Name".to_string(),
        value: buffer.host_name.clone(),
    };
    my_metrics.insert("Host Name".to_string(), ValueType::String(buffer.host_name));
    end_values.push(host_name);

    metrics
        .values
        .insert(SYSTEMINFO_FILE_NAME.to_string(), my_metrics);

    Ok(serde_json::to_string(&end_values)?)
}

impl GetData for SystemInfo {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::SystemInfo(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let processed_data = ProcessedData::SystemInfo((*raw_value).clone());
        Ok(processed_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(
        &mut self,
        buffer: Vec<ProcessedData>,
        query: String,
        metrics: &mut DataMetrics,
    ) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::SystemInfo(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "values" => get_values(values[0].clone(), metrics),
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_systeminfo() {
    let system_info = SystemInfo::new();
    let file_name = SYSTEMINFO_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::SystemInfo(system_info.clone()),
        file_name.clone(),
        true,
    );
    let js_file_name = file_name.clone() + ".js";
    let dv = DataVisualizer::new(
        ProcessedData::SystemInfo(system_info.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/system_info.js")).to_string(),
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
    use super::{SUTConfigEntry, SystemInfo};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;

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

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut system_info = SystemInfo::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        system_info.collect_data(&params).unwrap();
        buffer.push(Data::SystemInfo(system_info));
        processed_buffer.push(
            SystemInfo::new()
                .process_raw_data(buffer[0].clone())
                .unwrap(),
        );
        let json = SystemInfo::new()
            .get_data(
                processed_buffer,
                "run=test&get=values".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<SUTConfigEntry> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }
}
