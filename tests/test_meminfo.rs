use aperf::data::meminfo::MeminfoDataRaw;
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::data_processing::ReportParams;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
struct ExpectedMeminfoStats {
    pub stats: HashMap<String, u64>,
}

impl ExpectedMeminfoStats {
    fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    fn set_stat(&mut self, metric_name: &str, value: u64) {
        self.stats.insert(metric_name.to_string(), value);
    }
}

fn generate_meminfo_raw_data(
    expected_per_sample_stats: &Vec<ExpectedMeminfoStats>,
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();

    for (sample_idx, expected_stats) in expected_per_sample_stats.iter().enumerate() {
        // Generate /proc/meminfo format data with only the fields that have data
        let mut meminfo_data = String::new();

        // Helper function to get KB value or original value
        let _get_value =
            |key: &str| -> u64 { expected_stats.stats.get(key).copied().unwrap_or(0) / 1024 };
        let _get_original_value =
            |key: &str| -> u64 { expected_stats.stats.get(key).copied().unwrap_or(0) };

        // Only generate lines for metrics that have non-zero values
        for (metric_name, value) in &expected_stats.stats {
            match metric_name.as_str() {
                "HugePages_Total" | "HugePages_Free" | "HugePages_Rsvd" | "HugePages_Surp" => {
                    meminfo_data.push_str(&format!("{}:   {}\n", metric_name, value));
                }
                _ => {
                    let kb_value = value / 1024;
                    meminfo_data.push_str(&format!("{}:       {} kB\n", metric_name, kb_value));
                }
            }
        }

        let time = TimeEnum::DateTime(
            Utc::now() + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64)),
        );

        let meminfo_raw = MeminfoDataRaw {
            hugetlb_files: Vec::new(),
            time,
            data: meminfo_data,
        };
        raw_data.push(Data::MeminfoDataRaw(meminfo_raw));
    }

    raw_data
}

#[test]
fn test_process_meminfo_raw_data_complex() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate 50 samples with various meminfo patterns
    for sample_idx in 0..50 {
        let mut expected_stats = ExpectedMeminfoStats::new();

        // Memory stats in bytes
        expected_stats.set_stat("MemTotal", 16777216 + sample_idx * 1024); // 16GB base
        expected_stats.set_stat("MemFree", 8388608 - sample_idx * 2 * 1024); // Decreasing free memory
        expected_stats.set_stat("MemAvailable", 10485760 - sample_idx * 1024 * 2);
        expected_stats.set_stat("Buffers", 524288 + sample_idx * 1024);
        expected_stats.set_stat("Cached", 2097152 + sample_idx * 1024 * 3);

        // Swap stats
        expected_stats.set_stat("SwapTotal", 4194304); // 4GB swap
        expected_stats.set_stat("SwapFree", 4194304 - sample_idx * 1024);
        expected_stats.set_stat("SwapCached", sample_idx * 1024);

        // Active/Inactive memory
        expected_stats.set_stat("Active", 4194304 + sample_idx * 1024 * 2);
        expected_stats.set_stat("Inactive", 2097152 + sample_idx * 1024);
        expected_stats.set_stat("Active(anon)", 2097152 + sample_idx * 1024 * 3);
        expected_stats.set_stat("Inactive(anon)", 1048576 + sample_idx * 1024);

        // HugePages (count values, not converted)
        expected_stats.set_stat("HugePages_Total", 100 + sample_idx);
        expected_stats.set_stat("HugePages_Free", 50 + sample_idx / 2);
        expected_stats.set_stat("Hugepagesize", 2048); // 2MB pages

        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 2);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Check that we have metrics for the expected fields that have data
        let expected_metrics = vec![
            "MemTotal",
            "MemFree",
            "MemAvailable",
            "Buffers",
            "Cached",
            "SwapTotal",
            "SwapFree",
            "SwapCached",
            "Active",
            "Inactive",
            "Active(anon)",
            "Inactive(anon)",
            "HugePages_Total",
            "HugePages_Free",
            "Hugepagesize",
        ];

        // Verify we have at least the metrics we set
        for metric_name in &expected_metrics {
            assert!(
                time_series_data.metrics.contains_key(*metric_name),
                "Missing metric: {}",
                metric_name
            );
        }

        for metric_name in &expected_metrics {
            if let Some(metric) = time_series_data.metrics.get(*metric_name) {
                assert_eq!(
                    metric.series.len(),
                    1,
                    "Should have 1 series for {}",
                    metric_name
                );
                assert_eq!(
                    metric.series[0].values.len(),
                    50,
                    "Should have 50 data points for {}",
                    metric_name
                );

                // Check ALL values for this metric using expected data
                for sample_idx in 0..50 {
                    let expected_stats = &expected_per_sample_stats[sample_idx];
                    let expected_value =
                        expected_stats.stats.get(*metric_name).copied().unwrap_or(0) as f64;

                    assert!(
                        (metric.series[0].values[sample_idx] - expected_value).abs() < 1e-5,
                        "Metric {} sample {}: expected {}, got {}",
                        metric_name,
                        sample_idx,
                        expected_value,
                        metric.series[0].values[sample_idx]
                    );
                }

                // Verify all values are non-negative
                for value in &metric.series[0].values {
                    assert!(
                        *value >= 0.0,
                        "Value should be non-negative for {}",
                        metric_name
                    );
                }
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_meminfo_raw_data_simple() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate 3 samples with simple patterns
    for sample_idx in 0..3 {
        let mut expected_stats = ExpectedMeminfoStats::new();
        expected_stats.set_stat("MemTotal", 8388608 + sample_idx * 1024); // 8GB base
        expected_stats.set_stat("MemFree", 4194304 - sample_idx * 1024 * 2);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 1);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Should have metrics for the fields we set
        assert!(time_series_data.metrics.len() >= 2);
        assert!(time_series_data.sorted_metric_names.len() >= 2);

        // Check specific metrics that we set
        let mem_total = &time_series_data.metrics["MemTotal"];
        assert_eq!(mem_total.series[0].values.len(), 3);
        assert!((mem_total.series[0].values[0] - 8388608.0).abs() < 1e-5);
        assert!((mem_total.series[0].values[1] - 8389632.0).abs() < 1e-5); // 8388608 + 1024
        assert!((mem_total.series[0].values[2] - 8390656.0).abs() < 1e-5); // 8388608 + 2048

        let mem_free = &time_series_data.metrics["MemFree"];
        assert_eq!(mem_free.series[0].values.len(), 3);
        assert!((mem_free.series[0].values[0] - 4194304.0).abs() < 1e-5);
        assert!((mem_free.series[0].values[1] - 4192256.0).abs() < 1e-5); // 4194304 - 2 * 1024
        assert!((mem_free.series[0].values[2] - 4190208.0).abs() < 1e-5); // 4194304 - 4 * 1024

        // Check time progression
        for metric in time_series_data.metrics.values() {
            assert_eq!(metric.series[0].time_diff[0], 0);
            assert_eq!(metric.series[0].time_diff[1], 1);
            assert_eq!(metric.series[0].time_diff[2], 2);
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_meminfo_hugepages_no_conversion() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate 3 samples focusing on hugepages (count values)
    for sample_idx in 0..3 {
        let mut expected_stats = ExpectedMeminfoStats::new();
        expected_stats.set_stat("HugePages_Total", 100 + sample_idx * 10);
        expected_stats.set_stat("HugePages_Free", 50 + sample_idx * 5);
        expected_stats.set_stat("Hugepagesize", 2048); // Should not be converted
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 1);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Check hugepages metrics are not converted (should equal original values)
        let hugepages_total = &time_series_data.metrics["HugePages_Total"];
        assert_eq!(hugepages_total.series[0].values[0], 100.0); // Not divided by 1024
        assert_eq!(hugepages_total.series[0].values[1], 110.0);
        assert_eq!(hugepages_total.series[0].values[2], 120.0);

        let hugepages_free = &time_series_data.metrics["HugePages_Free"];
        assert_eq!(hugepages_free.series[0].values[0], 50.0); // Not divided by 1024
        assert_eq!(hugepages_free.series[0].values[1], 55.0);
        assert_eq!(hugepages_free.series[0].values[2], 60.0);

        let hugepagesize = &time_series_data.metrics["Hugepagesize"];
        assert_eq!(hugepagesize.series[0].values[0], 2048.0); // Not divided by 1024
    } else {
        panic!("Expected TimeSeries data");
    }
}

/// The per-size HugeTLB pool counters are appended to the raw /proc/meminfo data as synthetic
/// "Name: value" lines; like the default pool's HugePages_* lines, their bare counts must pass
/// through without a unit conversion, even though the metric names end in "kB".
#[test]
fn test_process_meminfo_per_size_hugetlb_counters() {
    let mut raw_data = Vec::new();
    for sample_idx in 0..3u64 {
        let data = format!(
            "MemTotal:       16384 kB\n\
             HugePages_Total:   100\n\
             HugePages_Total_2048kB: {}\n\
             HugePages_Free_2048kB: {}\n\
             HugePages_Rsvd_2048kB: 3\n\
             HugePages_Surp_2048kB: 1\n\
             HugePages_Total_1048576kB: 2\n",
            100 + sample_idx,
            5 + sample_idx,
        );
        let time = TimeEnum::DateTime(Utc::now() + chrono::Duration::seconds(sample_idx as i64));
        raw_data.push(Data::MeminfoDataRaw(MeminfoDataRaw {
            hugetlb_files: Vec::new(),
            time,
            data,
        }));
    }

    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // The sized line is converted to bytes while the counts pass through unchanged.
        let expected_values = vec![
            ("MemTotal", vec![16777216.0, 16777216.0, 16777216.0]),
            ("HugePages_Total", vec![100.0, 100.0, 100.0]),
            ("HugePages_Total_2048kB", vec![100.0, 101.0, 102.0]),
            ("HugePages_Free_2048kB", vec![5.0, 6.0, 7.0]),
            ("HugePages_Rsvd_2048kB", vec![3.0, 3.0, 3.0]),
            ("HugePages_Surp_2048kB", vec![1.0, 1.0, 1.0]),
            ("HugePages_Total_1048576kB", vec![2.0, 2.0, 2.0]),
        ];
        assert_eq!(time_series_data.metrics.len(), expected_values.len());
        for (metric_name, values) in expected_values {
            let metric = time_series_data
                .metrics
                .get(metric_name)
                .unwrap_or_else(|| panic!("Missing metric: {}", metric_name));
            assert_eq!(
                metric.series[0].values, values,
                "Unexpected values for {}",
                metric_name
            );
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_meminfo_empty_data() {
    let raw_data = Vec::new();
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        // Sorted metric names should be empty for empty data
        assert_eq!(time_series_data.sorted_metric_names.len(), 0);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_meminfo_missing_optional_fields() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate samples with only required fields (some fields are optional in /proc/meminfo)
    for _sample_idx in 0..3 {
        let mut expected_stats = ExpectedMeminfoStats::new();
        // Only set core required fields
        expected_stats.set_stat("MemTotal", 8388608);
        expected_stats.set_stat("MemFree", 4194304);
        expected_stats.set_stat("Buffers", 524288);
        expected_stats.set_stat("Cached", 1048576);
        expected_stats.set_stat("SwapTotal", 2097152);
        expected_stats.set_stat("SwapFree", 2097152);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 1);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Should have metrics for the fields we set
        let expected_metrics = vec![
            "MemTotal",
            "MemFree",
            "Buffers",
            "Cached",
            "SwapTotal",
            "SwapFree",
        ];
        assert_eq!(time_series_data.metrics.len(), expected_metrics.len());

        // Check that set fields have correct values
        let mem_total = &time_series_data.metrics["MemTotal"];
        assert!((mem_total.series[0].values[0] - 8388608.0).abs() < 1e-5); // Raw kB value

        // Check that unset fields are not present (since we only generate what we set)
        assert!(!time_series_data.metrics.contains_key("MemAvailable"));
    } else {
        panic!("Expected TimeSeries data");
    }
}

/// A pool that is mostly idle: 100 x 2MiB reserved, 60 free of which 10 are committed, so 50
/// pages x 2MiB = 100MiB is idle out of a 200MiB pool on a 1GiB host.
#[test]
fn test_process_meminfo_hugetlb_unused_derived_metrics() {
    let mut raw_data = Vec::new();
    for sample_idx in 0..3u64 {
        let data = "MemTotal:       1048576 kB\n\
             Hugetlb:         204800 kB\n\
             Hugepagesize:      2048 kB\n\
             HugePages_Total:   100\n\
             HugePages_Free:     60\n\
             HugePages_Rsvd:     10\n\
             HugePages_Total_2048kB: 100\n\
             HugePages_Free_2048kB: 60\n\
             HugePages_Rsvd_2048kB: 10\n"
            .to_string();
        let time = TimeEnum::DateTime(Utc::now() + chrono::Duration::seconds(sample_idx as i64));
        raw_data.push(Data::MeminfoDataRaw(MeminfoDataRaw {
            hugetlb_files: Vec::new(),
            time,
            data,
        }));
    }

    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // 50 idle pages x 2MiB = 100MiB out of 1GiB of memory.
        for (metric_name, expected) in [("Hugetlb_Unused_Memory_Percent", 100.0 / 1024.0 * 100.0)] {
            let metric = time_series_data
                .metrics
                .get(metric_name)
                .unwrap_or_else(|| panic!("Missing metric: {}", metric_name));
            for value in &metric.series[0].values {
                assert!(
                    (value - expected).abs() < 0.001,
                    "Unexpected {metric_name}: {value} instead of {expected}"
                );
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

/// Without a pool there is nothing idle, so the share of memory is zero and stays well under the
/// rule's threshold.
#[test]
fn test_process_meminfo_no_hugetlb_pool_reports_zero_unused() {
    let data = "MemTotal:       1048576 kB\n\
         Hugetlb:              0 kB\n\
         Hugepagesize:      2048 kB\n\
         HugePages_Total:     0\n\
         HugePages_Free:      0\n\
         HugePages_Rsvd:      0\n"
        .to_string();
    let raw_data = vec![Data::MeminfoDataRaw(MeminfoDataRaw {
        hugetlb_files: Vec::new(),
        time: TimeEnum::DateTime(Utc::now()),
        data,
    })];

    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        let memory_percent = time_series_data
            .metrics
            .get("Hugetlb_Unused_Memory_Percent")
            .expect("Missing Hugetlb_Unused_Memory_Percent");
        assert_eq!(memory_percent.series[0].values, vec![0.0]);
    } else {
        panic!("Expected TimeSeries data");
    }
}

/// Data recorded before the per-size counters existed still yields the derived metric, from the
/// unsuffixed counters and Hugepagesize.
#[test]
fn test_process_meminfo_hugetlb_unused_falls_back_to_default_size() {
    let data = "MemTotal:       1048576 kB\n\
         Hugetlb:         204800 kB\n\
         Hugepagesize:      2048 kB\n\
         HugePages_Total:   100\n\
         HugePages_Free:     60\n\
         HugePages_Rsvd:     10\n"
        .to_string();
    let raw_data = vec![Data::MeminfoDataRaw(MeminfoDataRaw {
        hugetlb_files: Vec::new(),
        time: TimeEnum::DateTime(Utc::now()),
        data,
    })];

    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // 50 idle pages x 2MiB = 100MiB out of 1GiB of memory.
        let memory_percent = time_series_data
            .metrics
            .get("Hugetlb_Unused_Memory_Percent")
            .expect("Missing Hugetlb_Unused_Memory_Percent");
        assert_eq!(
            memory_percent.series[0].values,
            vec![100.0 / 1024.0 * 100.0]
        );
    } else {
        panic!("Expected TimeSeries data");
    }
}
