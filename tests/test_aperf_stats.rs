#[cfg(test)]
mod aperf_stats_tests {
    use aperf::data::aperf_stats::AperfStat;
    use aperf::data::data_formats::AperfData;
    use aperf::data::TimeEnum;
    use aperf::visualizer::{GetData, ReportParams};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[derive(Clone, Debug)]
    struct ExpectedAperfStats {
        pub stats: HashMap<String, u64>,
    }

    impl Default for ExpectedAperfStats {
        fn default() -> Self {
            ExpectedAperfStats {
                stats: HashMap::new(),
            }
        }
    }

    fn create_named_report_params(data_file_path: String) -> ReportParams {
        ReportParams {
            data_dir: PathBuf::new(),
            tmp_dir: PathBuf::new(),
            report_dir: PathBuf::new(),
            run_name: String::new(),
            data_file_path: PathBuf::from(data_file_path),
        }
    }

    /// Write aperf_stats data to a binary file for testing
    fn write_aperf_stats_to_file(
        expected_per_sample_stats: &Vec<ExpectedAperfStats>,
        interval_seconds: u64,
        file_path: &str,
    ) {
        let base_time = Utc::now();

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)
            .unwrap();

        for (sample_idx, expected_stats) in expected_per_sample_stats.iter().enumerate() {
            let sample_time = base_time
                + chrono::Duration::seconds((sample_idx as u64 * interval_seconds) as i64);

            let aperf_stat = AperfStat {
                time: TimeEnum::DateTime(sample_time),
                name: "aperf".to_string(),
                data: expected_stats.stats.clone(),
            };

            bincode::serialize_into(&mut file, &aperf_stat).unwrap();
        }
    }

    #[test]
    fn test_process_aperf_stats_raw_data_complex() {
        let mut expected_stats = Vec::new();

        // Generate 100 samples with various aperf timing stats
        for sample in 0..100 {
            let mut stats = ExpectedAperfStats::default();

            // Simulate different data collection timings with varying patterns
            stats
                .stats
                .insert("cpu_utilization-collect".to_string(), 1000 + (sample * 10));
            stats
                .stats
                .insert("cpu_utilization-print".to_string(), 500 + (sample * 5));
            stats
                .stats
                .insert("diskstats-collect".to_string(), 2000 + (sample * 20));
            stats
                .stats
                .insert("diskstats-print".to_string(), 800 + (sample * 8));
            stats
                .stats
                .insert("interrupts-collect".to_string(), 1500 + (sample * 15));
            stats
                .stats
                .insert("interrupts-print".to_string(), 600 + (sample * 6));
            stats
                .stats
                .insert("perf_stat-collect".to_string(), 3000 + (sample * 30));
            stats
                .stats
                .insert("perf_stat-print".to_string(), 1200 + (sample * 12));
            stats
                .stats
                .insert("aperf".to_string(), 8000 + (sample * 80)); // Total aperf time

            expected_stats.push(stats);
        }

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        write_aperf_stats_to_file(&expected_stats, 2, temp_path);

        let mut aperf_stats = AperfStat::new();
        let params = create_named_report_params(temp_path.to_string());

        let result = aperf_stats.process_raw_data_new(params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Validate structure - should have 5 metrics (aperf + 4 data types)
            assert_eq!(time_series_data.metrics.len(), 5);

            // Check sorted metric names (sorted by average value, descending)
            let expected_metrics = vec![
                "aperf",
                "perf_stat",
                "diskstats",
                "interrupts",
                "cpu_utilization",
            ];
            assert_eq!(time_series_data.sorted_metric_names, expected_metrics);

            // Validate aperf metric (single series)
            let aperf_metric = &time_series_data.metrics["aperf"];
            assert_eq!(aperf_metric.series.len(), 1);
            assert_eq!(aperf_metric.series[0].values.len(), 100);

            // Validate data type metrics (multiple series + total)
            for data_type in ["cpu_utilization", "diskstats", "interrupts", "perf_stat"] {
                let metric = &time_series_data.metrics[data_type];
                assert_eq!(metric.series.len(), 3); // collect + print + total

                // Check that total series exists
                let total_series = metric
                    .series
                    .iter()
                    .find(|s| s.series_name.as_ref().unwrap().contains("total"))
                    .unwrap();
                assert_eq!(total_series.values.len(), 100);
            }

            // Validate all values against expected data
            for (sample_idx, expected_stats) in expected_stats.iter().enumerate() {
                // Validate aperf metric
                let aperf_metric = &time_series_data.metrics["aperf"];
                let expected_aperf = expected_stats.stats["aperf"] as f64;
                assert_eq!(aperf_metric.series[0].values[sample_idx], expected_aperf);

                // Validate data type metrics
                for data_type in ["cpu_utilization", "diskstats", "interrupts", "perf_stat"] {
                    let metric = &time_series_data.metrics[data_type];

                    let collect_key = format!("{}-collect", data_type);
                    let print_key = format!("{}-print", data_type);

                    if let Some(expected_collect) = expected_stats.stats.get(&collect_key) {
                        let collect_series = metric
                            .series
                            .iter()
                            .find(|s| s.series_name.as_ref().unwrap().contains("collect"))
                            .unwrap();
                        assert_eq!(collect_series.values[sample_idx], *expected_collect as f64);
                    }

                    if let Some(expected_print) = expected_stats.stats.get(&print_key) {
                        let print_series = metric
                            .series
                            .iter()
                            .find(|s| s.series_name.as_ref().unwrap().contains("write"))
                            .unwrap();
                        assert_eq!(print_series.values[sample_idx], *expected_print as f64);
                    }
                }
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_raw_data_simple() {
        let mut expected_stats = Vec::new();

        // Generate 3 samples with basic timing stats
        for sample in 0..3 {
            let mut stats = ExpectedAperfStats::default();
            stats
                .stats
                .insert("cpu_utilization-collect".to_string(), 1000 + (sample * 100));
            stats
                .stats
                .insert("diskstats-collect".to_string(), 2000 + (sample * 200));
            stats
                .stats
                .insert("aperf".to_string(), 5000 + (sample * 500));
            expected_stats.push(stats);
        }

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        write_aperf_stats_to_file(&expected_stats, 1, temp_path);

        let mut aperf_stats = AperfStat::new();
        let params = create_named_report_params(temp_path.to_string());

        let result = aperf_stats.process_raw_data_new(params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            assert_eq!(time_series_data.metrics.len(), 3); // aperf + cpu_utilization + diskstats

            // Validate time progression (1-second intervals)
            let aperf_metric = &time_series_data.metrics["aperf"];
            assert_eq!(aperf_metric.series[0].time_diff, vec![0, 1, 2]);

            // Validate values
            assert_eq!(aperf_metric.series[0].values, vec![5000.0, 5500.0, 6000.0]);
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_dynamic_stats() {
        let mut expected_stats = Vec::new();

        // Generate 50 samples with stats appearing at different times
        for sample in 0..50 {
            let mut stats = ExpectedAperfStats::default();

            // aperf always present
            stats
                .stats
                .insert("aperf".to_string(), 1000 + (sample * 10));

            // cpu_utilization from start
            stats
                .stats
                .insert("cpu_utilization-collect".to_string(), 500 + (sample * 5));

            // diskstats after sample 10
            if sample >= 10 {
                stats
                    .stats
                    .insert("diskstats-collect".to_string(), 800 + (sample * 8));
            }

            // interrupts after sample 30
            if sample >= 30 {
                stats
                    .stats
                    .insert("interrupts-collect".to_string(), 1200 + (sample * 12));
            }

            expected_stats.push(stats);
        }

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        write_aperf_stats_to_file(&expected_stats, 2, temp_path);

        let mut aperf_stats = AperfStat::new();
        let params = create_named_report_params(temp_path.to_string());

        let result = aperf_stats.process_raw_data_new(params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Validate series lengths match appearance timing

            // aperf and cpu_utilization: 50 samples each
            assert_eq!(time_series_data.metrics["aperf"].series[0].values.len(), 50);
            assert_eq!(
                time_series_data.metrics["cpu_utilization"].series[0]
                    .values
                    .len(),
                50
            );

            // diskstats: 40 samples (from sample 10 onwards)
            assert_eq!(
                time_series_data.metrics["diskstats"].series[0].values.len(),
                40
            );

            // interrupts: 20 samples (from sample 30 onwards)
            assert_eq!(
                time_series_data.metrics["interrupts"].series[0]
                    .values
                    .len(),
                20
            );
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_single_metric() {
        let mut expected_stats = Vec::new();

        // Generate 3 samples with only aperf timing
        for sample in 0..3 {
            let mut stats = ExpectedAperfStats::default();
            stats
                .stats
                .insert("aperf".to_string(), 1000 + (sample * 100));
            expected_stats.push(stats);
        }

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        write_aperf_stats_to_file(&expected_stats, 1, temp_path);

        let mut aperf_stats = AperfStat::new();
        let params = create_named_report_params(temp_path.to_string());

        let result = aperf_stats.process_raw_data_new(params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            assert_eq!(time_series_data.metrics.len(), 1);
            assert!(time_series_data.metrics.contains_key("aperf"));

            let aperf_metric = &time_series_data.metrics["aperf"];
            assert_eq!(aperf_metric.series.len(), 1);
            assert_eq!(aperf_metric.series[0].values, vec![1000.0, 1100.0, 1200.0]);
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_aperf_stats_empty_data() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        // Create empty file
        std::fs::File::create(temp_path).unwrap();

        let mut aperf_stats = AperfStat::new();
        let params = create_named_report_params(temp_path.to_string());

        let result = aperf_stats.process_raw_data_new(params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Empty data should result in no metrics
            assert_eq!(time_series_data.metrics.len(), 0);
            assert_eq!(time_series_data.sorted_metric_names.len(), 0);
        } else {
            panic!("Expected TimeSeries data");
        }
    }
    #[test]
    fn test_process_aperf_stats_multiple_data_types() {
        let mut expected_stats = Vec::new();

        // Generate 20 samples with multiple data types having different timing patterns
        for sample in 0..20 {
            let mut stats = ExpectedAperfStats::default();

            // aperf total time (always present)
            stats
                .stats
                .insert("aperf".to_string(), 10000 + (sample * 100));

            // cpu_utilization with linear growth
            stats
                .stats
                .insert("cpu_utilization-collect".to_string(), 1000 + (sample * 50));
            stats
                .stats
                .insert("cpu_utilization-print".to_string(), 200 + (sample * 10));

            // diskstats with quadratic growth
            stats
                .stats
                .insert("diskstats-collect".to_string(), 500 + (sample * sample * 2));
            stats
                .stats
                .insert("diskstats-print".to_string(), 100 + (sample * sample));

            // vmstat with exponential-like growth
            stats
                .stats
                .insert("vmstat-collect".to_string(), 300 + (sample * 25));
            stats
                .stats
                .insert("vmstat-print".to_string(), 50 + (sample * 5));

            expected_stats.push(stats);
        }

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_str().unwrap();

        write_aperf_stats_to_file(&expected_stats, 1, temp_path);

        let mut aperf_stats = AperfStat::new();
        let params = create_named_report_params(temp_path.to_string());

        let result = aperf_stats.process_raw_data_new(params, vec![]).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Should have 4 metrics: aperf + cpu_utilization + diskstats + vmstat
            assert_eq!(time_series_data.metrics.len(), 4);

            // Validate that all expected metrics exist
            assert!(time_series_data.metrics.contains_key("aperf"));
            assert!(time_series_data.metrics.contains_key("cpu_utilization"));
            assert!(time_series_data.metrics.contains_key("diskstats"));
            assert!(time_series_data.metrics.contains_key("vmstat"));

            // Validate aperf metric (single series)
            let aperf_metric = &time_series_data.metrics["aperf"];
            assert_eq!(aperf_metric.series.len(), 1);
            assert_eq!(aperf_metric.series[0].values.len(), 20);

            // Validate data type metrics (collect + print + total series)
            for data_type in ["cpu_utilization", "diskstats", "vmstat"] {
                let metric = &time_series_data.metrics[data_type];
                assert_eq!(metric.series.len(), 3); // collect + print + total

                // Find collect and print series
                let collect_series = metric
                    .series
                    .iter()
                    .find(|s| s.series_name.as_ref().unwrap().contains("collect"))
                    .unwrap();
                let print_series = metric
                    .series
                    .iter()
                    .find(|s| s.series_name.as_ref().unwrap().contains("write"))
                    .unwrap();
                let total_series = metric
                    .series
                    .iter()
                    .find(|s| s.series_name.as_ref().unwrap().contains("total"))
                    .unwrap();

                assert_eq!(collect_series.values.len(), 20);
                assert_eq!(print_series.values.len(), 20);
                assert_eq!(total_series.values.len(), 20);
            }

            // Validate time progression
            let aperf_metric = &time_series_data.metrics["aperf"];
            let expected_times: Vec<u64> = (0..20).collect();
            assert_eq!(aperf_metric.series[0].time_diff, expected_times);

            // Validate all values against expected data
            for (sample_idx, expected_stats) in expected_stats.iter().enumerate() {
                // Validate aperf metric
                let aperf_metric = &time_series_data.metrics["aperf"];
                let expected_aperf = expected_stats.stats["aperf"] as f64;
                assert_eq!(aperf_metric.series[0].values[sample_idx], expected_aperf);

                // Validate data type metrics
                for data_type in ["cpu_utilization", "diskstats", "vmstat"] {
                    let metric = &time_series_data.metrics[data_type];

                    let collect_key = format!("{}-collect", data_type);
                    let print_key = format!("{}-print", data_type);

                    let collect_series = metric
                        .series
                        .iter()
                        .find(|s| s.series_name.as_ref().unwrap().contains("collect"))
                        .unwrap();
                    let print_series = metric
                        .series
                        .iter()
                        .find(|s| s.series_name.as_ref().unwrap().contains("write"))
                        .unwrap();

                    let expected_collect = expected_stats.stats[&collect_key] as f64;
                    let expected_print = expected_stats.stats[&print_key] as f64;

                    assert_eq!(collect_series.values[sample_idx], expected_collect);
                    assert_eq!(print_series.values[sample_idx], expected_print);
                }
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }
}
