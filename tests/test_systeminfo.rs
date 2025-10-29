use aperf::data::data_formats::AperfData;
use aperf::data::systeminfo::{EC2Metadata, SystemInfo};
use aperf::data::{Data, TimeEnum};
use aperf::visualizer::{GetData, ReportParams};
use chrono::Utc;

fn create_test_systeminfo() -> SystemInfo {
    SystemInfo {
        time: TimeEnum::DateTime(Utc::now()),
        system_name: "Linux".to_string(),
        kernel_version: "5.4.0-test".to_string(),
        os_version: "Ubuntu 20.04".to_string(),
        host_name: "test-host".to_string(),
        total_cpus: 4,
        instance_metadata: EC2Metadata {
            instance_id: "i-1234567890abcdef0".to_string(),
            local_hostname: "ip-10-0-0-1.ec2.internal".to_string(),
            ami_id: "ami-0123456789abcdef0".to_string(),
            region: "us-west-2".to_string(),
            instance_type: "m5.large".to_string(),
        },
    }
}

#[test]
fn test_process_raw_data_new() {
    let mut systeminfo = SystemInfo::new();
    let test_data = create_test_systeminfo();
    let raw_data = vec![Data::SystemInfo(test_data.clone())];
    let params = ReportParams::new();

    let result = systeminfo.process_raw_data_new(params, raw_data).unwrap();

    match result {
        AperfData::KeyValue(key_value_data) => {
            let key_value_group = key_value_data.key_value_groups.get("").unwrap();

            assert_eq!(
                key_value_group.key_values.get("System Name").unwrap(),
                "Linux"
            );
            assert_eq!(
                key_value_group.key_values.get("OS Version").unwrap(),
                "Ubuntu 20.04"
            );
            assert_eq!(
                key_value_group.key_values.get("Kernel Version").unwrap(),
                "5.4.0-test"
            );
            assert_eq!(
                key_value_group.key_values.get("Hostname").unwrap(),
                "test-host"
            );
            assert_eq!(key_value_group.key_values.get("CPUs").unwrap(), "4");
            assert_eq!(
                key_value_group.key_values.get("Instance ID").unwrap(),
                "i-1234567890abcdef0"
            );
            assert_eq!(
                key_value_group.key_values.get("Region").unwrap(),
                "us-west-2"
            );
            assert_eq!(
                key_value_group.key_values.get("Instance Type").unwrap(),
                "m5.large"
            );
            assert_eq!(
                key_value_group.key_values.get("AMD ID").unwrap(),
                "ami-0123456789abcdef0"
            );
            assert_eq!(
                key_value_group.key_values.get("Local Hostname").unwrap(),
                "ip-10-0-0-1.ec2.internal"
            );
        }
        _ => panic!("Expected KeyValue data type"),
    }
}

#[test]
fn test_process_raw_data_new_empty_data() {
    let mut systeminfo = SystemInfo::new();
    let raw_data = vec![];
    let params = ReportParams::new();

    let result = systeminfo.process_raw_data_new(params, raw_data).unwrap();

    match result {
        AperfData::KeyValue(key_value_data) => {
            let key_value_group = key_value_data.key_value_groups.get("").unwrap();
            assert!(key_value_group.key_values.is_empty());
        }
        _ => panic!("Expected KeyValue data type"),
    }
}

#[test]
fn test_process_raw_data_new_na_metadata() {
    let mut systeminfo = SystemInfo::new();
    let mut test_data = create_test_systeminfo();
    test_data.instance_metadata = EC2Metadata {
        instance_id: "N/A".to_string(),
        local_hostname: "N/A".to_string(),
        ami_id: "N/A".to_string(),
        region: "N/A".to_string(),
        instance_type: "N/A".to_string(),
    };

    let raw_data = vec![Data::SystemInfo(test_data)];
    let params = ReportParams::new();

    let result = systeminfo.process_raw_data_new(params, raw_data).unwrap();

    match result {
        AperfData::KeyValue(key_value_data) => {
            let key_value_group = key_value_data.key_value_groups.get("").unwrap();

            assert_eq!(
                key_value_group.key_values.get("Instance ID").unwrap(),
                "N/A"
            );
            assert_eq!(key_value_group.key_values.get("Region").unwrap(), "N/A");
            assert_eq!(
                key_value_group.key_values.get("Instance Type").unwrap(),
                "N/A"
            );
            assert_eq!(key_value_group.key_values.get("AMD ID").unwrap(), "N/A");
            assert_eq!(
                key_value_group.key_values.get("Local Hostname").unwrap(),
                "N/A"
            );
        }
        _ => panic!("Expected KeyValue data type"),
    }
}
