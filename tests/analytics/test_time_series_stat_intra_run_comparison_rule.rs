use aperf::analytics::time_series_stat_intra_run_comparison_rule::TimeSeriesStatIntraRunComparisonRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::computations::{Comparator, Stat};
use aperf::data::data_formats::AperfData;

use super::test_helpers::{create_processed_data, create_time_series_data, DataFindingsExt};

#[test]
fn test_no_significant_delta() {
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![100.0, 100.0, 100.0]),
        ("comparison_metric", vec![105.0, 105.0, 105.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![100.0, 100.0, 100.0]),
        ("comparison_metric", vec![150.0, 150.0, 150.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
    assert!(findings.has_findings_for_run("run1"));
}

#[test]
fn test_significant_negative_delta() {
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![100.0, 100.0, 100.0]),
        ("comparison_metric", vec![50.0, 50.0, 50.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![100.0, 100.0, 100.0]),
        ("comparison_metric", vec![50.0, 50.0, 50.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
fn test_baseline_stat_zero() {
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![0.0, 0.0, 0.0]),
        ("comparison_metric", vec![50.0, 50.0, 50.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![0.0, 0.0, 0.0]),
        ("comparison_metric", vec![0.0, 0.0, 0.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![10.0, 20.0, 100.0]),
        ("comparison_metric", vec![10.0, 20.0, 200.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
fn test_min_stat() {
    let ts_data = create_time_series_data(vec![
        ("baseline_metric", vec![10.0, 20.0, 100.0]),
        ("comparison_metric", vec![5.0, 20.0, 100.0]),
    ]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
        stat: Stat::Min,
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
fn test_baseline_metric_not_found() {
    let ts_data = create_time_series_data(vec![("comparison_metric", vec![150.0, 150.0, 150.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
fn test_comparison_metric_not_found() {
    let ts_data = create_time_series_data(vec![("baseline_metric", vec![100.0, 100.0, 100.0])]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::TimeSeries(ts_data))]);

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
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
fn test_multiple_runs() {
    let ts_data1 = create_time_series_data(vec![
        ("baseline_metric", vec![100.0, 100.0, 100.0]),
        ("comparison_metric", vec![180.0, 125.0, 135.0]),
    ]);
    let ts_data2 = create_time_series_data(vec![
        ("baseline_metric", vec![100.0, 100.0, 100.0]),
        ("comparison_metric", vec![150.0, 150.0, 150.0]),
    ]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
        ],
    );

    let rule = TimeSeriesStatIntraRunComparisonRule {
        rule_name: "test_rule",
        baseline_metric_name: "baseline_metric",
        comparison_metric_name: "comparison_metric",
        stat: Stat::Average,
        comparator: Comparator::Greater,
        abs: false,
        delta_ratio: 0.1,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 2);
    assert!(findings.has_findings_for_run("run1"));
    assert!(findings.has_findings_for_run("run2"));
}
