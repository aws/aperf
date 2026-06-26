// PMU perf_stat processing tests.
//
// The PMU feature is Linux-only: `PerfStatRaw` carries a Linux-only
// `pmu_collectors` field and is constructed via `PerfStatRaw::new()`, so this
// whole test file only compiles/runs on Linux (matches the rest of the perf
// collection code, which is gated the same way).
#![cfg(target_os = "linux")]

use aperf::data::common::data_formats::{AperfData, Series, TimeSeriesMetric};
use aperf::data::perf_stat::{PerfStat, PerfStatRaw};
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use aperf::{GROUPED_PMU_MODE, UNGROUPED_PMU_MODE};
use chrono::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;

const EPS: f64 = 1e-6;

fn base_time(secs: i64) -> TimeEnum {
    TimeEnum::DateTime(
        Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap() + chrono::Duration::seconds(secs),
    )
}

fn approx(actual: f64, expected: f64, ctx: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{ctx}: expected {expected}, got {actual}"
    );
}

// ===========================================================================
// Legacy (pre-revamp) format: "<cpu> <stat>; <nr...>; <dr...>;<scale>"
//   per-CPU value     = sum(nr) / sum(dr) * scale
//   aggregate value   = sum(nr * scale) / sum(dr)   (over all CPUs)
// One comprehensive test covers the legacy path; the rest of this file covers
// the new (events + metric-formula) grouped/ungrouped flows.
// ===========================================================================

/// Expected PMU statistics for a single CPU and stat name (legacy format).
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

/// Generate legacy-format raw PMU data: "<cpu> <stat>; <nrs>; <drs>;<scale>".
fn generate_legacy_raw_data(
    expected_per_sample_per_cpu_per_stat: &Vec<HashMap<usize, HashMap<String, ExpectedPmuStats>>>,
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();

    for (sample_idx, per_cpu_per_stat) in expected_per_sample_per_cpu_per_stat.iter().enumerate() {
        let mut data_lines = Vec::new();

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
                data_lines.push(format!(
                    "{} {}; {}; {};{}",
                    cpu, stat_name, numerators_str, denominators_str, expected_stats.scale
                ));
            }
        }

        let mut raw = PerfStatRaw::new();
        raw.time = base_time((sample_idx as i64) * (interval_seconds as i64));
        raw.data = data_lines.join("\n");
        raw_data.push(Data::PerfStatRaw(raw));
    }

    raw_data
}

#[test]
fn test_legacy_format_comprehensive() {
    // 100 samples x 4 CPUs x 3 stats, exercising multiple numerators/denominators,
    // distinct scales, per-CPU values, the aggregate series, time diffs, the
    // top-down metric ordering, and aggregate-series marking — all in the legacy
    // nr/dr format (empty pmu_counter_mode).
    let mut expected_data = Vec::new();
    for sample in 0usize..100 {
        let mut per_cpu_per_stat = HashMap::new();
        for cpu in 0usize..4 {
            let mut per_stat = HashMap::new();

            // ipc: two numerators + two denominators, scale 1.
            per_stat.insert(
                "ipc".to_string(),
                ExpectedPmuStats::new(
                    vec![
                        (1000 + sample * 10 + cpu * 5) as u64,
                        (2000 + sample * 15 + cpu * 7) as u64,
                    ],
                    vec![
                        (5000 + sample * 50 + cpu * 25) as u64,
                        (10000 + sample * 75 + cpu * 35) as u64,
                    ],
                    1,
                ),
            );
            // l3-mpki: single nr/dr, scale 100.
            per_stat.insert(
                "l3-mpki".to_string(),
                ExpectedPmuStats::new(
                    vec![(100 + sample * 2 + cpu) as u64],
                    vec![(10000 + sample * 100 + cpu * 50) as u64],
                    100,
                ),
            );
            // branch-mpki: single nr/dr, scale 1000.
            per_stat.insert(
                "branch-mpki".to_string(),
                ExpectedPmuStats::new(
                    vec![(9000 + sample * 5 + cpu * 2) as u64],
                    vec![(10000 + sample * 10 + cpu * 3) as u64],
                    1000,
                ),
            );

            per_cpu_per_stat.insert(cpu, per_stat);
        }
        expected_data.push(per_cpu_per_stat);
    }

    let raw_data = generate_legacy_raw_data(&expected_data, 1);
    // Legacy path: pmu_counter_mode is empty.
    let result = PerfStat::new()
        .process_raw_data(ReportParams::new(), raw_data)
        .unwrap();

    let ts = match result {
        AperfData::TimeSeries(ts) => ts,
        _ => panic!("Expected TimeSeries data"),
    };

    // Structure: 3 metrics, ordered per the top-down debug guidance.
    assert_eq!(ts.metrics.len(), 3);
    assert_eq!(
        ts.sorted_metric_names,
        vec!["ipc", "branch-mpki", "l3-mpki"]
    );

    // Each metric: 4 CPU series + 1 aggregate, each with 100 points.
    for metric in ts.metrics.values() {
        assert_eq!(metric.series.len(), 5);
        for series in &metric.series {
            assert_eq!(series.values.len(), 100);
            assert_eq!(series.time_diff.len(), 100);
        }
    }

    // Validate every per-CPU and aggregate value.
    for (stat_name, metric) in &ts.metrics {
        for sample in 0..100 {
            let mut agg_numerator = 0.0;
            let mut agg_denominator = 0.0;
            for cpu in 0..4 {
                // Series sort: "Aggregate" < "CPU0".."CPU3" → CPU{n} at index n+1.
                let cpu_series = &metric.series[cpu + 1];
                let expected = &expected_data[sample][&cpu][stat_name];
                approx(
                    cpu_series.values[sample],
                    expected.calculate_value(),
                    &format!("legacy sample {sample} CPU {cpu} {stat_name}"),
                );
                let nr: u64 = expected.numerators.iter().sum();
                let dr: u64 = expected.denominators.iter().sum();
                agg_numerator += (nr as f64) * (expected.scale as f64);
                agg_denominator += dr as f64;
            }
            let aggregate_series = &metric.series[0];
            assert!(aggregate_series.is_aggregate);
            assert_eq!(aggregate_series.series_name, "Aggregate");
            approx(
                aggregate_series.values[sample],
                agg_numerator / agg_denominator,
                &format!("legacy sample {sample} {stat_name} aggregate"),
            );
        }
    }

    // Time diffs are 1s intervals.
    for metric in ts.metrics.values() {
        for series in &metric.series {
            for sample in 0..100 {
                assert_eq!(series.time_diff[sample], sample as u64);
            }
        }
    }
}

// ===========================================================================
// New format helpers (events + metric-formula config, evaluated via exmex).
//
// The report path reads the PMU config that `record` saved next to the data
// (data_dir/pmu_config.json), so each new-format test writes one to a temp dir.
//
// IMPORTANT ordering contract: a metric's counters are serialized in exmex
// var_names() order, which is ALPHABETICAL by event name. eval() consumes them
// positionally in that same order. Test data below mirrors that exactly.
// ===========================================================================

/// Write a v2 PMU config (events + metrics) to `<dir>/pmu_config.json`,
/// preserving the given key order (matters for metric display order).
fn write_pmu_config(dir: &Path, events: &[(&str, &str)], metrics: &[(&str, &str)]) {
    fn obj(pairs: &[(&str, &str)]) -> String {
        pairs
            .iter()
            .map(|(k, v)| format!("    {:?}: {:?}", k, v))
            .collect::<Vec<_>>()
            .join(",\n")
    }
    let json = format!(
        "{{\n  \"events\": {{\n{}\n  }},\n  \"metrics\": {{\n{}\n  }}\n}}",
        obj(events),
        obj(metrics)
    );
    std::fs::write(dir.join("pmu_config.json"), json).unwrap();
}

fn report_params(data_dir: &Path, mode: &str) -> ReportParams {
    let mut params = ReportParams::new();
    params.data_dir = data_dir.to_path_buf();
    params.pmu_counter_mode = mode.to_string();
    params
}

/// Grouped raw line: "cpu;metric;c1;c2;...;time_enabled;time_running".
/// `counters` MUST be in exmex var_names() (alphabetical) order.
fn grouped_line(
    cpu: usize,
    metric: &str,
    counters: &[u64],
    time_enabled: f64,
    time_running: f64,
) -> String {
    let mut line = format!("{cpu};{metric};");
    for c in counters {
        line.push_str(&format!("{c};"));
    }
    line.push_str(&format!("{time_enabled};{time_running}"));
    line
}

/// Ungrouped raw line: "cpu;event;value;time_enabled;time_running".
fn ungrouped_line(
    cpu: usize,
    event: &str,
    value: u64,
    time_enabled: f64,
    time_running: f64,
) -> String {
    format!("{cpu};{event};{value};{time_enabled};{time_running}")
}

fn snapshot(secs: i64, lines: Vec<String>) -> Data {
    let mut raw = PerfStatRaw::new();
    raw.time = base_time(secs);
    raw.data = lines.join("\n");
    Data::PerfStatRaw(raw)
}

fn process(
    params: ReportParams,
    raw_data: Vec<Data>,
) -> aperf::data::common::data_formats::TimeSeriesData {
    match PerfStat::new().process_raw_data(params, raw_data).unwrap() {
        AperfData::TimeSeries(ts) => ts,
        _ => panic!("Expected TimeSeries data"),
    }
}

fn series<'a>(metric: &'a TimeSeriesMetric, name: &str) -> &'a Series {
    metric
        .series
        .iter()
        .find(|s| s.series_name == name)
        .unwrap_or_else(|| panic!("series {name} not found"))
}

fn aggregate<'a>(metric: &'a TimeSeriesMetric) -> &'a Series {
    metric
        .series
        .iter()
        .find(|s| s.is_aggregate)
        .expect("aggregate series not found")
}

// ===========================================================================
// New format — GROUPED collection
// ===========================================================================

#[test]
fn test_grouped_single_metric_multi_cpu() {
    // ipc = Instructions / Cycles ; var_names = [Cycles, Instructions]
    // so each group line's counters are [Cycles, Instructions]. No multiplexing
    // (time_enabled == time_running => scale 1). First sample uses the first
    // accumulative value as its own delta.
    let dir = TempDir::new().unwrap();
    write_pmu_config(
        dir.path(),
        &[
            ("Cycles", "armv8_pmuv3_0/event=0x11/"),
            ("Instructions", "armv8_pmuv3_0/event=0x8/"),
        ],
        &[("ipc", "Instructions / Cycles")],
    );

    // Counters are PER-INTERVAL (collection resets the group each interval);
    // only time_enabled/time_running are accumulative (the kernel cannot reset
    // those). With te == tr each interval the multiplexing scale is 1, so the
    // per-CPU value is just (Instructions / Cycles) of that interval.
    // CPU0: (Cycles,Instr) -> (1000,2000) (2000,4000) (3000,4000)
    // CPU1: (Cycles,Instr) -> ( 500,1500) (1000,2000) (1500,2000)
    let raw = vec![
        snapshot(
            0,
            vec![
                grouped_line(0, "ipc", &[1000, 2000], 1.0, 1.0),
                grouped_line(1, "ipc", &[500, 1500], 1.0, 1.0),
            ],
        ),
        snapshot(
            1,
            vec![
                grouped_line(0, "ipc", &[2000, 4000], 2.0, 2.0),
                grouped_line(1, "ipc", &[1000, 2000], 2.0, 2.0),
            ],
        ),
        snapshot(
            2,
            vec![
                grouped_line(0, "ipc", &[3000, 4000], 3.0, 3.0),
                grouped_line(1, "ipc", &[1500, 2000], 3.0, 3.0),
            ],
        ),
    ];

    let ts = process(report_params(dir.path(), GROUPED_PMU_MODE), raw);

    assert_eq!(ts.sorted_metric_names, vec!["ipc"]);
    let ipc = &ts.metrics["ipc"];
    // 2 CPU series + 1 aggregate.
    assert_eq!(ipc.series.len(), 3);

    let cpu0 = series(ipc, "CPU0");
    let cpu1 = series(ipc, "CPU1");
    // CPU0: 2000/1000, 4000/2000, 4000/3000
    approx(cpu0.values[0], 2.0, "cpu0 s0");
    approx(cpu0.values[1], 2.0, "cpu0 s1");
    approx(cpu0.values[2], 4000.0 / 3000.0, "cpu0 s2");
    // CPU1: 1500/500, 2000/1000, 2000/1500
    approx(cpu1.values[0], 3.0, "cpu1 s0");
    approx(cpu1.values[1], 2.0, "cpu1 s1");
    approx(cpu1.values[2], 2000.0 / 1500.0, "cpu1 s2");

    // Aggregate = sum(Instructions) / sum(Cycles) across CPUs, per sample.
    let agg = aggregate(ipc);
    approx(agg.values[0], 3500.0 / 1500.0, "agg s0");
    approx(agg.values[1], 6000.0 / 3000.0, "agg s1");
    approx(agg.values[2], 6000.0 / 4500.0, "agg s2");

    assert_eq!(agg.time_diff, vec![0, 1, 2]);
}

#[test]
fn test_grouped_multiplexing_scale_cancels_for_ratio() {
    // A group shares one (time_enabled, time_running) across all its counters, so
    // every counter is scaled by the same factor and the factor cancels in a ratio
    // metric. This guards against double-applying or mis-applying the scale: ipc
    // must stay the pure Instructions/Cycles ratio regardless of the scale factor.
    let dir = TempDir::new().unwrap();
    write_pmu_config(
        dir.path(),
        &[
            ("Cycles", "armv8_pmuv3_0/event=0x11/"),
            ("Instructions", "armv8_pmuv3_0/event=0x8/"),
        ],
        &[("ipc", "Instructions / Cycles")],
    );

    // s0: delta (Cycles=1000, Instr=2000), te=2 tr=1 => scale x2
    // s1: delta (Cycles=1000, Instr=2000), te=4 tr=1 => scale x4
    let raw = vec![
        snapshot(0, vec![grouped_line(0, "ipc", &[1000, 2000], 2.0, 1.0)]),
        snapshot(1, vec![grouped_line(0, "ipc", &[2000, 4000], 6.0, 2.0)]),
    ];

    let ts = process(report_params(dir.path(), GROUPED_PMU_MODE), raw);
    let cpu0 = series(&ts.metrics["ipc"], "CPU0");
    // Despite scale x2 then x4, the ratio is unchanged.
    approx(cpu0.values[0], 2.0, "scaled ratio s0");
    approx(cpu0.values[1], 2.0, "scaled ratio s1");
}

#[test]
fn test_grouped_metric_order_follows_config() {
    // sorted_metric_names must follow the config's metric insertion order, NOT
    // alphabetical. Config order here is deliberately non-alphabetical.
    let dir = TempDir::new().unwrap();
    write_pmu_config(
        dir.path(),
        &[
            ("Branches", "p/event=0x10/"),
            ("Cycles", "p/event=0x11/"),
            ("Instructions", "p/event=0x8/"),
            ("L3", "p/event=0x37/"),
        ],
        &[
            ("l3-mpki", "L3 / Instructions * 1000"),
            ("ipc", "Instructions / Cycles"),
            ("branch-mpki", "Branches / Instructions * 1000"),
        ],
    );

    // One sample each; counters in var_names (alphabetical) order:
    //   l3-mpki     var_names [Instructions, L3]
    //   ipc         var_names [Cycles, Instructions]
    //   branch-mpki var_names [Branches, Instructions]
    let raw = vec![snapshot(
        0,
        vec![
            grouped_line(0, "l3-mpki", &[1000, 50], 1.0, 1.0),
            grouped_line(0, "ipc", &[1000, 2000], 1.0, 1.0),
            grouped_line(0, "branch-mpki", &[100, 2000], 1.0, 1.0),
        ],
    )];

    let ts = process(report_params(dir.path(), GROUPED_PMU_MODE), raw);
    assert_eq!(
        ts.sorted_metric_names,
        vec!["l3-mpki", "ipc", "branch-mpki"]
    );

    approx(
        series(&ts.metrics["l3-mpki"], "CPU0").values[0],
        50.0,
        "l3-mpki",
    );
    approx(series(&ts.metrics["ipc"], "CPU0").values[0], 2.0, "ipc");
    approx(
        series(&ts.metrics["branch-mpki"], "CPU0").values[0],
        50.0,
        "branch-mpki",
    );
}

#[test]
fn test_grouped_frozen_time_running_sample_skipped() {
    // If time_running does not advance between snapshots (delta 0), that snapshot
    // is dropped to avoid a divide-by-zero scale.
    let dir = TempDir::new().unwrap();
    write_pmu_config(
        dir.path(),
        &[
            ("Cycles", "p/event=0x11/"),
            ("Instructions", "p/event=0x8/"),
        ],
        &[("ipc", "Instructions / Cycles")],
    );

    let raw = vec![
        // s0: first sample, time_running delta = 5 (nonzero) -> processed.
        snapshot(0, vec![grouped_line(0, "ipc", &[1000, 2000], 5.0, 5.0)]),
        // s1: time_running stays at 5 -> delta 0 -> skipped.
        snapshot(1, vec![grouped_line(0, "ipc", &[2000, 4000], 5.0, 5.0)]),
        // s2: time_running advances again -> processed.
        snapshot(2, vec![grouped_line(0, "ipc", &[3000, 6000], 7.0, 7.0)]),
    ];

    let ts = process(report_params(dir.path(), GROUPED_PMU_MODE), raw);
    let cpu0 = series(&ts.metrics["ipc"], "CPU0");
    // Only s0 and s2 produced points (s1 dropped).
    assert_eq!(cpu0.values.len(), 2);
    assert_eq!(cpu0.time_diff, vec![0, 2]);
    approx(cpu0.values[0], 2.0, "frozen-tr s0");
    approx(cpu0.values[1], 2.0, "frozen-tr s2");
}

// ===========================================================================
// New format — UNGROUPED collection
// ===========================================================================

#[test]
fn test_ungrouped_event_dedup_across_metrics() {
    // Ungrouped collects each EVENT once (deduplicated across metrics) and feeds
    // the same event value into every metric that references it. Here both ipc and
    // branch-mpki use Instructions, collected from a single Instructions line.
    // No multiplexing (te == tr => scale 1).
    let dir = TempDir::new().unwrap();
    write_pmu_config(
        dir.path(),
        &[
            ("Branches", "p/event=0x10/"),
            ("Cycles", "p/event=0x11/"),
            ("Instructions", "p/event=0x8/"),
        ],
        &[
            ("ipc", "Instructions / Cycles"),
            ("branch-mpki", "Branches / Instructions * 1000"),
        ],
    );

    // Counter values are PER-INTERVAL (reset-based collection); only te/tr are
    // accumulative. te == tr each interval => scale 1.
    // CPU0: s0 (C1000,I2000,B200) s1 (C1000,I2000,B100)
    // CPU1: s0 (C 500,I1500,B150) s1 (C1000,I2000,B250)
    let raw = vec![
        snapshot(
            0,
            vec![
                ungrouped_line(0, "Cycles", 1000, 1.0, 1.0),
                ungrouped_line(0, "Instructions", 2000, 1.0, 1.0),
                ungrouped_line(0, "Branches", 200, 1.0, 1.0),
                ungrouped_line(1, "Cycles", 500, 1.0, 1.0),
                ungrouped_line(1, "Instructions", 1500, 1.0, 1.0),
                ungrouped_line(1, "Branches", 150, 1.0, 1.0),
            ],
        ),
        snapshot(
            1,
            vec![
                ungrouped_line(0, "Cycles", 1000, 2.0, 2.0),
                ungrouped_line(0, "Instructions", 2000, 2.0, 2.0),
                ungrouped_line(0, "Branches", 100, 2.0, 2.0),
                ungrouped_line(1, "Cycles", 1000, 2.0, 2.0),
                ungrouped_line(1, "Instructions", 2000, 2.0, 2.0),
                ungrouped_line(1, "Branches", 250, 2.0, 2.0),
            ],
        ),
    ];

    let ts = process(report_params(dir.path(), UNGROUPED_PMU_MODE), raw);

    assert_eq!(ts.metrics.len(), 2);
    assert_eq!(ts.sorted_metric_names, vec!["ipc", "branch-mpki"]);

    // ipc per CPU.
    approx(
        series(&ts.metrics["ipc"], "CPU0").values[0],
        2000.0 / 1000.0,
        "ipc cpu0 s0",
    );
    approx(
        series(&ts.metrics["ipc"], "CPU1").values[0],
        1500.0 / 500.0,
        "ipc cpu1 s0",
    );
    approx(
        series(&ts.metrics["ipc"], "CPU0").values[1],
        4000.0 / 2000.0,
        "ipc cpu0 s1",
    );
    approx(
        series(&ts.metrics["ipc"], "CPU1").values[1],
        2000.0 / 1000.0,
        "ipc cpu1 s1",
    );

    // branch-mpki = Branches / Instructions * 1000 per CPU.
    approx(
        series(&ts.metrics["branch-mpki"], "CPU0").values[0],
        200.0 / 2000.0 * 1000.0,
        "bm cpu0 s0",
    );
    approx(
        series(&ts.metrics["branch-mpki"], "CPU1").values[0],
        150.0 / 1500.0 * 1000.0,
        "bm cpu1 s0",
    );
    approx(
        series(&ts.metrics["branch-mpki"], "CPU0").values[1],
        100.0 / 2000.0 * 1000.0,
        "bm cpu0 s1",
    );
    approx(
        series(&ts.metrics["branch-mpki"], "CPU1").values[1],
        250.0 / 2000.0 * 1000.0,
        "bm cpu1 s1",
    );

    // Aggregate uses summed event values across CPUs.
    // s1: ipc = (4000+2000)/(2000+1000) = 2.0 ; bm = (100+250)/(2000+2000)*1000 = 87.5
    approx(
        aggregate(&ts.metrics["ipc"]).values[1],
        6000.0 / 3000.0,
        "ipc agg s1",
    );
    approx(
        aggregate(&ts.metrics["branch-mpki"]).values[1],
        350.0 / 4000.0 * 1000.0,
        "bm agg s1",
    );
}

#[test]
fn test_ungrouped_per_event_scaling_changes_result() {
    // Unlike grouped, each ungrouped event carries its OWN (time_enabled,
    // time_running), so the multiplexing scale does NOT cancel in a ratio when the
    // two events are scaled differently. Here Cycles is scaled x2 (ran half the
    // time) while Instructions is unscaled, turning a raw 2.0 ratio into 1.0.
    let dir = TempDir::new().unwrap();
    write_pmu_config(
        dir.path(),
        &[
            ("Cycles", "p/event=0x11/"),
            ("Instructions", "p/event=0x8/"),
        ],
        &[("ipc", "Instructions / Cycles")],
    );

    let raw = vec![
        // s0 baseline: both events te=1 tr=1.
        snapshot(
            0,
            vec![
                ungrouped_line(0, "Cycles", 1000, 1.0, 1.0),
                ungrouped_line(0, "Instructions", 2000, 1.0, 1.0),
            ],
        ),
        // s1 deltas: Cycles counter +1000 with te_delta=2, tr_delta=1 -> scale x2 -> 2000;
        //            Instructions counter +2000 with te_delta=2, tr_delta=2 -> scale x1 -> 2000.
        //   (time_running must advance, else the snapshot is skipped.)
        //   ipc = 2000 / 2000 = 1.0 (would be 2.0 without per-event scaling).
        snapshot(
            1,
            vec![
                ungrouped_line(0, "Cycles", 2000, 3.0, 2.0),
                ungrouped_line(0, "Instructions", 4000, 3.0, 3.0),
            ],
        ),
    ];

    let ts = process(report_params(dir.path(), UNGROUPED_PMU_MODE), raw);
    let cpu0 = series(&ts.metrics["ipc"], "CPU0");
    approx(cpu0.values[1], 1.0, "per-event scaled ipc s1");
}

// ===========================================================================
// New format — shared edge cases
// ===========================================================================

#[test]
fn test_new_format_empty_data() {
    // Empty raw data with a valid saved config yields no metrics (no panic).
    let dir = TempDir::new().unwrap();
    write_pmu_config(
        dir.path(),
        &[
            ("Cycles", "p/event=0x11/"),
            ("Instructions", "p/event=0x8/"),
        ],
        &[("ipc", "Instructions / Cycles")],
    );

    for mode in [GROUPED_PMU_MODE, UNGROUPED_PMU_MODE] {
        let ts = process(report_params(dir.path(), mode), Vec::new());
        assert!(ts.metrics.is_empty(), "{mode}: metrics should be empty");
        assert!(
            ts.sorted_metric_names.is_empty(),
            "{mode}: names should be empty"
        );
    }
}
