use aperf::analytics::time_series_data_point_threshold_rule::TimeSeriesDataPointThresholdRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::computations::Comparator;
use aperf::data::data_formats::AperfData;

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
}
