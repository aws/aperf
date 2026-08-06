#[cfg(test)]
mod aperf_stats_tests {
    use aperf::data::aperf_stats::{AperfStat, AperfStats};
    use aperf::data::common::data_formats::AperfData;
    use aperf::data::{ProcessData, TimeEnum};
    use aperf::data_processing::ReportParams;
    use aperf::{data_file_path, get_data_name_from_type};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    // The processor reads the aperf_stats file from `run_data_dir`, so point `run_data_dir` at
    // the directory containing the temp file written by the writer helpers. Non-empty
    // aperf_process_pids selects the new-format (AperfStats) processing path.
    fn create_named_report_params(data_file_path: String) -> ReportParams {
        let run_data_dir = Path::new(&data_file_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        ReportParams {
            run_data_dir,
            tmp_dir: PathBuf::new(),
            report_dir: PathBuf::new(),
            run_name: String::new(),
            collection_start: None,
            pmu_counter_mode: String::new(),
            aperf_process_pids: vec![4242],
            page_size: 0,
        }
    }

    /// Write new-format aperf_stats data (AperfStats records) to the run dir.
    /// Sample values are given as (stat_name, data_name, value) triples per sample.
    fn write_aperf_stats_to_file(
        per_sample_stats: &[HashMap<String, HashMap<String, f64>>],
        interval_seconds: u64,
        file_path: &str,
    ) {
        let base_time = Utc::now();
        let run_data_dir = Path::new(file_path)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(data_file_path(
                get_data_name_from_type::<AperfStats>(),
                &run_data_dir,
            ))
            .unwrap();

        for (sample_idx, stats) in per_sample_stats.iter().enumerate() {
            let aperf_stats = AperfStats {
                time: TimeEnum::DateTime(
                    base_time
                        + chrono::Duration::seconds((sample_idx as u64 * interval_seconds) as i64),
                ),
                stats: stats.clone(),
            };
            bincode::serialize_into(&mut file, &aperf_stats).unwrap();
        }
    }

    #[test]
    fn test_process_aperf_stats_raw_data_complex() {
        // 100 samples; "collect" and "write" stats each carry four data series, plus the
        // "aperf" stat carrying the per-interval total collection time.
        let data_names = ["cpu_utilization", "diskstats", "interrupts", "perf_stat"];
        let mut per_sample_stats = Vec::new();
        for sample in 0..100u64 {
            let mut stats: HashMap<String, HashMap<String, f64>> = HashMap::new();
            let mut collect = HashMap::new();
            let mut write = HashMap::new();
            for (idx, data_name) in data_names.iter().enumerate() {
                collect.insert(
                    data_name.to_string(),
                    1000.0 * (idx as f64 + 1.0) + sample as f64 * 10.0,
                );
                write.insert(
                    data_name.to_string(),
                    500.0 * (idx as f64 + 1.0) + sample as f64 * 5.0,
                );
            }
            stats.insert("collect".to_string(), collect);
            stats.insert("write".to_string(), write);
            stats.insert(
                "aperf".to_string(),
                HashMap::from([("collect".to_string(), 8000.0 + sample as f64 * 80.0)]),
            );
            per_sample_stats.push(stats);
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file_path = temp_dir.path().join("aperf_stats_seed");
        let temp_path = temp_file_path.to_str().unwrap();
        write_aperf_stats_to_file(&per_sample_stats, 2, temp_path);

        let mut aperf_stats = AperfStats::new();
        let params = create_named_report_params(temp_path.to_string());
        let result = aperf_stats.process_raw_data(&params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // One metric per stat name.
            assert_eq!(time_series_data.metrics.len(), 3);

            // "aperf" is in the fixed metric order list, so it sorts ahead of the
            // unlisted "collect"/"write" metrics.
            assert_eq!(time_series_data.sorted_metric_names[0], "aperf");
            assert!(time_series_data
                .sorted_metric_names
                .contains(&"collect".to_string()));
            assert!(time_series_data
                .sorted_metric_names
                .contains(&"write".to_string()));

            // aperf: single data series (redundant aggregate is stripped).
            let aperf_metric = &time_series_data.metrics["aperf"];
            assert_eq!(aperf_metric.series.len(), 1);
            assert_eq!(aperf_metric.series[0].series_name, "collect");
            assert_eq!(aperf_metric.series[0].values.len(), 100);

            // collect/write: four data series plus the "total" sum-aggregate.
            for stat_name in ["collect", "write"] {
                let metric = &time_series_data.metrics[stat_name];
                assert_eq!(metric.series.len(), 5);
                let total_series = metric
                    .series
                    .iter()
                    .find(|s| s.is_aggregate)
                    .expect("total series should exist");
                assert_eq!(total_series.series_name, "total");
                assert_eq!(total_series.values.len(), 100);
            }

            // Validate every value against the generated data, and the total as the sum.
            for (sample_idx, stats) in per_sample_stats.iter().enumerate() {
                assert_eq!(
                    aperf_metric.series[0].values[sample_idx],
                    stats["aperf"]["collect"]
                );

                for stat_name in ["collect", "write"] {
                    let metric = &time_series_data.metrics[stat_name];
                    let mut expected_total = 0.0;
                    for data_name in data_names {
                        let expected = stats[stat_name][data_name];
                        expected_total += expected;
                        let series = metric
                            .series
                            .iter()
                            .find(|s| s.series_name == data_name)
                            .unwrap();
                        assert_eq!(series.values[sample_idx], expected);
                    }
                    let total_series = metric.series.iter().find(|s| s.is_aggregate).unwrap();
                    assert_eq!(total_series.values[sample_idx], expected_total);
                }
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_raw_data_simple() {
        // 3 samples; "collect" with two data series and "prepare" with one.
        let mut per_sample_stats = Vec::new();
        for sample in 0..3u64 {
            let mut stats: HashMap<String, HashMap<String, f64>> = HashMap::new();
            stats.insert(
                "collect".to_string(),
                HashMap::from([
                    (
                        "cpu_utilization".to_string(),
                        1000.0 + sample as f64 * 100.0,
                    ),
                    ("diskstats".to_string(), 2000.0 + sample as f64 * 200.0),
                ]),
            );
            stats.insert(
                "prepare".to_string(),
                HashMap::from([("perf_stat".to_string(), 300.0 + sample as f64 * 30.0)]),
            );
            per_sample_stats.push(stats);
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file_path = temp_dir.path().join("aperf_stats_seed");
        let temp_path = temp_file_path.to_str().unwrap();
        write_aperf_stats_to_file(&per_sample_stats, 1, temp_path);

        let mut aperf_stats = AperfStats::new();
        let params = create_named_report_params(temp_path.to_string());
        let result = aperf_stats.process_raw_data(&params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // One metric per stat name.
            assert_eq!(time_series_data.metrics.len(), 2);

            // collect: two data series plus the sum-aggregate "total" series.
            let collect_metric = &time_series_data.metrics["collect"];
            assert_eq!(collect_metric.series.len(), 3);
            let cpu_series = collect_metric
                .series
                .iter()
                .find(|s| s.series_name == "cpu_utilization")
                .expect("cpu_utilization series should exist");
            assert_eq!(cpu_series.values, vec![1000.0, 1100.0, 1200.0]);
            assert_eq!(cpu_series.time_diff, vec![0, 1, 2]);
            let total_series = collect_metric
                .series
                .iter()
                .find(|s| s.is_aggregate)
                .expect("total series should exist");
            assert_eq!(total_series.series_name, "total");
            assert_eq!(total_series.values, vec![3000.0, 3300.0, 3600.0]);

            // prepare: a single data series, so the redundant aggregate is stripped.
            let prepare_metric = &time_series_data.metrics["prepare"];
            assert_eq!(prepare_metric.series.len(), 1);
            assert_eq!(prepare_metric.series[0].series_name, "perf_stat");
            assert_eq!(prepare_metric.series[0].values, vec![300.0, 330.0, 360.0]);

            // Fixed metric name order: the listed "prepare" comes before the unlisted
            // "collect".
            assert_eq!(
                time_series_data.sorted_metric_names,
                vec!["prepare", "collect"]
            );
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_dynamic_stats() {
        // 50 samples; data series appear within the "collect" stat at different times.
        let mut per_sample_stats = Vec::new();
        for sample in 0..50u64 {
            let mut collect =
                HashMap::from([("cpu_utilization".to_string(), 500.0 + sample as f64 * 5.0)]);
            if sample >= 10 {
                collect.insert("diskstats".to_string(), 800.0 + sample as f64 * 8.0);
            }
            if sample >= 30 {
                collect.insert("interrupts".to_string(), 1200.0 + sample as f64 * 12.0);
            }
            per_sample_stats.push(HashMap::from([("collect".to_string(), collect)]));
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file_path = temp_dir.path().join("aperf_stats_seed");
        let temp_path = temp_file_path.to_str().unwrap();
        write_aperf_stats_to_file(&per_sample_stats, 2, temp_path);

        let mut aperf_stats = AperfStats::new();
        let params = create_named_report_params(temp_path.to_string());
        let result = aperf_stats.process_raw_data(&params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            let collect_metric = &time_series_data.metrics["collect"];
            let series_len = |name: &str| {
                collect_metric
                    .series
                    .iter()
                    .find(|s| s.series_name == name)
                    .unwrap_or_else(|| panic!("{name} series should exist"))
                    .values
                    .len()
            };
            // Series lengths match when each data series appeared.
            assert_eq!(series_len("cpu_utilization"), 50);
            assert_eq!(series_len("diskstats"), 40);
            assert_eq!(series_len("interrupts"), 20);
            // The total covers every sample where the metric had any data.
            assert_eq!(series_len("total"), 50);
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_single_metric() {
        // 3 samples with only the aperf total collection time.
        let mut per_sample_stats = Vec::new();
        for sample in 0..3u64 {
            per_sample_stats.push(HashMap::from([(
                "aperf".to_string(),
                HashMap::from([("collect".to_string(), 1000.0 + sample as f64 * 100.0)]),
            )]));
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file_path = temp_dir.path().join("aperf_stats_seed");
        let temp_path = temp_file_path.to_str().unwrap();
        write_aperf_stats_to_file(&per_sample_stats, 1, temp_path);

        let mut aperf_stats = AperfStats::new();
        let params = create_named_report_params(temp_path.to_string());
        let result = aperf_stats.process_raw_data(&params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            assert_eq!(time_series_data.metrics.len(), 1);
            let aperf_metric = &time_series_data.metrics["aperf"];
            assert_eq!(aperf_metric.series.len(), 1);
            assert_eq!(aperf_metric.series[0].values, vec![1000.0, 1100.0, 1200.0]);
            assert_eq!(aperf_metric.series[0].time_diff, vec![0, 1, 2]);
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_empty_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_file_path = temp_dir.path().join("aperf_stats_seed");
        let temp_path = temp_file_path.to_str().unwrap();

        // Create an empty aperf_stats file in the run dir the processor reads from.
        let run_data_dir = Path::new(temp_path).parent().unwrap().to_path_buf();
        std::fs::File::create(data_file_path(
            get_data_name_from_type::<AperfStats>(),
            &run_data_dir,
        ))
        .unwrap();

        let mut aperf_stats = AperfStats::new();
        let params = create_named_report_params(temp_path.to_string());
        let result = aperf_stats.process_raw_data(&params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Empty data should result in no metrics
            assert_eq!(time_series_data.metrics.len(), 0);
            assert_eq!(time_series_data.sorted_metric_names.len(), 0);
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_legacy_aperf_stat_backward_compatibility() {
        // Archives recorded by older APerf versions carry legacy AperfStat records
        // ("<data>-<stat>" keys) and no saved APerf PIDs. They must be processed via the
        // legacy path: metric = data name, series = stat name, "print" renamed "write".
        let base_time = Utc::now();
        let temp_dir = tempfile::tempdir().unwrap();
        let run_data_dir = temp_dir.path().to_path_buf();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(data_file_path(
                get_data_name_from_type::<AperfStats>(),
                &run_data_dir,
            ))
            .unwrap();
        for sample in 0..3u64 {
            let aperf_stat = AperfStat {
                time: TimeEnum::DateTime(base_time + chrono::Duration::seconds(sample as i64)),
                name: "aperf".to_string(),
                data: HashMap::from([
                    ("cpu_utilization-collect".to_string(), 1000 + sample * 100),
                    ("cpu_utilization-print".to_string(), 500 + sample * 50),
                    ("aperf".to_string(), 5000 + sample * 500),
                ]),
            };
            bincode::serialize_into(&mut file, &aperf_stat).unwrap();
        }

        let mut aperf_stats = AperfStats::new();
        // Legacy runs have no saved APerf PIDs, which selects the legacy path.
        let mut params =
            create_named_report_params(run_data_dir.join("seed").to_string_lossy().to_string());
        params.aperf_process_pids = Vec::new();

        let result = aperf_stats.process_raw_data(&params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Metric per data name: cpu_utilization + aperf.
            assert_eq!(time_series_data.metrics.len(), 2);

            let cpu_metric = &time_series_data.metrics["cpu_utilization"];
            let collect_series = cpu_metric
                .series
                .iter()
                .find(|s| s.series_name == "collect")
                .expect("collect series should exist");
            assert_eq!(collect_series.values, vec![1000.0, 1100.0, 1200.0]);
            assert_eq!(collect_series.time_diff, vec![0, 1, 2]);
            // The legacy "print" stat is renamed to "write".
            assert!(cpu_metric.series.iter().any(|s| s.series_name == "write"));
            assert!(!cpu_metric.series.iter().any(|s| s.series_name == "print"));

            let aperf_metric = &time_series_data.metrics["aperf"];
            assert_eq!(aperf_metric.series.len(), 1);
            assert_eq!(aperf_metric.series[0].values, vec![5000.0, 5500.0, 6000.0]);
        } else {
            panic!("Expected TimeSeries data");
        }
    }
}
