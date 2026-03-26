use aperf::data::memalloc::MemallocDataRaw;
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
struct ExpectedMemallocStats {
    pub buddyinfo: HashMap<String, HashMap<usize, f64>>, // zone -> order -> value
    pub pageblocks: HashMap<String, HashMap<String, f64>>, // zone -> migrate_type -> value
    pub pagetype: HashMap<String, HashMap<String, HashMap<usize, f64>>>, // zone -> migrate_type -> order -> value
    pub slabinfo: HashMap<String, HashMap<String, f64>>, // slab_name -> metric_type -> value
}

impl ExpectedMemallocStats {
    fn new() -> Self {
        Self::default()
    }

    fn set_buddyinfo(&mut self, zone: &str, order: usize, value: f64) {
        self.buddyinfo
            .entry(zone.to_string())
            .or_default()
            .insert(order, value);
    }

    fn set_pageblock(&mut self, zone: &str, migrate_type: &str, value: f64) {
        self.pageblocks
            .entry(zone.to_string())
            .or_default()
            .insert(migrate_type.to_string(), value);
    }

    fn set_pagetype(&mut self, zone: &str, migrate_type: &str, order: usize, value: f64) {
        self.pagetype
            .entry(zone.to_string())
            .or_default()
            .entry(migrate_type.to_string())
            .or_default()
            .insert(order, value);
    }

    fn set_slabinfo(&mut self, slab_name: &str, metric_type: &str, value: f64) {
        self.slabinfo
            .entry(slab_name.to_string())
            .or_default()
            .insert(metric_type.to_string(), value);
    }
}

fn generate_memalloc_raw_data(
    expected_per_sample_stats: &[ExpectedMemallocStats],
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();

    for (sample_idx, expected_stats) in expected_per_sample_stats.iter().enumerate() {
        let mut buddyinfo_data = String::new();
        for (zone, orders) in &expected_stats.buddyinfo {
            let mut line = format!("Node 0, zone {} ", zone);
            let max_order = orders.keys().max().copied().unwrap_or(0);
            for order in 0..=max_order {
                let value = orders.get(&order).copied().unwrap_or(0.0);
                line.push_str(&format!("{} ", value as u64));
            }
            buddyinfo_data.push_str(&line.trim_end());
            buddyinfo_data.push('\n');
        }

        let mut pagetypeinfo_data = String::new();
        for (zone, migrate_types) in &expected_stats.pagetype {
            for (migrate_type, orders) in migrate_types {
                let mut line = format!("Node 0, zone {}, type {} ", zone, migrate_type);
                let max_order = orders.keys().max().copied().unwrap_or(0);
                for order in 0..=max_order {
                    let value = orders.get(&order).copied().unwrap_or(0.0);
                    line.push_str(&format!("{} ", value as u64));
                }
                pagetypeinfo_data.push_str(&line.trim_end());
                pagetypeinfo_data.push('\n');
            }
        }

        for (zone, migrate_types) in &expected_stats.pageblocks {
            let mut line = format!("Node 0, zone {}, ", zone);
            for mt in &[
                "Unmovable",
                "Movable",
                "Reclaimable",
                "HighAtomic",
                "CMA",
                "Isolate",
            ] {
                let value = migrate_types.get(*mt).copied().unwrap_or(0.0);
                line.push_str(&format!("{} ", value as u64));
            }
            pagetypeinfo_data.push_str(&line.trim_end());
            pagetypeinfo_data.push('\n');
        }

        let mut slabinfo_data = String::from("slabinfo - version: 2.1\n# name            <active_objs> <num_objs> <objsize> <objperslab> <pagesperslab> : tunables <limit> <batchcount> <sharedfactor> : slabdata <active_slabs> <num_slabs> <sharedavail>\n");
        for (slab_name, metrics) in &expected_stats.slabinfo {
            let active_objs = metrics.get("active_objs").copied().unwrap_or(0.0) as u64;
            let num_objs = metrics.get("num_objs").copied().unwrap_or(0.0) as u64;
            let objsize = metrics.get("objsize").copied().unwrap_or(0.0) as u64;
            let objperslab = metrics.get("objperslab").copied().unwrap_or(0.0) as u64;
            let pagesperslab = metrics.get("pagesperslab").copied().unwrap_or(0.0) as u64;
            let limit = metrics.get("limit").copied().unwrap_or(0.0) as u64;
            let batchcount = metrics.get("batchcount").copied().unwrap_or(0.0) as u64;
            let sharedfactor = metrics.get("sharedfactor").copied().unwrap_or(0.0) as u64;
            let active_slabs = metrics.get("active_slabs").copied().unwrap_or(0.0) as u64;
            let num_slabs = metrics.get("num_slabs").copied().unwrap_or(0.0) as u64;
            let sharedavail = metrics.get("sharedavail").copied().unwrap_or(0.0) as u64;

            slabinfo_data.push_str(&format!(
                "{} {} {} {} {} {} : tunables {} {} {} : slabdata {} {} {}\n",
                slab_name,
                active_objs,
                num_objs,
                objsize,
                objperslab,
                pagesperslab,
                limit,
                batchcount,
                sharedfactor,
                active_slabs,
                num_slabs,
                sharedavail
            ));
        }

        let time = TimeEnum::DateTime(
            Utc::now() + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64)),
        );

        raw_data.push(Data::MemallocDataRaw(MemallocDataRaw {
            time,
            buddyinfo_data,
            pagetypeinfo_data,
            slabinfo_data,
        }));
    }

    raw_data
}

#[test]
fn test_process_memalloc_empty_data() {
    let raw_data = Vec::new();
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        assert_eq!(time_series_data.sorted_metric_names.len(), 0);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_buddyinfo_simple() {
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..3 {
        let mut expected_stats = ExpectedMemallocStats::new();
        expected_stats.set_buddyinfo("DMA", 0, 100.0 + sample_idx as f64 * 10.0);
        expected_stats.set_buddyinfo("DMA", 1, 200.0 + sample_idx as f64 * 20.0);
        expected_stats.set_buddyinfo("Normal", 0, 300.0 + sample_idx as f64 * 30.0);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_memalloc_raw_data(&expected_per_sample_stats, 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert!(time_series_data
            .metrics
            .contains_key("BuddyInfo order_0 (4KB)"));
        assert!(time_series_data
            .metrics
            .contains_key("BuddyInfo order_1 (8KB)"));

        let order_0 = &time_series_data.metrics["BuddyInfo order_0 (4KB)"];
        assert_eq!(order_0.series.len(), 3); // DMA and Normal zones + aggregate

        let dma_series = order_0
            .series
            .iter()
            .find(|s| s.series_name == "node_0_zone_DMA")
            .unwrap();
        assert_eq!(dma_series.values.len(), 3);
        assert!((dma_series.values[0] - 100.0).abs() < 1e-5);
        assert!((dma_series.values[1] - 110.0).abs() < 1e-5);
        assert!((dma_series.values[2] - 120.0).abs() < 1e-5);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_buddyinfo_complex() {
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..50 {
        let mut expected_stats = ExpectedMemallocStats::new();
        for order in 0..11 {
            expected_stats.set_buddyinfo("DMA", order, (100 + sample_idx * 10 + order * 5) as f64);
            expected_stats.set_buddyinfo(
                "Normal",
                order,
                (500 + sample_idx * 20 + order * 10) as f64,
            );
        }
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_memalloc_raw_data(&expected_per_sample_stats, 2);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        for order in 0..11 {
            let size_kb = 4 * (1 << order);
            let metric_name = if size_kb >= 1024 {
                format!("BuddyInfo order_{} ({}MB)", order, size_kb / 1024)
            } else {
                format!("BuddyInfo order_{} ({}KB)", order, size_kb)
            };
            assert!(time_series_data.metrics.contains_key(&metric_name));

            let metric = &time_series_data.metrics[&metric_name];
            assert_eq!(metric.series.len(), 3);

            for series in &metric.series {
                assert_eq!(series.values.len(), 50);
                for value in &series.values {
                    assert!(*value >= 0.0);
                }
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_pageblocks() {
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..5 {
        let mut expected_stats = ExpectedMemallocStats::new();
        expected_stats.set_pageblock("Normal", "Unmovable", 100.0 + sample_idx as f64 * 5.0);
        expected_stats.set_pageblock("Normal", "Movable", 200.0 + sample_idx as f64 * 10.0);
        expected_stats.set_pageblock("Normal", "Reclaimable", 50.0 + sample_idx as f64 * 2.0);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_memalloc_raw_data(&expected_per_sample_stats, 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert!(time_series_data
            .metrics
            .contains_key("PageBlocks - Unmovable"));
        assert!(time_series_data
            .metrics
            .contains_key("PageBlocks - Movable"));
        assert!(time_series_data
            .metrics
            .contains_key("PageBlocks - Reclaimable"));

        let unmovable = &time_series_data.metrics["PageBlocks - Unmovable"];
        assert_eq!(unmovable.series[0].values.len(), 5);
        assert!((unmovable.series[0].values[0] - 100.0).abs() < 1e-5);
        assert!((unmovable.series[0].values[4] - 120.0).abs() < 1e-5);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_pagetype() {
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..5 {
        let mut expected_stats = ExpectedMemallocStats::new();
        expected_stats.set_pagetype("Normal", "Unmovable", 0, 50.0 + sample_idx as f64 * 5.0);
        expected_stats.set_pagetype("Normal", "Unmovable", 1, 100.0 + sample_idx as f64 * 10.0);
        expected_stats.set_pagetype("Normal", "Movable", 0, 75.0 + sample_idx as f64 * 7.0);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_memalloc_raw_data(&expected_per_sample_stats, 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert!(time_series_data
            .metrics
            .contains_key("PageType Unmovable - order_0 (4KB)"));
        assert!(time_series_data
            .metrics
            .contains_key("PageType Unmovable - order_1 (8KB)"));
        assert!(time_series_data
            .metrics
            .contains_key("PageType Movable - order_0 (4KB)"));

        let unmovable_0 = &time_series_data.metrics["PageType Unmovable - order_0 (4KB)"];
        assert_eq!(unmovable_0.series[0].values.len(), 5);
        assert!((unmovable_0.series[0].values[0] - 50.0).abs() < 1e-5);
        assert!((unmovable_0.series[0].values[4] - 70.0).abs() < 1e-5);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_slabinfo_simple() {
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..3 {
        let mut expected_stats = ExpectedMemallocStats::new();
        expected_stats.set_slabinfo("dentry", "active_objs", 1000.0 + sample_idx as f64 * 100.0);
        expected_stats.set_slabinfo("dentry", "num_objs", 2000.0 + sample_idx as f64 * 200.0);
        expected_stats.set_slabinfo(
            "inode_cache",
            "active_objs",
            500.0 + sample_idx as f64 * 50.0,
        );
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_memalloc_raw_data(&expected_per_sample_stats, 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert!(time_series_data
            .metrics
            .contains_key("SlabInfo active objs"));
        assert!(time_series_data.metrics.contains_key("SlabInfo num objs"));

        let active_objs = &time_series_data.metrics["SlabInfo active objs"];
        assert_eq!(active_objs.series.len(), 3); // dentry and inode_cache + aggregate

        let dentry_series = active_objs
            .series
            .iter()
            .find(|s| s.series_name == "dentry")
            .unwrap();
        assert_eq!(dentry_series.values.len(), 3);
        assert!((dentry_series.values[0] - 1000.0).abs() < 1e-5);
        assert!((dentry_series.values[2] - 1200.0).abs() < 1e-5);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_slabinfo_complex() {
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..50 {
        let mut expected_stats = ExpectedMemallocStats::new();
        for slab in &["dentry", "inode_cache", "buffer_head", "kmalloc-512"] {
            expected_stats.set_slabinfo(slab, "active_objs", (1000 + sample_idx * 10) as f64);
            expected_stats.set_slabinfo(slab, "num_objs", (2000 + sample_idx * 20) as f64);
            expected_stats.set_slabinfo(slab, "objsize", 256.0);
            expected_stats.set_slabinfo(slab, "active_slabs", (50 + sample_idx) as f64);
        }
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_memalloc_raw_data(&expected_per_sample_stats, 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        let active_objs = &time_series_data.metrics["SlabInfo active objs"];
        assert_eq!(active_objs.series.len(), 5);

        for series in &active_objs.series {
            assert_eq!(series.values.len(), 50);
            for value in &series.values {
                assert!(*value >= 0.0);
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_all_data_types() {
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..10 {
        let mut expected_stats = ExpectedMemallocStats::new();

        // BuddyInfo
        for order in 0..5 {
            expected_stats.set_buddyinfo(
                "Normal",
                order,
                (100 + sample_idx * 10 + order * 5) as f64,
            );
        }

        // PageBlocks
        expected_stats.set_pageblock("Normal", "Unmovable", (200 + sample_idx * 5) as f64);
        expected_stats.set_pageblock("Normal", "Movable", (300 + sample_idx * 10) as f64);
        expected_stats.set_pageblock("Normal", "Reclaimable", (300 + sample_idx * 10) as f64);
        expected_stats.set_pageblock("Normal", "HighAtomic", (300 + sample_idx * 10) as f64);
        expected_stats.set_pageblock("Normal", "CMA", (300 + sample_idx * 10) as f64);
        expected_stats.set_pageblock("Normal", "Isolate", (300 + sample_idx * 10) as f64);

        // PageType
        expected_stats.set_pagetype("Normal", "Unmovable", 0, (50 + sample_idx * 2) as f64);
        expected_stats.set_pagetype("Normal", "Movable", 0, (75 + sample_idx * 3) as f64);

        // SlabInfo
        expected_stats.set_slabinfo("dentry", "active_objs", (1000 + sample_idx * 50) as f64);
        expected_stats.set_slabinfo("dentry", "num_objs", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "objsize", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "objperslab", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "pagesperslab", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "limit", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "batchcount", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "sharedfactor", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "active_slabs", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "num_slabs", (2000 + sample_idx * 100) as f64);
        expected_stats.set_slabinfo("dentry", "sharedavail", (2000 + sample_idx * 100) as f64);

        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_memalloc_raw_data(&expected_per_sample_stats, 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Verify all data types are present
        assert!(time_series_data
            .metrics
            .keys()
            .any(|k| k.starts_with("BuddyInfo")));
        assert!(time_series_data
            .metrics
            .keys()
            .any(|k| k.starts_with("PageBlocks")));
        assert!(time_series_data
            .metrics
            .keys()
            .any(|k| k.starts_with("PageType")));
        assert!(time_series_data
            .metrics
            .keys()
            .any(|k| k.starts_with("SlabInfo")));

        // Verify time progression
        for metric in time_series_data.metrics.values() {
            for series in &metric.series {
                assert_eq!(series.time_diff.len(), 10);
                assert_eq!(series.time_diff[0], 0);
                assert_eq!(series.time_diff[9], 9);
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_metric_name_formatting() {
    let mut expected_stats = ExpectedMemallocStats::new();
    expected_stats.set_buddyinfo("Normal", 0, 100.0);
    expected_stats.set_buddyinfo("Normal", 8, 200.0); // 1MB
    expected_stats.set_buddyinfo("Normal", 10, 300.0); // 4MB

    let raw_data = generate_memalloc_raw_data(&[expected_stats], 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert!(time_series_data
            .metrics
            .contains_key("BuddyInfo order_0 (4KB)"));
        assert!(time_series_data
            .metrics
            .contains_key("BuddyInfo order_8 (1MB)"));
        assert!(time_series_data
            .metrics
            .contains_key("BuddyInfo order_10 (4MB)"));
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_sorted_metric_names() {
    let mut expected_stats = ExpectedMemallocStats::new();
    expected_stats.set_buddyinfo("Normal", 0, 100.0);
    expected_stats.set_buddyinfo("Normal", 1, 200.0);
    expected_stats.set_pageblock("Normal", "Unmovable", 50.0);
    expected_stats.set_pagetype("Normal", "Movable", 0, 75.0);
    expected_stats.set_slabinfo("dentry", "active_objs", 1000.0);

    let raw_data = generate_memalloc_raw_data(&[expected_stats], 1);
    let mut memalloc = aperf::data::memalloc::MemallocData::new();
    let result = memalloc
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::common::data_formats::AperfData::TimeSeries(time_series_data) = result {
        let sorted_names = &time_series_data.sorted_metric_names;

        // BuddyInfo should come first, then PageBlocks
        let buddy_idx = sorted_names
            .iter()
            .position(|n| n.starts_with("BuddyInfo"))
            .unwrap();
        let pageblock_idx = sorted_names
            .iter()
            .position(|n| n.starts_with("PageBlocks"))
            .unwrap();

        assert!(buddy_idx < pageblock_idx);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_memalloc_archive_exists() {
    use std::path::PathBuf;

    let test_data_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/test_run_4.tar.gz");

    assert!(test_data_path.exists(), "test_run_4.tar.gz should exist");

    // Verify it contains memalloc data
    let output = std::process::Command::new("tar")
        .args(&["-tzf", test_data_path.to_str().unwrap()])
        .output()
        .unwrap();

    let contents = String::from_utf8(output.stdout).unwrap();
    assert!(
        contents.contains("memalloc_"),
        "Archive should contain memalloc data file"
    );
}
