use aperf::data::data_formats::AperfData;
use aperf::data::perf_stat::{PerfStat, PerfStatRaw};
use aperf::data::{Data, TimeEnum};
use aperf::visualizer::GetData;
use aperf::visualizer::ReportParams;
use chrono::prelude::*;
use std::collections::HashMap;

/// Expected PMU statistics for a single CPU and stat name
#[derive(Debug, Clone)]
struct ExpectedPmuStats {
    pub numerators: Vec<u64>,
    pub denominators: Vec<u64>,
    pub scale: u64,
}

impl ExpectedPmuStats {
    fn new(numerators: Vec<u64>, denominators: Vec<u64>, scale: u64) -> Self {
        Self {
            numerators,
            denominators,
            scale,
        }
    }

    fn calculate_value(&self) -> f64 {
        let numerator_sum: u64 = self.numerators.iter().sum();
        let denominator_sum: u64 = self.denominators.iter().sum();
        (numerator_sum as f64) / (denominator_sum as f64) * (self.scale as f64)
    }
}

/// Generate fake PMU stat raw data in the format: "<cpu> <stat_name>; <numerators>; <denominators>;<scale>"
fn generate_pmu_stat_raw_data(
    expected_per_sample_per_cpu_per_stat: &Vec<HashMap<usize, HashMap<String, ExpectedPmuStats>>>, // [sample][cpu][stat_name]
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();
    let base_time = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    for (sample_idx, per_cpu_per_stat) in expected_per_sample_per_cpu_per_stat.iter().enumerate() {
        let mut data_lines = Vec::new();

        // Sort CPUs to ensure consistent ordering
        let mut sorted_cpus: Vec<_> = per_cpu_per_stat.keys().collect();
        sorted_cpus.sort();

        for &cpu in sorted_cpus {
            let per_stat = &per_cpu_per_stat[&cpu];
            for (stat_name, expected_stats) in per_stat {
                let numerators_str = expected_stats
                    .numerators
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                let denominators_str = expected_stats
                    .denominators
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                let line = format!(
                    "{} {}; {}; {};{}",
                    cpu, stat_name, numerators_str, denominators_str, expected_stats.scale
                );
                data_lines.push(line);
            }
        }

        let time = TimeEnum::DateTime(
            base_time + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64)),
        );
        let raw_stat = PerfStatRaw {
            time,
            data: data_lines.join("\n"),
        };
        raw_data.push(Data::PerfStatRaw(raw_stat));
    }

    raw_data
}

#[test]
fn test_process_pmu_stat_raw_data_complex() {
    let mut expected_data = Vec::new();

    // Generate 100 samples with 4 CPUs and 3 PMU stats
    for sample in 0usize..100 {
        let mut per_cpu_per_stat = HashMap::new();

        for cpu in 0usize..4 {
            let mut per_stat = HashMap::new();

            // IPC stat with varying patterns
            let ipc_numerators = vec![
                (1000 + sample * 10 + cpu * 5) as u64,
                (2000 + sample * 15 + cpu * 7) as u64,
            ];
            let ipc_denominators = vec![
                (5000 + sample * 50 + cpu * 25) as u64,
                (10000 + sample * 75 + cpu * 35) as u64,
            ];
            per_stat.insert(
                "ipc".to_string(),
                ExpectedPmuStats::new(ipc_numerators, ipc_denominators, 1),
            );

            // Cache miss rate with different scale
            let cache_numerators = vec![(100 + sample * 2 + cpu) as u64];
            let cache_denominators = vec![(10000 + sample * 100 + cpu * 50) as u64];
            per_stat.insert(
                "cache_miss_rate".to_string(),
                ExpectedPmuStats::new(cache_numerators, cache_denominators, 100),
            );

            // Branch prediction accuracy
            let branch_numerators = vec![(9000 + sample * 5 + cpu * 2) as u64];
            let branch_denominators = vec![(10000 + sample * 10 + cpu * 3) as u64];
            per_stat.insert(
                "branch_accuracy".to_string(),
                ExpectedPmuStats::new(branch_numerators, branch_denominators, 1000),
            );

            per_cpu_per_stat.insert(cpu, per_stat);
        }

        expected_data.push(per_cpu_per_stat);
    }

    let raw_data = generate_pmu_stat_raw_data(&expected_data, 1);
    let result = PerfStat::new()
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        // Validate structure: 3 metrics (branch_accuracy, cache_miss_rate, ipc - sorted alphabetically)
        assert_eq!(time_series_data.metrics.len(), 3);
        assert_eq!(time_series_data.sorted_metric_names.len(), 3);
        assert_eq!(
            time_series_data.sorted_metric_names,
            vec!["branch_accuracy", "cache_miss_rate", "ipc"]
        );

        // Validate each metric has 5 series (4 CPUs + 1 aggregate)
        for metric in time_series_data.metrics.values() {
            assert_eq!(metric.series.len(), 5);
            // Each series should have 100 data points
            for series in &metric.series {
                assert_eq!(series.values.len(), 100);
                assert_eq!(series.time_diff.len(), 100);
            }
        }

        // Validate ALL values for ALL metrics and ALL CPUs
        for (stat_name, metric) in &time_series_data.metrics {
            for sample in 0..100 {
                let mut aggregate_numerator_sum = 0.0;
                let mut aggregate_denominator_sum = 0.0;

                // Check each CPU series value
                for cpu in 0..4 {
                    let cpu_series = &metric.series[cpu];
                    let expected_stats = &expected_data[sample][&cpu][stat_name];
                    let expected_value = expected_stats.calculate_value();

                    assert!(
                        (cpu_series.values[sample] - expected_value).abs() < 1e-5,
                        "Sample {}, CPU {}, stat {}: expected {}, got {}",
                        sample,
                        cpu,
                        stat_name,
                        expected_value,
                        cpu_series.values[sample]
                    );

                    // Accumulate for aggregate calculation
                    let numerator_sum: u64 = expected_stats.numerators.iter().sum();
                    let denominator_sum: u64 = expected_stats.denominators.iter().sum();
                    aggregate_numerator_sum +=
                        (numerator_sum as f64) * (expected_stats.scale as f64);
                    aggregate_denominator_sum += denominator_sum as f64;
                }

                // Check aggregate series value
                let aggregate_series = &metric.series[4];
                let expected_aggregate_value = aggregate_numerator_sum / aggregate_denominator_sum;
                assert!(
                    (aggregate_series.values[sample] - expected_aggregate_value).abs() < 1e-5,
                    "Sample {}, stat {} aggregate: expected {}, got {}",
                    sample,
                    stat_name,
                    expected_aggregate_value,
                    aggregate_series.values[sample]
                );
            }
        }

        // Validate aggregate series is marked correctly
        for metric in time_series_data.metrics.values() {
            let aggregate_series = &metric.series[4]; // Last series should be aggregate
            assert!(aggregate_series.is_aggregate);
            assert_eq!(aggregate_series.series_name, Some("Aggregate".to_string()));
        }

        // Validate time differences
        for metric in time_series_data.metrics.values() {
            for series in &metric.series {
                for sample in 0..100 {
                    assert_eq!(series.time_diff[sample], sample as u64);
                }
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_pmu_stat_raw_data_simple() {
    let mut expected_data = Vec::new();

    // Generate 3 samples with 2 CPUs and 1 PMU stat
    for sample in 0usize..3 {
        let mut per_cpu_per_stat = HashMap::new();

        for cpu in 0usize..2 {
            let mut per_stat = HashMap::new();
            let numerators = vec![(1000 + sample * 100 + cpu * 10) as u64];
            let denominators = vec![(5000 + sample * 200 + cpu * 20) as u64];
            per_stat.insert(
                "simple_stat".to_string(),
                ExpectedPmuStats::new(numerators, denominators, 1),
            );
            per_cpu_per_stat.insert(cpu, per_stat);
        }

        expected_data.push(per_cpu_per_stat);
    }

    let raw_data = generate_pmu_stat_raw_data(&expected_data, 2);
    let result = PerfStat::new()
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 1);
        assert_eq!(time_series_data.sorted_metric_names, vec!["simple_stat"]);

        let metric = &time_series_data.metrics["simple_stat"];
        assert_eq!(metric.series.len(), 3); // 2 CPUs + 1 aggregate

        // Validate ALL values for ALL samples and CPUs
        for sample in 0..3 {
            let mut aggregate_numerator_sum = 0.0;
            let mut aggregate_denominator_sum = 0.0;

            // Check each CPU series value
            for cpu in 0..2 {
                let cpu_series = &metric.series[cpu];
                let expected_stats = &expected_data[sample][&cpu]["simple_stat"];
                let expected_value = expected_stats.calculate_value();

                assert!(
                    (cpu_series.values[sample] - expected_value).abs() < 1e-5,
                    "Sample {}, CPU {}: expected {}, got {}",
                    sample,
                    cpu,
                    expected_value,
                    cpu_series.values[sample]
                );

                // Accumulate for aggregate calculation
                let numerator_sum: u64 = expected_stats.numerators.iter().sum();
                let denominator_sum: u64 = expected_stats.denominators.iter().sum();
                aggregate_numerator_sum += (numerator_sum as f64) * (expected_stats.scale as f64);
                aggregate_denominator_sum += denominator_sum as f64;
            }

            // Check aggregate series value
            let aggregate_series = &metric.series[2];
            let expected_aggregate_value = aggregate_numerator_sum / aggregate_denominator_sum;
            assert!(
                (aggregate_series.values[sample] - expected_aggregate_value).abs() < 1e-5,
                "Sample {} aggregate: expected {}, got {}",
                sample,
                expected_aggregate_value,
                aggregate_series.values[sample]
            );
        }

        // Validate time intervals
        for series in &metric.series {
            assert_eq!(series.time_diff, vec![0, 2, 4]); // 2-second intervals
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_pmu_stat_multiple_numerators_denominators() {
    let mut expected_data = Vec::new();

    // Generate 3 samples with multiple numerators and denominators
    for sample in 0usize..3 {
        let mut per_cpu_per_stat = HashMap::new();

        for cpu in 0usize..2 {
            let mut per_stat = HashMap::new();
            // Multiple numerators and denominators that should be summed
            let numerators = vec![
                (100 + sample * 10) as u64,
                (200 + sample * 20) as u64,
                (300 + sample * 30) as u64,
            ];
            let denominators = vec![(1000 + sample * 100) as u64, (2000 + sample * 200) as u64];
            per_stat.insert(
                "multi_counter_stat".to_string(),
                ExpectedPmuStats::new(numerators, denominators, 1),
            );
            per_cpu_per_stat.insert(cpu, per_stat);
        }

        expected_data.push(per_cpu_per_stat);
    }

    let raw_data = generate_pmu_stat_raw_data(&expected_data, 1);
    let result = PerfStat::new()
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        let metric = &time_series_data.metrics["multi_counter_stat"];

        // Validate ALL values for ALL samples and CPUs
        for sample in 0..3 {
            let mut aggregate_numerator_sum = 0.0;
            let mut aggregate_denominator_sum = 0.0;

            // Check each CPU series value
            for cpu in 0..2 {
                let cpu_series = &metric.series[cpu];
                let expected_stats = &expected_data[sample][&cpu]["multi_counter_stat"];
                let expected_value = expected_stats.calculate_value();

                assert!(
                    (cpu_series.values[sample] - expected_value).abs() < 1e-5,
                    "Sample {}, CPU {}: expected {}, got {}",
                    sample,
                    cpu,
                    expected_value,
                    cpu_series.values[sample]
                );

                // Accumulate for aggregate calculation
                let numerator_sum: u64 = expected_stats.numerators.iter().sum();
                let denominator_sum: u64 = expected_stats.denominators.iter().sum();
                aggregate_numerator_sum += (numerator_sum as f64) * (expected_stats.scale as f64);
                aggregate_denominator_sum += denominator_sum as f64;
            }

            // Check aggregate series value
            let aggregate_series = &metric.series[2];
            let expected_aggregate_value = aggregate_numerator_sum / aggregate_denominator_sum;
            assert!(
                (aggregate_series.values[sample] - expected_aggregate_value).abs() < 1e-5,
                "Sample {} aggregate: expected {}, got {}",
                sample,
                expected_aggregate_value,
                aggregate_series.values[sample]
            );
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_pmu_stat_empty_data() {
    let raw_data = Vec::new();
    let result = PerfStat::new()
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        assert_eq!(time_series_data.sorted_metric_names.len(), 0);
    } else {
        panic!("Expected TimeSeries data");
    }
}
