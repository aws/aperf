use aperf::analytics::key_value_key_expected_rule::KeyValueKeyExpectedRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::data::data_formats::AperfData;

use super::test_helpers::{create_key_value_data, create_processed_data, DataFindingsExt};

#[test]
fn test_key_matches_expected_value() {
    let kv_data = create_key_value_data(vec![("test_key", "expected_value")]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::KeyValue(kv_data))]);

    let rule = KeyValueKeyExpectedRule {
        rule_name: "test_rule",
        key: "test_key",
        expected_value: "expected_value",
        score: Score::Good.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 0);
}

#[test]
fn test_key_does_not_match_expected_value() {
    let kv_data = create_key_value_data(vec![("test_key", "wrong_value")]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::KeyValue(kv_data))]);

    let rule = KeyValueKeyExpectedRule {
        rule_name: "test_rule",
        key: "test_key",
        expected_value: "expected_value",
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(findings.has_findings_for_run("run1"));
}

#[test]
fn test_key_missing() {
    let kv_data = create_key_value_data(vec![("other_key", "value")]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::KeyValue(kv_data))]);

    let rule = KeyValueKeyExpectedRule {
        rule_name: "test_rule",
        key: "test_key",
        expected_value: "expected_value",
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(findings.has_findings_for_run("run1"));
}

#[test]
fn test_multiple_runs() {
    let kv_data1 = create_key_value_data(vec![("test_key", "expected_value")]);
    let kv_data2 = create_key_value_data(vec![("test_key", "wrong_value")]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::KeyValue(kv_data1)),
            ("run2", AperfData::KeyValue(kv_data2)),
        ],
    );

    let rule = KeyValueKeyExpectedRule {
        rule_name: "test_rule",
        key: "test_key",
        expected_value: "expected_value",
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
fn test_empty_key_value_data() {
    let kv_data = create_key_value_data(vec![]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::KeyValue(kv_data))]);

    let rule = KeyValueKeyExpectedRule {
        rule_name: "test_rule",
        key: "test_key",
        expected_value: "expected_value",
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(&mut findings, &processed_data);

    assert_eq!(findings.num_runs_with_findings(), 1);
}
