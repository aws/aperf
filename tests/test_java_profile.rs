use aperf::data::data_formats::AperfData;
use aperf::data::java_profile::JavaProfile;
use aperf::data::ProcessData;
use aperf::visualizer::ReportParams;
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
        data_dir: data_dir.clone(),
        tmp_dir: temp_dir.path().to_path_buf(),
        report_dir: report_dir.clone(),
        run_name: "test_run".to_string(),
        data_file_path: PathBuf::new(),
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
    let (_temp_dir, data_dir, _report_dir, params) = setup_test_env();

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
    let result = java_profile.process_raw_data(params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Graph(graph_data)) = result {
        assert_eq!(graph_data.graph_groups.len(), 4);
        for metric in &["cpu", "alloc", "wall", "legacy"] {
            let group = graph_data
                .graph_groups
                .iter()
                .find(|g| g.group_name == *metric)
                .unwrap();
            assert_eq!(group.graphs.len(), 2);
        }
    }
}

#[test]
fn test_process_raw_data_with_missing_jps_map() {
    let (_temp_dir, _data_dir, _report_dir, params) = setup_test_env();

    let mut java_profile = JavaProfile::new();
    let result = java_profile.process_raw_data(params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Graph(graph_data)) = result {
        for group in &graph_data.graph_groups {
            assert!(group.graphs.is_empty());
        }
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
    let result = java_profile.process_raw_data(params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Graph(graph_data)) = result {
        let cpu_group = graph_data
            .graph_groups
            .iter()
            .find(|g| g.group_name == "cpu")
            .unwrap();
        assert_eq!(cpu_group.graphs.len(), 3);

        let graph_names: Vec<String> = cpu_group.graphs.keys().cloned().collect();
        assert!(graph_names.iter().any(|name| name.contains("TestApp")
            && !name.contains("(1)")
            && !name.contains("(2)")));
        assert!(graph_names.iter().any(|name| name.contains("TestApp (1)")));
        assert!(graph_names.iter().any(|name| name.contains("TestApp (2)")));
    }
}

#[test]
fn test_process_raw_data_with_no_html_files() {
    let (_temp_dir, data_dir, _report_dir, params) = setup_test_env();

    let mut process_map = HashMap::new();
    process_map.insert("12345".to_string(), vec!["TestApp".to_string()]);

    create_jps_map(&data_dir, &params.run_name, process_map);

    let mut java_profile = JavaProfile::new();
    let result = java_profile.process_raw_data(params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Graph(graph_data)) = result {
        for group in &graph_data.graph_groups {
            assert!(group.graphs.is_empty());
        }
    }
}

#[test]
fn test_process_raw_data_with_complex_duplicate_names_and_missing_files() {
    let (_temp_dir, data_dir, _report_dir, params) = setup_test_env();

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
    let result = java_profile.process_raw_data(params, vec![]);

    assert!(result.is_ok());
    if let Ok(AperfData::Graph(graph_data)) = result {
        let cpu_group = graph_data
            .graph_groups
            .iter()
            .find(|g| g.group_name == "cpu")
            .unwrap();
        assert_eq!(cpu_group.graphs.len(), 7);
        let cpu_names: Vec<String> = cpu_group.graphs.keys().cloned().collect();
        assert!(cpu_names.iter().any(|name| name == "(cpu) JVM: App"));
        assert!(cpu_names.iter().any(|name| name == "(cpu) JVM: App (1)"));
        assert!(cpu_names.iter().any(|name| name == "(cpu) JVM: App (2)"));
        assert!(cpu_names.iter().any(|name| name == "(cpu) JVM: App (3)"));
        assert!(cpu_names.iter().any(|name| name == "(cpu) JVM: App (4)"));
        assert!(cpu_names.iter().any(|name| name == "(cpu) JVM: App (5)"));
        assert!(cpu_names.iter().any(|name| name == "(cpu) JVM: Service"));

        let alloc_group = graph_data
            .graph_groups
            .iter()
            .find(|g| g.group_name == "alloc")
            .unwrap();
        assert_eq!(alloc_group.graphs.len(), 5);
        let alloc_names: Vec<String> = alloc_group.graphs.keys().cloned().collect();
        assert_eq!(
            alloc_names
                .iter()
                .filter(|name| name.starts_with("(alloc) JVM: App"))
                .count(),
            3
        );
        assert_eq!(
            alloc_names
                .iter()
                .filter(|name| name.starts_with("(alloc) JVM: Service"))
                .count(),
            2
        );

        let wall_group = graph_data
            .graph_groups
            .iter()
            .find(|g| g.group_name == "wall")
            .unwrap();
        assert_eq!(wall_group.graphs.len(), 4);
        let wall_names: Vec<String> = wall_group.graphs.keys().cloned().collect();
        assert_eq!(
            wall_names
                .iter()
                .filter(|name| name.starts_with("(wall) JVM: App"))
                .count(),
            3
        );
        assert_eq!(
            wall_names
                .iter()
                .filter(|name| name.starts_with("(wall) JVM: Service"))
                .count(),
            1
        );

        let legacy_group = graph_data
            .graph_groups
            .iter()
            .find(|g| g.group_name == "legacy")
            .unwrap();
        assert_eq!(legacy_group.graphs.len(), 2);
        let legacy_names: Vec<String> = legacy_group.graphs.keys().cloned().collect();
        assert!(legacy_names
            .iter()
            .any(|name| name == "(legacy) JVM: Service"));
    }
}
