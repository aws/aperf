#[cfg(test)]
mod aperf_stats_tests {
    #[cfg(target_os = "linux")]
    use aperf::data::aperf_stats::AperfStatsCollector;
    use aperf::data::aperf_stats::{AperfStat, AperfStats};
    use aperf::data::common::data_formats::{AperfData, TimeSeriesData};
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
            let mut aperf_stats = AperfStats::for_time(TimeEnum::DateTime(
                base_time
                    + chrono::Duration::seconds((sample_idx as u64 * interval_seconds) as i64),
            ));
            aperf_stats.stats = stats.clone();
            bincode::serialize_into(&mut file, &aperf_stats).unwrap();
        }
    }

    /// Build the resource usage that getrusage would report for a subprocess. Max RSS is rounded
    /// down to a whole kilobyte, which is the resolution getrusage reports it at.
    #[cfg(target_os = "linux")]
    fn sub_process_rusage(user_time: f64, kernel_time: f64, max_rss_bytes: f64) -> libc::rusage {
        let mut rusage: libc::rusage = unsafe { std::mem::zeroed() };
        rusage.ru_utime.tv_sec = user_time.trunc() as libc::time_t;
        rusage.ru_utime.tv_usec = (user_time.fract() * 1e6).round() as libc::suseconds_t;
        rusage.ru_stime.tv_sec = kernel_time.trunc() as libc::time_t;
        rusage.ru_stime.tv_usec = (kernel_time.fract() * 1e6).round() as libc::suseconds_t;
        rusage.ru_maxrss = (max_rss_bytes / 1024.0) as libc::c_long;
        rusage
    }

    /// Record the given subprocess usages through the collector, each described as
    /// (name, start_seconds, end_seconds, user_time, kernel_time, max_rss_bytes) relative to the
    /// collection start, then process them once and return every metric's series as
    /// Map<metric, Map<series, Vec<(second, value)>>>. All the usages land in the same stats, as
    /// happens when a subprocess blocks the collector, so the report has to rebuild the seconds
    /// they spanned from the timestamps they carry.
    #[cfg(target_os = "linux")]
    fn sub_process_usage_points(
        usages: &[(&str, f64, f64, f64, f64, f64)],
    ) -> HashMap<String, HashMap<String, Vec<(u64, f64)>>> {
        let temp_dir = tempfile::tempdir().unwrap();
        let collection_start = Utc::now();
        let offset = |seconds: f64| {
            TimeEnum::DateTime(
                collection_start + chrono::Duration::microseconds((seconds * 1e6) as i64),
            )
        };

        let mut aperf_stats_collector = AperfStatsCollector::new();
        aperf_stats_collector.initialize(temp_dir.path().to_path_buf());
        for &(name, start_time, end_time, user_time, kernel_time, max_rss_bytes) in usages {
            aperf_stats_collector.add_sub_process_usage(
                name,
                offset(start_time),
                offset(end_time),
                sub_process_rusage(user_time, kernel_time, max_rss_bytes),
            );
        }
        aperf_stats_collector.flush().unwrap();

        let time_series_data =
            process_run_dir(temp_dir.path(), TimeEnum::DateTime(collection_start));
        time_series_data
            .metrics
            .iter()
            .map(|(metric_name, metric)| {
                let per_series = metric
                    .series
                    .iter()
                    .filter(|series| !series.is_aggregate)
                    .map(|series| {
                        (
                            series.series_name.clone(),
                            series
                                .time_diff
                                .iter()
                                .copied()
                                .zip(series.values.iter().copied())
                                .collect(),
                        )
                    })
                    .collect();
                (metric_name.clone(), per_series)
            })
            .collect()
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

    fn assert_close(actual: f64, expected: f64, what: &str) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "{what}: expected {expected}, got {actual}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_sub_process_usage_is_spread_over_the_seconds_it_ran() {
        // Every shape a run can take, processed in one pass. A data point at second n covers
        // (n-1, n], which is what an accumulative metric sampled once a second reports, so a run
        // over [0, 40] occupies seconds 1 through 40.
        // (name, start_seconds, end_seconds, user_time, kernel_time, max_rss_bytes)
        let usages = [
            ("blocking", 0.0, 40.0, 30.0, 10.0, 8192.0),
            ("sub-second", 10.2, 10.3, 0.08, 0.0, 0.0),
            ("partial", 20.0, 22.5, 2.5, 0.0, 0.0),
            ("instant", 30.0, 30.0, 0.05, 0.0, 0.0),
            ("pre-start", -0.5, -0.2, 0.03, 0.0, 0.0),
            ("straddling", -1.0, 1.0, 2.0, 0.0, 0.0),
            ("repeated", 0.0, 2.0, 2.0, 0.0, 1024.0),
            ("repeated", 0.5, 1.5, 1.0, 0.0, 3072.0),
        ];
        let points = sub_process_usage_points(&usages);
        let user_time = &points["process_user_space_time"];

        // The case this exists for: a subprocess that blocked the collector for 40s leaves no stats
        // behind while it runs, so a second's worth of data is created for each second it covered.
        // 30 CPU seconds over 40 of them is 0.75 cores, the unit the process metrics sampled from
        // /proc are already in, rather than a lone 30.0 spike where the subprocess happened to exit.
        let blocking = &user_time["blocking"];
        assert_eq!(blocking.len(), 40, "one data point per second of the run");
        assert_eq!(blocking.first().unwrap().0, 1);
        assert_eq!(blocking.last().unwrap().0, 40);
        for &(second, value) in blocking {
            assert_close(
                value,
                0.75,
                &format!("blocking user CPU at second {second}"),
            );
        }
        for &(second, value) in &points["process_kernel_space_time"]["blocking"] {
            assert_close(
                value,
                0.25,
                &format!("blocking kernel CPU at second {second}"),
            );
        }

        for (name, expected) in [
            // Wholly inside one second, where its CPU time already is the per-second rate.
            ("sub-second", vec![(11, 0.08)]),
            // Two whole seconds and half of a third, divided in proportion to that.
            ("partial", vec![(21, 1.0), (22, 1.0), (23, 0.5)]),
            // Nothing to divide, so it is charged to the second it ended in.
            ("instant", vec![(30, 0.05)]),
            // Seconds before the collection start have no data point of their own, so they fold
            // into second zero rather than being dropped.
            ("pre-start", vec![(0, 0.03)]),
            // Second zero covers (-1, 0], so this splits evenly with second one, folding nothing.
            ("straddling", vec![(0, 1.0), (1, 1.0)]),
            // Two runs of one command covering the same seconds: their CPU adds up there.
            ("repeated", vec![(1, 1.5), (2, 1.5)]),
        ] {
            assert_eq!(user_time[name], expected, "{name}");
        }

        // Whatever the shape, the per-second rates have to add back up to the CPU time the kernel
        // reported, which is what makes the metric trustworthy.
        let mut reported_cpu: HashMap<&str, f64> = HashMap::new();
        for &(name, _, _, user_time_seconds, _, _) in &usages {
            *reported_cpu.entry(name).or_default() += user_time_seconds;
        }
        for (name, reported) in reported_cpu {
            let spread: f64 = user_time[name].iter().map(|&(_, value)| value).sum();
            assert_close(spread, reported, &format!("{name} total CPU time"));
        }

        // Peak RSS is a high-water mark rather than something accumulated over the run, so instead
        // of being divided up it is reported in full for every second the run covered.
        let max_rss = &points["process_resident_set_size_bytes"];
        assert_eq!(
            max_rss["blocking"],
            (1..=40u64)
                .map(|second| (second, 8192.0))
                .collect::<Vec<_>>()
        );
        assert_eq!(max_rss["repeated"], vec![(1, 3072.0), (2, 3072.0)]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_run_command_and_wait_records_a_sub_process_usage() {
        // End to end over a real subprocess: timed and reaped by run_command_and_wait, flushed to
        // disk, and read back spread across the seconds it ran. This is also the only coverage
        // that flush keeps a stats holding nothing but subprocess runs, which is the normal case
        // for a subprocess that blocks the collector long enough that no other stat is recorded
        // alongside it.
        let temp_dir = tempfile::tempdir().unwrap();
        let collection_start = Utc::now();
        aperf::aperf_stats_initialize(temp_dir.path().to_path_buf());

        // Burn CPU for long enough that the usage is certain to span more than one second.
        let output = aperf::run_command_and_wait(
            "sh",
            [
                "-c",
                "end=$(( $(date +%s) + 3 )); while [ $(date +%s) -lt $end ]; do :; done",
            ],
            "burn",
            None,
        )
        .unwrap();
        assert!(output.status.success());
        aperf::aperf_stats_flush().unwrap();

        let mut params =
            create_named_report_params(temp_dir.path().join("seed").to_string_lossy().to_string());
        params.collection_start = Some(TimeEnum::DateTime(collection_start));

        let result = AperfStats::new().process_raw_data(&params, vec![]).unwrap();
        let AperfData::TimeSeries(time_series_data) = result else {
            panic!("Expected TimeSeries data");
        };
        let series = time_series_data.metrics["process_user_space_time"]
            .series
            .iter()
            .find(|series| series.series_name == "burn")
            .expect("the burn series should exist");

        assert!(
            series.values.len() >= 2,
            "a run of over two seconds should cover more than one second, got {:?}",
            series.time_diff
        );
        // The values are per-second rates, so they add back up to the CPU seconds the busy loop
        // used. Individual values are not bounded from below, because a second the loop only
        // partly covered gets a correspondingly small share of the total.
        let cpu_seconds: f64 = series.values.iter().sum();
        assert!(
            (0.2..6.0).contains(&cpu_seconds),
            "expected a few CPU seconds from a multi-second busy loop, got {cpu_seconds}"
        );
    }

    /// Append one stats record carrying a single stat, so a test can place stats at an exact time.
    fn append_stats_at(
        run_data_dir: &Path,
        time: TimeEnum,
        stat_name: &str,
        data_name: &str,
        value: f64,
    ) {
        let mut aperf_stats = AperfStats::for_time(time);
        aperf_stats.stats.insert(
            stat_name.to_string(),
            HashMap::from([(data_name.to_string(), value)]),
        );
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(data_file_path(
                get_data_name_from_type::<AperfStats>(),
                &run_data_dir.to_path_buf(),
            ))
            .unwrap();
        bincode::serialize_into(&mut file, &aperf_stats).unwrap();
    }

    fn process_run_dir(run_data_dir: &Path, collection_start: TimeEnum) -> TimeSeriesData {
        let mut params =
            create_named_report_params(run_data_dir.join("seed").to_string_lossy().to_string());
        params.collection_start = Some(collection_start);
        let result = AperfStats::new().process_raw_data(&params, vec![]).unwrap();
        let AperfData::TimeSeries(time_series_data) = result else {
            panic!("Expected TimeSeries data");
        };
        time_series_data
    }

    fn series_points(
        time_series_data: &TimeSeriesData,
        metric: &str,
        series_name: &str,
    ) -> Vec<(u64, f64)> {
        let series = time_series_data.metrics[metric]
            .series
            .iter()
            .find(|series| series.series_name == series_name)
            .unwrap_or_else(|| panic!("{metric}/{series_name} should exist"));
        series
            .time_diff
            .iter()
            .copied()
            .zip(series.values.iter().copied())
            .collect()
    }

    #[test]
    fn test_stats_recorded_before_the_collection_started_land_at_second_zero() {
        // Preparing the collectors is timed and recorded before the collection start is stamped, so
        // those stats have no second of their own. They must be anchored at second 0 rather than
        // dropped, and subtracting the collection start from them must not underflow.
        let temp_dir = tempfile::tempdir().unwrap();
        let collection_start = Utc::now();
        append_stats_at(
            temp_dir.path(),
            TimeEnum::DateTime(collection_start - chrono::Duration::milliseconds(1500)),
            "prepare",
            "perf_stat",
            42.0,
        );
        append_stats_at(
            temp_dir.path(),
            TimeEnum::DateTime(collection_start + chrono::Duration::seconds(1)),
            "collect",
            "perf_stat",
            7.0,
        );

        let time_series_data =
            process_run_dir(temp_dir.path(), TimeEnum::DateTime(collection_start));
        assert_eq!(
            series_points(&time_series_data, "prepare", "perf_stat"),
            vec![(0, 42.0)]
        );
        assert_eq!(
            series_points(&time_series_data, "collect", "perf_stat"),
            vec![(1, 7.0)]
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_usage_lands_on_the_second_the_processor_gives_the_stats_it_joins() {
        // Stats are recorded at whatever moment a stat is added, so their time carries a fraction of
        // a second. Locating the stats for a second has to round that fraction the same way the
        // processor does, or the usage written into those stats surfaces at a different second.
        let temp_dir = tempfile::tempdir().unwrap();
        let collection_start = Utc::now();
        let at = |millis: i64| {
            TimeEnum::DateTime(collection_start + chrono::Duration::milliseconds(millis))
        };

        let mut aperf_stats_collector = AperfStatsCollector::new();
        aperf_stats_collector.initialize(temp_dir.path().to_path_buf());
        // This usage occupies second 10, since a data point at second n covers (n-1, n].
        aperf_stats_collector.add_sub_process_usage(
            "probe",
            at(9_000),
            at(10_000),
            sub_process_rusage(1.0, 0.0, 0.0),
        );
        aperf_stats_collector.flush().unwrap();

        // These stats round up to second 11. Truncating instead would locate them at second 10 and
        // so hand them the usage, which the processor would then surface a second late.
        append_stats_at(temp_dir.path(), at(10_600), "collect", "perf_stat", 5.0);

        let time_series_data =
            process_run_dir(temp_dir.path(), TimeEnum::DateTime(collection_start));
        assert_eq!(
            series_points(&time_series_data, "collect", "perf_stat"),
            vec![(11, 5.0)]
        );
        assert_eq!(
            series_points(&time_series_data, "process_user_space_time", "probe"),
            vec![(10, 1.0)],
            "the usage ran in second 10 and should surface there"
        );
    }
}
