use aperf::analytics::profile_metadata_expected_rule::ProfileMetadataExpectedRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::data::common::data_formats::{
    AperfData, GraphData, KeyValueData, KeyValueGroup, ProfilerData,
};
use aperf::data::common::processed_data_accessor::ProcessedDataAccessor;
use std::collections::HashMap;

use super::test_helpers::{create_processed_data, DataFindingsExt};

fn create_key_value_data(group: &str, key: &str, value: Option<&str>) -> KeyValueData {
    let mut key_values = HashMap::new();
    if let Some(v) = value {
        key_values.insert(key.to_string(), v.to_string());
    }

    let mut key_value_groups = HashMap::new();
    key_value_groups.insert(group.to_string(), KeyValueGroup { key_values });

    KeyValueData { key_value_groups }
}

fn create_graph_data(metadata: Vec<KeyValueData>) -> GraphData {
    let mut map = HashMap::new();
    for (i, data) in metadata.into_iter().enumerate() {
        map.insert(
            format!("profile_{}", i),
            ProfilerData {
                start_time_ms: 0,
                block_width_ms: 0,
                metadata: data,
                profiles: HashMap::new(),
            },
        );
    }
    GraphData {
        graph_groups: vec![],
        profiler_data_map: map,
    }
}

#[test]
fn test_field_matches_expected_value() {
    let graph_data = create_graph_data(vec![create_key_value_data("cpu", "mode", Some("kernel"))]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::Graph(graph_data))]);

    let rule = ProfileMetadataExpectedRule {
        rule_name: "test_rule",
        group: "cpu",
        key: "mode",
        expected_value: "kernel",
        should_exist: true,
        score: Score::Good.as_f64(),
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
fn test_field_does_not_match_expected_value() {
    let graph_data = create_graph_data(vec![create_key_value_data("cpu", "mode", Some("user"))]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::Graph(graph_data))]);

    let rule = ProfileMetadataExpectedRule {
        rule_name: "test_rule",
        group: "cpu",
        key: "mode",
        expected_value: "kernel",
        should_exist: true,
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
fn test_field_missing_should_exist() {
    let graph_data = create_graph_data(vec![create_key_value_data("cpu", "mode", None)]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::Graph(graph_data))]);

    let rule = ProfileMetadataExpectedRule {
        rule_name: "test_rule",
        group: "cpu",
        key: "mode",
        expected_value: "kernel",
        should_exist: true,
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
fn test_regex_pattern_match() {
    let graph_data = create_graph_data(vec![create_key_value_data(
        "cpu",
        "mode",
        Some("kernel_mode"),
    )]);
    let processed_data =
        create_processed_data("test_data", vec![("run1", AperfData::Graph(graph_data))]);

    let rule = ProfileMetadataExpectedRule {
        rule_name: "test_rule",
        group: "cpu",
        key: "mode",
        expected_value: "kernel.*",
        should_exist: true,
        score: Score::Good.as_f64(),
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
    let graph_data1 = create_graph_data(vec![create_key_value_data("cpu", "mode", Some("kernel"))]);
    let graph_data2 = create_graph_data(vec![create_key_value_data("cpu", "mode", Some("user"))]);
    let processed_data = create_processed_data(
        "test_data",
        vec![
            ("run1", AperfData::Graph(graph_data1)),
            ("run2", AperfData::Graph(graph_data2)),
        ],
    );

    let rule = ProfileMetadataExpectedRule {
        rule_name: "test_rule",
        group: "cpu",
        key: "mode",
        expected_value: "kernel",
        should_exist: true,
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
