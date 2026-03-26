use std::collections::HashMap;

use aperf::analytics::time_series_stat_run_comparison_rule::TimeSeriesStatRunComparisonRule;
use aperf::analytics::{Analyze, DataFindings, Score, BASE_RUN_NAME};
use aperf::computations::{Comparator, Stat};
use aperf::data::common::data_formats::AperfData;
use aperf::data::common::processed_data_accessor::ProcessedDataAccessor;

use super::test_helpers::{create_processed_data, create_time_series_data, DataFindingsExt};

fn set_base_run(name: &str) {
    *BASE_RUN_NAME.lock().unwrap() = name.to_string();
}

#[test]
fn test_no_significant_delta() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![100.0, 100.0, 100.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![105.0, 105.0, 105.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 0);
}

#[test]
fn test_significant_positive_delta() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![100.0, 100.0, 100.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![150.0, 150.0, 150.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(findings.has_findings_for_run("run2"));
}

#[test]
fn test_significant_negative_delta() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![100.0, 100.0, 100.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![50.0, 50.0, 50.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Less,
        abs: false,
        delta_ratio: -0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 1);
}

#[test]
fn test_abs_delta() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![100.0, 100.0, 100.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![50.0, 50.0, 50.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: true,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 1);
}

#[test]
fn test_base_stat_zero() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![0.0, 0.0, 0.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![50.0, 50.0, 50.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.5,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 1);
}

#[test]
fn test_equal_zero_stats() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![0.0, 0.0, 0.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![0.0, 0.0, 0.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 0);
}

#[test]
fn test_max_stat() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 100.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 200.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Max,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.5,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 1);
}

#[test]
fn test_metric_not_in_base_run() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("other_metric", vec![100.0, 100.0, 100.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![150.0, 150.0, 150.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 0);
}

#[test]
fn test_multiple_non_base_runs() {
    set_base_run("run1");

    let ts_data1 = create_time_series_data(vec![("metric1", vec![100.0, 100.0, 100.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![105.0, 105.0, 105.0])]);
    let ts_data3 = create_time_series_data(vec![("metric1", vec![150.0, 150.0, 150.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
            ("run3", AperfData::TimeSeries(ts_data3)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(!findings.has_findings_for_run("run1"));
    assert!(!findings.has_findings_for_run("run2"));
    assert!(findings.has_findings_for_run("run3"));
}

#[test]
fn test_time_range_affects_comparison() {
    set_base_run("run1");

    // run1: [100, 100, 100, 100, 100, 100]
    // run2: [100, 100, 100, 200, 200, 200]
    let ts_data1 = create_time_series_data(vec![(
        "metric1",
        vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
    )]);
    let ts_data2 = create_time_series_data(vec![(
        "metric1",
        vec![100.0, 100.0, 100.0, 200.0, 200.0, 200.0],
    )]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    // Full range: run1 avg=100, run2 avg=150 → 50% delta → finding
    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 1);

    // Time range 0:2 → both runs have avg=100 → no finding
    let mut accessor = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([("run1".to_string(), 0), ("run2".to_string(), 0)]),
        HashMap::from([("run1".to_string(), 2), ("run2".to_string(), 2)]),
    );
    let mut findings2 = DataFindings::default();
    rule.analyze(&mut findings2, &processed_data, &mut accessor);
    assert_eq!(findings2.num_runs_with_findings(), 0);

    // Time range 3:5 → run1 avg=100, run2 avg=200 → 100% delta → finding
    let mut accessor2 = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([("run1".to_string(), 3), ("run2".to_string(), 3)]),
        HashMap::from([("run1".to_string(), 5), ("run2".to_string(), 5)]),
    );
    let mut findings3 = DataFindings::default();
    rule.analyze(&mut findings3, &processed_data, &mut accessor2);
    assert_eq!(findings3.num_runs_with_findings(), 1);
}

#[test]
fn test_time_range_multi_run() {
    set_base_run("run1");

    // run1 (base): [100, 100, 100, 100, 100, 100]  flat baseline
    // run2:        [100, 100, 100, 200, 200, 200]  diverges late
    // run3:        [200, 200, 200, 100, 100, 100]  diverges early
    let ts_data1 = create_time_series_data(vec![(
        "metric1",
        vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
    )]);
    let ts_data2 = create_time_series_data(vec![(
        "metric1",
        vec![100.0, 100.0, 100.0, 200.0, 200.0, 200.0],
    )]);
    let ts_data3 = create_time_series_data(vec![(
        "metric1",
        vec![200.0, 200.0, 200.0, 100.0, 100.0, 100.0],
    )]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
            ("run3", AperfData::TimeSeries(ts_data3)),
        ],
    );

    let rule = TimeSeriesStatRunComparisonRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    // Full range → run2 avg=150 (+50%), run3 avg=150 (+50%) → both findings
    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 2);
    assert!(findings.has_findings_for_run("run2"));
    assert!(findings.has_findings_for_run("run3"));

    // Time range 0:2 → run1=100, run2=100 (no delta), run3=200 (+100%) → only run3
    let mut accessor = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([
            ("run1".to_string(), 0),
            ("run2".to_string(), 0),
            ("run3".to_string(), 0),
        ]),
        HashMap::from([
            ("run1".to_string(), 2),
            ("run2".to_string(), 2),
            ("run3".to_string(), 2),
        ]),
    );
    let mut findings2 = DataFindings::default();
    rule.analyze(&mut findings2, &processed_data, &mut accessor);
    assert_eq!(findings2.num_runs_with_findings(), 1);
    assert!(findings2.has_findings_for_run("run3"));

    // Time range 3:5 → run1=100, run2=200 (+100%), run3=100 (no delta) → only run2
    let mut accessor2 = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([
            ("run1".to_string(), 3),
            ("run2".to_string(), 3),
            ("run3".to_string(), 3),
        ]),
        HashMap::from([
            ("run1".to_string(), 5),
            ("run2".to_string(), 5),
            ("run3".to_string(), 5),
        ]),
    );
    let mut findings3 = DataFindings::default();
    rule.analyze(&mut findings3, &processed_data, &mut accessor2);
    assert_eq!(findings3.num_runs_with_findings(), 1);
    assert!(findings3.has_findings_for_run("run2"));
}
