use aperf::data::sysctl::SysctlData;
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::Utc;
use std::collections::BTreeMap;

fn generate_sysctl_raw_data(expected_sysctl_data: &BTreeMap<String, String>) -> Vec<Data> {
    vec![Data::SysctlData(SysctlData {
        time: TimeEnum::DateTime(Utc::now()),
        sysctl_data: expected_sysctl_data.clone(),
    })]
}

#[test]
fn test_process_sysctl_raw_data_complex() {
    let mut expected_sysctl_data = BTreeMap::new();
    expected_sysctl_data.insert("kernel.pid_max".to_string(), "32768".to_string());
    expected_sysctl_data.insert("kernel.threads-max".to_string(), "126982".to_string());
    expected_sysctl_data.insert("vm.swappiness".to_string(), "60".to_string());
    expected_sysctl_data.insert("net.core.rmem_default".to_string(), "212992".to_string());
    expected_sysctl_data.insert("fs.file-max".to_string(), "9223372036854775807".to_string());
    expected_sysctl_data.insert(
        "net.ipv4.tcp_congestion_control".to_string(),
        "cubic".to_string(),
    );
    expected_sysctl_data.insert("net.ipv4.tcp_window_scaling".to_string(), "1".to_string());
    expected_sysctl_data.insert(
        "net.core.netdev_max_backlog".to_string(),
        "1000".to_string(),
    );
    expected_sysctl_data.insert("kernel.hostname".to_string(), "test-host".to_string());
    expected_sysctl_data.insert("vm.dirty_ratio".to_string(), "20".to_string());
    expected_sysctl_data.insert("vm.dirty_background_ratio".to_string(), "10".to_string());
    expected_sysctl_data.insert("kernel.randomize_va_space".to_string(), "2".to_string());
    expected_sysctl_data.insert("kernel.dmesg_restrict".to_string(), "0".to_string());

    let raw_data = generate_sysctl_raw_data(&expected_sysctl_data);
    let mut sysctl = SysctlData::new();
    let result = sysctl
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        assert_eq!(key_value_data.key_value_groups.len(), 1);
        assert!(key_value_data.key_value_groups.contains_key(""));

        let default_group = &key_value_data.key_value_groups[""];
        assert_eq!(default_group.key_values.len(), 13);

        // Verify all expected values
        for (key, expected_value) in &expected_sysctl_data {
            assert_eq!(default_group.key_values.get(key), Some(expected_value));
        }
    } else {
        panic!("Expected KeyValue data");
    }
}

#[test]
fn test_process_sysctl_raw_data_simple() {
    let mut expected_sysctl_data = BTreeMap::new();
    expected_sysctl_data.insert(
        "kernel.version".to_string(),
        "Linux version 5.4.0".to_string(),
    );
    expected_sysctl_data.insert("vm.swappiness".to_string(), "60".to_string());
    expected_sysctl_data.insert("net.ipv4.ip_forward".to_string(), "0".to_string());

    let raw_data = generate_sysctl_raw_data(&expected_sysctl_data);
    let mut sysctl = SysctlData::new();
    let result = sysctl
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        assert_eq!(key_value_data.key_value_groups.len(), 1);

        let default_group = &key_value_data.key_value_groups[""];
        assert_eq!(default_group.key_values.len(), 3);

        assert_eq!(
            default_group.key_values.get("kernel.version"),
            Some(&"Linux version 5.4.0".to_string())
        );
        assert_eq!(
            default_group.key_values.get("vm.swappiness"),
            Some(&"60".to_string())
        );
        assert_eq!(
            default_group.key_values.get("net.ipv4.ip_forward"),
            Some(&"0".to_string())
        );
    } else {
        panic!("Expected KeyValue data");
    }
}

#[test]
fn test_process_sysctl_raw_data_special_values() {
    let mut expected_sysctl_data = BTreeMap::new();
    expected_sysctl_data.insert(
        "kernel.hostname".to_string(),
        "test-host.example.com".to_string(),
    );
    expected_sysctl_data.insert("kernel.ostype".to_string(), "Linux".to_string());
    expected_sysctl_data.insert("kernel.version".to_string(), "#1 SMP PREEMPT".to_string());
    expected_sysctl_data.insert("vm.zone_reclaim_mode".to_string(), "0".to_string());
    expected_sysctl_data.insert(
        "net.ipv4.tcp_congestion_control".to_string(),
        "bbr".to_string(),
    );

    let raw_data = generate_sysctl_raw_data(&expected_sysctl_data);
    let mut sysctl = SysctlData::new();
    let result = sysctl
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        let default_group = &key_value_data.key_value_groups[""];
        assert_eq!(default_group.key_values.len(), 5);

        assert_eq!(
            default_group.key_values.get("kernel.hostname"),
            Some(&"test-host.example.com".to_string())
        );
        assert_eq!(
            default_group.key_values.get("kernel.ostype"),
            Some(&"Linux".to_string())
        );
        assert_eq!(
            default_group.key_values.get("kernel.version"),
            Some(&"#1 SMP PREEMPT".to_string())
        );
        assert_eq!(
            default_group
                .key_values
                .get("net.ipv4.tcp_congestion_control"),
            Some(&"bbr".to_string())
        );
        assert_eq!(
            default_group.key_values.get("vm.zone_reclaim_mode"),
            Some(&"0".to_string())
        );
    } else {
        panic!("Expected KeyValue data");
    }
}

#[test]
fn test_process_sysctl_raw_data_empty_data() {
    let raw_data = Vec::new();
    let mut sysctl = SysctlData::new();
    let result = sysctl
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        assert_eq!(key_value_data.key_value_groups.len(), 1);

        let default_group = &key_value_data.key_value_groups[""];
        assert_eq!(default_group.key_values.len(), 0);
    } else {
        panic!("Expected KeyValue data");
    }
}
