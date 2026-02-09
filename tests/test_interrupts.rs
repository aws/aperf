use aperf::data::interrupts::InterruptData;
use aperf::data::interrupts::InterruptDataRaw;
use aperf::data::ProcessData;
use aperf::data::{Data, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::prelude::*;
use chrono::Duration;
use std::collections::HashMap;

fn generate_interrupts_raw_data(
    expected_per_sample_per_interrupt_stats: &Vec<HashMap<String, Vec<u64>>>, // [sample][interrupt_name][cpu_id]
    num_cpus: usize,
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();
    let mut accumulated_stats: HashMap<String, Vec<u64>> = HashMap::new();

    for (sample_idx, sample_stats) in expected_per_sample_per_interrupt_stats.iter().enumerate() {
        // Update accumulated stats
        for (interrupt_name, expected_stats) in sample_stats {
            let acc_stats = accumulated_stats
                .entry(interrupt_name.clone())
                .or_insert_with(|| vec![0; num_cpus]);

            // Update per-CPU counts (accumulated)
            for (cpu_idx, &count) in expected_stats.iter().enumerate() {
                acc_stats[cpu_idx] += count;
            }
        }

        // Generate /proc/interrupts format data
        let mut proc_interrupts_data = String::new();

        // Header line with CPU names
        proc_interrupts_data.push_str("           ");
        for cpu_idx in 0..num_cpus {
            proc_interrupts_data.push_str(&format!("CPU{:<8} ", cpu_idx));
        }
        proc_interrupts_data.push('\n');

        // Generate interrupt lines
        for (interrupt_name, stats) in &accumulated_stats {
            // Handle special MIS/ERR interrupts (single value, but need per-CPU format for parser)
            if interrupt_name.to_uppercase() == "MIS" || interrupt_name.to_uppercase() == "ERR" {
                let total_count: u64 = stats.iter().sum();
                // Format with per-CPU columns (first CPU gets total, others get 0)
                proc_interrupts_data.push_str(&format!("{:>3}:", interrupt_name));
                proc_interrupts_data.push_str(&format!("{:>11}", total_count));
                proc_interrupts_data.push('\n');
                continue;
            }

            // Regular interrupts with per-CPU counts
            proc_interrupts_data.push_str(&format!("{:>3}:", interrupt_name));
            for &count in stats {
                proc_interrupts_data.push_str(&format!("{:>11}", count));
            }
            if let Ok(interrupt_number) = interrupt_name.parse::<u64>() {
                proc_interrupts_data.push_str(&format!(
                    "     Some info about irq {} ACPI:Ged",
                    interrupt_number
                ));
            } else {
                proc_interrupts_data.push_str("    Some other info about this named interrupt");
            }
            proc_interrupts_data.push('\n');
        }

        let time = TimeEnum::DateTime(
            Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()
                + Duration::seconds(sample_idx as i64 * interval_seconds as i64),
        );
        let raw_interrupt_data = InterruptDataRaw {
            time,
            data: proc_interrupts_data,
        };

        raw_data.push(Data::InterruptDataRaw(raw_interrupt_data));
    }

    raw_data
}

#[test]
fn test_process_interrupts_raw_data_complex() {
    let num_cpus = 4;
    let num_samples = 100;
    let interval_seconds = 1;

    // Generate complex test data with various interrupt types
    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..num_samples {
        let mut sample_stats = HashMap::new();

        // Numbered interrupt (timer)
        let sound_counts = vec![
            1, // CPU0
            2, // CPU1
            3, // CPU2
            4, // CPU3
        ];
        sample_stats.insert("5".to_string(), sound_counts);
        let timer_counts = vec![
            10 + sample_idx * 5, // CPU0
            12 + sample_idx * 6, // CPU1
            8 + sample_idx * 4,  // CPU2
            15 + sample_idx * 7, // CPU3
        ];
        sample_stats.insert("10".to_string(), timer_counts);
        let ott_counts = vec![
            (3 + num_samples - sample_idx) * 2, // CPU0
            (9 + num_samples - sample_idx) * 3, // CPU1
            (2 + num_samples - sample_idx) * 4, // CPU2
            (4 + num_samples - sample_idx) * 5, // CPU3
        ];
        sample_stats.insert("123".to_string(), ott_counts);

        // Named interrupt (IPI)
        let ipi_counts = vec![
            100 + sample_idx * 20, // CPU0
            95 + sample_idx * 18,  // CPU1
            110 + sample_idx * 22, // CPU2
            105 + sample_idx * 21, // CPU3
        ];
        sample_stats.insert("IPI0".to_string(), ipi_counts);
        // Named interrupt (IPI)
        let ipi_6_counts = vec![
            16 + sample_idx * 20,  // CPU0
            96 + sample_idx * 18,  // CPU1
            116 + sample_idx * 22, // CPU2
            106 + sample_idx * 21, // CPU3
        ];
        sample_stats.insert("IPI6".to_string(), ipi_6_counts);

        // Network interrupt
        let net_counts = vec![
            if sample_idx % 4 == 0 {
                50 + sample_idx * 3
            } else {
                0
            }, // CPU0
            if sample_idx % 4 == 1 {
                45 + sample_idx * 2
            } else {
                0
            }, // CPU1
            if sample_idx % 4 == 2 {
                55 + sample_idx * 4
            } else {
                0
            }, // CPU2
            if sample_idx % 4 == 3 {
                48 + sample_idx * 3
            } else {
                0
            }, // CPU3
        ];
        sample_stats.insert("25".to_string(), net_counts);

        // Special MIS interrupt (single value)
        sample_stats.insert("MIS".to_string(), vec![sample_idx * 2; 1]);

        // Special ERR interrupt (single value)
        sample_stats.insert(
            "ERR".to_string(),
            vec![if sample_idx > 50 { 1 } else { 0 }; 1],
        );

        expected_per_sample_stats.push(sample_stats);
    }

    let raw_data =
        generate_interrupts_raw_data(&expected_per_sample_stats, num_cpus, interval_seconds);
    let result = InterruptData::new()
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        let mut expected_metric_names: HashMap<&str, &str> = HashMap::new();
        expected_metric_names.insert("5", "(Some info about irq 5 ACPI:Ged)");
        expected_metric_names.insert("10", "(Some info about irq 10 ACPI:Ged)");
        expected_metric_names.insert("25", "(Some info about irq 25 ACPI:Ged)");
        expected_metric_names.insert("123", "(Some info about irq 123 ACPI:Ged)");
        expected_metric_names.insert("IPI0", "IPI0 (Some other info about this named interrupt)");
        expected_metric_names.insert("IPI6", "IPI6 (Some other info about this named interrupt)");
        expected_metric_names.insert("ERR", "ERR");
        expected_metric_names.insert("MIS", "MIS");

        // Verify metric names are sorted correctly
        let sorted_metric_keys = vec!["IPI0", "IPI6", "10", "123", "MIS", "25", "5", "ERR"];
        let sorted_metric_names: Vec<&str> = sorted_metric_keys
            .iter()
            .map(|&key| expected_metric_names[key])
            .collect();
        assert_eq!(time_series_data.metrics.len(), sorted_metric_keys.len());
        assert_eq!(time_series_data.sorted_metric_names, sorted_metric_names);

        for metric_key in sorted_metric_keys {
            let metric_name = expected_metric_names.get(metric_key).unwrap().to_owned();
            let metric = &time_series_data.metrics[metric_name];

            if metric_name == "MIS" || metric_name == "ERR" {
                assert_eq!(metric.series.len(), 1);
                assert_eq!(metric.series[0].values.len() as u64, num_samples);
                for (sample_idx, &series_value) in metric.series[0].values.iter().enumerate() {
                    if sample_idx == 0 {
                        assert_eq!(series_value, 0.0);
                    } else {
                        assert_eq!(
                            series_value as u64,
                            expected_per_sample_stats[sample_idx][metric_key][0],
                            "Metric {} sample {}: unexpected series value",
                            metric_name,
                            sample_idx
                        );
                    }
                }
            } else {
                assert_eq!(metric.series.len(), num_cpus + 1);
                let mut per_sample_sum: Vec<u64> = vec![0; num_samples as usize];
                for cpu in 0..num_cpus {
                    assert_eq!(metric.series[cpu].values.len() as u64, num_samples);
                    for (sample_idx, &series_value) in metric.series[cpu].values.iter().enumerate()
                    {
                        if sample_idx == 0 {
                            assert_eq!(series_value, 0.0);
                        } else {
                            assert_eq!(
                                series_value as u64,
                                expected_per_sample_stats[sample_idx][metric_key][cpu],
                                "Metric {} series {:?} sample {}: unexpected series value",
                                metric_name,
                                cpu,
                                sample_idx
                            );
                        }
                        per_sample_sum[sample_idx] += series_value as u64;
                    }
                }
                // Check if aggregate values are the average of the per-cpu values
                for (sample_idx, &series_value) in metric.series[num_cpus].values.iter().enumerate()
                {
                    if sample_idx == 0 {
                        assert_eq!(series_value, 0.0);
                    } else {
                        assert!(
                            (series_value - per_sample_sum[sample_idx] as f64 / num_cpus as f64)
                                .abs()
                                < 1e-5,
                            "Metric {} aggregate series sample {}: unexpected series value",
                            metric_name,
                            sample_idx
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
fn test_process_interrupts_raw_data_simple() {
    let num_cpus = 2;
    let interval_seconds = 1;

    let mut expected_per_sample_stats = Vec::new();

    // Sample 0
    let mut sample0 = HashMap::new();
    sample0.insert("11".to_string(), vec![100, 150]);
    expected_per_sample_stats.push(sample0);

    // Sample 1
    let mut sample1 = HashMap::new();
    sample1.insert("11".to_string(), vec![50, 75]);
    sample1.insert("15".to_string(), vec![50, 75]);
    expected_per_sample_stats.push(sample1);

    // Sample 2
    let mut sample2 = HashMap::new();
    sample2.insert("11".to_string(), vec![25, 30]);
    sample2.insert("15".to_string(), vec![60, 70]);
    expected_per_sample_stats.push(sample2);

    let raw_data =
        generate_interrupts_raw_data(&expected_per_sample_stats, num_cpus, interval_seconds);
    let result = InterruptData::new()
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 2);

        let irq_11_metric = &time_series_data.metrics["(Some info about irq 11 ACPI:Ged)"];
        assert_eq!(irq_11_metric.series.len(), 3); // 2 CPUs + 1 aggregate
        assert_eq!(irq_11_metric.series[0].values, vec![0.0, 50.0, 25.0]); // CPU0
        assert_eq!(irq_11_metric.series[1].values, vec![0.0, 75.0, 30.0]); // CPU1
        assert_eq!(irq_11_metric.series[2].values, vec![0.0, 62.5, 27.5]); // Average

        let irq_12_metric = &time_series_data.metrics["(Some info about irq 15 ACPI:Ged)"];
        assert_eq!(irq_12_metric.series.len(), 3); // 2 CPUs + 1 aggregate
        assert_eq!(irq_12_metric.series[0].values, vec![0.0, 60.0]); // CPU0
        assert_eq!(irq_12_metric.series[1].values, vec![0.0, 70.0]); // CPU1
        assert_eq!(irq_12_metric.series[2].values, vec![0.0, 65.0]); // Average
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_interrupts_empty_data() {
    let raw_data: Vec<Data> = Vec::new();
    let result = InterruptData::new()
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        assert_eq!(time_series_data.sorted_metric_names.len(), 0);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_interrupts_mis_err_only() {
    let num_cpus = 2;
    let num_samples = 3usize;
    let interval_seconds = 1;

    let mut expected_per_sample_stats = Vec::new();

    for sample_idx in 0..num_samples {
        let mut sample_stats = HashMap::new();

        // MIS interrupt (single value, not per-CPU)
        sample_stats.insert("MIS".to_string(), vec![sample_idx as u64 * 3; 1]);

        // ERR interrupt (single value, not per-CPU)
        sample_stats.insert(
            "ERR".to_string(),
            vec![if sample_idx > 0 { 1 } else { 0 }; 1],
        );

        expected_per_sample_stats.push(sample_stats);
    }

    let raw_data =
        generate_interrupts_raw_data(&expected_per_sample_stats, num_cpus, interval_seconds);
    let result = InterruptData::new()
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 2);

        // Verify MIS and ERR are at the end of sorted names
        assert!(time_series_data
            .sorted_metric_names
            .contains(&"MIS".to_string()));
        assert!(time_series_data
            .sorted_metric_names
            .contains(&"ERR".to_string()));

        // MIS and ERR should only have aggregate series (no per-CPU)
        let mis_metric = &time_series_data.metrics["MIS"];
        assert_eq!(mis_metric.series.len(), 1); // Only aggregate
        assert_eq!(mis_metric.series[0].values, vec![0.0, 3.0, 6.0]); // Deltas: 0, 3, 6 (accumulated: 0, 3, 9)

        let err_metric = &time_series_data.metrics["ERR"];
        assert_eq!(err_metric.series.len(), 1); // Only aggregate
        assert_eq!(err_metric.series[0].values, vec![0.0, 1.0, 1.0]); // Deltas: 0, 1, 1
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_decreasing_counter() {
    use aperf::data::data_formats::AperfData;
    use aperf::data::interrupts::InterruptDataRaw;
    use aperf::data::TimeEnum;
    use chrono::Utc;

    let base_time = Utc::now();
    let raw_samples = vec![
        InterruptDataRaw {
            time: TimeEnum::DateTime(base_time),
            data: "           CPU0       CPU1\n  0:        100        200\n".to_string(),
        },
        InterruptDataRaw {
            time: TimeEnum::DateTime(base_time + chrono::Duration::seconds(1)),
            data: "           CPU0       CPU1\n  0:         50        100\n".to_string(),
        },
    ];

    let raw_data: Vec<Data> = raw_samples
        .into_iter()
        .map(|s| Data::InterruptDataRaw(s))
        .collect();

    let mut interrupt_data = InterruptData::new();
    let result = interrupt_data
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    if let AperfData::TimeSeries(time_series_data) = result {
        for metric in time_series_data.metrics.values() {
            for series in &metric.series {
                if !series.is_aggregate {
                    assert_eq!(series.values.len(), 2);
                    assert_eq!(series.values[0], 0.0);
                    assert_eq!(series.values[1], 0.0);
                }
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}
