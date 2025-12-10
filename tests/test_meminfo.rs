use aperf::data::meminfo::{MeminfoDataRaw, MeminfoType};
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::Utc;
use std::collections::HashMap;
use strum::IntoEnumIterator;

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

    fn set_stat(&mut self, meminfo_type: MeminfoType, value: u64) {
        self.stats.insert(meminfo_type.to_string(), value);
    }
}

fn generate_meminfo_raw_data(
    expected_per_sample_stats: &Vec<ExpectedMeminfoStats>,
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();

    for (sample_idx, expected_stats) in expected_per_sample_stats.iter().enumerate() {
        // Generate /proc/meminfo format data with all required fields
        let mut meminfo_data = String::new();

        // Helper function to get KB value or original value
        let get_value =
            |key: &str| -> u64 { expected_stats.stats.get(key).copied().unwrap_or(0) / 1024 };
        let get_original_value =
            |key: &str| -> u64 { expected_stats.stats.get(key).copied().unwrap_or(0) };

        // Generate in the order that /proc/meminfo typically appears
        meminfo_data.push_str(&format!("MemTotal:       {} kB\n", get_value("mem_total")));
        meminfo_data.push_str(&format!("MemFree:        {} kB\n", get_value("mem_free")));
        meminfo_data.push_str(&format!(
            "MemAvailable:   {} kB\n",
            get_value("mem_available")
        ));
        meminfo_data.push_str(&format!("Buffers:        {} kB\n", get_value("buffers")));
        meminfo_data.push_str(&format!("Cached:         {} kB\n", get_value("cached")));
        meminfo_data.push_str(&format!(
            "SwapCached:     {} kB\n",
            get_value("swap_cached")
        ));
        meminfo_data.push_str(&format!("Active:         {} kB\n", get_value("active")));
        meminfo_data.push_str(&format!("Inactive:       {} kB\n", get_value("inactive")));
        meminfo_data.push_str(&format!(
            "Active(anon):   {} kB\n",
            get_value("active_anon")
        ));
        meminfo_data.push_str(&format!(
            "Inactive(anon): {} kB\n",
            get_value("inactive_anon")
        ));
        meminfo_data.push_str(&format!(
            "Active(file):   {} kB\n",
            get_value("active_file")
        ));
        meminfo_data.push_str(&format!(
            "Inactive(file): {} kB\n",
            get_value("inactive_file")
        ));
        meminfo_data.push_str(&format!(
            "Unevictable:    {} kB\n",
            get_value("unevictable")
        ));
        meminfo_data.push_str(&format!("Mlocked:        {} kB\n", get_value("mlocked")));
        meminfo_data.push_str(&format!("MmapCopy:       {} kB\n", get_value("mmap_copy")));
        meminfo_data.push_str(&format!("SwapTotal:      {} kB\n", get_value("swap_total")));
        meminfo_data.push_str(&format!("SwapFree:       {} kB\n", get_value("swap_free")));
        meminfo_data.push_str(&format!("Dirty:          {} kB\n", get_value("dirty")));
        meminfo_data.push_str(&format!("Writeback:      {} kB\n", get_value("writeback")));
        meminfo_data.push_str(&format!("AnonPages:      {} kB\n", get_value("anon_pages")));
        meminfo_data.push_str(&format!("Mapped:         {} kB\n", get_value("mapped")));
        meminfo_data.push_str(&format!("Shmem:          {} kB\n", get_value("shmem")));
        meminfo_data.push_str(&format!(
            "KReclaimable:   {} kB\n",
            get_value("k_reclaimable")
        ));
        meminfo_data.push_str(&format!("Slab:           {} kB\n", get_value("slab")));
        meminfo_data.push_str(&format!(
            "SReclaimable:   {} kB\n",
            get_value("s_reclaimable")
        ));
        meminfo_data.push_str(&format!(
            "SUnreclaim:     {} kB\n",
            get_value("s_unreclaim")
        ));
        meminfo_data.push_str(&format!(
            "KernelStack:    {} kB\n",
            get_value("kernel_stack")
        ));
        meminfo_data.push_str(&format!(
            "PageTables:     {} kB\n",
            get_value("page_tables")
        ));
        meminfo_data.push_str(&format!("Quicklists:     {} kB\n", get_value("quicklists")));
        meminfo_data.push_str(&format!(
            "NFS_Unstable:   {} kB\n",
            get_value("nfs_unstable")
        ));
        meminfo_data.push_str(&format!("Bounce:         {} kB\n", get_value("bounce")));
        meminfo_data.push_str(&format!(
            "WritebackTmp:   {} kB\n",
            get_value("writeback_tmp")
        ));
        meminfo_data.push_str(&format!(
            "CommitLimit:    {} kB\n",
            get_value("commit_limit")
        ));
        meminfo_data.push_str(&format!(
            "Committed_AS:   {} kB\n",
            get_value("committed_as")
        ));
        meminfo_data.push_str(&format!(
            "VmallocTotal:   {} kB\n",
            get_value("vmalloc_total")
        ));
        meminfo_data.push_str(&format!(
            "VmallocUsed:    {} kB\n",
            get_value("vmalloc_used")
        ));
        meminfo_data.push_str(&format!(
            "VmallocChunk:   {} kB\n",
            get_value("vmalloc_chunk")
        ));
        meminfo_data.push_str(&format!("Percpu:         {} kB\n", get_value("per_cpu")));
        meminfo_data.push_str(&format!(
            "HardwareCorrupted: {} kB\n",
            get_value("hardware_corrupted")
        ));
        meminfo_data.push_str(&format!(
            "AnonHugePages:  {} kB\n",
            get_value("anon_hugepages")
        ));
        meminfo_data.push_str(&format!(
            "ShmemHugePages: {} kB\n",
            get_value("shmem_hugepages")
        ));
        meminfo_data.push_str(&format!(
            "ShmemPmdMapped: {} kB\n",
            get_value("shmem_pmd_mapped")
        ));
        meminfo_data.push_str(&format!(
            "FileHugePages:  {} kB\n",
            get_value("file_huge_pages")
        ));
        meminfo_data.push_str(&format!(
            "FilePmdMapped:  {} kB\n",
            get_value("file_pmd_mapped")
        ));
        meminfo_data.push_str(&format!("CmaTotal:       {} kB\n", get_value("cma_total")));
        meminfo_data.push_str(&format!("CmaFree:        {} kB\n", get_value("cma_free")));
        meminfo_data.push_str(&format!(
            "HugePages_Total:   {}\n",
            get_original_value("hugepages_total")
        ));
        meminfo_data.push_str(&format!(
            "HugePages_Free:    {}\n",
            get_original_value("hugepages_free")
        ));
        meminfo_data.push_str(&format!(
            "HugePages_Rsvd:    {}\n",
            get_original_value("hugepages_rsvd")
        ));
        meminfo_data.push_str(&format!(
            "HugePages_Surp:    {}\n",
            get_original_value("hugepages_surp")
        ));
        meminfo_data.push_str(&format!(
            "Hugepagesize:   {} kB\n",
            get_value("hugepagesize")
        ));
        meminfo_data.push_str(&format!("Hugetlb:        {} kB\n", get_value("hugetlb")));
        meminfo_data.push_str(&format!(
            "DirectMap4k:    {} kB\n",
            get_value("direct_map_4k")
        ));
        meminfo_data.push_str(&format!(
            "DirectMap4M:    {} kB\n",
            get_value("direct_map_4m")
        ));
        meminfo_data.push_str(&format!(
            "DirectMap2M:    {} kB\n",
            get_value("direct_map_2m")
        ));
        meminfo_data.push_str(&format!(
            "DirectMap1G:    {} kB\n",
            get_value("direct_map_1g")
        ));

        let time = TimeEnum::DateTime(
            Utc::now() + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64)),
        );

        let meminfo_raw = MeminfoDataRaw {
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
        expected_stats.set_stat(MeminfoType::MemTotal, 16777216 + sample_idx * 1024); // 16GB base
        expected_stats.set_stat(MeminfoType::MemFree, 8388608 - sample_idx * 2 * 1024); // Decreasing free memory
        expected_stats.set_stat(MeminfoType::MemAvailable, 10485760 - sample_idx * 1024 * 2);
        expected_stats.set_stat(MeminfoType::Buffers, 524288 + sample_idx * 1024);
        expected_stats.set_stat(MeminfoType::Cached, 2097152 + sample_idx * 1024 * 3);

        // Swap stats
        expected_stats.set_stat(MeminfoType::SwapTotal, 4194304); // 4GB swap
        expected_stats.set_stat(MeminfoType::SwapFree, 4194304 - sample_idx * 1024);
        expected_stats.set_stat(MeminfoType::SwapCached, sample_idx * 1024);

        // Active/Inactive memory
        expected_stats.set_stat(MeminfoType::Active, 4194304 + sample_idx * 1024 * 2);
        expected_stats.set_stat(MeminfoType::Inactive, 2097152 + sample_idx * 1024);
        expected_stats.set_stat(MeminfoType::ActiveAnon, 2097152 + sample_idx * 1024 * 3);
        expected_stats.set_stat(MeminfoType::InactiveAnon, 1048576 + sample_idx * 1024);

        // HugePages (count values, not converted)
        expected_stats.set_stat(MeminfoType::HugepagesTotal, 100 + sample_idx);
        expected_stats.set_stat(MeminfoType::HugepagesFree, 50 + sample_idx / 2);
        expected_stats.set_stat(MeminfoType::Hugepagesize, 2048); // 2MB pages

        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 2);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Check that we have metrics for all MeminfoType variants that have data
        let expected_metrics: Vec<String> = MeminfoType::iter().map(|t| t.to_string()).collect();
        assert_eq!(time_series_data.metrics.len(), expected_metrics.len());

        // Verify sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            expected_metrics.len()
        );
        assert_eq!(time_series_data.sorted_metric_names, expected_metrics);

        for metric_name in &expected_metrics {
            if let Some(metric) = time_series_data.metrics.get(metric_name) {
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
                        expected_stats.stats.get(metric_name).copied().unwrap_or(0) as f64;

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
        expected_stats.set_stat(MeminfoType::MemTotal, 8388608 + sample_idx * 1024); // 8GB base
        expected_stats.set_stat(MeminfoType::MemFree, 4194304 - sample_idx * 1024 * 2);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 1);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Should have metrics for all MeminfoType variants
        let expected_count = MeminfoType::iter().count();
        assert_eq!(time_series_data.metrics.len(), expected_count);
        assert_eq!(time_series_data.sorted_metric_names.len(), expected_count);

        // Check specific metrics that we set
        let mem_total = &time_series_data.metrics["mem_total"];
        assert_eq!(mem_total.series[0].values.len(), 3);
        assert!((mem_total.series[0].values[0] - 8388608.0).abs() < 1e-5);
        assert!((mem_total.series[0].values[1] - 8389632.0).abs() < 1e-5); // 8388608 + 1024
        assert!((mem_total.series[0].values[2] - 8390656.0).abs() < 1e-5); // 8388608 + 2048

        let mem_free = &time_series_data.metrics["mem_free"];
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
        expected_stats.set_stat(MeminfoType::HugepagesTotal, 100 + sample_idx * 10);
        expected_stats.set_stat(MeminfoType::HugepagesFree, 50 + sample_idx * 5);
        expected_stats.set_stat(MeminfoType::Hugepagesize, 2048); // Should not be converted
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 1);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Check hugepages metrics are not converted (should equal original values)
        let hugepages_total = &time_series_data.metrics["hugepages_total"];
        assert_eq!(hugepages_total.series[0].values[0], 100.0); // Not divided by 1024
        assert_eq!(hugepages_total.series[0].values[1], 110.0);
        assert_eq!(hugepages_total.series[0].values[2], 120.0);

        let hugepages_free = &time_series_data.metrics["hugepages_free"];
        assert_eq!(hugepages_free.series[0].values[0], 50.0); // Not divided by 1024
        assert_eq!(hugepages_free.series[0].values[1], 55.0);
        assert_eq!(hugepages_free.series[0].values[2], 60.0);

        let hugepagesize = &time_series_data.metrics["hugepagesize"];
        assert_eq!(hugepagesize.series[0].values[0], 2048.0); // Not divided by 1024
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_meminfo_empty_data() {
    let raw_data = Vec::new();
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        // Sorted metric names should still be initialized from enum
        let expected_count = MeminfoType::iter().count();
        assert_eq!(time_series_data.sorted_metric_names.len(), expected_count);
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
        expected_stats.set_stat(MeminfoType::MemTotal, 8388608);
        expected_stats.set_stat(MeminfoType::MemFree, 4194304);
        expected_stats.set_stat(MeminfoType::Buffers, 524288);
        expected_stats.set_stat(MeminfoType::Cached, 1048576);
        expected_stats.set_stat(MeminfoType::SwapTotal, 2097152);
        expected_stats.set_stat(MeminfoType::SwapFree, 2097152);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_meminfo_raw_data(&expected_per_sample_stats, 1);
    let mut meminfo = aperf::data::meminfo::MeminfoData::new();
    let result = meminfo
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Should still have all metrics (missing ones will have 0 values)
        let expected_count = MeminfoType::iter().count();
        assert_eq!(time_series_data.metrics.len(), expected_count);

        // Check that set fields have correct values
        let mem_total = &time_series_data.metrics["mem_total"];
        assert!((mem_total.series[0].values[0] - 8388608.0).abs() < 1e-5); // Raw kB value

        // Check that unset fields have 0 values (from missing data)
        let mem_available = &time_series_data.metrics["mem_available"];
        assert_eq!(mem_available.series[0].values[0], 0.0);
    } else {
        panic!("Expected TimeSeries data");
    }
}
