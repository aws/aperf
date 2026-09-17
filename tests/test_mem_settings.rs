use aperf::data::common::data_formats::AperfData;
use aperf::data::mem_settings::{MemSettings, MemSettingsRaw};
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::data_processing::ReportParams;
use chrono::Utc;
use std::collections::HashMap;

/// Build a raw sample out of paths relative to /sys/kernel/mm and their verbatim contents. The
/// tests never read the host's own /sys/kernel/mm, so that they cover page sizes and values no
/// single host exposes and run on the platforms which have no /sys at all.
fn generate_mem_settings_raw_data(entries: &[(&str, &str)]) -> Vec<Data> {
    let mut mem_settings_data = HashMap::new();
    for (relative_path, value) in entries {
        mem_settings_data.insert(relative_path.to_string(), value.to_string());
    }
    vec![Data::MemSettingsRaw(MemSettingsRaw {
        time: TimeEnum::DateTime(Utc::now()),
        mem_settings_data,
    })]
}

/// Process the raw data and return every group, keyed by group name.
fn process_groups(raw_data: Vec<Data>) -> HashMap<String, HashMap<String, String>> {
    let mut mem_settings = MemSettings::new();
    let result = mem_settings
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::KeyValue(key_value_data) = result {
        key_value_data
            .key_value_groups
            .into_iter()
            .map(|(group, key_values)| (group, key_values.key_values))
            .collect()
    } else {
        panic!("Expected KeyValue data");
    }
}

/// The settings of every group together, for assertions that do not care which group a setting
/// is reported in. Keys are unique across groups, so flattening cannot lose one.
fn process(raw_data: Vec<Data>) -> HashMap<String, String> {
    process_groups(raw_data).into_values().flatten().collect()
}

fn process_entries(entries: &[(&str, &str)]) -> HashMap<String, String> {
    process(generate_mem_settings_raw_data(entries))
}

fn assert_value(key_values: &HashMap<String, String>, key: &str, value: &str) {
    assert_eq!(
        key_values.get(key),
        Some(&value.to_string()),
        "unexpected value for {key}"
    );
}

/// Everything an arm64 host with a 4kB base page size exposes: six transparent huge page
/// settings, nine multi-size transparent huge page directories (of which 8kB has no `enabled`,
/// since anonymous huge pages skip order 1), seven khugepaged settings and four hugetlb pools.
#[test]
fn test_process_mem_settings_raw_data_complex() {
    let key_values = process_entries(&[
        (
            "/sys/kernel/mm/transparent_hugepage/enabled",
            "always [madvise] never\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/defrag",
            "always defer defer+madvise [madvise] never\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/shmem_enabled",
            "always within_size advise [never] deny force\n",
        ),
        ("/sys/kernel/mm/transparent_hugepage/use_zero_page", "1\n"),
        (
            "/sys/kernel/mm/transparent_hugepage/hpage_pmd_size",
            "2097152\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/shrink_underused",
            "1\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-8kB/shmem_enabled",
            "always inherit within_size advise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-16kB/enabled",
            "always inherit madvise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-32kB/enabled",
            "always inherit madvise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-64kB/enabled",
            "always inherit madvise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-128kB/enabled",
            "always inherit madvise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-256kB/enabled",
            "always inherit madvise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-512kB/enabled",
            "always inherit madvise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-1024kB/enabled",
            "always inherit madvise [never]\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-2048kB/enabled",
            "always [inherit] madvise never\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/defrag",
            "1\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/max_ptes_none",
            "511\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/max_ptes_shared",
            "256\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/max_ptes_swap",
            "64\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/pages_to_scan",
            "4096\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/scan_sleep_millisecs",
            "10000\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/alloc_sleep_millisecs",
            "60000\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-64kB/nr_hugepages",
            "0\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-64kB/nr_overcommit_hugepages",
            "0\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages",
            "128\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_overcommit_hugepages",
            "0\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-32768kB/nr_hugepages",
            "0\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-32768kB/nr_overcommit_hugepages",
            "0\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages",
            "0\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-1048576kB/nr_overcommit_hugepages",
            "0\n",
        ),
    ]);

    assert_eq!(key_values.len(), 30);

    assert_value(&key_values, "transparent_hugepage/enabled", "madvise");
    assert_value(&key_values, "transparent_hugepage/defrag", "madvise");
    assert_value(&key_values, "transparent_hugepage/shmem_enabled", "never");
    assert_value(&key_values, "transparent_hugepage/use_zero_page", "1");
    assert_value(
        &key_values,
        "transparent_hugepage/hpage_pmd_size",
        "2097152",
    );
    assert_value(&key_values, "transparent_hugepage/shrink_underused", "1");
    // Order 1 has no anonymous policy, only a shared-memory one.
    assert!(!key_values.contains_key("transparent_hugepage/hugepages-8kB/enabled"));
    assert_value(
        &key_values,
        "transparent_hugepage/hugepages-8kB/shmem_enabled",
        "never",
    );
    assert_value(
        &key_values,
        "transparent_hugepage/hugepages-64kB/enabled",
        "never",
    );
    assert_value(
        &key_values,
        "transparent_hugepage/hugepages-2048kB/enabled",
        "inherit",
    );

    assert_value(&key_values, "transparent_hugepage/khugepaged/defrag", "1");
    assert_value(
        &key_values,
        "transparent_hugepage/khugepaged/max_ptes_none",
        "511",
    );
    assert_value(
        &key_values,
        "transparent_hugepage/khugepaged/max_ptes_shared",
        "256",
    );
    assert_value(
        &key_values,
        "transparent_hugepage/khugepaged/max_ptes_swap",
        "64",
    );
    assert_value(
        &key_values,
        "transparent_hugepage/khugepaged/pages_to_scan",
        "4096",
    );
    assert_value(
        &key_values,
        "transparent_hugepage/khugepaged/scan_sleep_millisecs",
        "10000",
    );
    assert_value(
        &key_values,
        "transparent_hugepage/khugepaged/alloc_sleep_millisecs",
        "60000",
    );

    assert_value(
        &key_values,
        "hugepages/hugepages-2048kB/nr_hugepages",
        "128",
    );
    assert_value(
        &key_values,
        "hugepages/hugepages-2048kB/nr_overcommit_hugepages",
        "0",
    );
    assert_value(&key_values, "hugepages/hugepages-64kB/nr_hugepages", "0");
    assert_value(
        &key_values,
        "hugepages/hugepages-1048576kB/nr_hugepages",
        "0",
    );
}

#[test]
fn test_process_mem_settings_bracketed_mode_normalization() {
    let expected_active_modes = [
        ("always [madvise] never\n", "madvise"),
        ("[always] madvise never\n", "always"),
        ("always madvise [never]\n", "never"),
        ("always defer defer+madvise [madvise] never\n", "madvise"),
        (
            "always defer [defer+madvise] madvise never\n",
            "defer+madvise",
        ),
        ("always within_size advise [never] deny force\n", "never"),
        ("always [inherit] madvise never\n", "inherit"),
        ("always inherit madvise [never]\n", "never"),
    ];

    for (raw_value, active_mode) in expected_active_modes {
        // Both the per-page-size files and their parents use the bracketed form.
        let key_values = process_entries(&[
            ("/sys/kernel/mm/transparent_hugepage/enabled", raw_value),
            (
                "/sys/kernel/mm/transparent_hugepage/hugepages-2048kB/enabled",
                raw_value,
            ),
        ]);

        assert_value(&key_values, "transparent_hugepage/enabled", active_mode);
        assert_value(
            &key_values,
            "transparent_hugepage/hugepages-2048kB/enabled",
            active_mode,
        );
    }
}

/// A value without a bracketed mode is reported as it is, apart from the trailing newline every
/// sysfs read comes with. khugepaged's `defrag` is a plain toggle while its parent directory's
/// `defrag` is a mode string, so the value's own format has to drive the normalization.
#[test]
fn test_process_mem_settings_plain_values_are_only_trimmed() {
    let key_values = process_entries(&[
        ("/sys/kernel/mm/transparent_hugepage/use_zero_page", "1\n"),
        (
            "/sys/kernel/mm/transparent_hugepage/hpage_pmd_size",
            "2097152\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/defrag",
            "1\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/max_ptes_swap",
            "  64 \n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages",
            "128\n",
        ),
    ]);

    assert_value(&key_values, "transparent_hugepage/use_zero_page", "1");
    assert_value(
        &key_values,
        "transparent_hugepage/hpage_pmd_size",
        "2097152",
    );
    assert_value(&key_values, "transparent_hugepage/khugepaged/defrag", "1");
    assert_value(
        &key_values,
        "transparent_hugepage/khugepaged/max_ptes_swap",
        "64",
    );
    assert_value(
        &key_values,
        "hugepages/hugepages-2048kB/nr_hugepages",
        "128",
    );
}

/// A setting that was collected but whose file could not be read still has to be reported,
/// without claiming any of the modes the kernel might have meant.
#[test]
fn test_process_mem_settings_unreadable_value_is_reported() {
    let key_values = process_entries(&[
        (
            "/sys/kernel/mm/transparent_hugepage/enabled",
            "always [madvise] never\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-8kB/shmem_enabled",
            "",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-16kB/enabled",
            "\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-2048kB/enabled",
            "always [inherit] madvise never\n",
        ),
    ]);

    for key in [
        "transparent_hugepage/hugepages-8kB/shmem_enabled",
        "transparent_hugepage/hugepages-16kB/enabled",
    ] {
        assert!(key_values.contains_key(key));
        assert_value(&key_values, key, "");
    }
    assert_value(&key_values, "transparent_hugepage/enabled", "madvise");
    assert_value(
        &key_values,
        "transparent_hugepage/hugepages-2048kB/enabled",
        "inherit",
    );
}

/// Keys are the sysfs path with the /sys/kernel/mm root stripped, reported in a group named after
/// the subsystem. The full directory path is kept so that the `defrag` of transparent_hugepage/
/// and of its khugepaged/ sub-directory stay distinct, and so that the hugepages-*kB directories
/// of the two subsystems, which share their naming, cannot be confused.
#[test]
fn test_process_mem_settings_keys_are_fully_qualified() {
    let groups = process_groups(generate_mem_settings_raw_data(&[
        (
            "/sys/kernel/mm/transparent_hugepage/defrag",
            "always defer defer+madvise [madvise] never\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/hugepages-2048kB/enabled",
            "always [inherit] madvise never\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/defrag",
            "1\n",
        ),
        (
            "/sys/kernel/mm/transparent_hugepage/khugepaged/pages_to_scan",
            "4096\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages",
            "0\n",
        ),
        // A path outside the two subsystems still gets reported, in the unnamed group.
        ("/sys/kernel/mm/ksm/run", "1\n"),
    ]));

    let mut group_names: Vec<&str> = groups.keys().map(String::as_str).collect();
    group_names.sort_unstable();
    assert_eq!(group_names, vec!["", "HugeTLB", "THP"]);
    assert_eq!(
        groups[""].keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["ksm/run"]
    );

    // The per-page-size directories of the two subsystems land in their own group despite the
    // shared hugepages-*kB naming.
    let mut thp: Vec<&str> = groups["THP"].keys().map(String::as_str).collect();
    thp.sort_unstable();
    assert_eq!(
        thp,
        vec![
            "transparent_hugepage/defrag",
            "transparent_hugepage/hugepages-2048kB/enabled",
            "transparent_hugepage/khugepaged/defrag",
            "transparent_hugepage/khugepaged/pages_to_scan",
        ]
    );
    assert_eq!(
        groups["HugeTLB"]
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["hugepages/hugepages-2048kB/nr_hugepages"]
    );

    // The two defrag settings keep their own values.
    let key_values: HashMap<String, String> = groups.into_values().flatten().collect();
    assert_value(&key_values, "transparent_hugepage/defrag", "madvise");
    assert_value(&key_values, "transparent_hugepage/khugepaged/defrag", "1");

    // An unqualified name is never a key on its own.
    for key in ["defrag", "pages_to_scan", "nr_hugepages"] {
        assert!(!key_values.contains_key(key));
    }
}

/// A kernel built without transparent huge pages still reports its hugetlb pools.
#[test]
fn test_process_mem_settings_missing_subsystems() {
    let key_values = process_entries(&[
        (
            "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages",
            "0\n",
        ),
        (
            "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_overcommit_hugepages",
            "0\n",
        ),
    ]);

    assert_eq!(key_values.len(), 2);
    assert_value(&key_values, "hugepages/hugepages-2048kB/nr_hugepages", "0");
}

#[test]
fn test_process_mem_settings_empty_data() {
    let key_values = process(Vec::new());

    assert!(key_values.is_empty());
}

/// Static data is collected once, but a raw file holding more than one sample must not produce
/// duplicated keys.
#[test]
fn test_process_mem_settings_multiple_samples() {
    let mut raw_data = generate_mem_settings_raw_data(&[(
        "/sys/kernel/mm/transparent_hugepage/enabled",
        "always [madvise] never\n",
    )]);
    raw_data.extend(generate_mem_settings_raw_data(&[(
        "/sys/kernel/mm/transparent_hugepage/enabled",
        "always madvise [never]\n",
    )]));

    let key_values = process(raw_data);

    assert_eq!(key_values.len(), 1);
    assert_value(&key_values, "transparent_hugepage/enabled", "never");
}
