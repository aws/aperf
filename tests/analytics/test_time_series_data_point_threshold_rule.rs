use std::collections::HashMap;

use aperf::analytics::time_series_data_point_threshold_rule::TimeSeriesDataPointThresholdRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::computations::Comparator;
use aperf::data::common::data_formats::AperfData;
use aperf::data::common::processed_data_accessor::ProcessedDataAccessor;

use super::test_helpers::{create_processed_data, create_time_series_data, DataFindingsExt};

#[test]
fn test_no_data_points_exceed_threshold() {
    let ts_data = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 30.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
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
fn test_one_data_point_exceeds_threshold() {
    let ts_data = create_time_series_data(vec![("metric1", vec![10.0, 60.0, 30.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
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
    assert!(findings.has_findings_for_run("run1"));
}

#[test]
fn test_multiple_data_points_exceed_threshold() {
    let ts_data = create_time_series_data(vec![("metric1", vec![60.0, 70.0, 80.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
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
fn test_less_than_comparator() {
    let ts_data = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 30.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Less,
        threshold: 15.0,
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
fn test_equal_comparator() {
    let ts_data = create_time_series_data(vec![("metric1", vec![10.0, 50.0, 30.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Equal,
        threshold: 50.0,
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
fn test_metric_not_found() {
    let ts_data = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 30.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric2",
        comparator: Comparator::Greater,
        threshold: 50.0,
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
fn test_empty_values() {
    let ts_data = create_time_series_data(vec![("metric1", vec![])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
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
fn test_multiple_runs() {
    let ts_data1 = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 30.0])]);
    let ts_data2 = create_time_series_data(vec![("metric1", vec![60.0, 70.0, 80.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
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
    assert!(findings.has_findings_for_run("run2"));
}

#[test]
fn test_negative_score() {
    let ts_data = create_time_series_data(vec![("metric1", vec![60.0, 70.0, 80.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
        score: -10.0,
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
fn test_time_range_filters_data_points() {
    // Values: [10, 20, 30, 60, 70, 80, 10, 20]  (time_diff: [0,1,2,3,4,5,6,7])
    let ts_data = create_time_series_data(vec![(
        "metric1",
        vec![10.0, 20.0, 30.0, 60.0, 70.0, 80.0, 10.0, 20.0],
    )]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    // Full range → values 60,70,80 exceed threshold → finding
    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 1);

    // Time range 0:2 → only [10,20,30] → no finding
    let mut accessor = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([("run1".to_string(), 0)]),
        HashMap::from([("run1".to_string(), 2)]),
    );
    let mut findings2 = DataFindings::default();
    rule.analyze(&mut findings2, &processed_data, &mut accessor);
    assert_eq!(findings2.num_runs_with_findings(), 0);

    // Time range 3:5 → [60,70,80] → finding
    let mut accessor2 = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([("run1".to_string(), 3)]),
        HashMap::from([("run1".to_string(), 5)]),
    );
    let mut findings3 = DataFindings::default();
    rule.analyze(&mut findings3, &processed_data, &mut accessor2);
    assert_eq!(findings3.num_runs_with_findings(), 1);
}

#[test]
fn test_time_range_multi_run() {
    // run1: [10, 20, 60, 70, 30, 10]  (spikes at t=2,3)
    // run2: [10, 10, 10, 10, 80, 90]  (spikes at t=4,5)
    // run3: [60, 70, 80, 10, 10, 10]  (spikes at t=0,1,2)
    let ts_data1 =
        create_time_series_data(vec![("metric1", vec![10.0, 20.0, 60.0, 70.0, 30.0, 10.0])]);
    let ts_data2 =
        create_time_series_data(vec![("metric1", vec![10.0, 10.0, 10.0, 10.0, 80.0, 90.0])]);
    let ts_data3 =
        create_time_series_data(vec![("metric1", vec![60.0, 70.0, 80.0, 10.0, 10.0, 10.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
            ("run3", AperfData::TimeSeries(ts_data3)),
        ],
    );

    let rule = TimeSeriesDataPointThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        comparator: Comparator::Greater,
        threshold: 50.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    // Full range → all 3 runs have spikes > 50
    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 3);

    // Time range 0:1 → only run3 has spikes (60,70)
    let mut accessor = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([
            ("run1".to_string(), 0),
            ("run2".to_string(), 0),
            ("run3".to_string(), 0),
        ]),
        HashMap::from([
            ("run1".to_string(), 1),
            ("run2".to_string(), 1),
            ("run3".to_string(), 1),
        ]),
    );
    let mut findings2 = DataFindings::default();
    rule.analyze(&mut findings2, &processed_data, &mut accessor);
    assert_eq!(findings2.num_runs_with_findings(), 1);
    assert!(findings2.has_findings_for_run("run3"));

    // Time range 4:5 → only run2 has spikes (80,90)
    let mut accessor2 = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([
            ("run1".to_string(), 4),
            ("run2".to_string(), 4),
            ("run3".to_string(), 4),
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

    // Per-run time ranges: run1=2:3, run2=4:5, run3=3:5 → run1 and run2 have findings, run3 doesn't
    let mut accessor3 = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([
            ("run1".to_string(), 2),
            ("run2".to_string(), 4),
            ("run3".to_string(), 3),
        ]),
        HashMap::from([
            ("run1".to_string(), 3),
            ("run2".to_string(), 5),
            ("run3".to_string(), 5),
        ]),
    );
    let mut findings4 = DataFindings::default();
    rule.analyze(&mut findings4, &processed_data, &mut accessor3);
    assert_eq!(findings4.num_runs_with_findings(), 2);
    assert!(findings4.has_findings_for_run("run1"));
    assert!(findings4.has_findings_for_run("run2"));
    assert!(!findings4.has_findings_for_run("run3"));
}
