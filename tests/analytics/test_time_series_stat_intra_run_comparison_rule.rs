use std::collections::HashMap;

use aperf::analytics::time_series_stat_intra_run_comparison_rule::TimeSeriesStatIntraRunComparisonRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::computations::{Comparator, Stat};
use aperf::data::common::data_formats::AperfData;
use aperf::data::common::processed_data_accessor::ProcessedDataAccessor;

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

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
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );

    assert_eq!(findings.num_runs_with_findings(), 2);
    assert!(findings.has_findings_for_run("run1"));
    assert!(findings.has_findings_for_run("run2"));
}

#[test]
fn test_time_range_affects_intra_run_comparison() {
    // baseline: [100, 100, 100, 100, 100, 100]
    // comparison: [100, 100, 100, 200, 200, 200]
    let ts_data = create_time_series_data(vec![
        (
            "baseline_metric",
            vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
        ),
        (
            "comparison_metric",
            vec![100.0, 100.0, 100.0, 200.0, 200.0, 200.0],
        ),
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

    // Full range: baseline avg=100, comparison avg=150 → 50% delta → finding
    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 1);

    // Time range 0:2 → baseline avg=100, comparison avg=100 → no finding
    let mut accessor = ProcessedDataAccessor::from_time_ranges(
        HashMap::from([("run1".to_string(), 0)]),
        HashMap::from([("run1".to_string(), 2)]),
    );
    let mut findings2 = DataFindings::default();
    rule.analyze(&mut findings2, &processed_data, &mut accessor);
    assert_eq!(findings2.num_runs_with_findings(), 0);

    // Time range 3:5 → baseline avg=100, comparison avg=200 → 100% delta → finding
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
    // run1: baseline flat, comparison diverges late
    // run2: baseline flat, comparison diverges early
    // run3: baseline flat, comparison flat (no divergence)
    let ts_data1 = create_time_series_data(vec![
        (
            "baseline_metric",
            vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
        ),
        (
            "comparison_metric",
            vec![100.0, 100.0, 100.0, 200.0, 200.0, 200.0],
        ),
    ]);
    let ts_data2 = create_time_series_data(vec![
        (
            "baseline_metric",
            vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
        ),
        (
            "comparison_metric",
            vec![200.0, 200.0, 200.0, 100.0, 100.0, 100.0],
        ),
    ]);
    let ts_data3 = create_time_series_data(vec![
        (
            "baseline_metric",
            vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
        ),
        (
            "comparison_metric",
            vec![105.0, 105.0, 105.0, 105.0, 105.0, 105.0],
        ),
    ]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::TimeSeries(ts_data1)),
            ("run2", AperfData::TimeSeries(ts_data2)),
            ("run3", AperfData::TimeSeries(ts_data3)),
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

    // Full range → run1 (50% delta), run2 (50% delta), run3 (5% delta) → run1 and run2
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

    // Time range 0:2 → run1 (0% delta), run2 (100% delta), run3 (5% delta) → only run2
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

    // Time range 3:5 → run1 (100% delta), run2 (0% delta), run3 (5% delta) → only run1
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
