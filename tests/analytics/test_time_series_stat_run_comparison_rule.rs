use aperf::analytics::time_series_stat_run_comparison_rule::TimeSeriesStatRunComparisonRule;
use aperf::analytics::{Analyze, DataFindings, Score, BASE_RUN_NAME};
use aperf::computations::{Comparator, Stat};
use aperf::data::data_formats::AperfData;

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

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
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(!findings.has_findings_for_run("run1"));
    assert!(!findings.has_findings_for_run("run2"));
    assert!(findings.has_findings_for_run("run3"));
}
