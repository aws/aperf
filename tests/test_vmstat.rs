use aperf::data::vmstat::VmstatRaw;
use aperf::data::TimeEnum;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
struct ExpectedVmstatStats {
    pub stats: HashMap<String, i64>,
}

/// Generate /proc/vmstat data based on expected vmstat values
fn generate_vmstat_raw_data(
    expected_per_sample_stats: &Vec<ExpectedVmstatStats>,
    interval_seconds: u64,
) -> Vec<VmstatRaw> {
    let mut samples: Vec<VmstatRaw> = Vec::new();
    let base_time = Utc::now();

    for (sample_idx, expected_stats) in expected_per_sample_stats.iter().enumerate() {
        let time =
            base_time + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64));

        let mut proc_vmstat = String::new();

        // Sort keys for consistent output
        let mut sorted_keys: Vec<&String> = expected_stats.stats.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
            let value = expected_stats.stats.get(key).unwrap();
            proc_vmstat.push_str(&format!("{} {}\n", key, value));
        }

        samples.push(VmstatRaw {
            time: TimeEnum::DateTime(time),
            data: proc_vmstat,
        });
    }

    samples
}

#[cfg(test)]
mod vmstat_tests {
    use crate::{generate_vmstat_raw_data, ExpectedVmstatStats};
    use aperf::data::data_formats::AperfData;
    use aperf::data::vmstat::Vmstat;
    use aperf::data::Data;
    use aperf::visualizer::GetData;
    use std::collections::HashMap;

    #[test]
    fn test_process_vmstat_empty_data() {
        let raw_data: Vec<Data> = Vec::new();

        let mut vmstat = Vmstat::new();
        let result = vmstat.process_raw_data_new(raw_data).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            assert_eq!(time_series_data.metrics.len(), 0);
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_vmstat_raw_data_simple() {
        let mut expected_per_sample_stats = Vec::new();

        // Sample 0: Initial values
        let mut stats0 = HashMap::new();
        stats0.insert("pgpgin".to_string(), 1000);
        stats0.insert("pgpgout".to_string(), 2000);
        stats0.insert("nr_dirty".to_string(), 50);
        expected_per_sample_stats.push(ExpectedVmstatStats { stats: stats0 });

        // Sample 1: Incremented values
        let mut stats1 = HashMap::new();
        stats1.insert("pgpgin".to_string(), 1100);
        stats1.insert("pgpgout".to_string(), 2200);
        stats1.insert("nr_dirty".to_string(), 60);
        expected_per_sample_stats.push(ExpectedVmstatStats { stats: stats1 });

        // Sample 2: Further incremented values
        let mut stats2 = HashMap::new();
        stats2.insert("pgpgin".to_string(), 1250);
        stats2.insert("pgpgout".to_string(), 2450);
        stats2.insert("nr_dirty".to_string(), 45);
        expected_per_sample_stats.push(ExpectedVmstatStats { stats: stats2 });

        let raw_samples = generate_vmstat_raw_data(&expected_per_sample_stats, 2);
        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::VmstatRaw(s))
            .collect();

        let mut vmstat = Vmstat::new();
        let result = vmstat.process_raw_data_new(raw_data).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            assert_eq!(time_series_data.metrics.len(), 3);

            // Verify all expected metrics exist
            let expected_metrics = vec!["pgpgin", "pgpgout", "nr_dirty"];
            for metric_name in &expected_metrics {
                assert!(
                    time_series_data.metrics.contains_key(*metric_name),
                    "Missing metric: {}",
                    metric_name
                );
            }

            // Comprehensive value verification for each metric
            for (metric_name, metric) in &time_series_data.metrics {
                assert_eq!(metric.series.len(), 1);
                let series = &metric.series[0];
                assert_eq!(series.values.len(), 3);

                // Verify each value against expected data
                for (sample_idx, &value) in series.values.iter().enumerate() {
                    if metric_name.contains("nr_") {
                        // Absolute metrics - values should match expected exactly
                        let expected =
                            expected_per_sample_stats[sample_idx].stats[metric_name] as f64;
                        assert_eq!(
                            value, expected,
                            "Absolute metric {} sample {} mismatch: got {}, expected {}",
                            metric_name, sample_idx, value, expected
                        );
                    } else {
                        // Delta metrics - first sample should be 0, others should be deltas
                        if sample_idx == 0 {
                            assert_eq!(
                                value, 0.0,
                                "Delta metric {} first sample should be 0, got {}",
                                metric_name, value
                            );
                        } else {
                            let current = expected_per_sample_stats[sample_idx].stats[metric_name];
                            let previous =
                                expected_per_sample_stats[sample_idx - 1].stats[metric_name];
                            let expected_delta = (current - previous) as f64;
                            assert_eq!(
                                value, expected_delta,
                                "Delta metric {} sample {} mismatch: got {}, expected {}",
                                metric_name, sample_idx, value, expected_delta
                            );
                        }
                    }
                }
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_vmstat_raw_data_complex() {
        let num_samples = 100;
        let mut expected_per_sample_stats = Vec::new();

        // Initialize accumulated values for delta metrics
        let mut pgpgin_acc = 10000i64;
        let mut pgpgout_acc = 20000i64;
        let mut pswpin_acc = 100i64;
        let mut pswpout_acc = 200i64;
        let mut pgalloc_dma_acc = 5000i64;
        let mut pgfault_acc = 50000i64;

        for i in 0..num_samples {
            let mut stats = HashMap::new();

            // Delta metrics (accumulated values)
            pgpgin_acc += (i as f64 * 1.5 + 50.0) as i64;
            pgpgout_acc += (i as f64 * 2.0 + 100.0) as i64;
            pswpin_acc += if i % 10 == 0 { 5 } else { 0 };
            pswpout_acc += if i % 15 == 0 { 3 } else { 0 };
            pgalloc_dma_acc += (i as f64 * 0.8 + 20.0) as i64;
            pgfault_acc += (i as f64 * 3.0 + 200.0) as i64;

            stats.insert("pgpgin".to_string(), pgpgin_acc);
            stats.insert("pgpgout".to_string(), pgpgout_acc);
            stats.insert("pswpin".to_string(), pswpin_acc);
            stats.insert("pswpout".to_string(), pswpout_acc);
            stats.insert("pgalloc_dma".to_string(), pgalloc_dma_acc);
            stats.insert("pgfault".to_string(), pgfault_acc);

            // Absolute metrics (nr_ prefixed)
            stats.insert(
                "nr_dirty".to_string(),
                (i as f64 * 0.5 + 30.0).sin() as i64 + 50,
            );
            stats.insert(
                "nr_writeback".to_string(),
                (i as f64 * 0.3 + 10.0).cos() as i64 + 20,
            );
            stats.insert(
                "nr_free_pages".to_string(),
                1000000 + (i as f64 * 100.0).sin() as i64,
            );

            expected_per_sample_stats.push(ExpectedVmstatStats { stats });
        }

        let raw_samples = generate_vmstat_raw_data(&expected_per_sample_stats, 1);
        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::VmstatRaw(s))
            .collect();

        let mut vmstat = Vmstat::new();
        let result = vmstat.process_raw_data_new(raw_data).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            assert_eq!(time_series_data.metrics.len(), 9);

            // Verify all expected metrics exist
            let expected_metrics = vec![
                "pgpgin",
                "pgpgout",
                "pswpin",
                "pswpout",
                "pgalloc_dma",
                "pgfault",
                "nr_dirty",
                "nr_writeback",
                "nr_free_pages",
            ];
            for metric_name in &expected_metrics {
                assert!(
                    time_series_data.metrics.contains_key(*metric_name),
                    "Missing metric: {}",
                    metric_name
                );
            }

            // Check structure and data completeness
            for (metric_name, metric) in &time_series_data.metrics {
                assert_eq!(
                    metric.series.len(),
                    1,
                    "Metric {} should have 1 series",
                    metric_name
                );
                let series = &metric.series[0];
                assert_eq!(
                    series.values.len(),
                    num_samples,
                    "Metric {} should have {} samples",
                    metric_name,
                    num_samples
                );

                // Verify time progression
                for (i, &time_diff) in series.time_diff.iter().enumerate() {
                    assert_eq!(time_diff, i as u64, "Time diff mismatch at sample {}", i);
                }

                // Verify each value against expected data
                for (sample_idx, &value) in series.values.iter().enumerate() {
                    if metric_name.contains("nr_") {
                        // Absolute metrics - values should match expected exactly
                        let expected =
                            expected_per_sample_stats[sample_idx].stats[metric_name] as f64;
                        assert_eq!(
                            value, expected,
                            "Absolute metric {} sample {} mismatch: got {}, expected {}",
                            metric_name, sample_idx, value, expected
                        );
                    } else {
                        // Delta metrics - first sample should be 0, others should be deltas
                        if sample_idx == 0 {
                            assert_eq!(
                                value, 0.0,
                                "Delta metric {} first sample should be 0, got {}",
                                metric_name, value
                            );
                        } else {
                            let current = expected_per_sample_stats[sample_idx].stats[metric_name];
                            let previous =
                                expected_per_sample_stats[sample_idx - 1].stats[metric_name];
                            let expected_delta = (current - previous) as f64;
                            assert_eq!(
                                value, expected_delta,
                                "Delta metric {} sample {} mismatch: got {}, expected {}",
                                metric_name, sample_idx, value, expected_delta
                            );
                        }
                    }

                    // All values should be non-negative
                    assert!(
                        value >= 0.0,
                        "Negative value {} for metric {} sample {}",
                        value,
                        metric_name,
                        sample_idx
                    );
                }
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_vmstat_dynamic_stats() {
        let num_samples = 50;
        let mut expected_per_sample_stats = Vec::new();

        let mut pgpgin_acc = 1000i64;
        let mut pgpgout_acc = 2000i64;
        let mut pgfault_acc = 10000i64;

        for i in 0..num_samples {
            let mut stats = HashMap::new();

            // pgpgin appears from the start
            pgpgin_acc += 100;
            stats.insert("pgpgin".to_string(), pgpgin_acc);
            stats.insert("nr_dirty".to_string(), 50 + i as i64);

            // pgpgout appears after sample 10
            if i >= 10 {
                pgpgout_acc += 200;
                stats.insert("pgpgout".to_string(), pgpgout_acc);
            }

            // pgfault appears after sample 30
            if i >= 30 {
                pgfault_acc += 300;
                stats.insert("pgfault".to_string(), pgfault_acc);
                stats.insert("nr_writeback".to_string(), 20 + i as i64);
            }

            expected_per_sample_stats.push(ExpectedVmstatStats { stats });
        }

        let raw_samples = generate_vmstat_raw_data(&expected_per_sample_stats, 1);
        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::VmstatRaw(s))
            .collect();

        let mut vmstat = Vmstat::new();
        let result = vmstat.process_raw_data_new(raw_data).unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Should have 5 metrics total
            assert_eq!(time_series_data.metrics.len(), 5);

            // Verify expected metrics exist
            let expected_metrics = vec!["pgpgin", "nr_dirty", "pgpgout", "pgfault", "nr_writeback"];
            for metric_name in &expected_metrics {
                assert!(
                    time_series_data.metrics.contains_key(*metric_name),
                    "Missing metric: {}",
                    metric_name
                );
            }

            // Comprehensive verification for each metric
            for (metric_name, metric) in &time_series_data.metrics {
                assert_eq!(metric.series.len(), 1);
                let series = &metric.series[0];

                // Determine expected series length based on when metric appears
                let expected_length = match metric_name.as_str() {
                    "pgpgin" | "nr_dirty" => num_samples, // appear from start
                    "pgpgout" => num_samples - 10,        // appear after sample 10
                    "pgfault" | "nr_writeback" => num_samples - 30, // appear after sample 30
                    _ => panic!("Unexpected metric: {}", metric_name),
                };

                assert_eq!(
                    series.values.len(),
                    expected_length,
                    "Metric {} should have {} samples",
                    metric_name,
                    expected_length
                );

                // Verify each value against expected data
                let start_sample = match metric_name.as_str() {
                    "pgpgin" | "nr_dirty" => 0,
                    "pgpgout" => 10,
                    "pgfault" | "nr_writeback" => 30,
                    _ => panic!("Unexpected metric: {}", metric_name),
                };

                for (series_idx, &value) in series.values.iter().enumerate() {
                    let sample_idx = start_sample + series_idx;

                    if metric_name.contains("nr_") {
                        // Absolute metrics - values should match expected exactly
                        let expected =
                            expected_per_sample_stats[sample_idx].stats[metric_name] as f64;
                        assert_eq!(
                            value, expected,
                            "Absolute metric {} sample {} mismatch: got {}, expected {}",
                            metric_name, sample_idx, value, expected
                        );
                    } else {
                        // Delta metrics - first sample should be 0, others should be deltas
                        if series_idx == 0 {
                            assert_eq!(
                                value, 0.0,
                                "Delta metric {} first sample should be 0, got {}",
                                metric_name, value
                            );
                        } else {
                            let current = expected_per_sample_stats[sample_idx].stats[metric_name];
                            let previous =
                                expected_per_sample_stats[sample_idx - 1].stats[metric_name];
                            let expected_delta = (current - previous) as f64;
                            assert_eq!(
                                value, expected_delta,
                                "Delta metric {} sample {} mismatch: got {}, expected {}",
                                metric_name, sample_idx, value, expected_delta
                            );
                        }
                    }
                }
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }
}
