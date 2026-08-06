use aperf::data::common::data_formats::AperfData;
use aperf::data::processes::{Processes, ProcessesRaw};
use aperf::data::ProcessData;
use aperf::data::{Data, TimeEnum};
use aperf::data_processing::ReportParams;
use aperf::ProcessMetric;
use chrono::prelude::*;
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;

/// Page size passed to the processing path via ReportParams. ResidentSetSize is
/// reported in pages and converted to bytes when the page size is known, so
/// expectations multiply the page-count stats by this value.
const PAGE_SIZE: u64 = 4096;

/// ReportParams carrying the page size that a modern recording captures in its
/// metadata, driving the pages-to-bytes conversion for ResidentSetSize.
fn report_params_with_page_size(page_size: u64) -> ReportParams {
    let mut params = ReportParams::new();
    params.page_size = page_size;
    params
}

struct ExpectedProcessStats {
    pub user_space_time: u64,
    pub kernel_space_time: u64,
    pub number_threads: u64,
    pub virtual_memory_size: u64,
    pub resident_set_size: u64,
}

impl Default for ExpectedProcessStats {
    fn default() -> Self {
        ExpectedProcessStats {
            user_space_time: 0,
            kernel_space_time: 0,
            number_threads: 1,
            virtual_memory_size: 1000000,
            resident_set_size: 500000,
        }
    }
}

fn generate_processes_raw_data(
    expected_per_sample_per_process_stats: &Vec<HashMap<String, ExpectedProcessStats>>, // [sample][process_name_pid]
    interval_seconds: u64,
    ticks_per_second: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();
    let base_time = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    for (sample_idx, expected_stats) in expected_per_sample_per_process_stats.iter().enumerate() {
        let time = TimeEnum::DateTime(
            base_time + chrono::Duration::seconds((sample_idx as u64 * interval_seconds) as i64),
        );

        let mut data_lines = Vec::new();

        for (process_name_pid, stats) in expected_stats {
            // Parse process name and PID from process_name_pid format "pid_name"
            let parts: Vec<&str> = process_name_pid.splitn(2, '_').collect();
            let (pid, name) = if parts.len() == 2 {
                (parts[0], parts[1])
            } else {
                ("1", process_name_pid.as_str())
            };

            // Generate /proc/pid/stat format line
            // Format: pid (name) state ppid pgrp session tty_nr tpgid flags minflt cminflt majflt cmajflt utime stime cutime cstime priority nice num_threads itrealvalue starttime vsize rss ...
            let line = format!(
                "{} ({}) S 0 0 0 0 0 0 0 0 0 0 {} {} 0 0 0 0 {} 0 0 {} {} 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
                pid, name,
                stats.user_space_time, stats.kernel_space_time,
                stats.number_threads,
                stats.virtual_memory_size, stats.resident_set_size
            );
            data_lines.push(line);
        }

        let processes_raw = ProcessesRaw {
            time,
            ticks_per_second,
            data: data_lines.join("\n"),
        };

        raw_data.push(Data::ProcessesRaw(processes_raw));
    }

    raw_data
}

#[test]
fn test_process_processes_raw_data_complex() {
    let ticks_per_second = 100;

    let mut expected_per_sample_per_process_stats = Vec::new();

    // Generate 50 samples with 3 processes
    for sample in 0..50 {
        let mut sample_stats = HashMap::new();

        // Process 1: High CPU usage
        let mut proc1_stats = ExpectedProcessStats::default();
        proc1_stats.user_space_time = 1000 + sample * 50; // High CPU growth
        proc1_stats.kernel_space_time = 500 + sample * 25;
        proc1_stats.number_threads = 4;
        proc1_stats.virtual_memory_size = 2000000 + sample * 1000;
        proc1_stats.resident_set_size = 1000000 + sample * 500;
        sample_stats.insert("1234_nginx".to_string(), proc1_stats);

        // Process 2: Medium CPU usage
        let mut proc2_stats = ExpectedProcessStats::default();
        proc2_stats.user_space_time = 500 + sample * 20;
        proc2_stats.kernel_space_time = 200 + sample * 10;
        proc2_stats.number_threads = 2;
        proc2_stats.virtual_memory_size = 1500000 + sample * 500;
        proc2_stats.resident_set_size = 750000 + sample * 250;
        sample_stats.insert("5678_apache".to_string(), proc2_stats);

        // Process 3: Low CPU usage
        let mut proc3_stats = ExpectedProcessStats::default();
        proc3_stats.user_space_time = 100 + sample * 5;
        proc3_stats.kernel_space_time = 50 + sample * 2;
        proc3_stats.number_threads = 1;
        proc3_stats.virtual_memory_size = 800000 + sample * 100;
        proc3_stats.resident_set_size = 400000 + sample * 50;
        sample_stats.insert("9999_sshd".to_string(), proc3_stats);

        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data =
        generate_processes_raw_data(&expected_per_sample_per_process_stats, 2, ticks_per_second);

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&report_params_with_page_size(PAGE_SIZE), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Validate structure
        assert_eq!(
            time_series_data.metrics.len(),
            ProcessMetric::iter().count()
        );
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessMetric::iter().count()
        );

        // Check each process key metric exists
        for process_key in ProcessMetric::iter() {
            let metric_name = process_key.to_string();
            assert!(time_series_data.metrics.contains_key(&metric_name));

            let metric = &time_series_data.metrics[&metric_name];
            assert_eq!(metric.metric_name, metric_name);

            if process_key == ProcessMetric::NumberProcesses {
                // System-wide aggregate: a single series with one point per
                // sample, each equal to the number of live processes (3).
                assert_eq!(metric.series.len(), 1);
                let series = &metric.series[0];
                assert_eq!(series.values.len(), 50);
                assert_eq!(series.time_diff.len(), 50);
                assert!(series.values.iter().all(|&v| v == 3.0));
                continue;
            }

            // Should have 3 series (one per process)
            assert_eq!(metric.series.len(), 3);

            // Each series should have 50 data points
            for series in &metric.series {
                assert_eq!(series.values.len(), 50);
                assert_eq!(series.time_diff.len(), 50);
            }
        }

        // Validate sorted metric names
        let expected_metrics: Vec<String> = ProcessMetric::iter().map(|k| k.to_string()).collect();
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            expected_metrics.len()
        );
        for expected_metric in expected_metrics {
            assert!(time_series_data
                .sorted_metric_names
                .contains(&expected_metric));
        }

        // Validate all per process data using expected values
        for process_key in ProcessMetric::iter() {
            if process_key == ProcessMetric::NumberProcesses {
                continue;
            }
            let metric_name = process_key.to_string();
            let metric = &time_series_data.metrics[&metric_name];

            for series in &metric.series {
                let process_name = &series.series_name;

                for (sample_idx, &value) in series.values.iter().enumerate() {
                    // First sample should be 0 for CPU metrics
                    if sample_idx == 0
                        && matches!(
                            process_key,
                            ProcessMetric::UserSpaceTime | ProcessMetric::KernelSpaceTime
                        )
                    {
                        assert_eq!(
                            value, 0.0,
                            "First sample should be 0 for CPU metric {} process {}",
                            metric_name, process_name
                        );
                        continue;
                    }

                    // Get expected stats for this sample and process
                    if let Some(expected_stats) =
                        expected_per_sample_per_process_stats[sample_idx].get(process_name)
                    {
                        let expected_value = match process_key {
                            ProcessMetric::UserSpaceTime => {
                                if sample_idx == 0 {
                                    0.0
                                } else {
                                    let prev_stats = &expected_per_sample_per_process_stats
                                        [sample_idx - 1][process_name];
                                    let delta =
                                        expected_stats.user_space_time - prev_stats.user_space_time;
                                    (delta as f64) / (ticks_per_second as f64 * 2.0)
                                }
                            }
                            ProcessMetric::KernelSpaceTime => {
                                if sample_idx == 0 {
                                    0.0
                                } else {
                                    let prev_stats = &expected_per_sample_per_process_stats
                                        [sample_idx - 1][process_name];
                                    let delta = expected_stats.kernel_space_time
                                        - prev_stats.kernel_space_time;
                                    (delta as f64) / (ticks_per_second as f64 * 2.0)
                                }
                            }
                            ProcessMetric::NumberThreads => expected_stats.number_threads as f64,
                            ProcessMetric::VirtualMemorySize => {
                                expected_stats.virtual_memory_size as f64
                            }
                            ProcessMetric::ResidentSetSize => {
                                expected_stats.resident_set_size as f64
                            }
                            ProcessMetric::ResidentSetSizeBytes => {
                                (expected_stats.resident_set_size * PAGE_SIZE) as f64
                            }
                            ProcessMetric::NumberProcesses => unreachable!("skipped above"),
                        };

                        assert!(
                            (value - expected_value).abs() < 1e-5,
                            "Metric {} process {} sample {}: expected {}, got {}",
                            metric_name,
                            process_name,
                            sample_idx,
                            expected_value,
                            value
                        );
                    }
                }
            }
        }

        // Validate sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessMetric::iter().count()
        );
        for process_key in ProcessMetric::iter() {
            assert!(time_series_data
                .sorted_metric_names
                .contains(&process_key.to_string()));
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_raw_data_simple() {
    let ticks_per_second = 100;

    let mut expected_per_sample_per_process_stats = Vec::new();

    // Generate 3 samples with 1 process
    for sample in 0..3 {
        let mut sample_stats = HashMap::new();

        let mut proc_stats = ExpectedProcessStats::default();
        proc_stats.user_space_time = 1000 + sample * 100;
        proc_stats.kernel_space_time = 500 + sample * 50;
        proc_stats.number_threads = 2;
        proc_stats.virtual_memory_size = 1000000;
        proc_stats.resident_set_size = 500000;
        sample_stats.insert("1_test_proc".to_string(), proc_stats);

        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data =
        generate_processes_raw_data(&expected_per_sample_per_process_stats, 1, ticks_per_second);

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&report_params_with_page_size(PAGE_SIZE), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(
            time_series_data.metrics.len(),
            ProcessMetric::iter().count()
        );

        // Validate all data using expected values
        for process_key in ProcessMetric::iter() {
            let metric_name = process_key.to_string();
            let metric = &time_series_data.metrics[&metric_name];

            assert_eq!(metric.series.len(), 1);
            let series = &metric.series[0];
            assert_eq!(series.values.len(), 3);

            if process_key == ProcessMetric::NumberProcesses {
                // One live process per sample, so the count is always 1.
                assert!(series.values.iter().all(|&v| v == 1.0));
                continue;
            }

            let process_name = &series.series_name;

            for (sample_idx, &value) in series.values.iter().enumerate() {
                // Get expected stats for this sample and process
                let expected_stats =
                    &expected_per_sample_per_process_stats[sample_idx][process_name];

                let expected_value = match process_key {
                    ProcessMetric::UserSpaceTime => {
                        if sample_idx == 0 {
                            0.0
                        } else {
                            let prev_stats = &expected_per_sample_per_process_stats[sample_idx - 1]
                                [process_name];
                            let delta = expected_stats.user_space_time - prev_stats.user_space_time;
                            (delta as f64) / (ticks_per_second as f64 * 1.0)
                        }
                    }
                    ProcessMetric::KernelSpaceTime => {
                        if sample_idx == 0 {
                            0.0
                        } else {
                            let prev_stats = &expected_per_sample_per_process_stats[sample_idx - 1]
                                [process_name];
                            let delta =
                                expected_stats.kernel_space_time - prev_stats.kernel_space_time;
                            (delta as f64) / (ticks_per_second as f64 * 1.0)
                        }
                    }
                    ProcessMetric::NumberThreads => expected_stats.number_threads as f64,
                    ProcessMetric::VirtualMemorySize => expected_stats.virtual_memory_size as f64,
                    ProcessMetric::ResidentSetSize => expected_stats.resident_set_size as f64,
                    ProcessMetric::ResidentSetSizeBytes => {
                        (expected_stats.resident_set_size * PAGE_SIZE) as f64
                    }
                    ProcessMetric::NumberProcesses => unreachable!("skipped above"),
                };

                assert!(
                    (value - expected_value).abs() < 1e-5,
                    "Metric {} sample {}: expected {}, got {}",
                    metric_name,
                    sample_idx,
                    expected_value,
                    value
                );
            }
        }

        // Validate sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessMetric::iter().count()
        );
        for process_key in ProcessMetric::iter() {
            assert!(time_series_data
                .sorted_metric_names
                .contains(&process_key.to_string()));
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_dynamic_processes() {
    let ticks_per_second = 100;

    let mut expected_per_sample_per_process_stats = Vec::new();

    // Generate 30 samples with processes appearing at different times
    for sample in 0..30 {
        let mut sample_stats = HashMap::new();

        // Process 1: appears from start
        let mut proc1_stats = ExpectedProcessStats::default();
        proc1_stats.user_space_time = 1000 + sample * 10;
        proc1_stats.kernel_space_time = 500 + sample * 5;
        sample_stats.insert("100_proc1".to_string(), proc1_stats);

        // Process 2: appears after sample 10
        if sample >= 10 {
            let mut proc2_stats = ExpectedProcessStats::default();
            proc2_stats.user_space_time = 2000 + (sample - 10) * 20;
            proc2_stats.kernel_space_time = 1000 + (sample - 10) * 10;
            sample_stats.insert("200_proc2".to_string(), proc2_stats);
        }

        // Process 3: appears after sample 20
        if sample >= 20 {
            let mut proc3_stats = ExpectedProcessStats::default();
            proc3_stats.user_space_time = 3000 + (sample - 20) * 30;
            proc3_stats.kernel_space_time = 1500 + (sample - 20) * 15;
            sample_stats.insert("300_proc3".to_string(), proc3_stats);
        }

        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data =
        generate_processes_raw_data(&expected_per_sample_per_process_stats, 1, ticks_per_second);

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&report_params_with_page_size(PAGE_SIZE), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Should have all process keys
        assert_eq!(
            time_series_data.metrics.len(),
            ProcessMetric::iter().count()
        );

        // Check that we have 3 processes (top 16 includes all)
        let user_space_metric = &time_series_data.metrics["user_space_time"];
        assert_eq!(user_space_metric.series.len(), 3);

        // Validate series lengths match process appearance timing
        let mut series_lengths: Vec<usize> = user_space_metric
            .series
            .iter()
            .map(|s| s.values.len())
            .collect();
        series_lengths.sort();

        // Should have series of lengths 30, 20, and 10 (or similar based on ranking)
        assert!(series_lengths.iter().any(|&len| len == 30)); // proc1 from start

        // Validate sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessMetric::iter().count()
        );
        for process_key in ProcessMetric::iter() {
            assert!(time_series_data
                .sorted_metric_names
                .contains(&process_key.to_string()));
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_top_16_ranking_simple() {
    let ticks_per_second = 100;

    let mut expected_per_sample_per_process_stats = Vec::new();

    // Generate 5 samples with 20 processes (more than top 16 limit)
    for sample in 0..5 {
        let mut sample_stats = HashMap::new();

        for proc_id in 1..=20 {
            let mut proc_stats = ExpectedProcessStats::default();
            // Give different CPU usage levels - higher proc_id = higher CPU
            proc_stats.user_space_time = 1000 + sample * (proc_id * 10);
            proc_stats.kernel_space_time = 500 + sample * (proc_id * 5);
            sample_stats.insert(format!("{}_proc", proc_id), proc_stats);
        }

        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data =
        generate_processes_raw_data(&expected_per_sample_per_process_stats, 1, ticks_per_second);

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&report_params_with_page_size(PAGE_SIZE), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Should have all process keys
        assert_eq!(
            time_series_data.metrics.len(),
            ProcessMetric::iter().count()
        );

        // Should only have top 16 processes
        let user_space_metric = &time_series_data.metrics["user_space_time"];
        assert_eq!(user_space_metric.series.len(), 16);

        // Each series should have 5 data points
        for series in &user_space_metric.series {
            assert_eq!(series.values.len(), 5);
        }

        let number_processes_metric =
            &time_series_data.metrics[&ProcessMetric::NumberProcesses.to_string()];
        assert_eq!(number_processes_metric.series.len(), 1);
        let number_processes_series = &number_processes_metric.series[0];
        assert_eq!(number_processes_series.values.len(), 5);
        assert!(number_processes_series.values.iter().all(|&v| v == 20.0));

        // Validate sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessMetric::iter().count()
        );
        for process_key in ProcessMetric::iter() {
            assert!(time_series_data
                .sorted_metric_names
                .contains(&process_key.to_string()));
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_top_16_ranking_complex() {
    let ticks_per_second = 100;

    let mut expected_per_sample_per_process_stats = Vec::new();

    // Generate 5 samples with 20 processes (more than top 16 limit)
    for sample in 0..5 {
        let mut sample_stats = HashMap::new();

        for proc_id in 1..=20 {
            let mut proc_stats = ExpectedProcessStats::default();
            // Create mixed usage patterns using different formulas per sample
            // Base values that vary by process
            let user_base = match proc_id % 3 {
                0 => proc_id * 100,
                1 => proc_id * proc_id + 200,
                _ => proc_id * 50 + 300,
            };

            let kernel_base = match proc_id % 4 {
                0 => (21 - proc_id) * 80,
                1 => proc_id * 60 + 100,
                2 => proc_id * 40 + 200,
                _ => proc_id * 30 + 150,
            };

            // Cumulative increases per sample
            let user_increment = match proc_id % 3 {
                0 => 50 + proc_id * 2,
                1 => 30 + proc_id * 3,
                _ => 40 + proc_id,
            };

            let kernel_increment = match proc_id % 4 {
                0 => 25 + proc_id,
                1 => 35 + proc_id * 2,
                2 => 20 + proc_id * 3,
                _ => 45 + proc_id,
            };

            proc_stats.user_space_time = user_base + sample * user_increment;
            proc_stats.kernel_space_time = kernel_base + sample * kernel_increment;
            sample_stats.insert(format!("{}_proc", proc_id), proc_stats);
        }

        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data =
        generate_processes_raw_data(&expected_per_sample_per_process_stats, 1, ticks_per_second);

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&report_params_with_page_size(PAGE_SIZE), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Should have all process keys
        assert_eq!(
            time_series_data.metrics.len(),
            ProcessMetric::iter().count()
        );

        // Should only have top 16 processes
        let user_space_metric = &time_series_data.metrics["user_space_time"];
        assert_eq!(user_space_metric.series.len(), 16);

        // Each series should have 5 data points
        for series in &user_space_metric.series {
            assert_eq!(series.values.len(), 5);
        }

        // The included processes are the top 16 by cumulative CPU ticks (the series within
        // the metric themselves are sorted by name, not by rank). Compute the expected set
        // from the last (largest, as the counters are monotonic) generated sample.
        let mut expected_totals: Vec<(String, u64)> = expected_per_sample_per_process_stats
            .last()
            .unwrap()
            .iter()
            .map(|(name, stats)| {
                (
                    name.clone(),
                    stats.user_space_time + stats.kernel_space_time,
                )
            })
            .collect();
        expected_totals.sort_by(|a, b| b.1.cmp(&a.1));
        let expected_included: HashSet<String> = expected_totals
            .iter()
            .take(16)
            .map(|(name, _)| name.clone())
            .collect();
        let included: HashSet<String> = user_space_metric
            .series
            .iter()
            .map(|s| s.series_name.clone())
            .collect();
        assert_eq!(included, expected_included);

        // Validate sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessMetric::iter().count()
        );
        for process_key in ProcessMetric::iter() {
            assert!(time_series_data
                .sorted_metric_names
                .contains(&process_key.to_string()));
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_empty_data() {
    let raw_data = Vec::new();

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        assert_eq!(time_series_data.sorted_metric_names.len(), 0);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_legacy_page_size_zero() {
    let ticks_per_second = 100;
    let rss_pages = 500000;

    let mut expected_per_sample_per_process_stats = Vec::new();
    for _ in 0..3 {
        let mut sample_stats = HashMap::new();
        let mut proc_stats = ExpectedProcessStats::default();
        proc_stats.resident_set_size = rss_pages;
        sample_stats.insert("1_test_proc".to_string(), proc_stats);
        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data =
        generate_processes_raw_data(&expected_per_sample_per_process_stats, 1, ticks_per_second);

    // A legacy run has no captured page size (ReportParams::new defaults it to 0).
    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // The pages metric is present; the bytes variant carries no data on a
        // legacy run, so it is absent from the metrics map.
        let pages_name = ProcessMetric::ResidentSetSize.to_string();
        let bytes_name = ProcessMetric::ResidentSetSizeBytes.to_string();
        assert_eq!(pages_name, "resident_set_size");
        assert_eq!(bytes_name, "resident_set_size_bytes");
        assert!(time_series_data.metrics.contains_key(&pages_name));
        assert!(!time_series_data.metrics.contains_key(&bytes_name));

        // Values remain in pages (no multiplication by page size).
        let metric = &time_series_data.metrics[&pages_name];
        for series in &metric.series {
            for &value in &series.values {
                assert!((value - rss_pages as f64).abs() < 1e-5);
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_rss_bytes_and_pages_coexist() {
    let ticks_per_second = 100;
    let rss_pages = 500000;

    let mut expected_per_sample_per_process_stats = Vec::new();
    for _ in 0..3 {
        let mut sample_stats = HashMap::new();
        let mut proc_stats = ExpectedProcessStats::default();
        proc_stats.resident_set_size = rss_pages;
        sample_stats.insert("1_test_proc".to_string(), proc_stats);
        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data =
        generate_processes_raw_data(&expected_per_sample_per_process_stats, 1, ticks_per_second);

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&report_params_with_page_size(PAGE_SIZE), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        let pages_metric = &time_series_data.metrics[&ProcessMetric::ResidentSetSize.to_string()];
        for series in &pages_metric.series {
            for &value in &series.values {
                assert!((value - rss_pages as f64).abs() < 1e-5);
            }
        }

        let bytes_metric =
            &time_series_data.metrics[&ProcessMetric::ResidentSetSizeBytes.to_string()];
        for series in &bytes_metric.series {
            for &value in &series.values {
                assert!((value - (rss_pages * PAGE_SIZE) as f64).abs() < 1e-5);
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

/// Builds one ProcessesRaw snapshot at `base + offset_ms` with a single process line.
fn processes_raw_at_ms(offset_ms: i64, utime: u64, stime: u64) -> Data {
    let base_time = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
    Data::ProcessesRaw(ProcessesRaw {
        time: TimeEnum::DateTime(base_time + chrono::Duration::milliseconds(offset_ms)),
        ticks_per_second: 100,
        data: format!(
            "1 (proc) S 0 0 0 0 0 0 0 0 0 0 {} {} 0 0 0 0 1 0 0 1000000 500000 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
            utime, stime
        ),
    })
}

#[test]
fn test_process_processes_duplicate_time_diff_keeps_last() {
    // The finish-stage extra collection can land within the same rounded second as the
    // last interval sample; only the later snapshot must be kept.
    let raw_data = vec![
        processes_raw_at_ms(0, 100, 50),
        processes_raw_at_ms(1000, 200, 100),
        // 1300ms rounds to the same time_diff (1s) as the 1000ms sample: replaces it.
        processes_raw_at_ms(1300, 300, 150),
    ];

    let mut processes = Processes::new();
    let result = processes
        .process_raw_data(&report_params_with_page_size(PAGE_SIZE), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        let number_threads_metric =
            &time_series_data.metrics[&ProcessMetric::NumberThreads.to_string()];
        let series = &number_threads_metric.series[0];
        // Two snapshots survive (0s and the deduped 1s), each contributing one point.
        assert_eq!(series.time_diff, vec![0, 1]);

        // The kept 1s snapshot is the LAST one (ticks 300+150): the user time rate is
        // (300-100)/100 ticks-per-sec / 1s elapsed = 2.0 cores, not (200-100)/100 = 1.0.
        let user_metric = &time_series_data.metrics[&ProcessMetric::UserSpaceTime.to_string()];
        let user_series = &user_metric.series[0];
        assert_eq!(user_series.values, vec![0.0, 2.0]);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_processes_aperf_pids_retained_beyond_top_16() {
    // 20 busy processes + one idle APerf process (pid 9999). The APerf process would
    // never make the top-16 CPU ranking, but must be retained via aperf_process_pids.
    let mut expected_per_sample_per_process_stats = Vec::new();
    for sample in 0..3u64 {
        let mut sample_stats = HashMap::new();
        for proc_id in 1..=20u64 {
            let mut stats = ExpectedProcessStats::default();
            stats.user_space_time = 10000 + sample * proc_id * 100;
            stats.kernel_space_time = 5000 + sample * proc_id * 50;
            sample_stats.insert(format!("{}_busy", proc_id), stats);
        }
        let mut aperf_stats = ExpectedProcessStats::default();
        aperf_stats.user_space_time = 1 + sample; // nearly idle
        aperf_stats.kernel_space_time = 1;
        sample_stats.insert("9999_aperf".to_string(), aperf_stats);
        expected_per_sample_per_process_stats.push(sample_stats);
    }

    let raw_data = generate_processes_raw_data(&expected_per_sample_per_process_stats, 1, 100);

    let mut params = report_params_with_page_size(PAGE_SIZE);
    params.aperf_process_pids = vec![9999];

    let mut processes = Processes::new();
    let result = processes.process_raw_data(&params, raw_data).unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        let user_metric = &time_series_data.metrics[&ProcessMetric::UserSpaceTime.to_string()];
        // Top 16 busy processes + the retained APerf process.
        assert_eq!(user_metric.series.len(), 17);
        assert!(user_metric
            .series
            .iter()
            .any(|s| s.series_name == "9999_aperf"));
    } else {
        panic!("Expected TimeSeries data");
    }
}
