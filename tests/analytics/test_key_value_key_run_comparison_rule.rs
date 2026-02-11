use aperf::analytics::key_value_key_run_comparison_rule::KeyValueKeyRunComparisonRule;
use aperf::analytics::{Analyze, DataFindings, Score, BASE_RUN_NAME};
use aperf::data::data_formats::AperfData;

use super::test_helpers::{
    create_key_value_data, create_processed_data, create_time_series_data, DataFindingsExt,
};

fn set_base_run(name: &str) {
    *BASE_RUN_NAME.lock().unwrap() = name.to_string();
}

#[test]
fn test_values_match_across_runs() {
    set_base_run("run1");

    let kv_data1 = create_key_value_data(vec![("test_key", "same_value")]);
    let kv_data2 = create_key_value_data(vec![("test_key", "same_value")]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::KeyValue(kv_data1)),
            ("run2", AperfData::KeyValue(kv_data2)),
        ],
    );

    let rule = KeyValueKeyRunComparisonRule {
        rule_name: "test_rule",
        key: "test_key",
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 0);
}

#[test]
fn test_values_differ_across_runs() {
    set_base_run("run1");

    let kv_data1 = create_key_value_data(vec![("test_key", "value1")]);
    let kv_data2 = create_key_value_data(vec![("test_key", "value2")]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::KeyValue(kv_data1)),
            ("run2", AperfData::KeyValue(kv_data2)),
        ],
    );

    let rule = KeyValueKeyRunComparisonRule {
        rule_name: "test_rule",
        key: "test_key",
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(findings.has_findings_for_run("run2"));
}

#[test]
fn test_key_missing_in_non_base_run() {
    set_base_run("run1");

    let kv_data1 = create_key_value_data(vec![("test_key", "value1")]);
    let kv_data2 = create_key_value_data(vec![("other_key", "value2")]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::KeyValue(kv_data1)),
            ("run2", AperfData::KeyValue(kv_data2)),
        ],
    );

    let rule = KeyValueKeyRunComparisonRule {
        rule_name: "test_rule",
        key: "test_key",
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(findings.has_findings_for_run("run2"));
}

#[test]
fn test_key_missing_in_base_run() {
    set_base_run("run1");

    let kv_data1 = create_key_value_data(vec![("other_key", "value1")]);
    let kv_data2 = create_key_value_data(vec![("test_key", "value2")]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::KeyValue(kv_data1)),
            ("run2", AperfData::KeyValue(kv_data2)),
        ],
    );

    let rule = KeyValueKeyRunComparisonRule {
        rule_name: "test_rule",
        key: "test_key",
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

    let kv_data1 = create_key_value_data(vec![("test_key", "base_value")]);
    let kv_data2 = create_key_value_data(vec![("test_key", "base_value")]);
    let kv_data3 = create_key_value_data(vec![("test_key", "different_value")]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::KeyValue(kv_data1)),
            ("run2", AperfData::KeyValue(kv_data2)),
            ("run3", AperfData::KeyValue(kv_data3)),
        ],
    );

    let rule = KeyValueKeyRunComparisonRule {
        rule_name: "test_rule",
        key: "test_key",
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
