use std::collections::HashMap;

use aperf::analytics::time_series_stat_threshold_rule::TimeSeriesStatThresholdRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::computations::{Comparator, Stat};
use aperf::data::common::data_formats::AperfData;
use aperf::data::common::processed_data_accessor::ProcessedDataAccessor;

use super::test_helpers::{
    create_processed_data, create_time_series_data, create_time_series_data_multi_series,
    DataFindingsExt,
};

#[test]
fn test_stat_below_threshold() {
    let ts_data = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 30.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
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
fn test_stat_above_threshold() {
    let ts_data = create_time_series_data(vec![("metric1", vec![50.0, 60.0, 70.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
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
fn test_stat_equal_threshold() {
    let ts_data = create_time_series_data(vec![("metric1", vec![50.0, 50.0, 50.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::GreaterEqual,
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

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Less,
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
fn test_max_stat() {
    let ts_data = create_time_series_data(vec![("metric1", vec![10.0, 20.0, 100.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Max,
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
fn test_min_stat() {
    let ts_data = create_time_series_data(vec![("metric1", vec![5.0, 20.0, 100.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Min,
        comparator: Comparator::Less,
        threshold: 10.0,
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

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric2",
        stat: Stat::Average,
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

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
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
fn test_multiple_series_uses_aggregate() {
    let ts_data = create_time_series_data_multi_series(vec![(
        "metric1",
        vec![
            (Some("cpu0"), vec![10.0, 20.0, 30.0]),
            (Some("cpu1"), vec![15.0, 25.0, 35.0]),
            (None, vec![60.0, 70.0, 80.0]), // aggregate series
        ],
    )]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
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
fn test_time_range_changes_stat() {
    // Values: [10, 20, 30, 90, 90, 90]  (time_diff: [0,1,2,3,4,5])
    let ts_data =
        create_time_series_data(vec![("metric1", vec![10.0, 20.0, 30.0, 90.0, 90.0, 90.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        threshold: 50.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    // Full range avg = 55 → exceeds threshold → finding
    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 1);

    // Time range 0:2 → [10,20,30], avg = 20 → no finding
    let mut accessor = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([("run1".to_string(), 0)]),
        HashMap::from([("run1".to_string(), 2)]),
    );
    let mut findings2 = DataFindings::default();
    rule.analyze(&mut findings2, &processed_data, &mut accessor);
    assert_eq!(findings2.num_runs_with_findings(), 0);

    // Time range 3:5 → [90,90,90], avg = 90 → finding
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
    // run1: [10, 20, 30, 90, 90, 90]  avg=55, spikes late
    // run2: [90, 90, 90, 10, 20, 30]  avg=55, spikes early
    // run3: [40, 40, 40, 40, 40, 40]  avg=40, flat
    let ts_data1 =
        create_time_series_data(vec![("metric1", vec![10.0, 20.0, 30.0, 90.0, 90.0, 90.0])]);
    let ts_data2 =
        create_time_series_data(vec![("metric1", vec![90.0, 90.0, 90.0, 10.0, 20.0, 30.0])]);
    let ts_data3 =
        create_time_series_data(vec![("metric1", vec![40.0, 40.0, 40.0, 40.0, 40.0, 40.0])]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
            ("run3", AperfData::TimeSeries(ts_data3)),
        ],
    );

    let rule = TimeSeriesStatThresholdRule {
        rule_name: "test_rule",
        metric_name: "metric1",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        threshold: 50.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    // Full range → run1 avg=55, run2 avg=55 exceed; run3 avg=40 doesn't
    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 2);
    assert!(findings.has_findings_for_run("run1"));
    assert!(findings.has_findings_for_run("run2"));
    assert!(!findings.has_findings_for_run("run3"));

    // Time range 0:2 → run1 avg=20, run2 avg=90, run3 avg=40 → only run2
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
    assert!(findings2.has_findings_for_run("run2"));

    // Time range 3:5 → run1 avg=90, run2 avg=20, run3 avg=40 → only run1
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
    assert!(findings3.has_findings_for_run("run1"));
}
