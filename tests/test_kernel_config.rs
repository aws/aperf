use aperf::data::kernel_config::{Entry, KernelConfig, KernelConfigEntry, KernelConfigEntryGroup};
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::Utc;

fn generate_kernel_config_raw_data() -> Vec<Data> {
    let mut kernel_config_groups = Vec::new();

    // Root group (empty name)
    let root_group = KernelConfigEntryGroup {
        name: "".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_64BIT".to_string(),
                value: "y".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_X86_64".to_string(),
                value: "y".to_string(),
            }),
        ],
    };

    // General setup group
    let mut general_setup_group = KernelConfigEntryGroup {
        name: "General setup".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_INIT_ENV_ARG_LIMIT".to_string(),
                value: "32".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_COMPILE_TEST".to_string(),
                value: "not set".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_LOCALVERSION".to_string(),
                value: "\"-custom\"".to_string(),
            }),
        ],
    };

    // Processor type and features (nested under General setup)
    let processor_group = KernelConfigEntryGroup {
        name: "Processor type and features".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_SMP".to_string(),
                value: "y".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_X86_FEATURE_NAMES".to_string(),
                value: "y".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_X86_FAST_FEATURE_TESTS".to_string(),
                value: "y".to_string(),
            }),
        ],
    };

    // Power management (nested under Processor type)
    let power_mgmt_group = KernelConfigEntryGroup {
        name: "Power management and ACPI options".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_SUSPEND".to_string(),
                value: "y".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_HIBERNATION".to_string(),
                value: "not set".to_string(),
            }),
        ],
    };

    // Create hierarchy: Power management -> Processor type -> General setup
    let mut processor_with_power = processor_group;
    processor_with_power
        .entries
        .push(Entry::ConfigGroup(power_mgmt_group));

    general_setup_group
        .entries
        .push(Entry::ConfigGroup(processor_with_power));

    // Networking support group (separate top-level)
    let networking_group = KernelConfigEntryGroup {
        name: "Networking support".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_NET".to_string(),
                value: "y".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_PACKET".to_string(),
                value: "y".to_string(),
            }),
        ],
    };

    // TCP/IP networking (nested under Networking)
    let tcpip_group = KernelConfigEntryGroup {
        name: "TCP/IP networking".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_INET".to_string(),
                value: "y".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_IP_MULTICAST".to_string(),
                value: "y".to_string(),
            }),
        ],
    };

    let mut networking_with_tcpip = networking_group;
    networking_with_tcpip
        .entries
        .push(Entry::ConfigGroup(tcpip_group));

    kernel_config_groups.push(root_group);
    kernel_config_groups.push(general_setup_group);
    kernel_config_groups.push(networking_with_tcpip);

    let kernel_config = KernelConfig {
        time: TimeEnum::DateTime(Utc::now()),
        kernel_config_data: kernel_config_groups,
    };

    vec![Data::KernelConfig(kernel_config)]
}

#[test]
fn test_process_kernel_config_hierarchical_data() {
    let raw_data = generate_kernel_config_raw_data();
    let mut kernel_config = KernelConfig::new();
    let result = kernel_config
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        // Should have flattened hierarchical groups with colon-separated names
        assert!(!key_value_data.key_value_groups.is_empty());

        // Check root group (empty prefix)
        assert!(key_value_data.key_value_groups.contains_key(""));
        let root_group = &key_value_data.key_value_groups[""];
        assert_eq!(
            root_group.key_values.get("CONFIG_64BIT"),
            Some(&"y".to_string())
        );
        assert_eq!(
            root_group.key_values.get("CONFIG_X86_64"),
            Some(&"y".to_string())
        );

        // Check General setup group
        assert!(key_value_data
            .key_value_groups
            .contains_key("General setup"));
        let general_group = &key_value_data.key_value_groups["General setup"];
        assert_eq!(
            general_group.key_values.get("CONFIG_INIT_ENV_ARG_LIMIT"),
            Some(&"32".to_string())
        );
        assert_eq!(
            general_group.key_values.get("CONFIG_COMPILE_TEST"),
            Some(&"not set".to_string())
        );
        assert_eq!(
            general_group.key_values.get("CONFIG_LOCALVERSION"),
            Some(&"\"-custom\"".to_string())
        );

        // Check nested Processor type group
        assert!(key_value_data
            .key_value_groups
            .contains_key("General setup:Processor type and features"));
        let processor_group =
            &key_value_data.key_value_groups["General setup:Processor type and features"];
        assert_eq!(
            processor_group.key_values.get("CONFIG_SMP"),
            Some(&"y".to_string())
        );
        assert_eq!(
            processor_group.key_values.get("CONFIG_X86_FEATURE_NAMES"),
            Some(&"y".to_string())
        );

        // Check deeply nested Power management group
        assert!(key_value_data.key_value_groups.contains_key(
            "General setup:Processor type and features:Power management and ACPI options"
        ));
        let power_group = &key_value_data.key_value_groups
            ["General setup:Processor type and features:Power management and ACPI options"];
        assert_eq!(
            power_group.key_values.get("CONFIG_SUSPEND"),
            Some(&"y".to_string())
        );
        assert_eq!(
            power_group.key_values.get("CONFIG_HIBERNATION"),
            Some(&"not set".to_string())
        );

        // Check Networking support group
        assert!(key_value_data
            .key_value_groups
            .contains_key("Networking support"));
        let networking_group = &key_value_data.key_value_groups["Networking support"];
        assert_eq!(
            networking_group.key_values.get("CONFIG_NET"),
            Some(&"y".to_string())
        );
        assert_eq!(
            networking_group.key_values.get("CONFIG_PACKET"),
            Some(&"y".to_string())
        );

        // Check nested TCP/IP group
        assert!(key_value_data
            .key_value_groups
            .contains_key("Networking support:TCP/IP networking"));
        let tcpip_group = &key_value_data.key_value_groups["Networking support:TCP/IP networking"];
        assert_eq!(
            tcpip_group.key_values.get("CONFIG_INET"),
            Some(&"y".to_string())
        );
        assert_eq!(
            tcpip_group.key_values.get("CONFIG_IP_MULTICAST"),
            Some(&"y".to_string())
        );

        // Verify total number of groups (should be 6: root + 5 named groups)
        assert_eq!(key_value_data.key_value_groups.len(), 6);

        // Verify hierarchy flattening works correctly
        let group_names: Vec<&String> = key_value_data.key_value_groups.keys().collect();
        assert!(group_names.contains(&&"".to_string()));
        assert!(group_names.contains(&&"General setup".to_string()));
        assert!(group_names.contains(&&"General setup:Processor type and features".to_string()));
        assert!(group_names.contains(
            &&"General setup:Processor type and features:Power management and ACPI options"
                .to_string()
        ));
        assert!(group_names.contains(&&"Networking support".to_string()));
        assert!(group_names.contains(&&"Networking support:TCP/IP networking".to_string()));
    } else {
        panic!("Expected KeyValue data");
    }
}

#[test]
fn test_process_kernel_config_simple_flat_data() {
    let mut kernel_config_groups = Vec::new();

    // Single flat group
    let simple_group = KernelConfigEntryGroup {
        name: "Simple Config".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_SIMPLE_OPTION".to_string(),
                value: "y".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_ANOTHER_OPTION".to_string(),
                value: "42".to_string(),
            }),
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_DISABLED_OPTION".to_string(),
                value: "not set".to_string(),
            }),
        ],
    };

    kernel_config_groups.push(simple_group);

    let kernel_config = KernelConfig {
        time: TimeEnum::DateTime(Utc::now()),
        kernel_config_data: kernel_config_groups,
    };

    let raw_data = vec![Data::KernelConfig(kernel_config)];
    let mut kernel_config_processor = KernelConfig::new();
    let result = kernel_config_processor
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        assert_eq!(key_value_data.key_value_groups.len(), 1);

        assert!(key_value_data
            .key_value_groups
            .contains_key("Simple Config"));
        let simple_group = &key_value_data.key_value_groups["Simple Config"];

        assert_eq!(simple_group.key_values.len(), 3);
        assert_eq!(
            simple_group.key_values.get("CONFIG_SIMPLE_OPTION"),
            Some(&"y".to_string())
        );
        assert_eq!(
            simple_group.key_values.get("CONFIG_ANOTHER_OPTION"),
            Some(&"42".to_string())
        );
        assert_eq!(
            simple_group.key_values.get("CONFIG_DISABLED_OPTION"),
            Some(&"not set".to_string())
        );
    } else {
        panic!("Expected KeyValue data");
    }
}

#[test]
fn test_process_kernel_config_empty_groups() {
    let mut kernel_config_groups = Vec::new();

    // Empty group
    let empty_group = KernelConfigEntryGroup {
        name: "Empty Group".to_string(),
        entries: vec![],
    };

    // Group with only nested empty groups
    let nested_empty_group = KernelConfigEntryGroup {
        name: "Nested Empty".to_string(),
        entries: vec![],
    };

    let parent_group = KernelConfigEntryGroup {
        name: "Parent Group".to_string(),
        entries: vec![Entry::ConfigGroup(nested_empty_group)],
    };

    kernel_config_groups.push(empty_group);
    kernel_config_groups.push(parent_group);

    let kernel_config = KernelConfig {
        time: TimeEnum::DateTime(Utc::now()),
        kernel_config_data: kernel_config_groups,
    };

    let raw_data = vec![Data::KernelConfig(kernel_config)];
    let mut kernel_config_processor = KernelConfig::new();
    let result = kernel_config_processor
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        // Should still create groups even if they're empty
        assert_eq!(key_value_data.key_value_groups.len(), 3);

        assert!(key_value_data.key_value_groups.contains_key("Empty Group"));
        assert!(key_value_data.key_value_groups.contains_key("Parent Group"));
        assert!(key_value_data
            .key_value_groups
            .contains_key("Parent Group:Nested Empty"));

        // All groups should be empty
        assert_eq!(
            key_value_data.key_value_groups["Empty Group"]
                .key_values
                .len(),
            0
        );
        assert_eq!(
            key_value_data.key_value_groups["Parent Group"]
                .key_values
                .len(),
            0
        );
        assert_eq!(
            key_value_data.key_value_groups["Parent Group:Nested Empty"]
                .key_values
                .len(),
            0
        );
    } else {
        panic!("Expected KeyValue data");
    }
}

#[test]
fn test_process_kernel_config_empty_data() {
    let raw_data = Vec::new();
    let mut kernel_config = KernelConfig::new();
    let result = kernel_config
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        assert_eq!(key_value_data.key_value_groups.len(), 0);
    } else {
        panic!("Expected KeyValue data");
    }
}

#[test]
fn test_process_kernel_config_deep_nesting() {
    let mut kernel_config_groups = Vec::new();

    // Create deeply nested structure: Level1 -> Level2 -> Level3 -> Level4
    let level4_group = KernelConfigEntryGroup {
        name: "Level4".to_string(),
        entries: vec![Entry::ConfigEntry(KernelConfigEntry {
            name: "CONFIG_DEEP_OPTION".to_string(),
            value: "deep_value".to_string(),
        })],
    };

    let level3_group = KernelConfigEntryGroup {
        name: "Level3".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_L3_OPTION".to_string(),
                value: "l3_value".to_string(),
            }),
            Entry::ConfigGroup(level4_group),
        ],
    };

    let level2_group = KernelConfigEntryGroup {
        name: "Level2".to_string(),
        entries: vec![Entry::ConfigGroup(level3_group)],
    };

    let level1_group = KernelConfigEntryGroup {
        name: "Level1".to_string(),
        entries: vec![
            Entry::ConfigEntry(KernelConfigEntry {
                name: "CONFIG_L1_OPTION".to_string(),
                value: "l1_value".to_string(),
            }),
            Entry::ConfigGroup(level2_group),
        ],
    };

    kernel_config_groups.push(level1_group);

    let kernel_config = KernelConfig {
        time: TimeEnum::DateTime(Utc::now()),
        kernel_config_data: kernel_config_groups,
    };

    let raw_data = vec![Data::KernelConfig(kernel_config)];
    let mut kernel_config_processor = KernelConfig::new();
    let result = kernel_config_processor
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::KeyValue(key_value_data) = result {
        assert_eq!(key_value_data.key_value_groups.len(), 4);

        // Check each level of nesting
        assert!(key_value_data.key_value_groups.contains_key("Level1"));
        assert!(key_value_data
            .key_value_groups
            .contains_key("Level1:Level2"));
        assert!(key_value_data
            .key_value_groups
            .contains_key("Level1:Level2:Level3"));
        assert!(key_value_data
            .key_value_groups
            .contains_key("Level1:Level2:Level3:Level4"));

        // Check values at different levels
        let level1 = &key_value_data.key_value_groups["Level1"];
        assert_eq!(
            level1.key_values.get("CONFIG_L1_OPTION"),
            Some(&"l1_value".to_string())
        );

        let level3 = &key_value_data.key_value_groups["Level1:Level2:Level3"];
        assert_eq!(
            level3.key_values.get("CONFIG_L3_OPTION"),
            Some(&"l3_value".to_string())
        );

        let level4 = &key_value_data.key_value_groups["Level1:Level2:Level3:Level4"];
        assert_eq!(
            level4.key_values.get("CONFIG_DEEP_OPTION"),
            Some(&"deep_value".to_string())
        );
    } else {
        panic!("Expected KeyValue data");
    }
}
