use aperf::data::common::data_formats::AperfData;
use aperf::data::java_profile::JavaProfile;
use aperf::data::ProcessData;
use aperf::data_processing::ReportParams;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, PathBuf, PathBuf, ReportParams) {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let report_dir = temp_dir.path().join("report");
    fs::create_dir_all(&report_dir).unwrap();
    fs::create_dir_all(report_dir.join("data/js")).unwrap();

    let params = ReportParams {
        run_data_dir: data_dir.clone(),
        tmp_dir: temp_dir.path().to_path_buf(),
        report_dir: report_dir.clone(),
        run_name: "test_run".to_string(),
        collection_start: None,
        pmu_counter_mode: String::new(),
        pid: None,
    };

    (temp_dir, data_dir, report_dir, params)
}

fn create_jps_map(data_dir: &PathBuf, run_name: &str, process_map: HashMap<String, Vec<String>>) {
    let jps_map_content = serde_json::to_string(&process_map).unwrap();
    fs::write(
        data_dir.join(format!("{}-jps-map.json", run_name)),
        jps_map_content,
    )
    .unwrap();
}

fn create_html_file(data_dir: &PathBuf, run_name: &str, pid: &str, metric: &str) {
    let filename = if metric == "legacy" {
        format!("{}-java-flamegraph-{}.html", run_name, pid)
    } else {
        format!("{}-java-profile-{}-{}.html", run_name, pid, metric)
    };
    fs::write(
        data_dir.join(&filename),
        format!("<html>Test {} profile for PID {}</html>", metric, pid),
    )
    .unwrap();
}

#[test]
fn test_process_raw_data_with_valid_files() {
    let (_temp_dir, data_dir, report_dir, params) = setup_test_env();

    let mut process_map = HashMap::new();
    process_map.insert("12345".to_string(), vec!["TestApp".to_string()]);
    process_map.insert("67890".to_string(), vec!["AnotherApp".to_string()]);

    create_jps_map(&data_dir, &params.run_name, process_map.clone());

    for metric in &["cpu", "alloc", "wall", "legacy"] {
        for (pid, _) in &process_map {
            create_html_file(&data_dir, &params.run_name, pid, metric);
        }
    }

    let mut java_profile = JavaProfile::new();
    let result = java_profile.process_raw_data(&params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Profile(profiling_data)) = result {
        // 2 JVMs from the jps map.
        assert_eq!(profiling_data.profilers.len(), 2);
        // No JFR profiler-data files were created in this test, but the processor
        // seeds an empty Profile for each metric in PROFILE_METRICS so the frontend
        // renders a tab for each of them.
        for (_name, profiler) in &profiling_data.profilers {
            assert_eq!(profiler.profiles.len(), 3);
            assert!(profiler.profiles.contains_key("cpu"));
            assert!(profiler.profiles.contains_key("wall"));
            assert!(profiler.profiles.contains_key("alloc"));
        }
    }

    // Verify the jfrconv-generated HTML files were copied to the report data dir.
    for metric in &["cpu", "alloc", "wall"] {
        for pid in &["12345", "67890"] {
            assert!(report_dir
                .join("data/js")
                .join(format!("test_run-java-profile-{}-{}.html", pid, metric))
                .exists());
        }
    }
    for pid in &["12345", "67890"] {
        assert!(report_dir
            .join("data/js")
            .join(format!("test_run-java-flamegraph-{}.html", pid))
            .exists());
    }
}

#[test]
fn test_process_raw_data_with_missing_jps_map() {
    let (_temp_dir, _data_dir, _report_dir, params) = setup_test_env();

    let mut java_profile = JavaProfile::new();
    let result = java_profile.process_raw_data(&params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Profile(profiling_data)) = result {
        assert!(profiling_data.profilers.is_empty());
    }
}

#[test]
fn test_process_raw_data_with_duplicate_jvm_names() {
    let (_temp_dir, data_dir, _report_dir, params) = setup_test_env();

    let mut process_map = HashMap::new();
    process_map.insert("12345".to_string(), vec!["TestApp".to_string()]);
    process_map.insert("67890".to_string(), vec!["TestApp".to_string()]);
    process_map.insert("11111".to_string(), vec!["TestApp".to_string()]);

    create_jps_map(&data_dir, &params.run_name, process_map.clone());

    for (pid, _) in &process_map {
        create_html_file(&data_dir, &params.run_name, pid, "cpu");
    }

    let mut java_profile = JavaProfile::new();
    let result = java_profile.process_raw_data(&params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Profile(profiling_data)) = result {
        // 3 deduped JVM entries
        assert_eq!(profiling_data.profilers.len(), 3);
        let names: Vec<String> = profiling_data.profilers.keys().cloned().collect();
        assert!(names.iter().any(|n| n == "TestApp"));
        assert!(names.iter().any(|n| n == "TestApp (1)"));
        assert!(names.iter().any(|n| n == "TestApp (2)"));
    }
}

#[test]
fn test_process_raw_data_with_no_html_files() {
    let (_temp_dir, data_dir, _report_dir, params) = setup_test_env();

    let mut process_map = HashMap::new();
    process_map.insert("12345".to_string(), vec!["TestApp".to_string()]);

    create_jps_map(&data_dir, &params.run_name, process_map);

    let mut java_profile = JavaProfile::new();
    let result = java_profile.process_raw_data(&params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Profile(profiling_data)) = result {
        // Even with no graphs and no profiler-data JSON, every JVM in the jps map
        // gets a default Profiler with one empty Profile per metric. Frontend
        // ProfilingDataPage will treat each as "no data".
        assert_eq!(profiling_data.profilers.len(), 1);
        let (_name, profiler) = profiling_data.profilers.iter().next().unwrap();
        assert_eq!(profiler.profiles.len(), 3);
        for metric in &["cpu", "wall", "alloc"] {
            let profile = profiler.profiles.get(*metric).unwrap();
            assert!(profile.blocks.is_empty());
        }
    }
}

#[test]
fn test_process_raw_data_with_complex_duplicate_names_and_missing_files() {
    let (_temp_dir, data_dir, report_dir, params) = setup_test_env();

    let mut process_map = HashMap::new();
    process_map.insert("1001".to_string(), vec!["App".to_string()]);
    process_map.insert("1002".to_string(), vec!["App".to_string()]);
    process_map.insert("1003".to_string(), vec!["App".to_string()]);
    process_map.insert("1004".to_string(), vec!["App".to_string()]);
    process_map.insert("1005".to_string(), vec!["App".to_string()]);
    process_map.insert("1006".to_string(), vec!["App".to_string()]);
    process_map.insert("2001".to_string(), vec!["Service".to_string()]);
    process_map.insert("2002".to_string(), vec!["Service".to_string()]);

    create_jps_map(&data_dir, &params.run_name, process_map);

    // Create files selectively - some metrics missing for some processes
    create_html_file(&data_dir, &params.run_name, "1001", "cpu");
    create_html_file(&data_dir, &params.run_name, "1002", "cpu");
    create_html_file(&data_dir, &params.run_name, "1003", "cpu");
    create_html_file(&data_dir, &params.run_name, "1004", "cpu");
    create_html_file(&data_dir, &params.run_name, "1005", "cpu");
    create_html_file(&data_dir, &params.run_name, "1006", "cpu");
    create_html_file(&data_dir, &params.run_name, "2002", "cpu");
    create_html_file(&data_dir, &params.run_name, "1001", "alloc");
    create_html_file(&data_dir, &params.run_name, "1004", "alloc");
    create_html_file(&data_dir, &params.run_name, "1005", "alloc");
    create_html_file(&data_dir, &params.run_name, "2001", "alloc");
    create_html_file(&data_dir, &params.run_name, "2002", "alloc");
    create_html_file(&data_dir, &params.run_name, "1003", "wall");
    create_html_file(&data_dir, &params.run_name, "1004", "wall");
    create_html_file(&data_dir, &params.run_name, "1006", "wall");
    create_html_file(&data_dir, &params.run_name, "2002", "wall");
    create_html_file(&data_dir, &params.run_name, "2001", "legacy");
    create_html_file(&data_dir, &params.run_name, "2002", "legacy");

    let mut java_profile = JavaProfile::new();
    let result = java_profile.process_raw_data(&params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Profile(profiling_data)) = result {
        // 8 JVMs total (6 App deduped + 2 Service deduped). Each PID had at least
        // one jfrconv HTML graph, so all of them are recorded.
        assert_eq!(profiling_data.profilers.len(), 8);

        // Verify deduped App names exist
        let names: Vec<String> = profiling_data.profilers.keys().cloned().collect();
        assert!(names.iter().any(|n| n == "App"));
        assert!(names.iter().any(|n| n == "App (1)"));
        assert!(names.iter().any(|n| n == "App (2)"));
        assert!(names.iter().any(|n| n == "App (3)"));
        assert!(names.iter().any(|n| n == "App (4)"));
        assert!(names.iter().any(|n| n == "App (5)"));
        assert!(names.iter().any(|n| n == "Service"));
    }

    // Verify the jfrconv-generated HTML files were copied to the report data dir.
    // Counts mirror the per-metric file creations above.
    let js_dir = report_dir.join("data/js");
    let cpu_files: Vec<_> = ["1001", "1002", "1003", "1004", "1005", "1006", "2002"]
        .iter()
        .filter(|pid| {
            js_dir
                .join(format!("test_run-java-profile-{}-cpu.html", pid))
                .exists()
        })
        .collect();
    assert_eq!(cpu_files.len(), 7);

    let alloc_files: Vec<_> = ["1001", "1004", "1005", "2001", "2002"]
        .iter()
        .filter(|pid| {
            js_dir
                .join(format!("test_run-java-profile-{}-alloc.html", pid))
                .exists()
        })
        .collect();
    assert_eq!(alloc_files.len(), 5);

    let wall_files: Vec<_> = ["1003", "1004", "1006", "2002"]
        .iter()
        .filter(|pid| {
            js_dir
                .join(format!("test_run-java-profile-{}-wall.html", pid))
                .exists()
        })
        .collect();
    assert_eq!(wall_files.len(), 4);

    let legacy_files: Vec<_> = ["2001", "2002"]
        .iter()
        .filter(|pid| {
            js_dir
                .join(format!("test_run-java-flamegraph-{}.html", pid))
                .exists()
        })
        .collect();
    assert_eq!(legacy_files.len(), 2);
}
