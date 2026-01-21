use aperf::data::diskstats::{DiskStatKey, DiskstatsRaw};
use aperf::data::TimeEnum;
use chrono::Utc;
use std::collections::HashMap;
use strum::IntoEnumIterator;

#[derive(Clone, Debug, Default)]
struct ExpectedDiskStats {
    pub reads: u64,
    pub merged: u64,
    pub sectors_read: u64,
    pub time_reading: u64,
    pub writes: u64,
    pub writes_merged: u64,
    pub sectors_written: u64,
    pub time_writing: u64,
    pub in_progress: u64,
    pub time_in_progress: u64,
    pub weighted_time_in_progress: u64,
    pub discards: u64,
    pub discards_merged: u64,
    pub sectors_discarded: u64,
    pub time_discarding: u64,
    pub flushes: u64,
    pub time_flushing: u64,
}

fn get_disk_stat_field(
    disk_stat_key: DiskStatKey,
    expected_disk_stats: &mut ExpectedDiskStats,
) -> &mut u64 {
    match disk_stat_key {
        DiskStatKey::Reads => &mut expected_disk_stats.reads,
        DiskStatKey::Merged => &mut expected_disk_stats.merged,
        DiskStatKey::SectorsRead => &mut expected_disk_stats.sectors_read,
        DiskStatKey::TimeReading => &mut expected_disk_stats.time_reading,
        DiskStatKey::Writes => &mut expected_disk_stats.writes,
        DiskStatKey::WritesMerged => &mut expected_disk_stats.writes_merged,
        DiskStatKey::SectorsWritten => &mut expected_disk_stats.sectors_written,
        DiskStatKey::TimeWriting => &mut expected_disk_stats.time_writing,
        DiskStatKey::InProgress => &mut expected_disk_stats.in_progress,
        DiskStatKey::TimeInProgress => &mut expected_disk_stats.time_in_progress,
        DiskStatKey::WeightedTimeInProgress => &mut expected_disk_stats.weighted_time_in_progress,
        DiskStatKey::Discards => &mut expected_disk_stats.discards,
        DiskStatKey::DiscardsMerged => &mut expected_disk_stats.discards_merged,
        DiskStatKey::SectorsDiscarded => &mut expected_disk_stats.sectors_discarded,
        DiskStatKey::TimeDiscarding => &mut expected_disk_stats.time_discarding,
        DiskStatKey::Flushes => &mut expected_disk_stats.flushes,
        DiskStatKey::TimeFlushing => &mut expected_disk_stats.time_flushing,
    }
}

/// Generate /proc/diskstats data based on expected disk statistics and wrap generated data
/// in DiskstatsRaw to mock collected diskstats data
fn generate_diskstats_raw_data(
    expected_per_sample_per_device_stats: &Vec<HashMap<String, ExpectedDiskStats>>, // [sample][device]
    interval_seconds: u64,
) -> Vec<DiskstatsRaw> {
    let mut samples: Vec<DiskstatsRaw> = Vec::new();
    let base_time = Utc::now();

    // Track accumulated stats for each device
    let mut per_device_accumulated_stats: HashMap<String, ExpectedDiskStats> = HashMap::new();

    for (sample_idx, expected_per_device_stats) in
        expected_per_sample_per_device_stats.iter().enumerate()
    {
        let time =
            base_time + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64));

        let mut proc_diskstats = String::new();

        for (device, expected_stats) in expected_per_device_stats {
            let accumulated_stats = per_device_accumulated_stats
                .entry(device.clone())
                .or_insert(ExpectedDiskStats::default());

            // Update accumulated stats and collect values for output
            let mut stat_values = Vec::new();
            for disk_stat_key in DiskStatKey::iter() {
                let expected_value =
                    *get_disk_stat_field(disk_stat_key, &mut expected_stats.clone());
                let accumulated_field = get_disk_stat_field(disk_stat_key, accumulated_stats);
                *accumulated_field = match disk_stat_key {
                    DiskStatKey::InProgress => expected_value, // Not accumulated
                    _ => *accumulated_field + expected_value,
                };
                stat_values.push(*accumulated_field);
            }

            // Generate /proc/diskstats line: major minor device_name stats...
            let major = if device.starts_with("nvme") { 259 } else { 8 };
            let minor = if device == "sda" {
                0
            } else if device == "sdb" {
                16
            } else {
                1
            };

            proc_diskstats.push_str(&format!(
                "{:4} {:4} {} {}\n",
                major,
                minor,
                device,
                stat_values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }

        samples.push(DiskstatsRaw {
            time: TimeEnum::DateTime(time),
            data: proc_diskstats,
        });
    }

    samples
}

#[cfg(test)]
mod diskstats_tests {
    use crate::{generate_diskstats_raw_data, get_disk_stat_field, ExpectedDiskStats};
    use aperf::data::data_formats::AperfData;
    use aperf::data::diskstats::{DiskStatKey, Diskstats};
    use aperf::data::{Data, ProcessData};
    use aperf::visualizer::ReportParams;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    #[test]
    fn test_process_diskstats_raw_data() {
        let num_samples = 100;
        let devices = vec!["sda".to_string(), "sdb".to_string(), "nvme0n1".to_string()];
        let mut expected_per_sample_per_device_stats = Vec::new();

        for i in 0..num_samples {
            let mut expected_per_device_stats = HashMap::new();

            for (device_idx, device) in devices.iter().enumerate() {
                // Create varying disk activity patterns
                let base = (i as f64 * 0.1 + device_idx as f64 * 5.0) % 30.0;
                let reads = ((base + (i as f64 * 0.2).sin() * 10.0).max(0.0) * 100.0) as u64;
                let writes = ((base * 0.8 + (i as f64 * 0.15).cos() * 8.0).max(0.0) * 80.0) as u64;
                let sectors_read = reads * 8; // 8 sectors per read on average
                let sectors_written = writes * 6; // 6 sectors per write on average
                let time_reading = reads / 10; // Time proportional to operations
                let time_writing = writes / 12;
                let in_progress = if i % 20 == 0 { 2 } else { 0 }; // Occasional in-progress operations
                let discards = if i % 10 == 0 { reads / 20 } else { 0 }; // Occasional discards
                let flushes = if i % 15 == 0 { 5 } else { 0 }; // Occasional flushes

                expected_per_device_stats.insert(
                    device.clone(),
                    ExpectedDiskStats {
                        reads,
                        merged: reads / 20,
                        sectors_read,
                        time_reading,
                        writes,
                        writes_merged: writes / 15,
                        sectors_written,
                        time_writing,
                        in_progress,
                        time_in_progress: time_reading + time_writing,
                        weighted_time_in_progress: (time_reading + time_writing) * 2,
                        discards,
                        discards_merged: discards / 10,
                        sectors_discarded: discards * 4,
                        time_discarding: discards / 5,
                        flushes,
                        time_flushing: flushes * 2,
                    },
                );
            }

            expected_per_sample_per_device_stats.push(expected_per_device_stats);
        }

        let raw_samples = generate_diskstats_raw_data(&expected_per_sample_per_device_stats, 1);
        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::DiskstatsRaw(s))
            .collect();

        let mut diskstats = Diskstats::new();
        let result = diskstats
            .process_raw_data(ReportParams::new(), raw_data)
            .unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Check each disk stat metric using the enum
            for disk_stat_key in DiskStatKey::iter() {
                let metric_name = disk_stat_key.to_string();
                assert!(
                    time_series_data.metrics.contains_key(&metric_name),
                    "Missing metric: {}",
                    metric_name
                );

                let metric = time_series_data.metrics.get(&metric_name).unwrap();

                // Should have series for all devices
                assert_eq!(metric.series.len(), devices.len());

                for series in &metric.series {
                    // Each series should have all data points
                    assert_eq!(series.values.len(), num_samples);

                    // Series name should match device name
                    assert!(devices.contains(series.series_name.as_ref().unwrap()));

                    // Verify values are reasonable
                    for (sample_idx, &value) in series.values.iter().enumerate() {
                        // First sample should be 0 for accumulated metrics (except in_progress)
                        if sample_idx == 0 && disk_stat_key != DiskStatKey::InProgress {
                            assert_eq!(
                                value,
                                0.0,
                                "First sample should be 0 for metric {} device {}",
                                metric_name,
                                series.series_name.as_ref().unwrap()
                            );
                            continue;
                        }

                        // Values should be non-negative
                        assert!(
                            value >= 0.0,
                            "Negative value {} for metric {} device {} sample {}",
                            value,
                            metric_name,
                            series.series_name.as_ref().unwrap(),
                            sample_idx
                        );

                        let device_name = series.series_name.as_ref().unwrap();
                        let expected_stats =
                            &expected_per_sample_per_device_stats[sample_idx][device_name];
                        let expected_value =
                            *get_disk_stat_field(disk_stat_key, &mut expected_stats.clone()) as f64;
                        assert!(
                            (value - expected_value).abs() < 1e-5,
                            "Metric {} device {} sample {}: expected {}, got {}",
                            metric_name,
                            device_name,
                            sample_idx,
                            expected_value,
                            value
                        );
                    }
                }
            }

            // Verify sorted metric names
            assert_eq!(
                time_series_data.sorted_metric_names.len(),
                DiskStatKey::iter().count()
            );
            for disk_stat_key in DiskStatKey::iter() {
                assert!(time_series_data
                    .sorted_metric_names
                    .contains(&disk_stat_key.to_string()));
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_diskstats_empty_data() {
        let raw_data: Vec<Data> = Vec::new();

        let mut diskstats = Diskstats::new();
        let result = diskstats
            .process_raw_data(ReportParams::new(), raw_data)
            .unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // With no raw data, no metrics are created
            assert_eq!(time_series_data.metrics.len(), 0);

            // Sorted metric names should still be present (initialized from enum)
            assert_eq!(
                time_series_data.sorted_metric_names.len(),
                DiskStatKey::iter().count()
            );
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_diskstats_dynamic_devices() {
        let num_samples = 50;
        let mut expected_per_sample_per_device_stats = Vec::new();

        for i in 0..num_samples {
            let mut expected_per_device_stats = HashMap::new();

            // sda exists from the beginning
            expected_per_device_stats.insert(
                "sda".to_string(),
                ExpectedDiskStats {
                    reads: 100,
                    writes: 50,
                    sectors_read: 800,
                    sectors_written: 400,
                    ..Default::default()
                },
            );

            // sdb appears after sample 10
            if i > 10 {
                expected_per_device_stats.insert(
                    "sdb".to_string(),
                    ExpectedDiskStats {
                        reads: 80,
                        writes: 40,
                        sectors_read: 640,
                        sectors_written: 320,
                        ..Default::default()
                    },
                );
            }

            // nvme0n1 appears after sample 30
            if i > 30 {
                expected_per_device_stats.insert(
                    "nvme0n1".to_string(),
                    ExpectedDiskStats {
                        reads: 120,
                        writes: 60,
                        sectors_read: 960,
                        sectors_written: 480,
                        ..Default::default()
                    },
                );
            }

            expected_per_sample_per_device_stats.push(expected_per_device_stats);
        }

        let raw_samples = generate_diskstats_raw_data(&expected_per_sample_per_device_stats, 1);
        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::DiskstatsRaw(s))
            .collect();

        let mut diskstats = Diskstats::new();
        let result = diskstats
            .process_raw_data(ReportParams::new(), raw_data)
            .unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            let reads_metric = time_series_data.metrics.get("reads").unwrap();

            // Should have 3 device series
            assert_eq!(reads_metric.series.len(), 3);

            // Find each device series
            let sda_series = reads_metric
                .series
                .iter()
                .find(|s| s.series_name.as_ref().unwrap() == "sda")
                .unwrap();
            let sdb_series = reads_metric
                .series
                .iter()
                .find(|s| s.series_name.as_ref().unwrap() == "sdb")
                .unwrap();
            let nvme_series = reads_metric
                .series
                .iter()
                .find(|s| s.series_name.as_ref().unwrap() == "nvme0n1")
                .unwrap();

            // sda should have full length (appears from sample 0)
            assert_eq!(sda_series.values.len(), num_samples);
            assert_eq!(sda_series.values[0], 0.0); // First sample is always 0
            for i in 1..num_samples {
                assert_eq!(sda_series.values[i], 100.0);
            }

            // sdb should have length from when it first appears (sample 11 onwards)
            let sdb_expected_length = num_samples - 11;
            assert_eq!(sdb_series.values.len(), sdb_expected_length);
            assert_eq!(sdb_series.values[0], 0.0); // First appearance is always 0
            for i in 1..sdb_expected_length {
                assert_eq!(sdb_series.values[i], 80.0);
            }

            // nvme0n1 should have length from when it first appears (sample 31 onwards)
            let nvme_expected_length = num_samples - 31;
            assert_eq!(nvme_series.values.len(), nvme_expected_length);
            assert_eq!(nvme_series.values[0], 0.0); // First appearance is always 0
            for i in 1..nvme_expected_length {
                assert_eq!(nvme_series.values[i], 120.0);
            }
        } else {
            panic!("Expected TimeSeries data");
        }
    }

    #[test]
    fn test_process_diskstats_simple() {
        let mut expected_per_sample_per_device_stats = Vec::new();

        // Create 3 simple samples with delta values
        for i in 0..3 {
            let mut expected_per_device_stats = HashMap::new();
            expected_per_device_stats.insert(
                "sda".to_string(),
                ExpectedDiskStats {
                    reads: 100,           // 100 reads per sample
                    writes: 50,           // 50 writes per sample
                    sectors_read: 800,    // 800 sectors per sample
                    sectors_written: 400, // 400 sectors per sample
                    in_progress: if i == 1 { 1 } else { 0 },
                    ..Default::default()
                },
            );
            expected_per_sample_per_device_stats.push(expected_per_device_stats);
        }

        let raw_samples = generate_diskstats_raw_data(&expected_per_sample_per_device_stats, 1);
        let raw_data: Vec<Data> = raw_samples
            .into_iter()
            .map(|s| Data::DiskstatsRaw(s))
            .collect();

        let mut diskstats = Diskstats::new();
        let result = diskstats
            .process_raw_data(ReportParams::new(), raw_data)
            .unwrap();

        if let AperfData::TimeSeries(time_series_data) = result {
            // Check reads metric
            let reads_metric = time_series_data.metrics.get("reads").unwrap();
            assert_eq!(reads_metric.series.len(), 1);
            let sda_series = &reads_metric.series[0];
            assert_eq!(sda_series.values, vec![0.0, 100.0, 100.0]); // Delta values

            // Check sectors_read metric (should be in KB)
            let sectors_read_metric = time_series_data.metrics.get("sectors_read").unwrap();
            let sda_sectors_series = &sectors_read_metric.series[0];
            assert_eq!(sda_sectors_series.values, vec![0.0, 800.0, 800.0]); // Delta values

            // Check in_progress metric (not accumulated)
            let in_progress_metric = time_series_data.metrics.get("in_progress").unwrap();
            let sda_in_progress_series = &in_progress_metric.series[0];
            assert_eq!(sda_in_progress_series.values, vec![0.0, 1.0, 0.0]); // Actual values
        } else {
            panic!("Expected TimeSeries data");
        }
    }
}
