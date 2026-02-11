use aperf::analytics::time_series_stat_threshold_rule::TimeSeriesStatThresholdRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::computations::{Comparator, Stat};
use aperf::data::data_formats::AperfData;

use super::test_helpers::{
    create_key_value_data, create_processed_data, create_time_series_data,
    create_time_series_data_multi_series, DataFindingsExt,
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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
}
