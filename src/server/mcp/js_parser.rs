//! Parse APerf JS report files.
//!
//! APerf report files are JavaScript files containing a single variable assignment:
//!   `processed_foo_data = { ... JSON ... }`
//! or findings:
//!   `foo_findings = { ... JSON ... }`
//!
//! This module strips the variable prefix and parses the JSON payload.

use crate::computations::Statistics;
use crate::data::common::data_formats::{
    AperfData, DataFormat, ProcessedData, Series, TimeSeriesData, TimeSeriesMetric,
};
use anyhow::{bail, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use flate2::read::ZlibDecoder;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;

/// Strip the JS variable assignment prefix and return the raw JSON string.
///
/// Handles patterns like:
///   `processed_cpu_utilization_data = {...};`
///   `cpu_utilization_findings = {...}`
///   `runs_raw = [...]`
pub fn extract_json_from_js(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find('=') {
            let after_eq = trimmed[idx + 1..].trim();
            let json_str = after_eq.trim_end_matches(';').trim();
            if json_str.starts_with('{') || json_str.starts_with('[') {
                return Some(json_str.to_string());
            }
        }
    }
    None
}

/// Extract all variable assignments from a JS file.
/// Returns Vec<(variable_name, json_string)>.
#[allow(dead_code)]
pub fn extract_all_variables(content: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find('=') {
            let var_name = trimmed[..idx].trim().to_string();
            let after_eq = trimmed[idx + 1..].trim();
            let json_str = after_eq.trim_end_matches(';').trim();
            if json_str.starts_with('{') || json_str.starts_with('[') {
                results.push((var_name, json_str.to_string()));
            }
        }
    }
    results
}

/// Find all `*_findings` variables in a JS file content.
/// Returns Vec<(category_name, json_string)> where category_name has `_findings` stripped.
pub fn extract_findings_variables(content: &str) -> Vec<(String, String)> {
    let re = Regex::new(r"(\w+_findings)\s*=\s*").unwrap();
    let mut results = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(caps) = re.captures(trimmed) {
            let var_name = caps.get(1).unwrap().as_str();
            let category = var_name.trim_end_matches("_findings").to_string();
            if let Some(idx) = trimmed.find('=') {
                let after_eq = trimmed[idx + 1..].trim();
                let json_str = after_eq.trim_end_matches(';').trim();
                if json_str.starts_with('{') {
                    results.push((category, json_str.to_string()));
                }
            }
        }
    }
    results
}

/// A processed time-series data as it is written into the report.
#[derive(Deserialize)]
struct ReportTimeSeriesData {
    data_name: String,
    data_format: DataFormat,
    runs: HashMap<String, ReportTimeSeriesRun>,
    #[serde(default)]
    run_errors: HashMap<String, String>,
}

#[derive(Deserialize)]
struct ReportTimeSeriesRun {
    time_diff: Vec<u64>,
    metrics: HashMap<String, ReportTimeSeriesMetric>,
    #[serde(default)]
    sorted_metric_names: Vec<String>,
}

#[derive(Deserialize)]
struct ReportTimeSeriesMetric {
    metric_name: String,
    series: Vec<String>,
    // Missing in reports from before it was added
    #[serde(default)]
    default_stats_series_idx: usize,
    values: Vec<String>,
    value_range: (u64, u64),
    stats: Statistics,
}

/// A processed time-series data in older reports, where every series has its own times.
#[derive(Deserialize)]
struct OldReportTimeSeriesData {
    data_name: String,
    data_format: DataFormat,
    runs: HashMap<String, OldReportTimeSeriesRun>,
    #[serde(default)]
    run_errors: HashMap<String, String>,
}

#[derive(Deserialize)]
struct OldReportTimeSeriesRun {
    metrics: HashMap<String, OldReportTimeSeriesMetric>,
    #[serde(default)]
    sorted_metric_names: Vec<String>,
}

#[derive(Deserialize)]
struct OldReportTimeSeriesMetric {
    metric_name: String,
    series: Vec<OldReportSeries>,
    value_range: (u64, u64),
    stats: Statistics,
}

#[derive(Deserialize)]
struct OldReportSeries {
    series_name: String,
    time_diff: Vec<u64>,
    values: Vec<f64>,
    #[serde(default)]
    is_aggregate: bool,
}

/// Parses the JSON of a processed data in the report into [`ProcessedData`]. Time-series data in
/// the report has the times of all series once per run, and the values of all series at these
/// times (NaN where a series has no value) in the base64 chunks of a zlib stream of little-endian
/// f64s, see `ProcessedDataAccessor::time_series_data_json_string`. Older reports, with the times
/// and values of every series, are also supported.
pub fn parse_processed_data(json: &str) -> Result<ProcessedData> {
    let report: ReportTimeSeriesData = match serde_json::from_str(json) {
        Ok(report) => report,
        // Not time-series data, or the time-series data of an older report
        Err(_) => return parse_old_processed_data(json),
    };
    let mut processed_data = ProcessedData::new(report.data_name);
    processed_data.data_format = report.data_format;
    processed_data.run_errors = report.run_errors;
    for (run_name, run) in report.runs {
        let num_times = run.time_diff.len();
        let mut time_series_data = TimeSeriesData {
            metrics: HashMap::new(),
            time_diff: run.time_diff,
            sorted_metric_names: run.sorted_metric_names,
        };
        for (metric_name, metric) in run.metrics {
            let values = decode_series_values(&metric.values)?;
            if values.len() != metric.series.len() * num_times {
                bail!(
                    "Metric {metric_name} of run {run_name} has {} values instead of {}",
                    values.len(),
                    metric.series.len() * num_times
                );
            }
            let mut time_series_metric = TimeSeriesMetric::new(metric.metric_name);
            time_series_metric.value_range = metric.value_range;
            time_series_metric.stats = metric.stats;
            time_series_metric.stats_series_idx = metric.default_stats_series_idx;
            for (i, series_name) in metric.series.into_iter().enumerate() {
                let mut series = Series::new(series_name);
                series.is_aggregate = i == metric.default_stats_series_idx;
                series.values = values[i * num_times..(i + 1) * num_times].to_vec();
                time_series_metric.series.push(series);
            }
            time_series_data
                .metrics
                .insert(metric_name, time_series_metric);
        }
        processed_data
            .runs
            .insert(run_name, AperfData::TimeSeries(time_series_data));
    }
    Ok(processed_data)
}

/// Parses the JSON of a processed data in older reports, whose time-series data has the times and
/// values of every series.
fn parse_old_processed_data(json: &str) -> Result<ProcessedData> {
    let Ok(report) = serde_json::from_str::<OldReportTimeSeriesData>(json) else {
        // Not time-series data
        return Ok(serde_json::from_str(json)?);
    };
    let mut processed_data = ProcessedData::new(report.data_name);
    processed_data.data_format = report.data_format;
    processed_data.run_errors = report.run_errors;
    for (run_name, run) in report.runs {
        let mut time_series_data = TimeSeriesData {
            sorted_metric_names: run.sorted_metric_names,
            ..Default::default()
        };
        for (metric_name, metric) in run.metrics {
            let mut time_series_metric = TimeSeriesMetric::new(metric.metric_name);
            time_series_metric.value_range = metric.value_range;
            time_series_metric.stats = metric.stats;
            time_series_data
                .metrics
                .insert(metric_name.clone(), time_series_metric);
            for series in metric.series {
                // Merging can move the values of the series already added to the data
                let values =
                    merge_time_diff(&mut time_series_data, &series.time_diff, &series.values);
                time_series_data
                    .metrics
                    .get_mut(&metric_name)
                    .unwrap()
                    .series
                    .push(Series {
                        series_name: series.series_name,
                        values,
                        is_aggregate: series.is_aggregate,
                    });
            }
        }
        processed_data
            .runs
            .insert(run_name, AperfData::TimeSeries(time_series_data));
    }
    Ok(processed_data)
}

/// Merges the times of a series into the times of the time-series data, and moves the values of
/// all series in the data to the merged times. Returns the values of the series at the merged times.
fn merge_time_diff(
    time_series_data: &mut TimeSeriesData,
    time_diff: &[u64],
    values: &[f64],
) -> Vec<f64> {
    let old_time_diff = std::mem::take(&mut time_series_data.time_diff);
    let merged_time_diff = &mut time_series_data.time_diff;
    // Where the old times and the times of the series are in the merged times
    let (mut old_indices, mut indices) = (Vec::new(), Vec::new());
    let (mut i, mut j) = (0, 0);
    while let Some(&time) = [old_time_diff.get(i), time_diff.get(j)]
        .into_iter()
        .flatten()
        .min()
    {
        if old_time_diff.get(i) == Some(&time) {
            old_indices.push(merged_time_diff.len());
            i += 1;
        }
        if time_diff.get(j) == Some(&time) {
            indices.push(merged_time_diff.len());
            j += 1;
        }
        merged_time_diff.push(time);
    }

    let num_times = merged_time_diff.len();
    let move_values = |values: &[f64], indices: &[usize]| {
        let mut moved_values = vec![f64::NAN; num_times];
        for (&index, &value) in indices.iter().zip(values) {
            moved_values[index] = value;
        }
        moved_values
    };
    if num_times > old_time_diff.len() {
        for series in time_series_data
            .metrics
            .values_mut()
            .flat_map(|metric| &mut metric.series)
        {
            series.values = move_values(&series.values, &old_indices);
        }
    }
    move_values(values, &indices)
}

/// Decodes the values of all series of a time-series metric in the report.
fn decode_series_values(chunks: &[String]) -> Result<Vec<f64>> {
    let compressed = BASE64.decode(chunks.concat())?;
    let mut bytes = Vec::new();
    ZlibDecoder::new(compressed.as_slice()).read_to_end(&mut bytes)?;
    Ok(bytes
        .chunks_exact(size_of::<f64>())
        .map(|value| f64::from_le_bytes(value.try_into().unwrap()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_processed_data() {
        let content = r#"processed_cpu_utilization_data = {"data_name":"cpu_utilization","data_format":"time_series"}"#;
        let json = extract_json_from_js(content).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.contains("cpu_utilization"));
    }

    #[test]
    fn test_extract_json_with_semicolon() {
        let content = r#"processed_foo_data = {"key":"value"};"#;
        let json = extract_json_from_js(content).unwrap();
        assert_eq!(json, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_extract_json_array() {
        let content = r#"runs_raw = ["run1","run2"]"#;
        let json = extract_json_from_js(content).unwrap();
        assert_eq!(json, r#"["run1","run2"]"#);
    }

    #[test]
    fn test_extract_all_variables() {
        let content = "processed_foo_data = {\"a\":1}\nfoo_findings = {\"b\":2}\n";
        let vars = extract_all_variables(content);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].0, "processed_foo_data");
        assert_eq!(vars[1].0, "foo_findings");
    }

    #[test]
    fn test_extract_findings_variables() {
        let content = "cpu_utilization_findings = {\"per_run_findings\":{}}\n";
        let findings = extract_findings_variables(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].0, "cpu_utilization");
    }

    #[test]
    fn test_no_json_returns_none() {
        let content = "// just a comment\n";
        assert!(extract_json_from_js(content).is_none());
    }

    #[test]
    fn test_parse_time_series_data() {
        use crate::data::common::processed_data_accessor::ProcessedDataAccessor;

        let make_series = |name: &str, values: Vec<f64>| Series {
            values,
            ..Series::new(name.to_string())
        };
        // s2 has no value at time 0, a non-finite value, and ends before the last time
        let mut time_series_data = TimeSeriesData {
            time_diff: vec![0, 1, 2],
            ..Default::default()
        };
        for (metric_name, series, stats_series_idx) in [
            (
                "a",
                vec![
                    make_series("s1", vec![1.0, 2.23456, 3.0]),
                    make_series("s2", vec![f64::NAN, f64::INFINITY]),
                ],
                0,
            ),
            (
                "b",
                vec![
                    make_series("s3", vec![4.0, 5.0, 6.0]),
                    make_series("s4", vec![1.0, 1.0, 1.0]),
                ],
                1,
            ),
        ] {
            let mut metric = TimeSeriesMetric::new(metric_name.to_string());
            metric.series = series;
            metric.stats_series_idx = stats_series_idx;
            time_series_data
                .metrics
                .insert(metric_name.to_string(), metric);
            time_series_data
                .sorted_metric_names
                .push(metric_name.to_string());
        }
        let mut processed_data = ProcessedData::new("test".to_string());
        processed_data.data_format = DataFormat::TimeSeries;
        processed_data
            .runs
            .insert("run1".to_string(), AperfData::TimeSeries(time_series_data));
        let json = ProcessedDataAccessor::new().json_string(&processed_data);

        let parsed = parse_processed_data(&json).unwrap();
        let AperfData::TimeSeries(run) = &parsed.runs["run1"] else {
            panic!("run1 is not time-series data");
        };
        assert_eq!(run.time_diff, vec![0, 1, 2]);
        assert_eq!(run.sorted_metric_names, vec!["a", "b"]);
        let a = &run.metrics["a"].series;
        assert_eq!(a[0].values, vec![1.0, 2.2346, 3.0]);
        assert!(a[1].values.iter().all(|value| value.is_nan()));
        let b = &run.metrics["b"];
        assert_eq!(b.series[0].values, vec![4.0, 5.0, 6.0]);
        assert_eq!(b.stats_series_idx, 1);
        assert!(!b.series[0].is_aggregate && b.series[1].is_aggregate);
    }

    #[test]
    fn test_parse_old_time_series_data() {
        // s1 has values at times 0 and 2, s2 at times 1 and 2
        let json = r#"{"data_name":"test","data_format":"time_series","runs":{"run1":{"metrics":{"m":{"metric_name":"m","series":[{"series_name":"s1","time_diff":[0,2],"values":[1.0,2.0]},{"series_name":"s2","time_diff":[1,2],"values":[3.0,4.0],"is_aggregate":true}],"value_range":[1,4],"stats":{"avg":2.5}}},"sorted_metric_names":["m"]}}}"#;
        let parsed = parse_processed_data(json).unwrap();
        let AperfData::TimeSeries(run) = &parsed.runs["run1"] else {
            panic!("run1 is not time-series data");
        };
        assert_eq!(run.time_diff, vec![0, 1, 2]);
        let series = &run.metrics["m"].series;
        assert_eq!(series[0].values[0], 1.0);
        assert!(series[0].values[1].is_nan());
        assert_eq!(series[0].values[2], 2.0);
        assert!(series[1].values[0].is_nan());
        assert_eq!(series[1].values[1..], [3.0, 4.0]);
        assert!(series[1].is_aggregate);
        assert_eq!(run.metrics["m"].stats.avg, 2.5);
    }
}
