use aperf::analytics::profile_stack_frame_threshold_rule::ProfileStackFrameThresholdRule;
use aperf::analytics::{Analyze, DataFindings, Score};
use aperf::data::common::data_formats::{AperfData, Profiler, ProfilingData};
use aperf::data::common::processed_data_accessor::ProcessedDataAccessor;
use aperf::profiling::ThreadState;
use std::collections::HashMap;

use super::test_helpers::{create_processed_data, DataFindingsExt};

/// Stacks:
///   frame1;frame2 100
///   frame1;frame2;frame3 110
///   frame4;frame5;frame6 75
///   frame1;frame7 90
fn create_profiler_instance(group_name: &str) -> Profiler {
    let mut pd = Profiler::new(0);
    let ts = ThreadState::from_str("STATE_DEFAULT");
    pd.insert_stack(
        group_name,
        0,
        ts,
        &["frame1", "frame2"].map(String::from),
        100,
    );
    pd.insert_stack(
        group_name,
        0,
        ts,
        &["frame1", "frame2", "frame3"].map(String::from),
        110,
    );
    pd.insert_stack(
        group_name,
        0,
        ts,
        &["frame4", "frame5", "frame6"].map(String::from),
        75,
    );
    pd.insert_stack(
        group_name,
        0,
        ts,
        &["frame1", "frame7"].map(String::from),
        90,
    );
    pd
}

fn create_profiler_data(group_name: &str) -> ProfilingData {
    let mut profilers = HashMap::new();
    profilers.insert(
        "profile_0".to_string(),
        create_profiler_instance(group_name),
    );
    ProfilingData { profilers }
}

#[test]
fn test_below_threshold() {
    let profiling_data = create_profiler_data("cpu");
    let mut processed_data = create_processed_data(
        "test_data",
        vec![("run1", AperfData::Profile(profiling_data))],
    );

    let rule = ProfileStackFrameThresholdRule {
        rule_name: "test_rule",
        profile_type: "cpu",
        stack_frame: &[&["frame5"]],
        frame_type: None,
        thread_states: &[],
        aggregate_occurences: false,
        total_samples: true,
        threshold: 60.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &mut processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 0);
}

#[test]
fn test_above_threshold() {
    let profiling_data = create_profiler_data("cpu");
    let mut processed_data = create_processed_data(
        "test_data",
        vec![("run1", AperfData::Profile(profiling_data))],
    );

    let rule = ProfileStackFrameThresholdRule {
        rule_name: "test_rule",
        profile_type: "cpu",
        stack_frame: &[&["frame1"]],
        frame_type: None,
        thread_states: &[],
        aggregate_occurences: false,
        total_samples: true,
        threshold: 50.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &mut processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(findings.has_findings_for_run("run1"));
}

#[test]
fn test_stack_pattern() {
    let profiling_data = create_profiler_data("cpu");
    let mut processed_data = create_processed_data(
        "test_data",
        vec![("run1", AperfData::Profile(profiling_data))],
    );

    let rule = ProfileStackFrameThresholdRule {
        rule_name: "test_rule",
        profile_type: "cpu",
        stack_frame: &[&["frame1", "frame2", "frame3"]],
        frame_type: None,
        thread_states: &[],
        aggregate_occurences: false,
        total_samples: true,
        threshold: 25.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &mut processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 1);
    assert!(findings.has_findings_for_run("run1"));
}

#[test]
fn test_missing_metric() {
    let profiling_data = ProfilingData {
        profilers: HashMap::new(),
    };
    let mut processed_data = create_processed_data(
        "test_data",
        vec![("run1", AperfData::Profile(profiling_data))],
    );

    let rule = ProfileStackFrameThresholdRule {
        rule_name: "test_rule",
        profile_type: "alloc",
        stack_frame: &[&["frame1"]],
        frame_type: None,
        thread_states: &[],
        aggregate_occurences: false,
        total_samples: true,
        threshold: 10.0,
        score: Score::Bad.as_f64(),
        message: "Test message",
    };

    let mut findings = DataFindings::default();
    rule.analyze(
        &mut findings,
        &mut processed_data,
        &mut ProcessedDataAccessor::new(),
    );
    assert_eq!(findings.num_runs_with_findings(), 0);
}
