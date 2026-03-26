use aperf::data::common::data_formats::AperfData;
#[cfg(target_os = "linux")]
use aperf::data::efa_stat::collect_efa_metrics_file_paths;
use aperf::data::efa_stat::{EfaStat, EfaStatRaw};
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::Utc;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use tempfile::TempDir;

/// Build raw data in the common time-series format used by ENA/EFA collectors.
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

fn make_efa_raw_data(
    samples: &[Vec<(&str, Vec<(&str, u64)>)>],
    interval_seconds: i64,
) -> Vec<Data> {
    let base = Utc::now();
    samples
        .iter()
        .enumerate()
        .map(|(i, components)| {
            Data::EfaStatRaw(EfaStatRaw {
                efa_metric_file_paths: HashMap::new(),
                time: TimeEnum::DateTime(
                    base + chrono::Duration::seconds(i as i64 * interval_seconds),
                ),
                data: build_common_raw_data(components),
            })
        })
        .collect()
}

fn unwrap_time_series(result: AperfData) -> aperf::data::common::data_formats::TimeSeriesData {
    match result {
        AperfData::TimeSeries(ts) => ts,
        _ => panic!("Expected TimeSeries data"),
    }
}

/// Helper: filter out the aggregate series and return the rest sorted by name.
fn data_series(
    metric: &aperf::data::common::data_formats::TimeSeriesMetric,
) -> Vec<&aperf::data::common::data_formats::Series> {
    metric.series.iter().filter(|s| !s.is_aggregate).collect()
}

// ---------------------------------------------------------------------------
// Basic tests
// ---------------------------------------------------------------------------

#[test]
fn test_efa_empty_data() {
    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), vec![]).unwrap());
    assert!(ts.metrics.is_empty());
    assert!(ts.sorted_metric_names.is_empty());
}

#[test]
fn test_efa_single_sample_single_driver() {
    let raw = make_efa_raw_data(
        &[vec![(
            "efa0",
            vec![("tx_bytes", 5000u64), ("rx_bytes", 3000)],
        )]],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    assert!(ts.metrics.contains_key("tx_bytes"));
    assert!(ts.metrics.contains_key("rx_bytes"));

    for metric in ts.metrics.values() {
        for s in data_series(metric) {
            assert_eq!(s.values, vec![0.0]);
        }
    }
}

// ---------------------------------------------------------------------------
// Multi-driver tests
// ---------------------------------------------------------------------------

#[test]
fn test_efa_two_drivers_accumulative_deltas() {
    let raw = make_efa_raw_data(
        &[
            vec![
                ("efa0", vec![("tx_bytes", 1000u64), ("rx_bytes", 500)]),
                ("efa0/1", vec![("tx_bytes", 2000u64), ("rx_bytes", 800)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 1500u64), ("rx_bytes", 800)]),
                ("efa0/1", vec![("tx_bytes", 2700u64), ("rx_bytes", 1200)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 2200u64), ("rx_bytes", 1300)]),
                ("efa0/1", vec![("tx_bytes", 3500u64), ("rx_bytes", 1800)]),
            ],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let tx = data_series(&ts.metrics["tx_bytes"]);
    assert_eq!(tx.len(), 2, "Expected 2 series for tx_bytes");

    let efa0 = tx.iter().find(|s| s.series_name == "efa0").unwrap();
    let efa0_1 = tx.iter().find(|s| s.series_name == "efa0/1").unwrap();

    assert_eq!(efa0.values, vec![0.0, 500.0, 700.0]);
    assert_eq!(efa0_1.values, vec![0.0, 700.0, 800.0]);

    let rx = data_series(&ts.metrics["rx_bytes"]);
    let efa0_rx = rx.iter().find(|s| s.series_name == "efa0").unwrap();
    let efa0_1_rx = rx.iter().find(|s| s.series_name == "efa0/1").unwrap();
    assert_eq!(efa0_rx.values, vec![0.0, 300.0, 500.0]);
    assert_eq!(efa0_1_rx.values, vec![0.0, 400.0, 600.0]);
}

#[test]
fn test_efa_three_drivers_aggregate() {
    let raw = make_efa_raw_data(
        &[
            vec![
                ("efa0", vec![("tx_bytes", 100u64)]),
                ("efa1", vec![("tx_bytes", 200u64)]),
                ("efa2", vec![("tx_bytes", 300u64)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 200u64)]),
                ("efa1", vec![("tx_bytes", 500u64)]),
                ("efa2", vec![("tx_bytes", 600u64)]),
            ],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let tx = &ts.metrics["tx_bytes"];
    let non_agg = data_series(tx);
    assert_eq!(non_agg.len(), 3);

    let agg = tx
        .series
        .iter()
        .find(|s| s.is_aggregate)
        .expect("Expected aggregate series with 3 drivers");
    // Deltas: efa0=100, efa1=300, efa2=300 → average = 233.33...
    assert_eq!(agg.values[0], 0.0);
    let expected_avg = (100.0 + 300.0 + 300.0) / 3.0;
    assert!(
        (agg.values[1] - expected_avg).abs() < 0.01,
        "Expected ~{}, got {}",
        expected_avg,
        agg.values[1]
    );
}

#[test]
fn test_efa_multi_driver_different_metrics() {
    let raw = make_efa_raw_data(
        &[
            vec![
                ("efa0", vec![("tx_bytes", 100u64), ("send_wrs", 10)]),
                ("efa1", vec![("tx_bytes", 200u64), ("rdma_write_bytes", 50)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 300u64), ("send_wrs", 25)]),
                (
                    "efa1",
                    vec![("tx_bytes", 500u64), ("rdma_write_bytes", 120)],
                ),
            ],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    // tx_bytes from both drivers
    let tx = data_series(&ts.metrics["tx_bytes"]);
    assert_eq!(tx.len(), 2);

    // send_wrs only from efa0
    let sw = data_series(&ts.metrics["send_wrs"]);
    assert_eq!(sw.len(), 1);
    assert_eq!(sw[0].series_name, "efa0");
    assert_eq!(sw[0].values, vec![0.0, 15.0]);

    // rdma_write_bytes only from efa1
    let rw = data_series(&ts.metrics["rdma_write_bytes"]);
    assert_eq!(rw.len(), 1);
    assert_eq!(rw[0].series_name, "efa1");
    assert_eq!(rw[0].values, vec![0.0, 70.0]);

    // avg_bytes_per_send_wr: efa0 has send_wrs but no send_bytes → 0.0
    let avg_send = data_series(&ts.metrics["avg_bytes_per_send_wr"]);
    assert_eq!(avg_send.len(), 1);
    assert_eq!(avg_send[0].series_name, "efa0");
    assert_eq!(avg_send[0].values, vec![0.0, 0.0]);

    // avg_bytes_per_rdma_write_wr: efa1 has rdma_write_bytes but no rdma_write_wrs → 0.0
    let avg_rdma_w = data_series(&ts.metrics["avg_bytes_per_rdma_write_wr"]);
    assert_eq!(avg_rdma_w.len(), 1);
    assert_eq!(avg_rdma_w[0].series_name, "efa1");
    assert_eq!(avg_rdma_w[0].values, vec![0.0, 0.0]);
}

#[test]
fn test_efa_driver_appearing_later() {
    let raw = make_efa_raw_data(
        &[
            vec![("efa0", vec![("tx_bytes", 100u64)])],
            vec![("efa0", vec![("tx_bytes", 200u64)])],
            vec![
                ("efa0", vec![("tx_bytes", 350u64)]),
                ("efa1", vec![("tx_bytes", 500u64)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 500u64)]),
                ("efa1", vec![("tx_bytes", 900u64)]),
            ],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let tx = data_series(&ts.metrics["tx_bytes"]);
    let efa0 = tx.iter().find(|s| s.series_name == "efa0").unwrap();
    let efa1 = tx.iter().find(|s| s.series_name == "efa1").unwrap();

    assert_eq!(efa0.values.len(), 4);
    assert_eq!(efa0.values, vec![0.0, 100.0, 150.0, 150.0]);

    assert_eq!(efa1.values.len(), 2);
    assert_eq!(efa1.values, vec![0.0, 400.0]);
}

// ---------------------------------------------------------------------------
// Decreasing counter
// ---------------------------------------------------------------------------

#[test]
fn test_efa_decreasing_counter_multi_driver() {
    let raw = make_efa_raw_data(
        &[
            vec![
                ("efa0", vec![("tx_bytes", 1000u64)]),
                ("efa1", vec![("tx_bytes", 2000u64)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 400u64)]),  // reset
                ("efa1", vec![("tx_bytes", 2500u64)]), // normal
            ],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let tx = data_series(&ts.metrics["tx_bytes"]);
    let efa0 = tx.iter().find(|s| s.series_name == "efa0").unwrap();
    let efa1 = tx.iter().find(|s| s.series_name == "efa1").unwrap();

    // Decreasing counter is skipped — efa0 only has the first sample (0.0)
    assert_eq!(efa0.values, vec![0.0]);
    assert_eq!(efa1.values, vec![0.0, 500.0]);
}

// ---------------------------------------------------------------------------
// No queue transformation (EFA does NOT do what ENA does)
// ---------------------------------------------------------------------------

#[test]
fn test_efa_no_queue_transformation() {
    let raw = make_efa_raw_data(
        &[
            vec![("efa0", vec![("queue_0_tx_bytes", 100u64)])],
            vec![("efa0", vec![("queue_0_tx_bytes", 200u64)])],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    assert!(
        ts.metrics.contains_key("queue_0_tx_bytes"),
        "EFA should keep queue_0_tx_bytes as-is, got: {:?}",
        ts.sorted_metric_names
    );
}

// ---------------------------------------------------------------------------
// Metric ordering
// ---------------------------------------------------------------------------

#[test]
fn test_efa_metric_ordering_multi_driver() {
    let raw = make_efa_raw_data(
        &[vec![
            (
                "efa0",
                vec![("tx_bytes", 1u64), ("send_bytes", 3), ("custom_counter", 5)],
            ),
            (
                "efa1",
                vec![
                    ("rx_bytes", 2u64),
                    ("rdma_write_wrs", 4),
                    ("another_custom", 6),
                ],
            ),
        ]],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let official = ["tx_bytes", "rx_bytes", "send_bytes", "rdma_write_wrs"];
    let custom = ["another_custom", "custom_counter"];

    for off in &official {
        for cust in &custom {
            let off_pos = ts.sorted_metric_names.iter().position(|n| n == off);
            let cust_pos = ts.sorted_metric_names.iter().position(|n| n == cust);
            if let (Some(op), Some(cp)) = (off_pos, cust_pos) {
                assert!(op < cp, "{} should appear before {}", off, cust);
            }
        }
    }
}

#[test]
fn test_efa_all_official_metrics() {
    let official_metrics: Vec<(&str, u64)> = vec![
        ("tx_bytes", 100),
        ("rx_bytes", 200),
        ("tx_pkts", 50),
        ("rx_pkts", 60),
        ("rx_drops", 1),
        ("send_bytes", 300),
        ("recv_bytes", 400),
        ("send_wrs", 10),
        ("recv_wrs", 20),
        ("rdma_write_wrs", 5),
        ("rdma_read_wrs", 3),
        ("rdma_write_bytes", 500),
        ("rdma_read_bytes", 600),
        ("rdma_write_wr_err", 0),
        ("rdma_read_wr_err", 0),
        ("rdma_read_resp_bytes", 700),
        ("rdma_write_recv_bytes", 800),
        ("retrans_bytes", 10),
        ("retrans_pkts", 2),
        ("retrans_timeout_events", 1),
        ("impaired_remote_conn_events", 0),
        ("unresponsive_remote_events", 0),
    ];

    let raw = make_efa_raw_data(&[vec![("efa0", official_metrics.clone())]], 1);

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    for (name, _) in &official_metrics {
        assert!(
            ts.metrics.contains_key(*name),
            "Missing official metric: {}",
            name
        );
    }

    // The new metric ordering groups bytes/wrs with avg_bytes_per_*_wr interleaved.
    // Verify the official metrics (excluding computed avg_bytes_per_*_wr) appear in the
    // expected order defined by get_time_series_data_with_metric_name_order.
    let expected_order = vec![
        "tx_bytes",
        "rx_bytes",
        "tx_pkts",
        "rx_pkts",
        "rx_drops",
        "send_bytes",
        "send_wrs",
        "recv_bytes",
        "recv_wrs",
        "rdma_write_bytes",
        "rdma_write_wrs",
        "rdma_read_bytes",
        "rdma_read_wrs",
        "rdma_write_wr_err",
        "rdma_read_wr_err",
        "rdma_read_resp_bytes",
        "rdma_write_recv_bytes",
        "retrans_bytes",
        "retrans_pkts",
        "retrans_timeout_events",
        "impaired_remote_conn_events",
        "unresponsive_remote_events",
    ];
    let actual_official: Vec<&str> = ts
        .sorted_metric_names
        .iter()
        .filter(|n| expected_order.contains(&n.as_str()))
        .map(|n| n.as_str())
        .collect();
    assert_eq!(actual_official, expected_order);

    // Verify the 4 computed avg_bytes_per_*_wr metrics are also present
    let computed_metrics = [
        "avg_bytes_per_send_wr",
        "avg_bytes_per_recv_wr",
        "avg_bytes_per_rdma_write_wr",
        "avg_bytes_per_rdma_read_wr",
    ];
    for name in &computed_metrics {
        assert!(
            ts.metrics.contains_key(*name),
            "Missing computed metric: {}",
            name
        );
    }

    // Verify each avg_bytes_per_*_wr appears right after its corresponding *_wrs metric
    for (wrs_metric, avg_metric) in [
        ("send_wrs", "avg_bytes_per_send_wr"),
        ("recv_wrs", "avg_bytes_per_recv_wr"),
        ("rdma_write_wrs", "avg_bytes_per_rdma_write_wr"),
        ("rdma_read_wrs", "avg_bytes_per_rdma_read_wr"),
    ] {
        let wrs_pos = ts
            .sorted_metric_names
            .iter()
            .position(|n| n == wrs_metric)
            .unwrap();
        let avg_pos = ts
            .sorted_metric_names
            .iter()
            .position(|n| n == avg_metric)
            .unwrap();
        assert_eq!(
            avg_pos,
            wrs_pos + 1,
            "{} should appear right after {}",
            avg_metric,
            wrs_metric
        );
    }
}

// ---------------------------------------------------------------------------
// Time progression & stress
// ---------------------------------------------------------------------------

#[test]
fn test_efa_time_progression_multi_driver() {
    let raw = make_efa_raw_data(
        &[
            vec![
                ("efa0", vec![("tx_bytes", 100u64)]),
                ("efa1", vec![("tx_bytes", 200u64)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 200u64)]),
                ("efa1", vec![("tx_bytes", 400u64)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 400u64)]),
                ("efa1", vec![("tx_bytes", 700u64)]),
            ],
        ],
        3,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    for s in &ts.metrics["tx_bytes"].series {
        assert_eq!(s.time_diff, vec![0, 3, 6]);
    }
}

#[test]
fn test_efa_many_samples_multi_driver() {
    let samples: Vec<Vec<(&str, Vec<(&str, u64)>)>> = (0..100u64)
        .map(|i| {
            vec![
                ("efa0", vec![("tx_bytes", i * 1000), ("rx_bytes", i * 500)]),
                ("efa1", vec![("tx_bytes", i * 2000), ("rx_bytes", i * 800)]),
            ]
        })
        .collect();

    let raw = make_efa_raw_data(&samples, 2);
    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let tx = data_series(&ts.metrics["tx_bytes"]);
    assert_eq!(tx.len(), 2);

    let efa0 = tx.iter().find(|s| s.series_name == "efa0").unwrap();
    let efa1 = tx.iter().find(|s| s.series_name == "efa1").unwrap();

    assert_eq!(efa0.values.len(), 100);
    assert_eq!(efa1.values.len(), 100);
    for i in 1..100 {
        assert_eq!(efa0.values[i], 1000.0, "efa0 tx_bytes sample {}", i);
        assert_eq!(efa1.values[i], 2000.0, "efa1 tx_bytes sample {}", i);
    }
}

#[test]
fn test_efa_zero_counters_multi_driver() {
    let raw = make_efa_raw_data(
        &[
            vec![
                ("efa0", vec![("tx_bytes", 0u64)]),
                ("efa1", vec![("tx_bytes", 0u64)]),
            ],
            vec![
                ("efa0", vec![("tx_bytes", 0u64)]),
                ("efa1", vec![("tx_bytes", 0u64)]),
            ],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    for metric in ts.metrics.values() {
        for s in &metric.series {
            for val in &s.values {
                assert_eq!(*val, 0.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// avg_bytes_per_wr computation tests
// ---------------------------------------------------------------------------

#[test]
fn test_efa_avg_bytes_per_wr_correct_division() {
    // When both *_bytes and *_wrs are present, avg = bytes_delta / wrs_delta
    let raw = make_efa_raw_data(
        &[
            vec![("efa0", vec![("send_bytes", 0u64), ("send_wrs", 0)])],
            vec![("efa0", vec![("send_bytes", 3000u64), ("send_wrs", 10)])],
            vec![("efa0", vec![("send_bytes", 8000u64), ("send_wrs", 30)])],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let avg = data_series(&ts.metrics["avg_bytes_per_send_wr"]);
    assert_eq!(avg.len(), 1);
    // t0: first sample → bytes=0, wrs=0 → 0.0
    // t1: bytes_delta=3000, wrs_delta=10 → 300.0
    // t2: bytes_delta=5000, wrs_delta=20 → 250.0
    assert_eq!(avg[0].values, vec![0.0, 300.0, 250.0]);
}

#[test]
fn test_efa_avg_bytes_per_wr_zero_when_no_wrs() {
    // When wrs_delta is 0, avg should be 0.0 (not NaN or panic)
    let raw = make_efa_raw_data(
        &[
            vec![("efa0", vec![("send_bytes", 100u64), ("send_wrs", 5)])],
            vec![(
                "efa0",
                vec![("send_bytes", 500u64), ("send_wrs", 5)], // wrs unchanged → delta=0
            )],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let avg = data_series(&ts.metrics["avg_bytes_per_send_wr"]);
    // t0: first sample → 0.0; t1: wrs_delta=0 → 0.0
    assert_eq!(avg[0].values, vec![0.0, 0.0]);
}

#[test]
fn test_efa_avg_bytes_per_wr_per_device_independence() {
    // Each device computes avg_bytes_per_wr independently
    let raw = make_efa_raw_data(
        &[
            vec![
                ("efa0", vec![("send_bytes", 0u64), ("send_wrs", 0)]),
                ("efa1", vec![("send_bytes", 0u64), ("send_wrs", 0)]),
            ],
            vec![
                ("efa0", vec![("send_bytes", 1000u64), ("send_wrs", 10)]), // avg=100
                ("efa1", vec![("send_bytes", 5000u64), ("send_wrs", 10)]), // avg=500
            ],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let avg = data_series(&ts.metrics["avg_bytes_per_send_wr"]);
    assert_eq!(avg.len(), 2);

    let efa0 = avg.iter().find(|s| s.series_name == "efa0").unwrap();
    let efa1 = avg.iter().find(|s| s.series_name == "efa1").unwrap();

    assert_eq!(efa0.values, vec![0.0, 100.0]);
    assert_eq!(efa1.values, vec![0.0, 500.0]);
}

#[test]
fn test_efa_avg_bytes_per_wr_all_four_types() {
    // Verify all 4 wr types produce their avg_bytes_per_*_wr metric
    let raw = make_efa_raw_data(
        &[
            vec![(
                "efa0",
                vec![
                    ("send_bytes", 0u64),
                    ("send_wrs", 0),
                    ("recv_bytes", 0),
                    ("recv_wrs", 0),
                    ("rdma_write_bytes", 0),
                    ("rdma_write_wrs", 0),
                    ("rdma_read_bytes", 0),
                    ("rdma_read_wrs", 0),
                ],
            )],
            vec![(
                "efa0",
                vec![
                    ("send_bytes", 1000u64),
                    ("send_wrs", 4),
                    ("recv_bytes", 2000),
                    ("recv_wrs", 8),
                    ("rdma_write_bytes", 3000),
                    ("rdma_write_wrs", 6),
                    ("rdma_read_bytes", 4000),
                    ("rdma_read_wrs", 10),
                ],
            )],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    let check = |metric: &str, expected: f64| {
        let s = data_series(&ts.metrics[metric]);
        assert_eq!(s[0].values[1], expected, "{} mismatch", metric);
    };

    check("avg_bytes_per_send_wr", 250.0); // 1000/4
    check("avg_bytes_per_recv_wr", 250.0); // 2000/8
    check("avg_bytes_per_rdma_write_wr", 500.0); // 3000/6
    check("avg_bytes_per_rdma_read_wr", 400.0); // 4000/10
}

#[test]
fn test_efa_avg_bytes_per_wr_with_counter_decrease() {
    // When a counter decrease causes the data point to be skipped, the avg should use 0
    // for the missing component
    let raw = make_efa_raw_data(
        &[
            vec![("efa0", vec![("send_bytes", 1000u64), ("send_wrs", 10)])],
            vec![(
                "efa0",
                vec![("send_bytes", 500u64), ("send_wrs", 20)], // bytes decreased → skipped
            )],
        ],
        1,
    );

    let mut efa = EfaStat::new();
    let ts = unwrap_time_series(efa.process_raw_data(ReportParams::new(), raw).unwrap());

    // send_bytes was skipped due to decrease, so avg_bytes_per_send_wr should have
    // wrs_delta=10 but bytes=0 → avg=0.0
    let avg = data_series(&ts.metrics["avg_bytes_per_send_wr"]);
    assert_eq!(avg[0].values, vec![0.0, 0.0]);
}

// ============================================================================
// Tests for collect_efa_metrics_file_paths
// ============================================================================

/// Helper: create a file with content inside a directory, creating parent dirs as needed.
#[cfg(target_os = "linux")]
fn create_metric_file(base: &std::path::Path, relative: &str, content: &str) {
    let path = base.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
}

/// Helper: read the content of a File (seeking to start first) and return it as a String.
#[cfg(target_os = "linux")]
fn read_file_content(file: &mut std::fs::File) -> String {
    use std::io::Seek;
    file.seek(std::io::SeekFrom::Start(0)).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    content
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_nonexistent_root_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let nonexistent = tmp.path().join("does_not_exist");
    let result = collect_efa_metrics_file_paths(nonexistent.to_str().unwrap());
    assert!(result.is_empty());
    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_empty_root_dir_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let result = collect_efa_metrics_file_paths(tmp.path().to_str().unwrap());
    assert!(result.is_empty());
    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_single_device_hw_counters_only() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    create_metric_file(&root, "rdmap0s31/hw_counters/tx_bytes", "100");
    create_metric_file(&root, "rdmap0s31/hw_counters/rx_bytes", "200");

    let mut result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    let device_counters = result.get_mut("rdmap0s31").unwrap();
    assert_eq!(device_counters.len(), 2);
    assert!(device_counters.contains_key("tx_bytes"));
    assert!(device_counters.contains_key("rx_bytes"));
    // Verify the file descriptors point to the correct files by reading content
    assert_eq!(
        read_file_content(device_counters.get_mut("tx_bytes").unwrap()),
        "100"
    );
    assert_eq!(
        read_file_content(device_counters.get_mut("rx_bytes").unwrap()),
        "200"
    );

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_single_device_with_port() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    create_metric_file(&root, "rdmap0s31/hw_counters/lifespan", "42");
    create_metric_file(&root, "rdmap0s31/ports/1/hw_counters/tx_bytes", "100");
    create_metric_file(&root, "rdmap0s31/ports/1/hw_counters/rx_drops", "5");

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    let device_counters = result.get("rdmap0s31").unwrap();
    assert_eq!(device_counters.len(), 1);
    assert!(device_counters.contains_key("lifespan"));

    let port_counters = result.get("rdmap0s31/1").unwrap();
    assert_eq!(port_counters.len(), 2);
    assert!(port_counters.contains_key("tx_bytes"));
    assert!(port_counters.contains_key("rx_drops"));

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_multiple_ports() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    create_metric_file(&root, "rdmap0s31/hw_counters/lifespan", "10");
    create_metric_file(&root, "rdmap0s31/ports/1/hw_counters/tx_bytes", "100");
    create_metric_file(&root, "rdmap0s31/ports/2/hw_counters/tx_bytes", "200");

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    assert!(result.contains_key("rdmap0s31"));
    assert!(result.contains_key("rdmap0s31/1"));
    assert!(result.contains_key("rdmap0s31/2"));
    assert_eq!(result.len(), 3);

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_multiple_devices() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    create_metric_file(&root, "rdmap0s31/hw_counters/tx_bytes", "100");
    create_metric_file(&root, "rdmap0s31/ports/1/hw_counters/rx_bytes", "200");
    create_metric_file(&root, "rdmap16s31/hw_counters/tx_bytes", "300");
    create_metric_file(&root, "rdmap16s31/ports/1/hw_counters/rx_bytes", "400");

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    assert!(result.contains_key("rdmap0s31"));
    assert!(result.contains_key("rdmap0s31/1"));
    assert!(result.contains_key("rdmap16s31"));
    assert!(result.contains_key("rdmap16s31/1"));
    assert_eq!(result.len(), 4);

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_device_without_hw_counters_dir() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    // Create a device directory but no hw_counters subdirectory
    fs::create_dir_all(root.join("rdmap0s31")).unwrap();

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    assert!(!result.contains_key("rdmap0s31"));

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_device_with_empty_hw_counters() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    fs::create_dir_all(root.join("rdmap0s31/hw_counters")).unwrap();
    fs::create_dir_all(root.join("rdmap0s31/ports")).unwrap();

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    let device_counters = result.get("rdmap0s31").unwrap();
    assert!(device_counters.is_empty());

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_subdirectories_in_hw_counters_are_ignored() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    create_metric_file(&root, "rdmap0s31/hw_counters/tx_bytes", "100");
    // Subdirectory inside hw_counters should be ignored, only files collected
    fs::create_dir_all(root.join("rdmap0s31/hw_counters/subdir")).unwrap();

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    let device_counters = result.get("rdmap0s31").unwrap();
    assert_eq!(device_counters.len(), 1);
    assert!(device_counters.contains_key("tx_bytes"));
    assert!(!device_counters.contains_key("subdir"));

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_port_without_hw_counters() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    create_metric_file(&root, "rdmap0s31/hw_counters/lifespan", "10");
    // Port directory exists but has no hw_counters subdirectory
    fs::create_dir_all(root.join("rdmap0s31/ports/1")).unwrap();

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    assert!(result.contains_key("rdmap0s31"));
    assert!(!result.contains_key("rdmap0s31/1"));

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_many_metrics_per_device() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    let metrics = [
        "tx_bytes",
        "rx_bytes",
        "tx_pkts",
        "rx_pkts",
        "rx_drops",
        "send_bytes",
        "recv_bytes",
        "send_wrs",
        "recv_wrs",
        "rdma_write_wrs",
        "rdma_read_wrs",
        "lifespan",
    ];
    for (i, metric) in metrics.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap0s31/ports/1/hw_counters/{}", metric),
            &i.to_string(),
        );
    }

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());
    let port_counters = result.get("rdmap0s31/1").unwrap();
    assert_eq!(port_counters.len(), metrics.len());
    for metric in &metrics {
        assert!(port_counters.contains_key(*metric));
    }

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_device_without_ports_dir() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    create_metric_file(&root, "rdmap0s31/hw_counters/tx_bytes", "100");

    let result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    assert!(result.contains_key("rdmap0s31"));
    assert_eq!(result.get("rdmap0s31").unwrap().len(), 1);
    assert_eq!(result.len(), 1);

    tmp.close().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn test_collect_efa_complex_topology() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    // Device 1: rdmap0s31 — 2 device-level metrics, 2 ports with different metric sets
    let dev1_hw = ["lifespan", "cmds_err"];
    let dev1_port1 = [
        "tx_bytes",
        "rx_bytes",
        "tx_pkts",
        "rx_pkts",
        "rx_drops",
        "send_bytes",
        "recv_bytes",
        "rdma_write_wrs",
        "rdma_read_wrs",
    ];
    let dev1_port2 = [
        "tx_bytes",
        "rx_bytes",
        "rdma_write_bytes",
        "rdma_read_bytes",
        "rdma_write_wr_err",
        "rdma_read_wr_err",
    ];

    for (i, m) in dev1_hw.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap0s31/hw_counters/{}", m),
            &i.to_string(),
        );
    }
    for (i, m) in dev1_port1.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap0s31/ports/1/hw_counters/{}", m),
            &i.to_string(),
        );
    }
    for (i, m) in dev1_port2.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap0s31/ports/2/hw_counters/{}", m),
            &i.to_string(),
        );
    }

    // Device 2: rdmap16s31 — 3 device-level metrics, 3 ports
    let dev2_hw = ["lifespan", "submitted_cmds", "completed_cmds"];
    let dev2_port1 = ["tx_bytes", "rx_bytes", "tx_pkts", "rx_pkts"];
    let dev2_port2 = ["tx_bytes", "rx_bytes"];
    let dev2_port3 = [
        "tx_bytes",
        "rx_bytes",
        "retrans_bytes",
        "retrans_pkts",
        "retrans_timeout_events",
        "impaired_remote_conn_events",
        "unresponsive_remote_events",
    ];

    for (i, m) in dev2_hw.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap16s31/hw_counters/{}", m),
            &i.to_string(),
        );
    }
    for (i, m) in dev2_port1.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap16s31/ports/1/hw_counters/{}", m),
            &i.to_string(),
        );
    }
    for (i, m) in dev2_port2.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap16s31/ports/2/hw_counters/{}", m),
            &i.to_string(),
        );
    }
    for (i, m) in dev2_port3.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap16s31/ports/3/hw_counters/{}", m),
            &i.to_string(),
        );
    }

    // Device 3: rdmap32s31 — device-level only, no ports
    let dev3_hw = ["lifespan", "keep_alive_rcvd", "mmap_err", "create_qp_err"];
    for (i, m) in dev3_hw.iter().enumerate() {
        create_metric_file(
            &root,
            &format!("rdmap32s31/hw_counters/{}", m),
            &i.to_string(),
        );
    }

    let mut result = collect_efa_metrics_file_paths(root.to_str().unwrap());

    // Expect 8 entries: 3 device-level + 2 ports (dev1) + 3 ports (dev2)
    assert_eq!(result.len(), 8);

    // Helper closure: verify a device/port entry has the expected metrics and readable files
    fn assert_device_metrics(
        result: &mut HashMap<String, HashMap<String, std::fs::File>>,
        key: &str,
        expected_metrics: &[&str],
    ) {
        let counters = result.get_mut(key).unwrap();
        assert_eq!(
            counters.len(),
            expected_metrics.len(),
            "metric count mismatch for {}",
            key
        );
        for (i, m) in expected_metrics.iter().enumerate() {
            let content = read_file_content(counters.get_mut(*m).unwrap());
            assert_eq!(content, i.to_string(), "content mismatch for {}/{}", key, m);
        }
    }

    // --- Device 1 ---
    assert_device_metrics(&mut result, "rdmap0s31", &dev1_hw);
    assert_device_metrics(&mut result, "rdmap0s31/1", &dev1_port1);
    assert_device_metrics(&mut result, "rdmap0s31/2", &dev1_port2);

    // --- Device 2 ---
    assert_device_metrics(&mut result, "rdmap16s31", &dev2_hw);
    assert_device_metrics(&mut result, "rdmap16s31/1", &dev2_port1);
    assert_device_metrics(&mut result, "rdmap16s31/2", &dev2_port2);
    assert_device_metrics(&mut result, "rdmap16s31/3", &dev2_port3);

    // --- Device 3 (no ports) ---
    assert_device_metrics(&mut result, "rdmap32s31", &dev3_hw);

    tmp.close().unwrap();
}
