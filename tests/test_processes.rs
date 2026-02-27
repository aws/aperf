use aperf::data::data_formats::AperfData;
use aperf::data::processes::{ProcessKey, Processes, ProcessesRaw};
use aperf::data::ProcessData;
use aperf::data::{Data, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::prelude::*;
use std::collections::HashMap;
use strum::IntoEnumIterator;

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
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Validate structure
        assert_eq!(time_series_data.metrics.len(), ProcessKey::iter().count());
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessKey::iter().count()
        );

        // Check each process key metric exists
        for process_key in ProcessKey::iter() {
            let metric_name = process_key.to_string();
            assert!(time_series_data.metrics.contains_key(&metric_name));

            let metric = &time_series_data.metrics[&metric_name];
            assert_eq!(metric.metric_name, metric_name);

            // Should have 3 series (one per process)
            assert_eq!(metric.series.len(), 3);

            // Each series should have 50 data points
            for series in &metric.series {
                assert_eq!(series.values.len(), 50);
                assert_eq!(series.time_diff.len(), 50);
            }
        }

        // Validate sorted metric names
        let expected_metrics: Vec<String> = ProcessKey::iter().map(|k| k.to_string()).collect();
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            expected_metrics.len()
        );
        for expected_metric in expected_metrics {
            assert!(time_series_data
                .sorted_metric_names
                .contains(&expected_metric));
        }

        // Validate all data using expected values
        for process_key in ProcessKey::iter() {
            let metric_name = process_key.to_string();
            let metric = &time_series_data.metrics[&metric_name];

            for series in &metric.series {
                let process_name = series.series_name.as_ref().unwrap();

                for (sample_idx, &value) in series.values.iter().enumerate() {
                    // First sample should be 0 for CPU metrics
                    if sample_idx == 0
                        && matches!(
                            process_key,
                            ProcessKey::UserSpaceTime | ProcessKey::KernelSpaceTime
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
                            ProcessKey::UserSpaceTime => {
                                if sample_idx == 0 {
                                    0.0
                                } else {
                                    let prev_stats = &expected_per_sample_per_process_stats
                                        [sample_idx - 1][process_name];
                                    let delta =
                                        expected_stats.user_space_time - prev_stats.user_space_time;
                                    (delta as f64) / (ticks_per_second as f64 * 2.0) * 100.0
                                }
                            }
                            ProcessKey::KernelSpaceTime => {
                                if sample_idx == 0 {
                                    0.0
                                } else {
                                    let prev_stats = &expected_per_sample_per_process_stats
                                        [sample_idx - 1][process_name];
                                    let delta = expected_stats.kernel_space_time
                                        - prev_stats.kernel_space_time;
                                    (delta as f64) / (ticks_per_second as f64 * 2.0) * 100.0
                                }
                            }
                            ProcessKey::NumberThreads => expected_stats.number_threads as f64,
                            ProcessKey::VirtualMemorySize => {
                                expected_stats.virtual_memory_size as f64
                            }
                            ProcessKey::ResidentSetSize => expected_stats.resident_set_size as f64,
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
            ProcessKey::iter().count()
        );
        for process_key in ProcessKey::iter() {
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
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), ProcessKey::iter().count());

        // Validate all data using expected values
        for process_key in ProcessKey::iter() {
            let metric_name = process_key.to_string();
            let metric = &time_series_data.metrics[&metric_name];

            assert_eq!(metric.series.len(), 1);
            let series = &metric.series[0];
            assert_eq!(series.values.len(), 3);

            let process_name = series.series_name.as_ref().unwrap();

            for (sample_idx, &value) in series.values.iter().enumerate() {
                // Get expected stats for this sample and process
                let expected_stats =
                    &expected_per_sample_per_process_stats[sample_idx][process_name];

                let expected_value = match process_key {
                    ProcessKey::UserSpaceTime => {
                        if sample_idx == 0 {
                            0.0
                        } else {
                            let prev_stats = &expected_per_sample_per_process_stats[sample_idx - 1]
                                [process_name];
                            let delta = expected_stats.user_space_time - prev_stats.user_space_time;
                            (delta as f64) / (ticks_per_second as f64 * 1.0) * 100.0
                        }
                    }
                    ProcessKey::KernelSpaceTime => {
                        if sample_idx == 0 {
                            0.0
                        } else {
                            let prev_stats = &expected_per_sample_per_process_stats[sample_idx - 1]
                                [process_name];
                            let delta =
                                expected_stats.kernel_space_time - prev_stats.kernel_space_time;
                            (delta as f64) / (ticks_per_second as f64 * 1.0) * 100.0
                        }
                    }
                    ProcessKey::NumberThreads => expected_stats.number_threads as f64,
                    ProcessKey::VirtualMemorySize => expected_stats.virtual_memory_size as f64,
                    ProcessKey::ResidentSetSize => expected_stats.resident_set_size as f64,
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
            ProcessKey::iter().count()
        );
        for process_key in ProcessKey::iter() {
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
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Should have all process keys
        assert_eq!(time_series_data.metrics.len(), ProcessKey::iter().count());

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
            ProcessKey::iter().count()
        );
        for process_key in ProcessKey::iter() {
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
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Should have all process keys
        assert_eq!(time_series_data.metrics.len(), ProcessKey::iter().count());

        // Should only have top 16 processes
        let user_space_metric = &time_series_data.metrics["user_space_time"];
        assert_eq!(user_space_metric.series.len(), 16);

        // Each series should have 5 data points
        for series in &user_space_metric.series {
            assert_eq!(series.values.len(), 5);
        }

        // Validate sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessKey::iter().count()
        );
        for process_key in ProcessKey::iter() {
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
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Should have all process keys
        assert_eq!(time_series_data.metrics.len(), ProcessKey::iter().count());

        // Should only have top 16 processes
        let user_space_metric = &time_series_data.metrics["user_space_time"];
        assert_eq!(user_space_metric.series.len(), 16);

        // Each series should have 5 data points
        for series in &user_space_metric.series {
            assert_eq!(series.values.len(), 5);
        }

        let user_space_metric = &time_series_data.metrics["user_space_time"];
        let kernel_space_metric = &time_series_data.metrics["kernel_space_time"];

        let mut process_total_cpu: Vec<(String, f64)> = Vec::new();
        for (i, series) in user_space_metric.series.iter().enumerate() {
            let kernel_series = &kernel_space_metric.series[i];
            let process_name = series.series_name.as_ref().unwrap().clone();

            let total_user: f64 = series.values.iter().sum();
            let total_kernel: f64 = kernel_series.values.iter().sum();
            let total_cpu = total_user + total_kernel;

            process_total_cpu.push((process_name, total_cpu));
        }

        for i in 1..process_total_cpu.len() {
            assert!(
                process_total_cpu[i - 1].1 >= process_total_cpu[i].1,
                "Processes should be ranked by total CPU usage: {} should be >= {}",
                process_total_cpu[i - 1].1,
                process_total_cpu[i].1
            );
        }

        // Validate sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            ProcessKey::iter().count()
        );
        for process_key in ProcessKey::iter() {
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
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        assert_eq!(time_series_data.sorted_metric_names.len(), 0);
    } else {
        panic!("Expected TimeSeries data");
    }
}
