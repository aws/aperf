//! Extract metadata from APerf report systeminfo.js files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use super::js_parser;

/// Metadata for a single run within a report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    pub run_id: String,
    pub alias: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// Top-level report metadata extracted from systeminfo.js.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_aliases: Option<serde_json::Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runs: Option<Vec<RunMetadata>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_readable: Option<String>,
}

/// Extract metadata from a systeminfo.js file.
pub fn extract_metadata(systeminfo_path: &Path, report_path: &str) -> Result<ReportMetadata> {
    let content = std::fs::read_to_string(systeminfo_path)
        .with_context(|| format!("Failed to read {}", systeminfo_path.display()))?;

    let json_str = js_parser::extract_json_from_js(&content)
        .with_context(|| "No JSON found in systeminfo.js")?;

    let data: Value =
        serde_json::from_str(&json_str).with_context(|| "Failed to parse systeminfo.js JSON")?;

    let mut metadata = ReportMetadata::default();

    if let Some(runs_obj) = data.get("runs").and_then(|v| v.as_object()) {
        let run_ids: Vec<String> = runs_obj.keys().cloned().collect();
        metadata.run_count = Some(run_ids.len());

        let mut run_aliases = serde_json::Map::new();
        let mut runs_meta = Vec::new();

        for (idx, run_id) in run_ids.iter().enumerate() {
            let alias = format!("run{}", idx + 1);
            run_aliases.insert(alias.clone(), Value::String(run_id.clone()));

            let mut run_meta = RunMetadata {
                run_id: run_id.clone(),
                alias,
                instance_type: None,
                cpu_count: None,
                os: None,
                kernel: None,
                region: None,
            };

            if let Some(run_data) = runs_obj.get(run_id) {
                extract_run_fields(run_data, &mut run_meta);
            }

            runs_meta.push(run_meta);
        }

        metadata.run_ids = Some(run_ids);
        metadata.run_aliases = Some(run_aliases);
        metadata.runs = Some(runs_meta);
    }

    // Extract timestamp from report path (pattern: YYYYMMDDTHHMMSS)
    extract_timestamp(report_path, &mut metadata);

    Ok(metadata)
}

fn extract_run_fields(run_data: &Value, run_meta: &mut RunMetadata) {
    let key_values = run_data
        .pointer("/key_value_groups//key_values")
        .and_then(|v| v.as_object());

    if let Some(kv) = key_values {
        run_meta.instance_type = kv
            .get("Instance Type")
            .and_then(|v| v.as_str())
            .map(String::from);
        run_meta.cpu_count = kv.get("CPUs").and_then(|v| v.as_str()).map(String::from);
        run_meta.kernel = kv
            .get("Kernel Version")
            .and_then(|v| v.as_str())
            .map(String::from);
        run_meta.region = kv.get("Region").and_then(|v| v.as_str()).map(String::from);

        let sys_name = kv.get("System Name").and_then(|v| v.as_str());
        let os_ver = kv.get("OS Version").and_then(|v| v.as_str());
        if let (Some(name), Some(ver)) = (sys_name, os_ver) {
            run_meta.os = Some(format!("{} {}", name, ver));
        }
    }
}

fn extract_timestamp(report_path: &str, metadata: &mut ReportMetadata) {
    let re = regex::Regex::new(r"(\d{8})T(\d{6})").unwrap();
    if let Some(caps) = re.captures(report_path) {
        let date_str = &caps[1];
        let time_str = &caps[2];
        let formatted_date = format!("{}-{}-{}", &date_str[..4], &date_str[4..6], &date_str[6..8]);
        let formatted_time = format!("{}:{}:{}", &time_str[..2], &time_str[2..4], &time_str[4..6]);
        metadata.timestamp = Some(format!("{} {}", formatted_date, formatted_time));

        // Human-readable date
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        if let Ok(month_num) = date_str[4..6].parse::<usize>() {
            if (1..=12).contains(&month_num) {
                if let Ok(day) = date_str[6..8].parse::<u32>() {
                    metadata.date_readable = Some(format!(
                        "{} {:02}, {}",
                        months[month_num - 1],
                        day,
                        &date_str[..4]
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/mcp_test_data/single_run_report/data/js")
    }

    #[test]
    fn test_extract_metadata_single_run() {
        let systeminfo_path = test_data_dir().join("systeminfo.js");
        if !systeminfo_path.exists() {
            return; // skip if test data not available
        }
        let metadata = extract_metadata(&systeminfo_path, "/some/path").unwrap();
        assert_eq!(metadata.run_count, Some(1));
        let runs = metadata.runs.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].instance_type.as_deref(), Some("m8g.24xlarge"));
        assert_eq!(runs[0].cpu_count.as_deref(), Some("96"));
    }

    #[test]
    fn test_extract_timestamp() {
        let mut metadata = ReportMetadata::default();
        extract_timestamp("/path/to/20240115T143022_report", &mut metadata);
        assert_eq!(metadata.timestamp.as_deref(), Some("2024-01-15 14:30:22"));
        assert_eq!(metadata.date_readable.as_deref(), Some("January 15, 2024"));
    }

    #[test]
    fn test_extract_timestamp_no_match() {
        let mut metadata = ReportMetadata::default();
        extract_timestamp("/path/to/some_report", &mut metadata);
        assert!(metadata.timestamp.is_none());
    }
}
