//! Report loading and validation.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::js_parser;
use super::metadata::{self, ReportMetadata};

/// Information about a single metric within a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub metric_type: String,
}

/// Metrics extracted from a single JS file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetrics {
    pub filename: String,
    pub metrics: Vec<MetricInfo>,
}

/// The loaded report state.
#[derive(Debug, Clone)]
pub struct LoadedReport {
    pub report_path: String,
    pub data_dir: PathBuf,
    pub metadata: ReportMetadata,
    pub metrics: HashMap<String, FileMetrics>,
}

/// Validate that a path is a valid APerf report directory.
pub fn validate_report_path(report_path: &str) -> Result<PathBuf> {
    let path = if report_path.starts_with('~') {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(report_path.replacen('~', &home, 1))
    } else {
        PathBuf::from(report_path)
    };

    if !path.exists() {
        bail!("Report path does not exist: {}", report_path);
    }
    if !path.is_dir() {
        bail!("Report path is not a directory: {}", report_path);
    }

    let data_dir = path.join("data").join("js");
    if !data_dir.exists() {
        bail!(
            "Not a valid APerf report. Missing data/js directory at: {}",
            data_dir.display()
        );
    }
    if !data_dir.is_dir() {
        bail!(
            "data/js path exists but is not a directory: {}",
            data_dir.display()
        );
    }

    // Check required files
    let required = ["runs.js", "systeminfo.js"];
    for filename in &required {
        let file_path = data_dir.join(filename);
        if !file_path.exists() {
            bail!("Missing required file in report: {}", filename);
        }
    }

    // Check minimum file count
    let js_count = std::fs::read_dir(&data_dir)
        .with_context(|| format!("Cannot list files in {}", data_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new("js")))
        .count();

    if js_count < 3 {
        bail!(
            "Report appears incomplete. Found only {} JS files in data/js directory.",
            js_count
        );
    }

    Ok(data_dir)
}

/// Load a report: validate, extract metadata, optionally extract metrics.
pub fn load_report(report_path: &str, include_metrics: bool) -> Result<LoadedReport> {
    let data_dir = validate_report_path(report_path)?;

    let systeminfo_path = data_dir.join("systeminfo.js");
    let metadata = metadata::extract_metadata(&systeminfo_path, report_path)?;

    let metrics = if include_metrics {
        extract_all_metrics(&data_dir)?
    } else {
        HashMap::new()
    };

    Ok(LoadedReport {
        report_path: report_path.to_string(),
        data_dir,
        metadata,
        metrics,
    })
}

/// Extract metrics from all JS files in the data directory.
pub fn extract_all_metrics(data_dir: &Path) -> Result<HashMap<String, FileMetrics>> {
    let exclude = ["runs.js", "version.js", "systeminfo.js"];
    let mut metrics_by_file = HashMap::new();

    let mut entries: Vec<_> = std::fs::read_dir(data_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".js") && !exclude.contains(&name.as_str())
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();

        match extract_metrics_from_file(&path) {
            Ok(Some((data_name, file_metrics))) => {
                metrics_by_file.insert(data_name, file_metrics);
            }
            Ok(None) => {} // no metrics in this file
            Err(e) => {
                log::warn!("Error extracting metrics from {}: {}", filename, e);
            }
        }
    }

    Ok(metrics_by_file)
}

/// Extract metrics from a single JS file.
/// Returns Ok(Some((data_name, FileMetrics))) or Ok(None) if no metrics found.
fn extract_metrics_from_file(path: &Path) -> Result<Option<(String, FileMetrics)>> {
    let content = std::fs::read_to_string(path)?;
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Find the processed_*_data variable
    let json_str = match js_parser::extract_json_from_js(&content) {
        Some(s) => s,
        None => return Ok(None),
    };

    let data: Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return Ok(None), // gracefully skip unparseable files
    };

    let data_name = data
        .get("data_name")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("unknown")
        })
        .to_string();

    // Extract metric names from the first run
    let metrics = extract_metric_names_from_data(&data);
    if metrics.is_empty() {
        return Ok(None);
    }

    Ok(Some((data_name, FileMetrics { filename, metrics })))
}

/// Extract metric names and types from parsed data.
fn extract_metric_names_from_data(data: &Value) -> Vec<MetricInfo> {
    let mut metrics = Vec::new();

    let runs = match data.get("runs").and_then(|v| v.as_object()) {
        Some(r) => r,
        None => return metrics,
    };

    // Use first run to discover metric structure
    let first_run = match runs.values().next() {
        Some(r) => r,
        None => return metrics,
    };

    if let Some(metrics_obj) = first_run.get("metrics").and_then(|v| v.as_object()) {
        for (name, value) in metrics_obj {
            let metric_type = if value.get("series").is_some() {
                "time_series"
            } else if value.get("value").is_some() {
                "scalar"
            } else if value.is_object() {
                "object"
            } else if value.is_array() {
                "array"
            } else if value.is_number() {
                "number"
            } else if value.is_string() {
                "string"
            } else {
                "unknown"
            };

            metrics.push(MetricInfo {
                name: name.clone(),
                metric_type: metric_type.to_string(),
            });
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_data_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/mcp_test_data")
    }

    #[test]
    fn test_validate_report_path_valid() {
        let report = test_data_base().join("single_run_report");
        if !report.exists() {
            return;
        }
        let result = validate_report_path(report.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_report_path_missing() {
        let result = validate_report_path("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_report_no_metrics() {
        let report = test_data_base().join("single_run_report");
        if !report.exists() {
            return;
        }
        let loaded = load_report(report.to_str().unwrap(), false).unwrap();
        assert!(loaded.metrics.is_empty());
        assert!(loaded.metadata.run_count.is_some());
    }

    #[test]
    fn test_extract_metrics_from_cpu_file() {
        let path = test_data_base().join("metric_files/cpu_utilization.js");
        if !path.exists() {
            return;
        }
        let result = extract_metrics_from_file(&path).unwrap();
        assert!(result.is_some());
        let (name, file_metrics) = result.unwrap();
        assert_eq!(name, "cpu_utilization");
        assert!(!file_metrics.metrics.is_empty());
        // Should have user, system, softirq, nice etc.
        let metric_names: Vec<&str> = file_metrics
            .metrics
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert!(metric_names.contains(&"user"));
    }
}
