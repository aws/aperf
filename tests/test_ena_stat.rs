use aperf::data::data_formats::AperfData;
use aperf::data::ena_stat::{EnaStat, EnaStatRaw};
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::Utc;

/// Build raw data in the common time-series format used by ENA/EFA collectors:
///
/// ```text
/// component_name:
/// metric_name metric_value
/// ...
/// ```
fn build_common_raw_data(components: &[(&str, Vec<(&str, u64)>)]) -> String {
    let mut raw = String::new();
    for (component, metrics) in components {
        raw.push_str(component);
        raw.push_str(":\n");
        for (name, value) in metrics {
            raw.push_str(&format!("{} {}\n", name, value));
        }
    }
    raw
}

fn make_ena_raw_data(
    samples: &[Vec<(&str, Vec<(&str, u64)>)>],
    interval_seconds: i64,
) -> Vec<Data> {
    let base = Utc::now();
    samples
        .iter()
        .enumerate()
        .map(|(i, components)| {
            Data::EnaStatRaw(EnaStatRaw {
                #[cfg(target_os = "linux")]
                ethtool: None,
                time: TimeEnum::DateTime(
                    base + chrono::Duration::seconds(i as i64 * interval_seconds),
                ),
                data: build_common_raw_data(components),
            })
        })
        .collect()
}

fn unwrap_time_series(result: AperfData) -> aperf::data::data_formats::TimeSeriesData {
    match result {
        AperfData::TimeSeries(ts) => ts,
        _ => panic!("Expected TimeSeries data"),
    }
}

/// Helper: filter out the "aggregate" series and return the rest sorted by name.
fn data_series(
    metric: &aperf::data::data_formats::TimeSeriesMetric,
) -> Vec<&aperf::data::data_formats::Series> {
    let mut v: Vec<_> = metric
        .series
        .iter()
        .filter(|s| s.series_name.as_deref() != Some("aggregate"))
        .collect();
    v.sort_by_key(|s| s.series_name.clone());
    v
}

// ---------------------------------------------------------------------------
// Basic tests
// ---------------------------------------------------------------------------

#[test]
fn test_ena_empty_data() {
    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), vec![]).unwrap());
    assert!(ts.metrics.is_empty());
    assert!(ts.sorted_metric_names.is_empty());
}

#[test]
fn test_ena_single_sample_single_interface() {
    let raw = make_ena_raw_data(
        &[vec![(
            "eth0",
            vec![("bw_in_allowance_exceeded", 100u64), ("rx_bytes", 5000)],
        )]],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    assert!(ts.metrics.contains_key("bw_in_allowance_exceeded"));
    assert!(ts.metrics.contains_key("rx_bytes"));

    // Single sample → all accumulative values are 0
    for metric in ts.metrics.values() {
        for s in data_series(metric) {
            assert_eq!(s.values, vec![0.0]);
        }
    }
}

// ---------------------------------------------------------------------------
// Multi-interface tests
// ---------------------------------------------------------------------------

#[test]
fn test_ena_two_interfaces_accumulative_deltas() {
    let raw = make_ena_raw_data(
        &[
            vec![
                ("eth0", vec![("rx_bytes", 1000u64), ("tx_bytes", 500)]),
                ("eth1", vec![("rx_bytes", 2000u64), ("tx_bytes", 800)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 1300u64), ("tx_bytes", 700)]),
                ("eth1", vec![("rx_bytes", 2500u64), ("tx_bytes", 1100)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 1800u64), ("tx_bytes", 1000)]),
                ("eth1", vec![("rx_bytes", 3200u64), ("tx_bytes", 1500)]),
            ],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    // rx_bytes should have two data series: eth0 and eth1
    let rx = data_series(&ts.metrics["rx_bytes"]);
    assert_eq!(rx.len(), 2, "Expected 2 series for rx_bytes");

    let eth0 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth0"))
        .unwrap();
    let eth1 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth1"))
        .unwrap();

    assert_eq!(eth0.values, vec![0.0, 300.0, 500.0]); // 1300-1000, 1800-1300
    assert_eq!(eth1.values, vec![0.0, 500.0, 700.0]); // 2500-2000, 3200-2500

    // tx_bytes likewise
    let tx = data_series(&ts.metrics["tx_bytes"]);
    assert_eq!(tx.len(), 2);
    let eth0_tx = tx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth0"))
        .unwrap();
    let eth1_tx = tx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth1"))
        .unwrap();
    assert_eq!(eth0_tx.values, vec![0.0, 200.0, 300.0]);
    assert_eq!(eth1_tx.values, vec![0.0, 300.0, 400.0]);
}

#[test]
fn test_ena_three_interfaces_aggregate() {
    // With average aggregate mode and >1 real series, an aggregate series should appear
    let raw = make_ena_raw_data(
        &[
            vec![
                ("eth0", vec![("rx_bytes", 100u64)]),
                ("eth1", vec![("rx_bytes", 200u64)]),
                ("eth2", vec![("rx_bytes", 300u64)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 200u64)]),
                ("eth1", vec![("rx_bytes", 400u64)]),
                ("eth2", vec![("rx_bytes", 600u64)]),
            ],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    let rx = &ts.metrics["rx_bytes"];
    let non_agg = data_series(rx);
    assert_eq!(non_agg.len(), 3);

    // Deltas: eth0=100, eth1=200, eth2=300 → average = 200
    let agg = rx
        .series
        .iter()
        .find(|s| s.series_name.as_deref() == Some("aggregate"))
        .expect("Expected aggregate series with 3 interfaces");
    assert_eq!(agg.values[0], 0.0); // first sample average of all zeros
    assert_eq!(agg.values[1], 200.0); // (100+200+300)/3
}

#[test]
fn test_ena_multi_interface_different_metrics() {
    // Interfaces reporting different sets of metrics
    let raw = make_ena_raw_data(
        &[
            vec![
                ("eth0", vec![("rx_bytes", 100u64), ("tx_bytes", 50)]),
                (
                    "eth1",
                    vec![("rx_bytes", 200u64), ("bw_in_allowance_exceeded", 10)],
                ),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 300u64), ("tx_bytes", 150)]),
                (
                    "eth1",
                    vec![("rx_bytes", 500u64), ("bw_in_allowance_exceeded", 30)],
                ),
            ],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    // rx_bytes has both interfaces
    let rx = data_series(&ts.metrics["rx_bytes"]);
    assert_eq!(rx.len(), 2);

    // tx_bytes only from eth0
    let tx = data_series(&ts.metrics["tx_bytes"]);
    assert_eq!(tx.len(), 1);
    assert_eq!(tx[0].series_name.as_deref(), Some("eth0"));
    assert_eq!(tx[0].values, vec![0.0, 100.0]);

    // bw_in_allowance_exceeded only from eth1
    let bw = data_series(&ts.metrics["bw_in_allowance_exceeded"]);
    assert_eq!(bw.len(), 1);
    assert_eq!(bw[0].series_name.as_deref(), Some("eth1"));
    assert_eq!(bw[0].values, vec![0.0, 20.0]);
}

#[test]
fn test_ena_interface_appearing_later() {
    // eth1 only appears starting from sample 2
    let raw = make_ena_raw_data(
        &[
            vec![("eth0", vec![("rx_bytes", 100u64)])],
            vec![("eth0", vec![("rx_bytes", 200u64)])],
            vec![
                ("eth0", vec![("rx_bytes", 350u64)]),
                ("eth1", vec![("rx_bytes", 500u64)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 500u64)]),
                ("eth1", vec![("rx_bytes", 800u64)]),
            ],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    let rx = data_series(&ts.metrics["rx_bytes"]);
    let eth0 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth0"))
        .unwrap();
    let eth1 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth1"))
        .unwrap();

    assert_eq!(eth0.values.len(), 4);
    assert_eq!(eth0.values, vec![0.0, 100.0, 150.0, 150.0]);

    // eth1 appears at sample 2, so only 2 data points
    assert_eq!(eth1.values.len(), 2);
    assert_eq!(eth1.values[0], 0.0);
    assert_eq!(eth1.values[1], 300.0); // 800 - 500
}

// ---------------------------------------------------------------------------
// Decreasing counter
// ---------------------------------------------------------------------------

#[test]
fn test_ena_decreasing_counter() {
    let raw = make_ena_raw_data(
        &[
            vec![
                ("eth0", vec![("rx_bytes", 1000u64)]),
                ("eth1", vec![("rx_bytes", 2000u64)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 500u64)]),
                ("eth1", vec![("rx_bytes", 2500u64)]),
            ],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    let rx = data_series(&ts.metrics["rx_bytes"]);
    let eth0 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth0"))
        .unwrap();
    let eth1 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth1"))
        .unwrap();

    // Decreasing counter is skipped — eth0 only has the first sample (0.0)
    assert_eq!(eth0.values, vec![0.0]);
    assert_eq!(eth1.values, vec![0.0, 500.0]); // normal
}

// ---------------------------------------------------------------------------
// Queue metric transformation (ENA-specific)
// ---------------------------------------------------------------------------

#[test]
fn test_ena_queue_metric_transformation_multi_interface() {
    // queue_N_metric across two interfaces should merge into metric with per-interface-per-queue series
    let raw = make_ena_raw_data(
        &[
            vec![
                (
                    "eth0",
                    vec![("queue_0_tx_bytes", 100u64), ("queue_1_tx_bytes", 200)],
                ),
                (
                    "eth1",
                    vec![("queue_0_tx_bytes", 300u64), ("queue_1_tx_bytes", 400)],
                ),
            ],
            vec![
                (
                    "eth0",
                    vec![("queue_0_tx_bytes", 250u64), ("queue_1_tx_bytes", 350)],
                ),
                (
                    "eth1",
                    vec![("queue_0_tx_bytes", 500u64), ("queue_1_tx_bytes", 700)],
                ),
            ],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    // All queue_N_tx_bytes should be merged into "tx_bytes"
    assert!(ts.metrics.contains_key("tx_bytes"));
    assert!(!ts.metrics.contains_key("queue_0_tx_bytes"));
    assert!(!ts.metrics.contains_key("queue_1_tx_bytes"));

    let tx = &ts.metrics["tx_bytes"];
    let non_agg = data_series(tx);
    // 2 interfaces × 2 queues = 4 series
    assert_eq!(
        non_agg.len(),
        4,
        "Expected 4 series (2 interfaces × 2 queues), got {}",
        non_agg.len()
    );

    // Each series name should contain "queue_0" or "queue_1"
    for s in &non_agg {
        let name = s.series_name.as_deref().unwrap();
        assert!(
            name.contains("queue_0") || name.contains("queue_1"),
            "Unexpected series name: {}",
            name
        );
    }
}

#[test]
fn test_ena_queue_series_naming() {
    // Verify the exact series naming: "{interface}_queue_{N}"
    let raw = make_ena_raw_data(
        &[
            vec![(
                "eth0",
                vec![("queue_0_tx_cnt", 10u64), ("queue_3_tx_cnt", 20)],
            )],
            vec![(
                "eth0",
                vec![("queue_0_tx_cnt", 20u64), ("queue_3_tx_cnt", 50)],
            )],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    assert!(ts.metrics.contains_key("tx_cnt"));
    let series_names: Vec<_> = data_series(&ts.metrics["tx_cnt"])
        .iter()
        .map(|s| s.series_name.clone().unwrap())
        .collect();
    // series_name = format!("{}_{}_{}",  interface, "queue", N)
    assert!(
        series_names.contains(&"eth0_queue_0".to_string()),
        "got {:?}",
        series_names
    );
    assert!(
        series_names.contains(&"eth0_queue_3".to_string()),
        "got {:?}",
        series_names
    );
}

#[test]
fn test_ena_non_queue_metric_not_transformed() {
    // "queue_info" has parts[1]="info" which is NOT numeric → no transform
    let raw = make_ena_raw_data(
        &[
            vec![(
                "eth0",
                vec![("bw_in_allowance_exceeded", 10u64), ("queue_info", 5)],
            )],
            vec![(
                "eth0",
                vec![("bw_in_allowance_exceeded", 20u64), ("queue_info", 15)],
            )],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    assert!(ts.metrics.contains_key("bw_in_allowance_exceeded"));
    assert!(ts.metrics.contains_key("queue_info"));
}

// ---------------------------------------------------------------------------
// Metric ordering
// ---------------------------------------------------------------------------

#[test]
fn test_ena_metric_ordering() {
    let raw = make_ena_raw_data(
        &[vec![
            (
                "eth0",
                vec![
                    ("bw_in_allowance_exceeded", 1u64),
                    ("pps_allowance_exceeded", 2),
                    ("some_custom_metric", 3),
                    ("conntrack_allowance_exceeded", 4),
                ],
            ),
            (
                "eth1",
                vec![("bw_in_allowance_exceeded", 5u64), ("another_custom", 6)],
            ),
        ]],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    let official = [
        "bw_in_allowance_exceeded",
        "conntrack_allowance_exceeded",
        "pps_allowance_exceeded",
    ];
    let customs = ["another_custom", "some_custom_metric"];
    for off in &official {
        for cust in &customs {
            let off_pos = ts.sorted_metric_names.iter().position(|n| n == off);
            let cust_pos = ts.sorted_metric_names.iter().position(|n| n == cust);
            if let (Some(op), Some(cp)) = (off_pos, cust_pos) {
                assert!(op < cp, "{} should appear before {}", off, cust);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Time progression & stress
// ---------------------------------------------------------------------------

#[test]
fn test_ena_time_progression_multi_interface() {
    let raw = make_ena_raw_data(
        &[
            vec![
                ("eth0", vec![("rx_bytes", 100u64)]),
                ("eth1", vec![("rx_bytes", 200u64)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 200u64)]),
                ("eth1", vec![("rx_bytes", 400u64)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 400u64)]),
                ("eth1", vec![("rx_bytes", 700u64)]),
            ],
        ],
        5,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    for s in &ts.metrics["rx_bytes"].series {
        assert_eq!(s.time_diff, vec![0, 5, 10]);
    }
}

#[test]
fn test_ena_many_samples_multi_interface() {
    let samples: Vec<Vec<(&str, Vec<(&str, u64)>)>> = (0..100u64)
        .map(|i| {
            vec![
                ("eth0", vec![("rx_bytes", i * 1000), ("tx_bytes", i * 500)]),
                ("eth1", vec![("rx_bytes", i * 2000), ("tx_bytes", i * 800)]),
            ]
        })
        .collect();

    let raw = make_ena_raw_data(&samples, 2);
    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    let rx = data_series(&ts.metrics["rx_bytes"]);
    assert_eq!(rx.len(), 2);

    let eth0 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth0"))
        .unwrap();
    let eth1 = rx
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth1"))
        .unwrap();

    assert_eq!(eth0.values.len(), 100);
    assert_eq!(eth1.values.len(), 100);
    for i in 1..100 {
        assert_eq!(eth0.values[i], 1000.0, "eth0 rx_bytes sample {}", i);
        assert_eq!(eth1.values[i], 2000.0, "eth1 rx_bytes sample {}", i);
    }
}

#[test]
fn test_ena_metric_appearing_later_multi_interface() {
    let raw = make_ena_raw_data(
        &[
            vec![
                ("eth0", vec![("rx_bytes", 100u64)]),
                ("eth1", vec![("rx_bytes", 200u64)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 200u64)]),
                ("eth1", vec![("rx_bytes", 400u64)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 300u64), ("new_metric", 50)]),
                ("eth1", vec![("rx_bytes", 600u64), ("new_metric", 80)]),
            ],
            vec![
                ("eth0", vec![("rx_bytes", 400u64), ("new_metric", 90)]),
                ("eth1", vec![("rx_bytes", 900u64), ("new_metric", 130)]),
            ],
        ],
        1,
    );

    let mut ena = EnaStat::new();
    let ts = unwrap_time_series(ena.process_raw_data(ReportParams::new(), raw).unwrap());

    let rx = data_series(&ts.metrics["rx_bytes"]);
    assert_eq!(rx[0].values.len(), 4);

    let nm = data_series(&ts.metrics["new_metric"]);
    assert_eq!(nm.len(), 2); // both interfaces
    let eth0_nm = nm
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth0"))
        .unwrap();
    let eth1_nm = nm
        .iter()
        .find(|s| s.series_name.as_deref() == Some("eth1"))
        .unwrap();
    assert_eq!(eth0_nm.values, vec![0.0, 40.0]); // 90-50
    assert_eq!(eth1_nm.values, vec![0.0, 50.0]); // 130-80
}
