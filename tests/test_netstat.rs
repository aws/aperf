use aperf::data::netstat::NetstatRaw;
use aperf::data::{Data, TimeEnum};
use aperf::visualizer::GetData;
use aperf::visualizer::ReportParams;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
struct ExpectedNetstatStats {
    pub stats: HashMap<String, u64>,
}

impl ExpectedNetstatStats {
    fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    fn set_stat(&mut self, name: &str, value: u64) {
        self.stats.insert(name.to_string(), value);
    }
}

/// Generate raw /proc/net/netstat data
fn generate_netstat_raw_data(
    expected_per_sample_stats: &Vec<ExpectedNetstatStats>,
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();

    for (sample_idx, expected_stats) in expected_per_sample_stats.iter().enumerate() {
        // Generate /proc/net/netstat format data
        let mut netstat_data = String::new();

        // Group stats by prefix (TcpExt:, IpExt:, MPTcpExt:)
        let mut prefixes: HashMap<String, Vec<(String, u64)>> = HashMap::new();
        for (stat_name, value) in &expected_stats.stats {
            // Parse the stat name to extract prefix and suffix
            // Expected format: "TcpExt:TCPPureAcks" (no space after colon)
            if let Some(colon_pos) = stat_name.find(':') {
                let prefix = stat_name[..colon_pos + 1].to_string(); // "TcpExt:"
                let suffix = stat_name[colon_pos + 1..].to_string(); // "TCPPureAcks"
                prefixes
                    .entry(prefix)
                    .or_insert_with(Vec::new)
                    .push((suffix, *value));
            }
        }

        // Sort prefixes for consistent output
        let mut sorted_prefixes: Vec<_> = prefixes.keys().collect();
        sorted_prefixes.sort();

        for prefix in sorted_prefixes {
            let stats = prefixes.get(prefix).unwrap();
            let mut sorted_stats = stats.clone();
            sorted_stats.sort_by(|a, b| a.0.cmp(&b.0));

            // Names line
            netstat_data.push_str(prefix);
            for (stat_name, _) in &sorted_stats {
                netstat_data.push(' ');
                netstat_data.push_str(stat_name);
            }
            netstat_data.push('\n');

            // Values line
            netstat_data.push_str(prefix);
            for (_, value) in &sorted_stats {
                netstat_data.push(' ');
                netstat_data.push_str(&value.to_string());
            }
            netstat_data.push('\n');
        }

        let time = TimeEnum::DateTime(
            Utc::now() + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64)),
        );

        let netstat_raw = NetstatRaw {
            time,
            data: netstat_data,
        };
        raw_data.push(Data::NetstatRaw(netstat_raw));
    }

    raw_data
}

#[test]
fn test_process_netstat_raw_data_complex() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate 100 samples with various netstat patterns
    for sample_idx in 0..100 {
        let mut expected_stats = ExpectedNetstatStats::new();

        // TcpExt stats with different patterns (absolute accumulated values)
        expected_stats.set_stat("TcpExt:TCPPureAcks", 100000 + sample_idx * 50);
        expected_stats.set_stat("TcpExt:TCPHPAcks", 200000 + sample_idx * 30);
        expected_stats.set_stat("TcpExt:TCPTimeouts", 10000 + sample_idx * 2);
        expected_stats.set_stat("TcpExt:TCPFastRetrans", 5000 + sample_idx);

        // IpExt stats
        expected_stats.set_stat("IpExt:InOctets", 1000000 + sample_idx * 50000);
        expected_stats.set_stat("IpExt:OutOctets", 800000 + sample_idx * 40000);
        expected_stats.set_stat("IpExt:InMcastPkts", sample_idx * 10);

        // MPTcpExt stats
        expected_stats.set_stat("MPTcpExt:MPCapableSYNRX", sample_idx / 10);
        expected_stats.set_stat("MPTcpExt:MPJoinSynRx", sample_idx / 20);

        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_netstat_raw_data(&expected_per_sample_stats, 2);
    let mut netstat = aperf::data::netstat::Netstat::new();
    let result = netstat
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Check each metric exists and has correct structure
        let expected_metrics = vec![
            "IpExt:InMcastPkts",
            "IpExt:InOctets",
            "IpExt:OutOctets",
            "MPTcpExt:MPCapableSYNRX",
            "MPTcpExt:MPJoinSynRx",
            "TcpExt:TCPFastRetrans",
            "TcpExt:TCPHPAcks",
            "TcpExt:TCPPureAcks",
            "TcpExt:TCPTimeouts",
        ];

        // Verify we have the expected number of metrics
        assert_eq!(time_series_data.metrics.len(), expected_metrics.len());

        // Verify sorted metric names
        assert_eq!(
            time_series_data.sorted_metric_names.len(),
            expected_metrics.len()
        );
        assert_eq!(time_series_data.sorted_metric_names, expected_metrics);

        for metric_name in expected_metrics {
            assert!(
                time_series_data.metrics.contains_key(metric_name),
                "Missing metric: {}",
                metric_name
            );

            let metric = &time_series_data.metrics[metric_name];
            assert_eq!(
                metric.series.len(),
                1,
                "Should have 1 series for {}",
                metric_name
            );
            assert_eq!(
                metric.series[0].values.len(),
                100,
                "Should have 100 data points for {}",
                metric_name
            );
            assert_eq!(
                metric.series[0].time_diff.len(),
                100,
                "Should have 100 time points for {}",
                metric_name
            );

            // First value should be 0 (no previous reference)
            assert_eq!(
                metric.series[0].values[0], 0.0,
                "First value should be 0 for {}",
                metric_name
            );

            // Check ALL values for this metric using expected data
            for sample_idx in 1..100 {
                let current_expected = &expected_per_sample_stats[sample_idx];
                let previous_expected = &expected_per_sample_stats[sample_idx - 1];

                let current_value = current_expected.stats.get(metric_name).unwrap_or(&0);
                let previous_value = previous_expected.stats.get(metric_name).unwrap_or(&0);
                let expected_delta = (*current_value as f64) - (*previous_value as f64);

                assert_eq!(
                    metric.series[0].values[sample_idx], expected_delta,
                    "Metric {} sample {}: expected {}, got {}",
                    metric_name, sample_idx, expected_delta, metric.series[0].values[sample_idx]
                );
            }

            // Verify all values are non-negative
            for value in &metric.series[0].values {
                assert!(
                    *value >= 0.0,
                    "Value should be non-negative for {}",
                    metric_name
                );
            }
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_netstat_raw_data_simple() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate 3 samples with simple patterns (absolute accumulated values)
    for sample_idx in 0..3 {
        let mut expected_stats = ExpectedNetstatStats::new();
        expected_stats.set_stat("TcpExt:TCPPureAcks", 1000 + sample_idx * 100);
        expected_stats.set_stat("IpExt:InOctets", 50000 + sample_idx * 10000);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_netstat_raw_data(&expected_per_sample_stats, 1);
    let mut netstat = aperf::data::netstat::Netstat::new();
    let result = netstat
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 2);
        assert_eq!(time_series_data.sorted_metric_names.len(), 2);
        assert_eq!(
            time_series_data.sorted_metric_names,
            vec!["IpExt:InOctets", "TcpExt:TCPPureAcks"]
        );

        // Check TcpExt:TCPPureAcks
        let tcp_metric = &time_series_data.metrics["TcpExt:TCPPureAcks"];
        assert_eq!(tcp_metric.series[0].values.len(), 3);
        assert_eq!(tcp_metric.series[0].values[0], 0.0); // First sample
        assert_eq!(tcp_metric.series[0].values[1], 100.0); // Delta: 1100 - 1000
        assert_eq!(tcp_metric.series[0].values[2], 100.0); // Delta: 1200 - 1100

        // Check IpExt:InOctets
        let ip_metric = &time_series_data.metrics["IpExt:InOctets"];
        assert_eq!(ip_metric.series[0].values.len(), 3);
        assert_eq!(ip_metric.series[0].values[0], 0.0); // First sample
        assert_eq!(ip_metric.series[0].values[1], 10000.0); // Delta: 60000 - 50000
        assert_eq!(ip_metric.series[0].values[2], 10000.0); // Delta: 70000 - 60000

        // Check time progression
        for metric in time_series_data.metrics.values() {
            assert_eq!(metric.series[0].time_diff[0], 0);
            assert_eq!(metric.series[0].time_diff[1], 1);
            assert_eq!(metric.series[0].time_diff[2], 2);
        }
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_netstat_dynamic_stats() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate 50 samples with stats appearing at different times
    for sample_idx in 0..50 {
        let mut expected_stats = ExpectedNetstatStats::new();

        // TcpExt:TCPPureAcks appears from the beginning
        expected_stats.set_stat("TcpExt:TCPPureAcks", 100 + sample_idx * 10);

        // IpExt:InOctets appears after sample 10
        if sample_idx >= 10 {
            expected_stats.set_stat("IpExt:InOctets", 50000 + (sample_idx - 10) * 5000);
        }

        // MPTcpExt:MPCapableSYNRX appears after sample 30
        if sample_idx >= 30 {
            expected_stats.set_stat("MPTcpExt:MPCapableSYNRX", 1000 + (sample_idx - 30) * 2);
        }

        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_netstat_raw_data(&expected_per_sample_stats, 1);
    let mut netstat = aperf::data::netstat::Netstat::new();
    let result = netstat
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 3);
        assert_eq!(time_series_data.sorted_metric_names.len(), 3);
        assert_eq!(
            time_series_data.sorted_metric_names,
            vec![
                "IpExt:InOctets",
                "MPTcpExt:MPCapableSYNRX",
                "TcpExt:TCPPureAcks"
            ]
        );

        // TcpExt:TCPPureAcks should have 50 data points
        let tcp_metric = &time_series_data.metrics["TcpExt:TCPPureAcks"];
        assert_eq!(tcp_metric.series[0].values.len(), 50);

        // IpExt:InOctets should have 40 data points (appears at sample 10)
        let ip_metric = &time_series_data.metrics["IpExt:InOctets"];
        assert_eq!(ip_metric.series[0].values.len(), 40);
        assert_eq!(ip_metric.series[0].values[0], 0.0); // First appearance
        assert_eq!(ip_metric.series[0].values[1], 5000.0); // Delta

        // MPTcpExt:MPCapableSYNRX should have 20 data points (appears at sample 30)
        let mptcp_metric = &time_series_data.metrics["MPTcpExt:MPCapableSYNRX"];
        assert_eq!(mptcp_metric.series[0].values.len(), 20);
        assert_eq!(mptcp_metric.series[0].values[0], 0.0); // First appearance
        assert_eq!(mptcp_metric.series[0].values[1], 2.0); // Delta
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_netstat_single_prefix() {
    let mut expected_per_sample_stats = Vec::new();

    // Generate 3 samples with only TcpExt stats
    for sample_idx in 0..3 {
        let mut expected_stats = ExpectedNetstatStats::new();
        expected_stats.set_stat("TcpExt:TCPPureAcks", 500 + sample_idx * 50);
        expected_stats.set_stat("TcpExt:TCPHPAcks", 300 + sample_idx * 30);
        expected_stats.set_stat("TcpExt:TCPTimeouts", 10 + sample_idx * 5);
        expected_per_sample_stats.push(expected_stats);
    }

    let raw_data = generate_netstat_raw_data(&expected_per_sample_stats, 2);
    let mut netstat = aperf::data::netstat::Netstat::new();
    let result = netstat
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 3);
        assert_eq!(time_series_data.sorted_metric_names.len(), 3);
        assert_eq!(
            time_series_data.sorted_metric_names,
            vec![
                "TcpExt:TCPHPAcks",
                "TcpExt:TCPPureAcks",
                "TcpExt:TCPTimeouts"
            ]
        );

        // All metrics should be TcpExt
        for metric_name in time_series_data.metrics.keys() {
            assert!(metric_name.starts_with("TcpExt:"));
        }

        // Check specific values
        let pure_acks = &time_series_data.metrics["TcpExt:TCPPureAcks"];
        assert_eq!(pure_acks.series[0].values[0], 0.0);
        assert_eq!(pure_acks.series[0].values[1], 50.0);
        assert_eq!(pure_acks.series[0].values[2], 50.0);

        let timeouts = &time_series_data.metrics["TcpExt:TCPTimeouts"];
        assert_eq!(timeouts.series[0].values[0], 0.0);
        assert_eq!(timeouts.series[0].values[1], 5.0);
        assert_eq!(timeouts.series[0].values[2], 5.0);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_netstat_empty_data() {
    let raw_data = Vec::new();
    let mut netstat = aperf::data::netstat::Netstat::new();
    let result = netstat
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        assert_eq!(time_series_data.metrics.len(), 0);
        assert_eq!(time_series_data.sorted_metric_names.len(), 0);
    } else {
        panic!("Expected TimeSeries data");
    }
}

#[test]
fn test_process_netstat_input_validation() {
    // Test case: Mix of invalid and valid samples - should still create metrics from valid ones
    let valid_data1 = NetstatRaw {
        time: TimeEnum::DateTime(Utc::now()),
        data: "TcpExt: TCPPureAcks TCPHPAcks\nTcpExt: 100 200\n".to_string(),
    };

    // Invalid sample (malformed names line)
    let invalid_data = NetstatRaw {
        time: TimeEnum::DateTime(Utc::now() + chrono::Duration::seconds(1)),
        data: "\nTcpExt: 150 250\n".to_string(),
    };

    // Another invalid sample (non-numeric value)
    let invalid_data2 = NetstatRaw {
        time: TimeEnum::DateTime(Utc::now() + chrono::Duration::seconds(2)),
        data: "TcpExt: TCPPureAcks TCPHPAcks\nTcpExt: invalid_number 300\n".to_string(),
    };

    // Valid sample again
    let valid_data2 = NetstatRaw {
        time: TimeEnum::DateTime(Utc::now() + chrono::Duration::seconds(3)),
        data: "TcpExt: TCPPureAcks TCPHPAcks\nTcpExt: 200 350\n".to_string(),
    };

    let raw_data = vec![
        Data::NetstatRaw(valid_data1),
        Data::NetstatRaw(invalid_data),
        Data::NetstatRaw(invalid_data2),
        Data::NetstatRaw(valid_data2),
    ];

    let mut netstat = aperf::data::netstat::Netstat::new();
    let result = netstat
        .process_raw_data_new(ReportParams::new(), raw_data)
        .unwrap();

    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = result {
        // Should have 2 metrics from the valid samples
        assert_eq!(time_series_data.metrics.len(), 2);
        assert!(time_series_data.metrics.contains_key("TcpExt:TCPPureAcks"));
        assert!(time_series_data.metrics.contains_key("TcpExt:TCPHPAcks"));

        // Check that we only have 2 data points (from 2 valid samples, invalid ones skipped)
        let tcp_pure_acks = &time_series_data.metrics["TcpExt:TCPPureAcks"];
        assert_eq!(tcp_pure_acks.series[0].values.len(), 2);
        assert_eq!(tcp_pure_acks.series[0].values[0], 0.0); // First valid sample
        assert_eq!(tcp_pure_acks.series[0].values[1], 100.0); // Delta: 200 - 100 (invalid samples skipped)

        let tcp_hp_acks = &time_series_data.metrics["TcpExt:TCPHPAcks"];
        assert_eq!(tcp_hp_acks.series[0].values.len(), 2);
        assert_eq!(tcp_hp_acks.series[0].values[0], 0.0); // First valid sample
        assert_eq!(tcp_hp_acks.series[0].values[1], 150.0); // Delta: 350 - 200
    }
}
