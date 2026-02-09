use aperf::data::cpu_utilization::{CpuState, CpuUtilizationRaw};
use aperf::data::TimeEnum;
use chrono::Utc;
use std::collections::HashMap;
use strum::IntoEnumIterator;

#[derive(Clone, Debug, Default)]
struct ExpectedCpuStateUtilization {
    pub user: f64,
    pub nice: f64,
    pub system: f64,
    pub idle: f64,
    pub iowait: f64,
    pub irq: f64,
    pub softirq: f64,
    pub steal: f64,
}

fn get_expected_utilization(
    cpu_state: CpuState,
    expected_cpu_state_utilization: &ExpectedCpuStateUtilization,
) -> f64 {
    match cpu_state {
        CpuState::USER => expected_cpu_state_utilization.user,
        CpuState::NICE => expected_cpu_state_utilization.nice,
        CpuState::SYSTEM => expected_cpu_state_utilization.system,
        CpuState::IDLE => expected_cpu_state_utilization.idle,
        CpuState::IOWAIT => expected_cpu_state_utilization.iowait,
        CpuState::IRQ => expected_cpu_state_utilization.irq,
        CpuState::SOFTIRQ => expected_cpu_state_utilization.softirq,
        CpuState::STEAL => expected_cpu_state_utilization.steal,
    }
}

/// Generate /proc/stat data based on expected CPU utilization and wrap generated data
/// in CpuUtilizationRaw to mock collected cpu_utilization data
fn generate_cpu_utilization_raw_data(
    expected_per_sample_per_cpu_utils: &Vec<Vec<ExpectedCpuStateUtilization>>, // [sample][cpu]
    interval_seconds: u64,
    ghz: f64,
) -> Vec<CpuUtilizationRaw> {
    let mut samples: Vec<CpuUtilizationRaw> = Vec::new();
    let base_time = Utc::now();

    let num_cpus = expected_per_sample_per_cpu_utils[0].len();
    let hertz = (ghz * 1_000_000_000.0) as u64;

    // Track accumulated per-state jiffies for each CPU
    let mut per_cpu_accumulated_jiffies: Vec<HashMap<CpuState, u64>> =
        vec![HashMap::new(); num_cpus];

    for (sample_idx, expected_per_cpu_utils) in expected_per_sample_per_cpu_utils.iter().enumerate()
    {
        let time =
            base_time + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64));

        // Calculate jiffies delta for this interval
        let interval_jiffies = interval_seconds * hertz;

        let mut proc_stat = String::new();

        // Process per-CPU data first to calculate aggregates
        let mut cpu_lines = Vec::new();

        for (cpu, expected_utils) in expected_per_cpu_utils.iter().enumerate() {
            let mut cpu_line: String = if cpu == num_cpus - 1 {
                "cpu  ".to_string()
            } else {
                format!("cpu{} ", cpu)
            };
            for cpu_state in CpuState::iter() {
                let expected_util = get_expected_utilization(cpu_state, &expected_utils);
                let delta = (interval_jiffies as f64 * expected_util / 100.0) as u64;
                let acc = per_cpu_accumulated_jiffies[cpu]
                    .entry(cpu_state)
                    .or_insert(0);
                *acc += delta;
                cpu_line.push_str(&format!("{} ", *acc));
                // cur_cpu_values.push(*acc);
                // *aggregate_jiffies.entry(cpu_state).or_insert(0) += delta;
            }
            cpu_line.push_str("0 0");

            if cpu == num_cpus - 1 {
                // Handling aggregate jiffies
                proc_stat.push_str(&cpu_line);
                proc_stat.push('\n');
            } else {
                cpu_lines.push(cpu_line);
            }
        }

        // Add per-CPU lines
        for cpu_line in cpu_lines {
            proc_stat.push_str(&cpu_line);
            proc_stat.push('\n');
        }

        // Add some additional lines that are typically in /proc/stat
        proc_stat.push_str("intr 0\n");
        proc_stat.push_str("ctxt 0\n");
        proc_stat.push_str("btime 1234567890\n");
        proc_stat.push_str("processes 0\n");
        proc_stat.push_str("procs_running 1\n");
        proc_stat.push_str("procs_blocked 0\n");

        samples.push(CpuUtilizationRaw {
            time: TimeEnum::DateTime(time),
            data: proc_stat,
        });
    }

    samples
}

#[cfg(test)]
mod cpu_utilization_tests {
    use crate::{
        generate_cpu_utilization_raw_data, get_expected_utilization, ExpectedCpuStateUtilization,
    };
    use aperf::data::cpu_utilization::{CpuState, CpuUtilization};
    use aperf::data::data_formats::AperfData;
    use aperf::data::{Data, ProcessData};
    use aperf::visualizer::ReportParams;
    use strum::IntoEnumIterator;

    #[test]
    fn test_process_cpu_utilization_empty_data() {
        let raw_data: Vec<Data> = Vec::new();

        let mut cpu_util = CpuUtilization::new();
        let result = cpu_util
            .process_raw_data(ReportParams::new(), raw_data)
            .unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // With no raw data, no metrics are created (including aggregate)
            assert_eq!(time_series_data.metrics.len(), 0);

            // Sorted metric names should still be present (initialized from enum)
            assert_eq!(
                time_series_data.sorted_metric_names.len(),
                CpuState::iter().count() + 1
            ); // +1 for aggregate
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_negative_time_difference() {
        use aperf::data::cpu_utilization::CpuUtilizationRaw;
        use aperf::data::TimeEnum;
        use chrono::Utc;

        let base_time = Utc::now();
        let raw_samples = vec![
            CpuUtilizationRaw {
                time: TimeEnum::DateTime(base_time),
                data: "cpu  100 0 50 1000 0 0 0 0 0 0\ncpu0 100 0 50 1000 0 0 0 0 0 0\n"
                    .to_string(),
            },
            CpuUtilizationRaw {
                time: TimeEnum::DateTime(base_time + chrono::Duration::seconds(1)),
                data: "cpu  50 0 25 500 0 0 0 0 0 0\ncpu0 50 0 25 500 0 0 0 0 0 0\n".to_string(),
            },
        ];

        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::CpuUtilizationRaw(s))
            .collect();

        let mut cpu_util = CpuUtilization::new();
        let result = cpu_util
            .process_raw_data(ReportParams::new(), raw_data)
            .unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            for metric in time_series_data.metrics.values() {
                for series in &metric.series {
                    assert_eq!(series.values.len(), 2);
                    assert_eq!(series.values[0], 0.0);
                    assert_eq!(series.values[1], 0.0);
                }
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_cpu_utilization_raw_data() {
        let num_cpus = 192;
        let num_samples = 2500;
        let mut expected_per_sample_per_cpu_utils = Vec::new();

        for i in 0..num_samples {
            let mut expected_per_cpu_utils =
                vec![ExpectedCpuStateUtilization::default(); num_cpus + 1];
            // The last one is for aggregate
            for cpu in 0..(num_cpus + 1) {
                // Create varying utilization patterns for different CPUs and states
                let base = (i as f64 * 0.5 + cpu as f64 * 10.0) % 60.0;
                let user = (base + (i as f64 * 0.3).sin() * 15.0).max(0.0).min(40.0);
                let system = ((i as f64 * 0.2).cos() * 10.0 + 8.0).max(0.0).min(20.0);
                let iowait = ((i as f64 * 0.1 + cpu as f64).sin() * 3.0 + 3.0)
                    .max(0.0)
                    .min(8.0);
                let irq = (i as f64 * 0.05 % 3.0).max(0.0);
                let softirq = ((i as f64 * 0.07).cos() * 2.0 + 1.0).max(0.0).min(4.0);
                let nice = (i as f64 * 0.03 % 2.0).max(0.0);
                let steal = if i % 30 == 0 { 1.0 } else { 0.0 };
                let idle =
                    (100.0 - user - system - iowait - irq - softirq - nice - steal).max(20.0);

                expected_per_cpu_utils[cpu] = ExpectedCpuStateUtilization {
                    user,
                    nice,
                    system,
                    idle,
                    iowait,
                    irq,
                    softirq,
                    steal,
                };
            }

            expected_per_sample_per_cpu_utils.push(expected_per_cpu_utils);
        }

        let raw_samples =
            generate_cpu_utilization_raw_data(&expected_per_sample_per_cpu_utils, 1, 2.8);
        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::CpuUtilizationRaw(s))
            .collect();

        let mut cpu_util = CpuUtilization::new();
        let result = cpu_util
            .process_raw_data(ReportParams::new(), raw_data)
            .unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Check each CPU state metric
            for cpu_state in CpuState::iter() {
                assert!(
                    time_series_data
                        .metrics
                        .contains_key(&cpu_state.to_string()),
                    "Missing metric: {}",
                    cpu_state
                );

                let metric = time_series_data
                    .metrics
                    .get(&cpu_state.to_string())
                    .unwrap();

                // Series should have all CPUs + 1 aggregate series
                assert_eq!(metric.series.len(), num_cpus + 1);

                for (cpu, series) in metric.series.iter().enumerate() {
                    // Each series should have all data points
                    assert_eq!(series.values.len(), num_samples);
                    if series.is_aggregate {
                        assert_eq!(
                            series.series_name.as_ref().unwrap(),
                            "Aggregate",
                            "Unexpected aggregate series for CPU {} {}",
                            cpu,
                            cpu_state
                        );
                    } else {
                        // Series name should indicate the corresponding CPU number
                        assert_eq!(series.series_name.as_ref().unwrap(), &format!("CPU{}", cpu));
                    }
                    // Verify that series values are as expected
                    for (sample_idx, &value) in series.values.iter().enumerate() {
                        let expected_utilization = if sample_idx == 0 {
                            0.0
                        } else {
                            get_expected_utilization(
                                cpu_state,
                                &expected_per_sample_per_cpu_utils[sample_idx][cpu],
                            )
                        };
                        assert!(
                            (value - expected_utilization).abs() < 1e-5,
                            "Metric {} series {:?} sample {}: expected {}, got {}",
                            metric.metric_name,
                            series.series_name,
                            sample_idx,
                            expected_utilization,
                            value
                        );
                        assert!(
                            value >= 0.0 && value <= 100.0,
                            "Metric {} series {:?} sample {}: invalid utilization value {}",
                            metric.metric_name,
                            series.series_name,
                            sample_idx,
                            value
                        );
                    }
                }
            }

            // Verify aggregate metric exists and has correct structure
            assert!(
                time_series_data.metrics.contains_key("aggregate"),
                "Mising aggregate metric"
            );
            let aggregate_metric = time_series_data.metrics.get("aggregate").unwrap();
            // Should have all CPU states + total utilization
            assert_eq!(aggregate_metric.series.len(), 9);
        } else {
            panic!("Expected TimeSeries data");
        }
    }
}
