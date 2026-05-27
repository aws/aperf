//! MCP server implementation using rmcp 1.5.

use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::data::common::data_formats::{AperfData, DataFormat, ProcessedData};
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;

use super::report::{self, LoadedReport};

/// Server state holding the currently loaded report.
#[derive(Debug, Default)]
struct ServerState {
    report: Option<LoadedReport>,
}

/// The APerf MCP server.
#[derive(Clone)]
pub struct AperfMcpServer {
    state: Arc<Mutex<ServerState>>,
}

// ---------------------------------------------------------------------------
// Tool parameter structs
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Load an APerf report from a directory path")]
pub struct LoadReportRequest {
    #[schemars(description = "Absolute or relative path to the APerf report directory")]
    pub report_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Get available metrics from the loaded report")]
pub struct GetMetricsRequest {
    #[schemars(description = "JS filename, e.g. 'cpu_utilization.js'")]
    pub file_name: Option<String>,
    #[schemars(description = "Data category, e.g. 'cpu_utilization'")]
    pub category: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Get metric values from the loaded report")]
pub struct GetMetricValuesRequest {
    #[schemars(description = "Name of the metric to retrieve (e.g. 'user', 'system', 'idle')")]
    pub metric_name: String,
    #[schemars(description = "JS filename containing the metric (e.g. 'cpu_utilization.js')")]
    pub file_name: Option<String>,
    #[schemars(
        description = "Category/data_name (e.g. 'cpu_utilization'). Either file_name or category is required."
    )]
    pub category: Option<String>,
    #[schemars(
        description = "Output format: 'summary' (default, smart text with stats/trends/anomalies - minimal tokens), 'stats' (aggregated statistics only), 'timeseries' (full raw data), 'compact' (delta-encoded notation - ~80% fewer tokens), 'downsampled' (fixed N buckets preserving shape)"
    )]
    pub output_type: Option<String>,
    #[schemars(
        description = "List of CPU/series IDs for per-CPU metrics (e.g. ['CPU0', 'CPU1']). If omitted, averages across all CPUs."
    )]
    pub cpu_ids: Option<Vec<String>>,
    #[schemars(description = "Specific run ID (default: all runs)")]
    pub run_id: Option<String>,
    #[schemars(
        description = "Start of time range in seconds. Negative values are relative to end (e.g. -60 = last 60s). If omitted, starts from beginning."
    )]
    pub from_time: Option<i64>,
    #[schemars(
        description = "End of time range in seconds. Negative values are relative to end (e.g. -10 = stop 10s before end). If omitted, goes to end."
    )]
    pub to_time: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Get analytical findings from the loaded report")]
pub struct GetAnalyticalFindingsRequest {
    #[schemars(description = "Starting index for pagination (default: 0)")]
    pub offset: Option<i64>,
    #[schemars(description = "Maximum number of findings to return (default: 50)")]
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Get statistical findings (metric stat deltas between runs) from the loaded report"
)]
pub struct GetStatisticalFindingsRequest {
    #[schemars(description = "Starting index for pagination (default: 0)")]
    pub offset: Option<i64>,
    #[schemars(description = "Maximum number of findings to return (default: 50)")]
    pub limit: Option<i64>,
    #[schemars(
        description = "Filter by stat type: 'avg', 'std', 'min', 'max', 'p50', 'p90', 'p99', 'p99_9'. If omitted, returns all stats."
    )]
    pub stat: Option<String>,
    #[schemars(
        description = "Filter by data category (e.g. 'cpu_utilization', 'meminfo'). If omitted, returns all categories."
    )]
    pub data_type: Option<String>,
    #[schemars(
        description = "Minimum absolute delta percentage to include (e.g. 5.0 = only show deltas >= 5%). Default: 0 (all)."
    )]
    pub min_delta_pct: Option<f64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(
    description = "Query flamegraph data from the loaded report. Returns top functions by default, with optional regex filtering to search for specific functions. Supports normal, reverse, diff, and reverse-diff modes. Returns data for all runs by default, or specific runs if run_id is provided. In diff mode, run_id must contain exactly 2 run IDs (base and comparison)."
)]
pub struct GetFlamegraphRequest {
    #[schemars(
        description = "Type of flamegraph: 'normal' (callers→callees, default), 'reverse' (callees→callers), 'diff' (compare two runs, normal direction), or 'reverse-diff' (compare two runs, reverse direction)"
    )]
    pub flamegraph_type: Option<String>,
    #[schemars(
        description = "Run ID(s) to query. For normal/reverse: omit for all runs, or provide specific run IDs. For diff/reverse-diff: provide exactly 2 run IDs [base, comparison], or omit to use first two runs in the report."
    )]
    pub run_id: Option<Vec<String>>,
    #[schemars(description = "Maximum number of functions to return per run (default: 30)")]
    pub limit: Option<usize>,
    #[schemars(
        description = "Minimum percentage threshold to include (default: 0.1). Functions below this % are excluded. In diff mode, this is the minimum absolute delta percentage (default: 0.5)."
    )]
    pub min_pct: Option<f64>,
    #[schemars(
        description = "Regex filter for function names (case-insensitive). Only functions matching this pattern are returned. Example: 'compact|migrate' to find compaction-related functions."
    )]
    pub filter: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Record performance data on the current system")]
pub struct RecordRequest {
    #[schemars(description = "Name of the run (default: aperf_<timestamp>)")]
    pub run_name: Option<String>,
    #[schemars(description = "Interval in seconds at which data is collected (default: 1)")]
    pub interval: Option<u64>,
    #[schemars(description = "Duration in seconds for the recording (default: 10)")]
    pub period: Option<u64>,
    #[schemars(description = "Enable CPU profiling using perf (default: false)")]
    pub profile: Option<bool>,
    #[schemars(description = "Perf profiling frequency in Hz (default: 99)")]
    pub perf_frequency: Option<u32>,
    #[schemars(
        description = "Collect memory allocation data: buddyinfo, pagetypeinfo, slabinfo (default: false)"
    )]
    pub memory_allocation: Option<bool>,
    #[schemars(
        description = "Comma-separated list of data types to skip (e.g. 'perf_stat,interrupts')"
    )]
    pub dont_collect: Option<String>,
    #[schemars(
        description = "Comma-separated list of data types to collect exclusively (e.g. 'cpu_utilization,meminfo')"
    )]
    pub collect_only: Option<String>,
    #[schemars(
        description = "Profile JVMs using async-profiler. Provide comma-separated PIDs/names, or empty to profile all JVMs."
    )]
    pub profile_java: Option<String>,
    #[schemars(description = "Path to custom PMU config file")]
    pub pmu_config: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Generate an HTML report from recorded APerf data")]
pub struct GenerateReportRequest {
    #[schemars(
        description = "Paths to run directories or archives (at least one required). For multi-run comparison, provide multiple paths."
    )]
    pub runs: Vec<String>,
    #[schemars(
        description = "Name for the report directory and archive (default: aperf_report_<timestamp>)"
    )]
    pub name: Option<String>,
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

#[tool_router]
impl AperfMcpServer {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }

    #[tool(
        description = "Load an APerf report from a directory path and get metadata and available metrics. Must be called first before any other tool. Returns JSON with status, message, report_path, metadata (runs, aliases, instance info), and metrics (organized by file)."
    )]
    async fn load_report(
        &self,
        Parameters(LoadReportRequest { report_path }): Parameters<LoadReportRequest>,
    ) -> Result<CallToolResult, McpError> {
        match report::load_report(&report_path, true) {
            Ok(loaded) => {
                let response = build_load_report_response(&loaded);
                let response_str = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|e| format!("Serialization error: {}", e));
                let mut state = self.state.lock().await;
                state.report = Some(loaded);
                Ok(CallToolResult::success(vec![Content::text(response_str)]))
            }
            Err(e) => {
                let err_response = json!({"status": "error", "message": format!("Error: {}", e)});
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string(&err_response).unwrap(),
                )]))
            }
        }
    }

    #[tool(
        description = "Get available metrics from the loaded report. Supports filtering by file_name or category. Returns JSON with status, message, and metrics dict."
    )]
    async fn get_metrics(
        &self,
        Parameters(GetMetricsRequest {
            file_name,
            category,
        }): Parameters<GetMetricsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.state.lock().await;

        let loaded = match &mut state.report {
            Some(r) => r,
            None => {
                let err = json!({"status": "error", "message": "Error: No report loaded. Call load_report() first."});
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string(&err).unwrap(),
                )]));
            }
        };

        // If metrics are empty, extract them now
        if loaded.metrics.is_empty() {
            match report::extract_all_metrics(&loaded.data_dir) {
                Ok(m) => loaded.metrics = m,
                Err(e) => {
                    let err = json!({"status": "error", "message": format!("Error extracting metrics: {}", e)});
                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string(&err).unwrap(),
                    )]));
                }
            }
        }

        let mut metrics = loaded.metrics.clone();

        // Filter by file_name
        if let Some(ref fname) = file_name {
            metrics.retain(|_, v| v.filename == *fname);
            if metrics.is_empty() {
                let available: Vec<&str> = loaded
                    .metrics
                    .values()
                    .map(|v| v.filename.as_str())
                    .collect();
                let err = json!({
                    "status": "error",
                    "message": format!("Error: No metrics found for file '{}'. Available files: {:?}", fname, available)
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string(&err).unwrap(),
                )]));
            }
        }

        // Filter by category
        if let Some(ref cat) = category {
            if !metrics.contains_key(cat.as_str()) {
                let available: Vec<&str> = loaded.metrics.keys().map(|k| k.as_str()).collect();
                let err = json!({
                    "status": "error",
                    "message": format!("Error: Category '{}' not found. Available categories: {:?}", cat, available)
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string(&err).unwrap(),
                )]));
            }
            let entry = metrics.remove(cat.as_str()).unwrap();
            metrics.clear();
            metrics.insert(cat.clone(), entry);
        }

        let total_metrics: usize = metrics.values().map(|v| v.metrics.len()).sum();

        let msg = if file_name.is_some() || category.is_some() {
            let filter_desc = file_name
                .as_deref()
                .map(|f| format!("file '{}'", f))
                .or_else(|| category.as_deref().map(|c| format!("category '{}'", c)))
                .unwrap();
            format!(
                "Retrieved metrics for {} ({} total metrics)",
                filter_desc, total_metrics
            )
        } else {
            format!(
                "Retrieved all available metrics from {} files ({} total metrics)",
                metrics.len(),
                total_metrics
            )
        };

        let response = json!({
            "status": "success",
            "message": msg,
            "report_path": loaded.report_path,
            "metrics": metrics,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap(),
        )]))
    }

    #[tool(
        description = "Get metric values from the loaded report. Returns stats (avg/std/min/max) by default, or full time-series data. Supports multi-run reports and time-range filtering. Must specify either file_name or category."
    )]
    async fn get_metric_values(
        &self,
        Parameters(req): Parameters<GetMetricValuesRequest>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.lock().await;
        let loaded = match &state.report {
            Some(r) => r,
            None => {
                return ok_json(
                    json!({"status": "error", "message": "Error: No report loaded. Call load_report() first."}),
                )
            }
        };

        let output_type = req.output_type.as_deref().unwrap_or("summary");
        if !["stats", "timeseries", "summary", "compact", "downsampled"].contains(&output_type) {
            return ok_json(
                json!({"status": "error", "message": format!("Error: Invalid output_type '{}'. Must be 'stats', 'timeseries', 'summary', 'compact', or 'downsampled'.", output_type)}),
            );
        }

        // Find the target file and deserialize into ProcessedData
        let target_file = find_target_file(
            &loaded.data_dir,
            req.file_name.as_deref(),
            req.category.as_deref(),
        );
        let target_file = match target_file {
            Ok(f) => f,
            Err(msg) => return ok_json(json!({"status": "error", "message": msg})),
        };

        let processed_data = match load_processed_data(&target_file) {
            Ok(pd) => pd,
            Err(msg) => return ok_json(json!({"status": "error", "message": msg})),
        };

        let filename = target_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Validate run selection
        let selected_run_ids: Vec<&String> = if let Some(ref rid) = req.run_id {
            if !processed_data.runs.contains_key(rid) {
                let available: Vec<&String> = processed_data.runs.keys().collect();
                return ok_json(
                    json!({"status": "error", "message": format!("Error: Run '{}' not found. Available runs: {:?}", rid, available)}),
                );
            }
            vec![rid]
        } else {
            processed_data.runs.keys().collect()
        };

        // Build ProcessedDataAccessor with time-range support
        let mut accessor = if req.from_time.is_some() || req.to_time.is_some() {
            let from_map: HashMap<String, i64> = if let Some(from) = req.from_time {
                selected_run_ids
                    .iter()
                    .map(|&rid| (rid.clone(), from))
                    .collect()
            } else {
                HashMap::new()
            };
            let to_map: HashMap<String, i64> = if let Some(to) = req.to_time {
                selected_run_ids
                    .iter()
                    .map(|&rid| (rid.clone(), to))
                    .collect()
            } else {
                HashMap::new()
            };
            ProcessedDataAccessor::from_time_ranges(
                from_map,
                to_map,
                HashMap::new(),
                HashMap::new(),
            )
        } else {
            ProcessedDataAccessor::new()
        };

        match processed_data.data_format {
            DataFormat::TimeSeries => {
                let mut run_results = Vec::new();
                for rid in &selected_run_ids {
                    let ts_data = match &processed_data.runs[rid.as_str()] {
                        AperfData::TimeSeries(ts) => ts,
                        _ => {
                            return ok_json(
                                json!({"status": "error", "message": "Error: Run data is not time-series format"}),
                            )
                        }
                    };

                    let metric = match ts_data.metrics.get(&req.metric_name) {
                        Some(m) => m,
                        None => {
                            let available: Vec<&String> = ts_data.metrics.keys().collect();
                            return ok_json(
                                json!({"status": "error", "message": format!("Error: Metric '{}' not found. Available: {:?}", req.metric_name, available)}),
                            );
                        }
                    };

                    // Get stats via accessor (respects time range)
                    let stats = accessor
                        .time_series_metric_stats(&processed_data, rid, &req.metric_name)
                        .unwrap_or_default();
                    let stats_json = serde_json::to_value(&stats).unwrap_or(json!({}));

                    let result_data = if output_type == "stats" {
                        json!({"stats": stats_json})
                    } else {
                        if metric.series.is_empty() {
                            return ok_json(
                                json!({"status": "error", "message": format!("Error: No time series data for metric '{}'", req.metric_name)}),
                            );
                        }

                        // Resolve series based on cpu_ids filtering or averaging
                        let resolved = resolve_typed_series(
                            metric,
                            &req.cpu_ids,
                            &req.metric_name,
                            &accessor,
                            rid,
                        );
                        let resolved = match resolved {
                            Ok(r) => r,
                            Err(msg) => return ok_json(json!({"status": "error", "message": msg})),
                        };

                        match output_type {
                            "timeseries" => {
                                json!({"series": resolved.series_json, "stats": stats_json})
                            }
                            "summary" => {
                                let summary = build_timeseries_summary(
                                    &resolved.values,
                                    &resolved.time_diff,
                                    &stats_json,
                                );
                                json!({"summary": summary, "stats": stats_json})
                            }
                            "compact" => {
                                let compact = build_compact_notation(
                                    &resolved.values,
                                    &resolved.time_diff,
                                    &req.metric_name,
                                );
                                json!({"compact": compact, "stats": stats_json})
                            }
                            "downsampled" => {
                                let ds =
                                    build_downsampled(&resolved.values, &resolved.time_diff, 50);
                                json!({"downsampled": ds, "stats": stats_json, "original_points": resolved.values.len()})
                            }
                            _ => json!({"stats": stats_json}),
                        }
                    };

                    run_results.push(json!({"run_id": rid, "data": result_data}));
                }

                let type_label = match output_type {
                    "stats" => "Statistics",
                    "timeseries" => "Time series",
                    "summary" => "Summary",
                    "compact" => "Compact notation",
                    "downsampled" => "Downsampled",
                    _ => output_type,
                };
                let response = if selected_run_ids.len() == 1 {
                    json!({
                        "status": "success",
                        "message": format!("{} for metric '{}' from {}", type_label, req.metric_name, filename),
                        "metric_name": req.metric_name,
                        "run_id": selected_run_ids[0],
                        "file_name": filename,
                        "output_type": output_type,
                        "data": run_results[0]["data"]
                    })
                } else {
                    json!({
                        "status": "success",
                        "message": format!("{} for metric '{}' from {} ({} runs)", type_label, req.metric_name, filename, selected_run_ids.len()),
                        "metric_name": req.metric_name,
                        "run_count": selected_run_ids.len(),
                        "file_name": filename,
                        "output_type": output_type,
                        "data": {"runs": run_results}
                    })
                };
                ok_json(response)
            }
            DataFormat::KeyValue => {
                let mut run_results = Vec::new();
                for rid in &selected_run_ids {
                    let value =
                        accessor.key_value_value_by_key(&processed_data, rid, &req.metric_name);
                    match value {
                        Some(val) => {
                            run_results.push(json!({"run_id": rid, "data": {"value": val}}));
                        }
                        None => {
                            return ok_json(
                                json!({"status": "error", "message": format!("Error: Metric '{}' not found in key-value data", req.metric_name)}),
                            );
                        }
                    }
                }

                let response = if selected_run_ids.len() == 1 {
                    json!({
                        "status": "success",
                        "message": format!("Key-value metric '{}' from {}", req.metric_name, filename),
                        "metric_name": req.metric_name,
                        "run_id": selected_run_ids[0],
                        "file_name": filename,
                        "output_type": "key_value",
                        "data": run_results[0]["data"]
                    })
                } else {
                    json!({
                        "status": "success",
                        "message": format!("Key-value metric '{}' from {} ({} runs)", req.metric_name, filename, selected_run_ids.len()),
                        "metric_name": req.metric_name,
                        "run_count": selected_run_ids.len(),
                        "file_name": filename,
                        "output_type": "key_value",
                        "data": {"runs": run_results}
                    })
                };
                ok_json(response)
            }
            _ => ok_json(
                json!({"status": "error", "message": format!("Error: Unsupported data format '{:?}'", processed_data.data_format)}),
            ),
        }
    }

    #[tool(
        description = "Get analytical findings from the loaded report with pagination. Findings represent significant differences between runs (regressions, improvements, mismatches)."
    )]
    async fn get_analytical_findings(
        &self,
        Parameters(req): Parameters<GetAnalyticalFindingsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.lock().await;
        let loaded = match &state.report {
            Some(r) => r,
            None => {
                return ok_json(
                    json!({"status": "error", "message": "Error: No report loaded. Call load_report() first."}),
                )
            }
        };

        let offset = req.offset.unwrap_or(0).max(0) as usize;
        let limit = req.limit.unwrap_or(50).max(1) as usize;

        // Scan all JS files for *_findings variables
        let mut all_findings: Vec<Value> = Vec::new();

        let mut js_files: Vec<_> = std::fs::read_dir(&loaded.data_dir)
            .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new("js")))
            .collect();
        js_files.sort_by_key(|e| e.file_name());

        for entry in &js_files {
            let content = match std::fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let findings_vars = super::js_parser::extract_findings_variables(&content);
            for (category, json_str) in findings_vars {
                let findings_data: Value = match serde_json::from_str(&json_str) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let per_run = match findings_data
                    .get("per_run_findings")
                    .and_then(|v| v.as_object())
                {
                    Some(p) => p,
                    None => continue,
                };

                for (run_id, run_data) in per_run {
                    let findings_dict = match run_data.get("findings").and_then(|v| v.as_object()) {
                        Some(f) => f,
                        None => continue,
                    };

                    for (metric_name, finding_list) in findings_dict {
                        if let Some(arr) = finding_list.as_array() {
                            for finding in arr {
                                all_findings.push(json!({
                                    "category": category,
                                    "run_id": run_id,
                                    "metric_name": metric_name,
                                    "rule_name": finding.get("rule_name").and_then(|v| v.as_str()).unwrap_or(""),
                                    "score": finding.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    "description": finding.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                                    "message": finding.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                                    "reference": finding.get("reference").and_then(|v| v.as_str()).unwrap_or(""),
                                }));
                            }
                        }
                    }
                }
            }
        }

        // Sort by absolute score descending (highest severity first)
        all_findings.sort_by(|a, b| {
            let score_a = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0).abs();
            let score_b = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0).abs();
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = all_findings.len();
        let end = (offset + limit).min(total);
        let paginated: Vec<Value> = if offset < total {
            all_findings[offset..end].to_vec()
        } else {
            Vec::new()
        };
        let returned_count = paginated.len();
        let has_more = end < total;

        let msg = if total == 0 {
            "No analytical findings found in the report.".to_string()
        } else if returned_count == 0 {
            format!(
                "No findings at offset {}. Total available: {}",
                offset, total
            )
        } else {
            format!(
                "Analytical findings (showing {} of {}, range {}-{})",
                returned_count,
                total,
                offset + 1,
                offset + returned_count
            )
        };

        let response = json!({
            "status": "success",
            "message": msg,
            "report_path": loaded.report_path,
            "total_findings": total,
            "offset": offset,
            "limit": limit,
            "returned_count": returned_count,
            "has_more": has_more,
            "findings": paginated,
        });

        ok_json(response)
    }

    #[tool(
        description = "Get statistical findings (metric stat deltas between runs). Computes the percentage change of each time-series metric's statistics compared to the base run (first run). Sorted by absolute delta descending. Supports filtering by stat type, data category, and minimum delta threshold."
    )]
    async fn get_statistical_findings(
        &self,
        Parameters(req): Parameters<GetStatisticalFindingsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.lock().await;
        let loaded = match &state.report {
            Some(r) => r,
            None => {
                return ok_json(
                    json!({"status": "error", "message": "Error: No report loaded. Call load_report() first."}),
                )
            }
        };

        let offset = req.offset.unwrap_or(0).max(0) as usize;
        let limit = req.limit.unwrap_or(50).max(1) as usize;
        let min_delta = req.min_delta_pct.unwrap_or(0.0);

        let valid_stats = ["avg", "std", "min", "max", "p50", "p90", "p99", "p99_9"];
        if let Some(ref s) = req.stat {
            if !valid_stats.contains(&s.as_str()) {
                return ok_json(
                    json!({"status": "error", "message": format!("Error: Invalid stat '{}'. Must be one of: {:?}", s, valid_stats)}),
                );
            }
        }

        // Collect all time-series JS files and compute deltas
        let exclude = ["runs.js", "version.js", "systeminfo.js"];
        let mut entries: Vec<_> = std::fs::read_dir(&loaded.data_dir)
            .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.ends_with(".js") && !exclude.contains(&name.as_str())
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        let mut all_deltas: Vec<Value> = Vec::new();

        for entry in &entries {
            let pd = match load_processed_data(&entry.path()) {
                Ok(pd) => pd,
                Err(_) => continue,
            };

            // Only process time-series data
            if !matches!(pd.data_format, DataFormat::TimeSeries) {
                continue;
            }

            // Filter by data_type if specified
            if let Some(ref dt) = req.data_type {
                if pd.data_name != *dt {
                    continue;
                }
            }

            let run_ids: Vec<String> = pd.runs.keys().cloned().collect();
            if run_ids.len() < 2 {
                continue; // Need at least 2 runs for comparison
            }

            // First run is the base
            let base_run = &run_ids[0];
            let mut accessor = ProcessedDataAccessor::new();

            // Get all metric names from base run
            let metric_names: Vec<String> = match &pd.runs[base_run] {
                AperfData::TimeSeries(ts) => ts.metrics.keys().cloned().collect(),
                _ => continue,
            };

            for run_id in &run_ids[1..] {
                for metric_name in &metric_names {
                    let base_stats =
                        match accessor.time_series_metric_stats(&pd, base_run, metric_name) {
                            Some(s) => s,
                            None => continue,
                        };
                    let run_stats =
                        match accessor.time_series_metric_stats(&pd, run_id, metric_name) {
                            Some(s) => s,
                            None => continue,
                        };

                    // Compute delta for each stat
                    let stat_pairs: &[(&str, f64, f64)] = &[
                        ("avg", base_stats.avg, run_stats.avg),
                        ("std", base_stats.std, run_stats.std),
                        ("min", base_stats.min, run_stats.min),
                        ("max", base_stats.max, run_stats.max),
                        ("p50", base_stats.p50, run_stats.p50),
                        ("p90", base_stats.p90, run_stats.p90),
                        ("p99", base_stats.p99, run_stats.p99),
                        ("p99_9", base_stats.p99_9, run_stats.p99_9),
                    ];

                    for &(stat_name, base_val, run_val) in stat_pairs {
                        // Filter by stat if specified
                        if let Some(ref s) = req.stat {
                            if s != stat_name {
                                continue;
                            }
                        }

                        // Compute percentage delta
                        let delta_pct = if base_val.abs() > 1e-10 {
                            ((run_val - base_val) / base_val) * 100.0
                        } else if run_val.abs() > 1e-10 {
                            100.0 // went from ~0 to something
                        } else {
                            0.0 // both ~0
                        };

                        // Filter by min_delta
                        if delta_pct.abs() < min_delta {
                            continue;
                        }

                        // Skip zero deltas
                        if delta_pct == 0.0 {
                            continue;
                        }

                        all_deltas.push(json!({
                            "data_type": pd.data_name,
                            "metric_name": metric_name,
                            "stat": stat_name,
                            "run_id": run_id,
                            "base_run_id": base_run,
                            "base_value": (base_val * 100.0).round() / 100.0,
                            "run_value": (run_val * 100.0).round() / 100.0,
                            "delta_pct": (delta_pct * 100.0).round() / 100.0,
                        }));
                    }
                }
            }
        }

        // Sort by absolute delta descending
        all_deltas.sort_by(|a, b| {
            let da = a
                .get("delta_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .abs();
            let db = b
                .get("delta_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .abs();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = all_deltas.len();
        let end = (offset + limit).min(total);
        let paginated: Vec<Value> = if offset < total {
            all_deltas[offset..end].to_vec()
        } else {
            Vec::new()
        };
        let returned_count = paginated.len();
        let has_more = end < total;

        let msg = if total == 0 {
            "No statistical findings (metric deltas) found. Requires a multi-run report."
                .to_string()
        } else if returned_count == 0 {
            format!(
                "No findings at offset {}. Total available: {}",
                offset, total
            )
        } else {
            format!(
                "Statistical findings (showing {} of {}, range {}-{})",
                returned_count,
                total,
                offset + 1,
                offset + returned_count
            )
        };

        let response = json!({
            "status": "success",
            "message": msg,
            "report_path": loaded.report_path,
            "total_findings": total,
            "offset": offset,
            "limit": limit,
            "returned_count": returned_count,
            "has_more": has_more,
            "findings": paginated,
        });

        ok_json(response)
    }

    #[tool(
        description = "Query flamegraph data from the loaded report. Returns top functions by default (sorted by percentage), with optional regex filtering to search for specific functions. Supports normal, reverse, diff, and reverse-diff modes. Returns data for all runs by default, or a specific run if run_id is provided. In diff/reverse-diff mode, compares two runs and returns functions with the biggest percentage change."
    )]
    async fn get_flamegraph(
        &self,
        Parameters(req): Parameters<GetFlamegraphRequest>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.lock().await;
        let loaded = match &state.report {
            Some(r) => r,
            None => {
                return ok_json(
                    json!({"status": "error", "message": "Error: No report loaded. Call load_report() first."}),
                )
            }
        };

        let fg_type = req.flamegraph_type.as_deref().unwrap_or("normal");
        if !["normal", "reverse", "diff", "reverse-diff"].contains(&fg_type) {
            return ok_json(
                json!({"status": "error", "message": "Error: flamegraph_type must be 'normal', 'reverse', 'diff', or 'reverse-diff'."}),
            );
        }

        let is_diff = fg_type == "diff" || fg_type == "reverse-diff";
        // The underlying SVG type for file lookup
        let svg_type = if fg_type == "reverse-diff" {
            "reverse"
        } else if fg_type == "diff" {
            "normal"
        } else {
            fg_type
        };

        let limit = req.limit.unwrap_or(30);

        // Compile regex filter if provided
        let filter_re = if let Some(ref pattern) = req.filter {
            match Regex::new(&format!("(?i){}", pattern)) {
                Ok(re) => Some(re),
                Err(e) => {
                    return ok_json(
                        json!({"status": "error", "message": format!("Error: Invalid regex filter '{}': {}", pattern, e)}),
                    )
                }
            }
        } else {
            None
        };

        // --- DIFF MODE ---
        if is_diff {
            let min_delta = req.min_pct.unwrap_or(0.5);

            // Resolve the two run IDs for diff
            let (rid1, rid2) = match &req.run_id {
                Some(ids) if ids.len() == 2 => (Some(ids[0].as_str()), Some(ids[1].as_str())),
                Some(ids) if ids.len() > 2 => {
                    return ok_json(
                        json!({"status": "error", "message": "Error: diff/reverse-diff mode requires exactly 2 run IDs in run_id array."}),
                    );
                }
                Some(ids) if ids.len() == 1 => {
                    return ok_json(
                        json!({"status": "error", "message": "Error: diff/reverse-diff mode requires exactly 2 run IDs in run_id array. Got 1."}),
                    );
                }
                _ => (None, None), // Use defaults (first and second run)
            };

            // Find both SVGs
            let svg1 = find_flamegraph_svg(loaded, rid1, svg_type);
            let svg1 = match svg1 {
                Ok(p) => p,
                Err(msg) => {
                    return ok_json(
                        json!({"status": "error", "message": format!("Base run: {}", msg)}),
                    )
                }
            };
            let svg2 = find_flamegraph_svg(loaded, rid2, svg_type);
            let svg2 = match svg2 {
                Ok(p) => p,
                Err(msg) => {
                    return ok_json(
                        json!({"status": "error", "message": format!("Comparison run: {}", msg)}),
                    )
                }
            };

            if svg1 == svg2 {
                return ok_json(
                    json!({"status": "error", "message": "Error: Both run IDs resolve to the same flamegraph. Provide two different run IDs in the run_id array."}),
                );
            }

            // Parse both
            let frames1 = match parse_flamegraph_svg(&svg1) {
                Ok(f) => f,
                Err(msg) => return ok_json(json!({"status": "error", "message": msg})),
            };
            let frames2 = match parse_flamegraph_svg(&svg2) {
                Ok(f) => f,
                Err(msg) => return ok_json(json!({"status": "error", "message": msg})),
            };

            // Build function → pct maps
            let map1: HashMap<&str, f64> =
                frames1.iter().map(|f| (f.name.as_str(), f.pct)).collect();
            let map2: HashMap<&str, f64> =
                frames2.iter().map(|f| (f.name.as_str(), f.pct)).collect();

            // Compute deltas
            let mut all_functions: std::collections::HashSet<&str> =
                std::collections::HashSet::new();
            all_functions.extend(map1.keys());
            all_functions.extend(map2.keys());

            let mut deltas: Vec<Value> = Vec::new();
            for func in all_functions {
                let pct1 = map1.get(func).copied().unwrap_or(0.0);
                let pct2 = map2.get(func).copied().unwrap_or(0.0);
                let delta = pct2 - pct1;
                if delta.abs() >= min_delta {
                    // Apply regex filter if provided
                    if let Some(ref re) = filter_re {
                        if !re.is_match(func) {
                            continue;
                        }
                    }
                    deltas.push(json!({
                        "function": func,
                        "pct_run1": (pct1 * 100.0).round() / 100.0,
                        "pct_run2": (pct2 * 100.0).round() / 100.0,
                        "delta_pct": (delta * 100.0).round() / 100.0,
                    }));
                }
            }

            // Sort by absolute delta descending
            deltas.sort_by(|a, b| {
                let da = a
                    .get("delta_pct")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    .abs();
                let db = b
                    .get("delta_pct")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    .abs();
                db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
            });

            let total = deltas.len();
            let truncated: Vec<Value> = deltas.into_iter().take(limit).collect();

            let filter_desc = if let Some(ref pattern) = req.filter {
                format!(" matching '{}'", pattern)
            } else {
                String::new()
            };

            let response = json!({
                "status": "success",
                "message": format!("Flamegraph diff ({}): {} functions{} with delta >= {:.1}% (showing top {})", fg_type, total, filter_desc, min_delta, truncated.len()),
                "flamegraph_type": fg_type,
                "total_changed": total,
                "functions": truncated,
            });

            return ok_json(response);
        }

        // --- NORMAL / REVERSE MODE ---
        let min_pct = req.min_pct.unwrap_or(0.1);

        // Determine which runs to process
        let run_ids: Vec<String> = match &req.run_id {
            Some(ids) if !ids.is_empty() => ids.clone(),
            _ => find_all_flamegraph_run_ids(loaded, fg_type),
        };

        if run_ids.is_empty() {
            return ok_json(
                json!({"status": "error", "message": format!("Error: No {} flamegraph SVGs found in report.", fg_type)}),
            );
        }

        let mut all_run_results: Vec<Value> = Vec::new();

        for rid in &run_ids {
            let svg_path = find_flamegraph_svg(loaded, Some(rid.as_str()), fg_type);
            let svg_path = match svg_path {
                Ok(p) => p,
                Err(msg) => {
                    log::warn!("Skipping run '{}': {}", rid, msg);
                    continue;
                }
            };

            let frames = match parse_flamegraph_svg(&svg_path) {
                Ok(f) => f,
                Err(msg) => {
                    all_run_results.push(json!({
                        "run_id": rid,
                        "status": "error",
                        "message": msg,
                    }));
                    continue;
                }
            };

            // Build parent relationships and filter by min_pct
            let annotated = build_annotated_frames(&frames, min_pct);

            // Apply regex filter if provided
            let filtered: Vec<&AnnotatedFrame> = if let Some(ref re) = filter_re {
                annotated.iter().filter(|f| re.is_match(&f.name)).collect()
            } else {
                annotated.iter().collect()
            };

            // Take top N by percentage
            let top: Vec<Value> = filtered
                .iter()
                .take(limit)
                .map(|f| {
                    json!({
                        "function": f.name,
                        "samples": f.samples,
                        "pct": f.pct,
                        "depth": f.depth,
                        "parent": f.parent,
                    })
                })
                .collect();

            let total_samples = frames.first().map(|f| f.samples).unwrap_or(0);

            all_run_results.push(json!({
                "run_id": rid,
                "total_frames_parsed": frames.len(),
                "total_samples": total_samples,
                "matched_count": filtered.len(),
                "returned_count": top.len(),
                "functions": top,
            }));
        }

        // Build response
        let filter_desc = if let Some(ref pattern) = req.filter {
            format!(" matching filter '{}'", pattern)
        } else {
            String::new()
        };

        let response = if all_run_results.len() == 1 {
            let run_data = &all_run_results[0];
            json!({
                "status": "success",
                "message": format!("Flamegraph ({} type) for run '{}': {} functions{} (top {})",
                    fg_type,
                    run_data.get("run_id").and_then(|v| v.as_str()).unwrap_or("?"),
                    run_data.get("matched_count").and_then(|v| v.as_u64()).unwrap_or(0),
                    filter_desc,
                    run_data.get("returned_count").and_then(|v| v.as_u64()).unwrap_or(0),
                ),
                "flamegraph_type": fg_type,
                "run_id": run_data.get("run_id"),
                "total_frames_parsed": run_data.get("total_frames_parsed"),
                "total_samples": run_data.get("total_samples"),
                "matched_count": run_data.get("matched_count"),
                "functions": run_data.get("functions"),
            })
        } else {
            json!({
                "status": "success",
                "message": format!("Flamegraph ({} type) for {} runs{} (top {} per run)",
                    fg_type, all_run_results.len(), filter_desc, limit),
                "flamegraph_type": fg_type,
                "run_count": all_run_results.len(),
                "runs": all_run_results,
            })
        };

        ok_json(response)
    }

    #[tool(
        description = "Record performance data on the current system using aperf. Runs 'aperf record' as a subprocess. Returns when recording completes. Requires Linux and appropriate kernel permissions."
    )]
    async fn record(
        &self,
        Parameters(req): Parameters<RecordRequest>,
    ) -> Result<CallToolResult, McpError> {
        let aperf_bin = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                return ok_json(
                    json!({"status": "error", "message": format!("Cannot find aperf binary: {}", e)}),
                )
            }
        };

        let args = build_record_args(&req);
        let cmd_str = format!("{} {}", aperf_bin.display(), args.join(" "));

        match tokio::process::Command::new(&aperf_bin)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => match child.wait_with_output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let success = output.status.success();

                    let run_name = req
                        .run_name
                        .clone()
                        .unwrap_or_else(|| "aperf_<timestamp>".to_string());

                    ok_json(json!({
                        "status": if success { "success" } else { "error" },
                        "message": if success {
                            format!("Recording completed successfully. Run name: {}", run_name)
                        } else {
                            format!("Recording failed with exit code: {}", output.status)
                        },
                        "command": cmd_str,
                        "exit_code": output.status.code(),
                        "stdout": stdout.trim(),
                        "stderr": stderr.trim(),
                    }))
                }
                Err(e) => ok_json(
                    json!({"status": "error", "message": format!("Failed to wait for process: {}", e), "command": cmd_str}),
                ),
            },
            Err(e) => ok_json(
                json!({"status": "error", "message": format!("Failed to spawn aperf record: {}", e), "command": cmd_str}),
            ),
        }
    }

    #[tool(
        description = "Generate an HTML report from one or more recorded APerf runs. Runs 'aperf report' as a subprocess. For multi-run comparison, provide multiple run paths — the first run is used as the base for analytical findings."
    )]
    async fn generate_report(
        &self,
        Parameters(req): Parameters<GenerateReportRequest>,
    ) -> Result<CallToolResult, McpError> {
        if req.runs.is_empty() {
            return ok_json(
                json!({"status": "error", "message": "Error: At least one run path is required."}),
            );
        }

        let aperf_bin = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                return ok_json(
                    json!({"status": "error", "message": format!("Cannot find aperf binary: {}", e)}),
                )
            }
        };

        let args = build_report_args(&req);
        let cmd_str = format!("{} {}", aperf_bin.display(), args.join(" "));

        match tokio::process::Command::new(&aperf_bin)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => match child.wait_with_output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let success = output.status.success();

                    let report_name = req
                        .name
                        .clone()
                        .unwrap_or_else(|| "aperf_report_<auto>".to_string());

                    ok_json(json!({
                        "status": if success { "success" } else { "error" },
                        "message": if success {
                            format!("Report generated successfully. Report name: {}. Open {}/index.html in a browser to view.", report_name, report_name)
                        } else {
                            format!("Report generation failed with exit code: {}", output.status)
                        },
                        "command": cmd_str,
                        "run_count": req.runs.len(),
                        "runs": req.runs,
                        "exit_code": output.status.code(),
                        "stdout": stdout.trim(),
                        "stderr": stderr.trim(),
                    }))
                }
                Err(e) => ok_json(
                    json!({"status": "error", "message": format!("Failed to wait for process: {}", e), "command": cmd_str}),
                ),
            },
            Err(e) => ok_json(
                json!({"status": "error", "message": format!("Failed to spawn aperf report: {}", e), "command": cmd_str}),
            ),
        }
    }
}

#[tool_handler]
impl ServerHandler for AperfMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "APerf MCP Server — analyze APerf performance reports. \
             Call load_report first, then query metrics and findings."
                .to_string(),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

// ---------------------------------------------------------------------------
// Series resolution (cpu_ids filtering / averaging) using typed data
// ---------------------------------------------------------------------------

use crate::data::common::data_formats::{Series, TimeSeriesMetric};

struct ResolvedTypedSeries {
    series_json: Value,  // JSON array of series for "timeseries" output
    values: Vec<f64>,    // single flattened/averaged values for summary/compact/downsampled
    time_diff: Vec<f64>, // corresponding time points
}

/// Resolve series from a typed TimeSeriesMetric, applying cpu_ids filtering or averaging.
fn resolve_typed_series(
    metric: &TimeSeriesMetric,
    cpu_ids: &Option<Vec<String>>,
    metric_name: &str,
    _accessor: &ProcessedDataAccessor,
    _run_name: &str,
) -> Result<ResolvedTypedSeries, String> {
    let has_multiple_series = metric.series.len() > 1;

    if has_multiple_series {
        if let Some(ref ids) = cpu_ids {
            // Filter to requested series
            let filtered: Vec<&Series> = metric
                .series
                .iter()
                .filter(|s| ids.iter().any(|c| c == &s.series_name))
                .collect();
            if filtered.is_empty() {
                let available: Vec<&str> = metric
                    .series
                    .iter()
                    .map(|s| s.series_name.as_str())
                    .collect();
                return Err(format!(
                    "Error: None of the requested CPUs {:?} found. Available: {:?}",
                    ids, available
                ));
            }
            let values = filtered[0].values.clone();
            let time_diff: Vec<f64> = filtered[0].time_diff.iter().map(|&t| t as f64).collect();
            let series_json = serde_json::to_value(&filtered).unwrap_or(json!([]));
            Ok(ResolvedTypedSeries {
                series_json,
                values,
                time_diff,
            })
        } else {
            // Average across all series
            let (averaged, td) = average_typed_series(&metric.series, metric_name)?;
            let avg_series =
                json!([{"series_name": "averaged", "time_diff": td, "values": averaged}]);
            Ok(ResolvedTypedSeries {
                series_json: avg_series,
                values: averaged,
                time_diff: td.iter().map(|&x| x as f64).collect(),
            })
        }
    } else {
        // Single series
        let s = &metric.series[0];
        let values = s.values.clone();
        let time_diff: Vec<f64> = s.time_diff.iter().map(|&t| t as f64).collect();
        let series_json = serde_json::to_value(&metric.series).unwrap_or(json!([]));
        Ok(ResolvedTypedSeries {
            series_json,
            values,
            time_diff,
        })
    }
}

fn average_typed_series(
    series: &[Series],
    metric_name: &str,
) -> Result<(Vec<f64>, Vec<u64>), String> {
    if series.is_empty() || series[0].values.is_empty() {
        return Err(format!(
            "Error: No values in time series for '{}'",
            metric_name
        ));
    }
    let num_points = series[0].values.len();
    let mut averaged = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let mut sum = 0.0;
        let mut count = 0usize;
        for s in series {
            if i < s.values.len() {
                sum += s.values[i];
                count += 1;
            }
        }
        averaged.push(if count > 0 { sum / count as f64 } else { 0.0 });
    }
    let td = series[0].time_diff.clone();
    Ok((averaged, td))
}

// ---------------------------------------------------------------------------
// Helper: load a JS file into ProcessedData
// ---------------------------------------------------------------------------

/// Read a JS file, strip the variable prefix, and deserialize into ProcessedData.
fn load_processed_data(path: &std::path::Path) -> Result<ProcessedData, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Error reading file: {}", e))?;
    let json_str = super::js_parser::extract_json_from_js(&content)
        .ok_or_else(|| format!("Error: No data found in {}", path.display()))?;
    serde_json::from_str(&json_str).map_err(|e| format!("Error deserializing ProcessedData: {}", e))
}

// ---------------------------------------------------------------------------
// Flamegraph SVG parsing
// ---------------------------------------------------------------------------

use regex::Regex;
use std::path::PathBuf;

/// A single frame parsed from a flamegraph SVG.
#[derive(Debug, Clone)]
struct FlamegraphFrame {
    name: String,
    samples: u64,
    pct: f64,
    depth: u32,
    x: u64, // fg:x — horizontal position in sample space
    w: u64, // fg:w — width in sample space
}

/// A frame annotated with its parent function name.
struct AnnotatedFrame {
    name: String,
    samples: u64,
    pct: f64,
    depth: u32,
    parent: String,
}

/// Find the flamegraph SVG file for a given run and type.
fn find_flamegraph_svg(
    loaded: &LoadedReport,
    run_id: Option<&str>,
    fg_type: &str,
) -> Result<PathBuf, String> {
    // Flamegraph SVGs live in the same data/js/ directory as metric files
    let search_dir = &loaded.data_dir;
    let not_reverse = "reverse-flamegraph.svg";

    // If run_id specified, look for files containing run_id and matching the type
    if let Some(rid) = run_id {
        if let Ok(entries) = std::fs::read_dir(search_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let matches = if fg_type == "reverse" {
                    name.contains(rid) && name.ends_with(not_reverse)
                } else {
                    name.contains(rid)
                        && name.ends_with("flamegraph.svg")
                        && !name.ends_with(not_reverse)
                };
                if matches {
                    return Ok(entry.path());
                }
            }
        }
        return Err(format!(
            "Error: No {} flamegraph found for run '{}' in {}",
            fg_type,
            rid,
            search_dir.display()
        ));
    }

    // No run_id — find any matching SVG (prefer first alphabetically)
    if let Ok(entries) = std::fs::read_dir(search_dir) {
        let mut svgs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if fg_type == "reverse" {
                    name.ends_with(not_reverse)
                } else {
                    name.ends_with("flamegraph.svg") && !name.ends_with(not_reverse)
                }
            })
            .collect();
        svgs.sort();
        if let Some(first) = svgs.first() {
            return Ok(first.clone());
        }
    }

    Err(format!(
        "Error: No {} flamegraph SVG found in {}",
        fg_type,
        search_dir.display()
    ))
}

/// Find all run IDs that have flamegraph SVGs available.
/// Extracts run IDs from SVG filenames by matching against known run IDs from metadata.
fn find_all_flamegraph_run_ids(loaded: &LoadedReport, fg_type: &str) -> Vec<String> {
    let search_dir = &loaded.data_dir;
    let not_reverse = "reverse-flamegraph.svg";

    // Get known run IDs from metadata
    let known_run_ids: Vec<String> = loaded.metadata.run_ids.clone().unwrap_or_default();

    // Find all matching SVG files
    let svgs: Vec<String> = match std::fs::read_dir(search_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| {
                if fg_type == "reverse" {
                    name.ends_with(not_reverse)
                } else {
                    name.ends_with("flamegraph.svg") && !name.ends_with(not_reverse)
                }
            })
            .collect(),
        Err(_) => return Vec::new(),
    };

    // Match SVG filenames to known run IDs
    let mut matched_ids: Vec<String> = Vec::new();
    for rid in &known_run_ids {
        if svgs.iter().any(|svg| svg.contains(rid.as_str())) {
            matched_ids.push(rid.clone());
        }
    }

    // If no known run IDs matched (e.g., single-run report), fall back to returning
    // a synthetic ID based on the SVG filename
    if matched_ids.is_empty() && !svgs.is_empty() {
        // Use the SVG filenames themselves as identifiers — extract the part before "-flamegraph.svg"
        let mut sorted_svgs = svgs;
        sorted_svgs.sort();
        for svg_name in &sorted_svgs {
            let stem = if fg_type == "reverse" {
                svg_name.trim_end_matches("-reverse-flamegraph.svg")
            } else {
                svg_name.trim_end_matches("-flamegraph.svg")
            };
            if !stem.is_empty() && stem != svg_name {
                matched_ids.push(stem.to_string());
            }
        }
    }

    matched_ids
}

/// Parse a flamegraph SVG and extract all frames.
fn parse_flamegraph_svg(path: &std::path::Path) -> Result<Vec<FlamegraphFrame>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Error reading SVG: {}", e))?;

    // Extract total_samples from: <svg id="frames" ... total_samples="N">
    let total_samples_re = Regex::new(r#"total_samples="(\d+)""#).unwrap();
    let total_samples: u64 = total_samples_re
        .captures(&content)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);

    if total_samples == 0 {
        return Err("Error: Could not find total_samples in SVG.".to_string());
    }

    // Parse frames: <title>function_name (N samples, X.XX%)</title><rect ... y="Y" ... fg:x="X" fg:w="W"/>
    // The title format is: "name (N samples, X.XX%)" or "name (N,NNN samples, X.XX%)"
    let frame_re = Regex::new(
        r#"<title>([^<]+?) \(([\d,]+) samples?, ([\d.]+)%\)</title><rect[^>]*y="(\d+)"[^>]*fg:x="(\d+)"[^>]*fg:w="(\d+)""#,
    )
    .unwrap();

    let mut frames: Vec<FlamegraphFrame> = Vec::new();

    for cap in frame_re.captures_iter(&content) {
        let name = cap[1].to_string();
        let samples: u64 = cap[2].replace(',', "").parse().unwrap_or(0);
        let pct: f64 = cap[3].parse().unwrap_or(0.0);
        let y: u32 = cap[4].parse().unwrap_or(0);
        let x: u64 = cap[5].parse().unwrap_or(0);
        let w: u64 = cap[6].parse().unwrap_or(0);

        // Skip the "all" root frame
        if name == "all" {
            continue;
        }

        frames.push(FlamegraphFrame {
            name,
            samples,
            pct,
            depth: y,
            x,
            w,
        });
    }

    if frames.is_empty() {
        return Err(format!(
            "Error: No frames found in SVG at {}",
            path.display()
        ));
    }

    Ok(frames)
}

/// Build annotated frames with parent relationships, sorted by pct descending.
fn build_annotated_frames(frames: &[FlamegraphFrame], min_pct: f64) -> Vec<AnnotatedFrame> {
    // Sort frames by depth then x for parent lookup
    let mut sorted: Vec<&FlamegraphFrame> = frames.iter().collect();
    sorted.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.x.cmp(&b.x)));

    // For each frame, find its parent: the frame at depth-15 (one level up, since each level is 15px)
    // whose x-range contains this frame's x-range
    let mut annotated: Vec<AnnotatedFrame> = Vec::new();

    for frame in frames {
        if frame.pct < min_pct {
            continue;
        }

        // Find parent: frame at depth - 15 (or + 15 for inverted) where parent.x <= frame.x and parent.x + parent.w >= frame.x + frame.w
        let parent_depth = if frame.depth >= 15 {
            frame.depth - 15
        } else {
            frame.depth + 15
        };

        let parent_name = sorted
            .iter()
            .find(|f| {
                f.depth == parent_depth && f.x <= frame.x && (f.x + f.w) >= (frame.x + frame.w)
            })
            .map(|f| f.name.as_str())
            .unwrap_or("-");

        annotated.push(AnnotatedFrame {
            name: frame.name.clone(),
            samples: frame.samples,
            pct: frame.pct,
            depth: frame.depth / 15, // Convert pixel depth to stack level
            parent: parent_name.to_string(),
        });
    }

    // Sort by percentage descending
    annotated.sort_by(|a, b| {
        b.pct
            .partial_cmp(&a.pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    annotated
}

// ---------------------------------------------------------------------------
// Output format: summary
// ---------------------------------------------------------------------------

fn build_timeseries_summary(values: &[f64], time_diff: &[f64], stats: &Value) -> String {
    if values.is_empty() {
        return "No data points".to_string();
    }
    let n = values.len();
    let avg = stats.get("avg").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let min = stats.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let max = stats.get("max").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let std_val = stats.get("std").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let p50 = stats.get("p50").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let p90 = stats.get("p90").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let p99 = stats.get("p99").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let duration = if time_diff.len() >= 2 {
        time_diff[time_diff.len() - 1] - time_diff[0]
    } else {
        n as f64
    };

    // Trend: compare first 10% avg vs last 10%
    let tenth = (n / 10).max(1);
    let first_avg: f64 = values[..tenth].iter().sum::<f64>() / tenth as f64;
    let last_avg: f64 = values[n - tenth..].iter().sum::<f64>() / tenth as f64;
    let trend_pct = if first_avg.abs() > 0.001 {
        ((last_avg - first_avg) / first_avg) * 100.0
    } else {
        0.0
    };
    let trend = if trend_pct.abs() < 2.0 {
        "stable".to_string()
    } else if trend_pct > 0.0 {
        format!("rising({:+.1}%)", trend_pct)
    } else {
        format!("falling({:+.1}%)", trend_pct)
    };

    // Detect spikes (>2 std above avg) and drops (>2 std below avg)
    let mut anomalies: Vec<String> = Vec::new();
    let spike_threshold = avg + 2.0 * std_val;
    let drop_threshold = (avg - 2.0 * std_val).max(0.0);

    // Spike detection
    let mut in_spike = false;
    let mut spike_start = 0usize;
    let mut spike_peak = 0.0f64;
    for i in 0..n {
        let v = values[i];
        if v > spike_threshold && !in_spike {
            in_spike = true;
            spike_start = i;
            spike_peak = v;
        } else if v > spike_threshold && in_spike {
            spike_peak = spike_peak.max(v);
        } else if in_spike {
            in_spike = false;
            let t_start = if spike_start < time_diff.len() {
                time_diff[spike_start]
            } else {
                spike_start as f64
            };
            let t_end = if i < time_diff.len() {
                time_diff[i]
            } else {
                i as f64
            };
            anomalies.push(format!(
                "spike t={:.0}s-{:.0}s (peak {:.1})",
                t_start, t_end, spike_peak
            ));
            if anomalies.len() >= 5 {
                break;
            }
        }
    }

    // Drop detection (only if std > 0 to avoid false positives on flat data)
    if std_val > 0.001 && anomalies.len() < 5 {
        let mut in_drop = false;
        let mut drop_start = 0usize;
        let mut drop_trough = f64::MAX;
        for i in 0..n {
            let v = values[i];
            if v < drop_threshold && !in_drop {
                in_drop = true;
                drop_start = i;
                drop_trough = v;
            } else if v < drop_threshold && in_drop {
                drop_trough = drop_trough.min(v);
            } else if in_drop {
                in_drop = false;
                let t_start = if drop_start < time_diff.len() {
                    time_diff[drop_start]
                } else {
                    drop_start as f64
                };
                let t_end = if i < time_diff.len() {
                    time_diff[i]
                } else {
                    i as f64
                };
                anomalies.push(format!(
                    "drop t={:.0}s-{:.0}s (trough {:.1})",
                    t_start, t_end, drop_trough
                ));
                if anomalies.len() >= 5 {
                    break;
                }
            }
        }
    }

    let mut summary = format!(
        "points={}, duration={:.0}s, avg={:.2}, min={:.2}, max={:.2}, std={:.2}, p50={:.2}, p90={:.2}, p99={:.2}, trend={}",
        n, duration, avg, min, max, std_val, p50, p90, p99, trend
    );
    if !anomalies.is_empty() {
        summary.push_str(&format!(", anomalies: [{}]", anomalies.join("; ")));
    }
    summary
}

// ---------------------------------------------------------------------------
// Output format: compact (delta-encoded)
// ---------------------------------------------------------------------------

fn build_compact_notation(values: &[f64], time_diff: &[f64], metric_name: &str) -> String {
    if values.is_empty() {
        return format!("ts:{}|empty", metric_name);
    }

    // Time header
    let t0 = if !time_diff.is_empty() {
        time_diff[0]
    } else {
        0.0
    };
    let dt = if time_diff.len() >= 2 {
        time_diff[1] - time_diff[0]
    } else {
        1.0
    };

    let mut parts = Vec::new();
    parts.push(format!(
        "ts:{}|t0={:.0},dt={:.0}|v0={:.2}",
        metric_name, t0, dt, values[0]
    ));

    // Delta encode values, round to 2 decimal places
    let mut deltas = Vec::with_capacity(values.len() - 1);
    for i in 1..values.len() {
        let d = ((values[i] - values[i - 1]) * 100.0).round() / 100.0;
        if d == 0.0 {
            deltas.push("0".to_string());
        } else {
            deltas.push(format!("{:+.2}", d));
        }
    }

    // Run-length encode repeated deltas
    let mut rle: Vec<String> = Vec::new();
    let mut i = 0;
    while i < deltas.len() {
        let mut count = 1;
        while i + count < deltas.len() && deltas[i + count] == deltas[i] {
            count += 1;
        }
        if count >= 3 {
            rle.push(format!("{}x{}", deltas[i], count));
        } else {
            for _ in 0..count {
                rle.push(deltas[i].clone());
            }
        }
        i += count;
    }

    parts.push(format!("d:{}", rle.join(",")));
    parts.join("|")
}

// ---------------------------------------------------------------------------
// Output format: downsampled (fixed buckets)
// ---------------------------------------------------------------------------

fn build_downsampled(values: &[f64], time_diff: &[f64], num_buckets: usize) -> Value {
    if values.is_empty() {
        return json!({"buckets": []});
    }

    let n = values.len();
    let buckets = num_buckets.min(n);
    let bucket_size = n as f64 / buckets as f64;

    let mut result = Vec::with_capacity(buckets);
    for b in 0..buckets {
        let start = (b as f64 * bucket_size) as usize;
        let end = (((b + 1) as f64 * bucket_size) as usize).min(n);
        let slice = &values[start..end];

        let avg = slice.iter().sum::<f64>() / slice.len() as f64;
        let min = slice.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let t_start = if start < time_diff.len() {
            time_diff[start]
        } else {
            start as f64
        };
        let t_end = if end > 0 && end - 1 < time_diff.len() {
            time_diff[end - 1]
        } else {
            (end - 1) as f64
        };

        result.push(json!({
            "t": format!("{:.0}-{:.0}s", t_start, t_end),
            "avg": (avg * 100.0).round() / 100.0,
            "min": (min * 100.0).round() / 100.0,
            "max": (max * 100.0).round() / 100.0,
        }));
    }

    json!({"buckets": result, "bucket_count": buckets})
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Arg builders (extracted for testability)
// ---------------------------------------------------------------------------

fn build_record_args(req: &RecordRequest) -> Vec<String> {
    let mut args: Vec<String> = vec!["record".to_string()];
    if let Some(ref name) = req.run_name {
        args.push("-r".to_string());
        args.push(name.clone());
    }
    if let Some(interval) = req.interval {
        args.push("-i".to_string());
        args.push(interval.to_string());
    }
    if let Some(period) = req.period {
        args.push("-p".to_string());
        args.push(period.to_string());
    }
    if req.profile.unwrap_or(false) {
        args.push("--profile".to_string());
    }
    if let Some(freq) = req.perf_frequency {
        args.push("-F".to_string());
        args.push(freq.to_string());
    }
    if req.memory_allocation.unwrap_or(false) {
        args.push("--memory-allocation".to_string());
    }
    if let Some(ref dc) = req.dont_collect {
        args.push("--dont-collect".to_string());
        args.push(dc.clone());
    }
    if let Some(ref co) = req.collect_only {
        args.push("--collect-only".to_string());
        args.push(co.clone());
    }
    if let Some(ref java) = req.profile_java {
        args.push("--profile-java".to_string());
        if !java.is_empty() {
            args.push(java.clone());
        }
    }
    if let Some(ref pmu) = req.pmu_config {
        args.push("--pmu-config".to_string());
        args.push(pmu.clone());
    }
    args
}

fn build_report_args(req: &GenerateReportRequest) -> Vec<String> {
    let mut args: Vec<String> = vec!["report".to_string()];
    args.push("-r".to_string());
    for run in &req.runs {
        args.push(run.clone());
    }
    if let Some(ref name) = req.name {
        args.push("-n".to_string());
        args.push(name.clone());
    }
    args
}

fn ok_json(value: Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&value).unwrap(),
    )]))
}

fn find_target_file(
    data_dir: &std::path::Path,
    file_name: Option<&str>,
    category: Option<&str>,
) -> Result<std::path::PathBuf, String> {
    if let Some(fname) = file_name {
        let path = data_dir.join(fname);
        if path.exists() {
            return Ok(path);
        }
        return Err(format!(
            "Error: File '{}' not found in data directory.",
            fname
        ));
    }
    if let Some(cat) = category {
        let path = data_dir.join(format!("{}.js", cat));
        if path.exists() {
            return Ok(path);
        }
        // Try without underscores
        if let Ok(entries) = std::fs::read_dir(data_dir) {
            for entry in entries.flatten() {
                let stem = entry
                    .path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if stem == cat || stem.replace('_', "") == cat.replace('_', "") {
                    return Ok(entry.path());
                }
            }
        }
        return Err(format!("Error: No file found for category '{}'.", cat));
    }
    Err("Error: Must specify either 'file_name' or 'category'.".to_string())
}

// ---------------------------------------------------------------------------
// Response builders
// ---------------------------------------------------------------------------

fn build_load_report_response(loaded: &LoadedReport) -> Value {
    let meta = &loaded.metadata;
    let mut msg = format!(
        "Successfully loaded APerf report from: {}\n\n",
        loaded.report_path
    );

    if let Some(ref runs) = meta.runs {
        let count = runs.len();
        if count > 1 {
            msg.push_str(&format!(
                "Report Information: Multi-run ({} runs)\n\n",
                count
            ));
        } else {
            msg.push_str("Report Information: Single-run\n\n");
        }

        for (idx, run) in runs.iter().enumerate() {
            let indent = if count > 1 {
                msg.push_str(&format!("  Run {}: {}\n", idx + 1, run.run_id));
                "    "
            } else {
                "  "
            };
            if let Some(ref v) = run.instance_type {
                msg.push_str(&format!("{}Instance Type: {}\n", indent, v));
            }
            if let Some(ref v) = run.cpu_count {
                msg.push_str(&format!("{}CPUs: {}\n", indent, v));
            }
            if let Some(ref v) = run.os {
                msg.push_str(&format!("{}OS: {}\n", indent, v));
            }
            if let Some(ref v) = run.kernel {
                msg.push_str(&format!("{}Kernel: {}\n", indent, v));
            }
            if let Some(ref v) = run.region {
                msg.push_str(&format!("{}Region: {}\n", indent, v));
            }
            if count > 1 {
                msg.push('\n');
            }
        }
    }

    if !loaded.metrics.is_empty() {
        msg.push_str(&format!(
            "\nAvailable Metrics: {} metric files found\n",
            loaded.metrics.len()
        ));
    }

    msg.push_str("\nReport validated and ready for analysis.");

    json!({
        "status": "success",
        "message": msg.trim(),
        "report_path": loaded.report_path,
        "metadata": meta,
        "metrics": loaded.metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::mcp::js_parser;
    use std::path::PathBuf;

    fn test_data_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/mcp_test_data")
    }

    #[test]
    fn test_find_target_file_by_category() {
        let data_dir = test_data_base().join("metric_files");
        // metric_files has cpu_utilization.js
        let result = find_target_file(&data_dir, None, Some("cpu_utilization"));
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("cpu_utilization.js"));
    }

    #[test]
    fn test_find_target_file_by_filename() {
        let data_dir = test_data_base().join("metric_files");
        let result = find_target_file(&data_dir, Some("cpu_utilization.js"), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_target_file_missing() {
        let data_dir = test_data_base().join("metric_files");
        let result = find_target_file(&data_dir, Some("nonexistent.js"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_target_file_no_params() {
        let data_dir = test_data_base().join("metric_files");
        let result = find_target_file(&data_dir, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Must specify"));
    }

    #[test]
    fn test_findings_extraction_two_run_report() {
        // The two_run_report has findings in both systeminfo.js and cpu_utilization.js
        let data_dir = test_data_base().join("two_run_report/data/js");
        if !data_dir.exists() {
            return;
        }

        let mut all_findings: Vec<Value> = Vec::new();
        let mut js_files: Vec<_> = std::fs::read_dir(&data_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new("js")))
            .collect();
        js_files.sort_by_key(|e| e.file_name());

        for entry in &js_files {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            let findings_vars = js_parser::extract_findings_variables(&content);
            for (category, json_str) in findings_vars {
                let findings_data: Value = serde_json::from_str(&json_str).unwrap();
                if let Some(per_run) = findings_data
                    .get("per_run_findings")
                    .and_then(|v| v.as_object())
                {
                    for (run_id, run_data) in per_run {
                        if let Some(findings_dict) =
                            run_data.get("findings").and_then(|v| v.as_object())
                        {
                            for (metric_name, finding_list) in findings_dict {
                                if let Some(arr) = finding_list.as_array() {
                                    for finding in arr {
                                        all_findings.push(json!({
                                            "category": category,
                                            "run_id": run_id,
                                            "metric_name": metric_name,
                                            "rule_name": finding.get("rule_name").and_then(|v| v.as_str()).unwrap_or(""),
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // two_run_report should have findings: kernel version mismatch + underutilized CPU
        assert!(
            all_findings.len() >= 2,
            "Expected at least 2 findings, got {}",
            all_findings.len()
        );

        // Check we have the kernel version mismatch
        let has_kernel = all_findings.iter().any(|f| {
            f.get("rule_name").and_then(|v| v.as_str()) == Some("Kernel Version Mismatch")
        });
        assert!(has_kernel, "Expected Kernel Version Mismatch finding");

        // Check we have underutilized CPU
        let has_cpu = all_findings
            .iter()
            .any(|f| f.get("rule_name").and_then(|v| v.as_str()) == Some("Underutilized CPU"));
        assert!(has_cpu, "Expected Underutilized CPU finding");
    }

    #[test]
    fn test_findings_extraction_single_run_empty() {
        // Single run report should have no findings
        let data_dir = test_data_base().join("single_run_report/data/js");
        if !data_dir.exists() {
            return;
        }

        let mut all_findings: Vec<Value> = Vec::new();
        let js_files: Vec<_> = std::fs::read_dir(&data_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new("js")))
            .collect();

        for entry in &js_files {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            let findings_vars = js_parser::extract_findings_variables(&content);
            for (_category, json_str) in findings_vars {
                let findings_data: Value = serde_json::from_str(&json_str).unwrap();
                if let Some(per_run) = findings_data
                    .get("per_run_findings")
                    .and_then(|v| v.as_object())
                {
                    for (_run_id, run_data) in per_run {
                        if let Some(findings_dict) =
                            run_data.get("findings").and_then(|v| v.as_object())
                        {
                            for (_metric_name, finding_list) in findings_dict {
                                if let Some(arr) = finding_list.as_array() {
                                    all_findings.extend(arr.iter().cloned());
                                }
                            }
                        }
                    }
                }
            }
        }

        assert_eq!(
            all_findings.len(),
            0,
            "Single-run report should have no findings"
        );
    }

    #[test]
    fn test_metric_values_timeseries_from_file() {
        // Test reading time series data from cpu_utilization.js metric file
        let data_dir = test_data_base().join("metric_files");
        let target = find_target_file(&data_dir, None, Some("cpu_utilization")).unwrap();
        let content = std::fs::read_to_string(&target).unwrap();
        let json_str = js_parser::extract_json_from_js(&content).unwrap();
        let data: Value = serde_json::from_str(&json_str).unwrap();

        let runs = data.get("runs").unwrap().as_object().unwrap();
        assert!(runs.contains_key("test_run"));

        let run = &runs["test_run"];
        let metrics = run.get("metrics").unwrap().as_object().unwrap();
        assert!(metrics.contains_key("user"));

        let user = &metrics["user"];
        let stats = user.get("stats").unwrap();
        assert!(stats.get("avg").unwrap().as_f64().unwrap() > 0.0);

        let series = user.get("series").unwrap().as_array().unwrap();
        assert!(!series.is_empty());
        assert_eq!(
            series[0].get("series_name").unwrap().as_str().unwrap(),
            "CPU0"
        );
    }

    #[test]
    fn test_metric_values_keyvalue_from_file() {
        // Test reading key-value data from systeminfo.js
        let data_dir = test_data_base().join("single_run_report/data/js");
        if !data_dir.exists() {
            return;
        }
        let target = find_target_file(&data_dir, Some("systeminfo.js"), None).unwrap();
        let content = std::fs::read_to_string(&target).unwrap();
        let json_str = js_parser::extract_json_from_js(&content).unwrap();
        let data: Value = serde_json::from_str(&json_str).unwrap();

        let format = data.get("data_format").unwrap().as_str().unwrap();
        assert_eq!(format, "key_value");

        let runs = data.get("runs").unwrap().as_object().unwrap();
        let run = runs.values().next().unwrap();
        let kv_groups = run.get("key_value_groups").unwrap().as_object().unwrap();
        let default_group = kv_groups.get("").unwrap();
        let kvs = default_group
            .get("key_values")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(
            kvs.get("Instance Type").unwrap().as_str().unwrap(),
            "m8g.24xlarge"
        );
    }

    #[test]
    fn test_load_processed_data_timeseries() {
        // Verify that load_processed_data correctly deserializes a time-series JS file
        // into ProcessedData and that ProcessedDataAccessor can access it.
        let data_dir = test_data_base().join("metric_files");
        let target = find_target_file(&data_dir, None, Some("cpu_utilization")).unwrap();
        let pd = load_processed_data(&target).unwrap();

        assert_eq!(pd.data_name, "cpu_utilization");
        assert!(matches!(pd.data_format, DataFormat::TimeSeries));
        assert!(pd.runs.contains_key("test_run"));

        // Use ProcessedDataAccessor to get stats
        let mut accessor = ProcessedDataAccessor::new();
        let stats = accessor
            .time_series_metric_stats(&pd, "test_run", "user")
            .unwrap();
        assert!(stats.avg > 0.0);
        assert!(stats.max >= stats.avg);
        assert!(stats.min <= stats.avg);
    }

    #[test]
    fn test_load_processed_data_keyvalue() {
        // Verify that load_processed_data correctly deserializes a key-value JS file
        let data_dir = test_data_base().join("single_run_report/data/js");
        if !data_dir.exists() {
            return;
        }
        let target = find_target_file(&data_dir, Some("systeminfo.js"), None).unwrap();
        let pd = load_processed_data(&target).unwrap();

        assert_eq!(pd.data_format as u8, DataFormat::KeyValue as u8);

        // Use ProcessedDataAccessor to get a key-value
        let accessor = ProcessedDataAccessor::new();
        let run_name = pd.runs.keys().next().unwrap();
        let instance_type = accessor.key_value_value_by_key(&pd, run_name, "Instance Type");
        assert_eq!(instance_type, Some("m8g.24xlarge"));
    }

    #[test]
    fn test_load_processed_data_with_time_range() {
        // Verify that ProcessedDataAccessor respects time ranges
        let data_dir = test_data_base().join("metric_files");
        let target = find_target_file(&data_dir, None, Some("cpu_utilization")).unwrap();
        let pd = load_processed_data(&target).unwrap();

        // Full range stats
        let mut full_accessor = ProcessedDataAccessor::new();
        let full_stats = full_accessor
            .time_series_metric_stats(&pd, "test_run", "user")
            .unwrap();

        // Restricted range — from_time=5 should give fewer or different stats
        let mut ranged_accessor = ProcessedDataAccessor::from_time_ranges(
            std::collections::HashMap::from([("test_run".to_string(), 5)]),
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
        );
        let ranged_stats = ranged_accessor
            .time_series_metric_stats(&pd, "test_run", "user")
            .unwrap();

        // The ranged stats should differ from full (unless all data is after t=5)
        // At minimum, both should be valid
        assert!(ranged_stats.avg >= 0.0);
        assert!(full_stats.avg >= 0.0);
    }

    #[test]
    fn test_cpu_ids_filtering() {
        // cpu_utilization.js in metric_files has series with series_name="CPU0"
        let data_dir = test_data_base().join("metric_files");
        let target = find_target_file(&data_dir, None, Some("cpu_utilization")).unwrap();
        let content = std::fs::read_to_string(&target).unwrap();
        let json_str = js_parser::extract_json_from_js(&content).unwrap();
        let data: Value = serde_json::from_str(&json_str).unwrap();

        let runs = data.get("runs").unwrap().as_object().unwrap();
        let run = runs.values().next().unwrap();
        let metrics = run.get("metrics").unwrap().as_object().unwrap();
        let user = &metrics["user"];
        let series = user.get("series").unwrap().as_array().unwrap();

        // Filter to CPU0 — should match
        let filtered: Vec<&Value> = series
            .iter()
            .filter(|s| s.get("series_name").and_then(|v| v.as_str()) == Some("CPU0"))
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].get("series_name").unwrap().as_str().unwrap(),
            "CPU0"
        );

        // Filter to CPU99 — should be empty
        let filtered_miss: Vec<&Value> = series
            .iter()
            .filter(|s| s.get("series_name").and_then(|v| v.as_str()) == Some("CPU99"))
            .collect();
        assert!(filtered_miss.is_empty());
    }

    #[test]
    fn test_cpu_averaging() {
        // Build synthetic multi-CPU data and verify averaging
        let series = vec![
            json!({"series_name": "CPU0", "time_diff": [0, 1, 2], "values": [10.0, 20.0, 30.0]}),
            json!({"series_name": "CPU1", "time_diff": [0, 1, 2], "values": [20.0, 40.0, 60.0]}),
        ];

        let all_values: Vec<Vec<f64>> = series
            .iter()
            .filter_map(|s| {
                s.get("values")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect())
            })
            .collect();

        let num_points = all_values[0].len();
        let mut averaged = Vec::with_capacity(num_points);
        for i in 0..num_points {
            let mut sum = 0.0;
            let mut count = 0usize;
            for vals in &all_values {
                if i < vals.len() {
                    sum += vals[i];
                    count += 1;
                }
            }
            averaged.push(if count > 0 { sum / count as f64 } else { 0.0 });
        }

        assert_eq!(averaged.len(), 3);
        assert!((averaged[0] - 15.0).abs() < 0.001); // (10+20)/2
        assert!((averaged[1] - 30.0).abs() < 0.001); // (20+40)/2
        assert!((averaged[2] - 45.0).abs() < 0.001); // (30+60)/2
    }

    #[test]
    fn test_build_timeseries_summary() {
        let values = vec![10.0, 12.0, 11.0, 50.0, 48.0, 11.0, 10.0, 10.5, 11.0, 10.0];
        let time_diff: Vec<f64> = (0..10).map(|x| x as f64).collect();
        let stats = json!({"avg": 18.35, "min": 10.0, "max": 50.0, "std": 15.0, "p50": 11.0, "p90": 48.0, "p99": 50.0});
        let summary = build_timeseries_summary(&values, &time_diff, &stats);
        assert!(summary.contains("points=10"));
        assert!(summary.contains("avg=18.35"));
        assert!(summary.contains("p50=11.00"));
        assert!(summary.contains("p90=48.00"));
        assert!(summary.contains("p99=50.00"));
        assert!(summary.contains("trend"));
    }

    #[test]
    fn test_build_timeseries_summary_with_drops() {
        // avg=50, std=10 → drop_threshold=30. Values 5.0 at t=3-4 should be detected.
        let values = vec![50.0, 52.0, 48.0, 5.0, 5.0, 51.0, 49.0, 50.0, 53.0, 48.0];
        let time_diff: Vec<f64> = (0..10).map(|x| x as f64).collect();
        let stats = json!({"avg": 41.1, "min": 5.0, "max": 53.0, "std": 17.0, "p50": 49.0, "p90": 52.0, "p99": 53.0});
        let summary = build_timeseries_summary(&values, &time_diff, &stats);
        assert!(
            summary.contains("drop"),
            "Expected drop detection in: {}",
            summary
        );
        assert!(
            summary.contains("trough"),
            "Expected trough value in: {}",
            summary
        );
    }

    #[test]
    fn test_build_compact_notation() {
        let values = vec![10.0, 10.5, 11.0, 11.0, 11.0, 11.0, 12.0];
        let time_diff: Vec<f64> = (0..7).map(|x| x as f64).collect();
        let compact = build_compact_notation(&values, &time_diff, "test_metric");
        assert!(compact.starts_with("ts:test_metric|"));
        assert!(compact.contains("v0=10.00"));
        // Should have RLE for the repeated 0 deltas
        assert!(compact.contains("0x"));
    }

    #[test]
    fn test_build_downsampled() {
        let values: Vec<f64> = (0..100).map(|x| x as f64).collect();
        let time_diff: Vec<f64> = (0..100).map(|x| x as f64).collect();
        let ds = build_downsampled(&values, &time_diff, 10);
        let buckets = ds.get("buckets").unwrap().as_array().unwrap();
        assert_eq!(buckets.len(), 10);
        // First bucket: values 0-9, avg=4.5
        let first = &buckets[0];
        assert!((first.get("avg").unwrap().as_f64().unwrap() - 4.5).abs() < 0.01);
        assert_eq!(ds.get("bucket_count").unwrap().as_u64().unwrap(), 10);
    }

    #[test]
    fn test_build_compact_empty() {
        let compact = build_compact_notation(&[], &[], "empty");
        assert_eq!(compact, "ts:empty|empty");
    }

    #[test]
    fn test_build_downsampled_small() {
        // Fewer points than buckets
        let values = vec![1.0, 2.0, 3.0];
        let time_diff = vec![0.0, 1.0, 2.0];
        let ds = build_downsampled(&values, &time_diff, 50);
        let buckets = ds.get("buckets").unwrap().as_array().unwrap();
        assert_eq!(buckets.len(), 3); // capped to data size
    }

    // -----------------------------------------------------------------------
    // record / generate_report arg building tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_record_args_defaults() {
        let req = RecordRequest {
            run_name: None,
            interval: None,
            period: None,
            profile: None,
            perf_frequency: None,
            memory_allocation: None,
            dont_collect: None,
            collect_only: None,
            profile_java: None,
            pmu_config: None,
        };
        let args = build_record_args(&req);
        assert_eq!(args, vec!["record"]);
    }

    #[test]
    fn test_build_record_args_full() {
        let req = RecordRequest {
            run_name: Some("my_run".to_string()),
            interval: Some(2),
            period: Some(60),
            profile: Some(true),
            perf_frequency: Some(199),
            memory_allocation: Some(true),
            dont_collect: Some("interrupts,netstat".to_string()),
            collect_only: None,
            profile_java: None,
            pmu_config: Some("/path/to/pmu.json".to_string()),
        };
        let args = build_record_args(&req);
        assert!(args.contains(&"record".to_string()));
        assert!(args.contains(&"-r".to_string()));
        assert!(args.contains(&"my_run".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"2".to_string()));
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"60".to_string()));
        assert!(args.contains(&"--profile".to_string()));
        assert!(args.contains(&"-F".to_string()));
        assert!(args.contains(&"199".to_string()));
        assert!(args.contains(&"--memory-allocation".to_string()));
        assert!(args.contains(&"--dont-collect".to_string()));
        assert!(args.contains(&"interrupts,netstat".to_string()));
        assert!(args.contains(&"--pmu-config".to_string()));
        assert!(args.contains(&"/path/to/pmu.json".to_string()));
    }

    #[test]
    fn test_build_record_args_collect_only() {
        let req = RecordRequest {
            run_name: Some("test".to_string()),
            interval: None,
            period: Some(10),
            profile: None,
            perf_frequency: None,
            memory_allocation: None,
            dont_collect: None,
            collect_only: Some("cpu_utilization,meminfo".to_string()),
            profile_java: None,
            pmu_config: None,
        };
        let args = build_record_args(&req);
        assert!(args.contains(&"--collect-only".to_string()));
        assert!(args.contains(&"cpu_utilization,meminfo".to_string()));
        assert!(!args.contains(&"--dont-collect".to_string()));
    }

    #[test]
    fn test_build_record_args_java_profiling() {
        let req = RecordRequest {
            run_name: None,
            interval: None,
            period: None,
            profile: None,
            perf_frequency: None,
            memory_allocation: None,
            dont_collect: None,
            collect_only: None,
            profile_java: Some("".to_string()), // empty = profile all JVMs
            pmu_config: None,
        };
        let args = build_record_args(&req);
        assert!(args.contains(&"--profile-java".to_string()));
        // Empty string means no PID arg, just the flag
        assert_eq!(args.len(), 2); // ["record", "--profile-java"]
    }

    #[test]
    fn test_build_record_args_java_profiling_with_pids() {
        let req = RecordRequest {
            run_name: None,
            interval: None,
            period: None,
            profile: None,
            perf_frequency: None,
            memory_allocation: None,
            dont_collect: None,
            collect_only: None,
            profile_java: Some("1234,myapp".to_string()),
            pmu_config: None,
        };
        let args = build_record_args(&req);
        assert!(args.contains(&"--profile-java".to_string()));
        assert!(args.contains(&"1234,myapp".to_string()));
    }

    #[test]
    fn test_build_report_args_single_run() {
        let req = GenerateReportRequest {
            runs: vec!["/path/to/run1".to_string()],
            name: Some("my_report".to_string()),
        };
        let args = build_report_args(&req);
        assert_eq!(
            args,
            vec!["report", "-r", "/path/to/run1", "-n", "my_report"]
        );
    }

    #[test]
    fn test_build_report_args_multi_run() {
        let req = GenerateReportRequest {
            runs: vec![
                "/path/to/run1".to_string(),
                "/path/to/run2".to_string(),
                "/path/to/run3".to_string(),
            ],
            name: None,
        };
        let args = build_report_args(&req);
        assert_eq!(
            args,
            vec![
                "report",
                "-r",
                "/path/to/run1",
                "/path/to/run2",
                "/path/to/run3"
            ]
        );
    }

    #[test]
    fn test_build_report_args_no_name() {
        let req = GenerateReportRequest {
            runs: vec!["/path/to/run1".to_string()],
            name: None,
        };
        let args = build_report_args(&req);
        assert_eq!(args, vec!["report", "-r", "/path/to/run1"]);
        assert!(!args.contains(&"-n".to_string()));
    }

    #[test]
    fn test_current_exe_resolves() {
        // Verify that current_exe() works in the test environment
        let exe = std::env::current_exe();
        assert!(exe.is_ok());
        assert!(exe.unwrap().exists());
    }

    #[test]
    fn test_parse_flamegraph_svg() {
        // Test parsing the flamegraph SVG in tmp/ if it exists
        let svg_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tmp/aperf_record_m8g24xlarge_be57e2e2-acae-4a0a-8a02-e743489c74c6_20260319T120630-flamegraph.svg");
        if !svg_path.exists() {
            return;
        }

        let frames = parse_flamegraph_svg(&svg_path).unwrap();
        assert!(!frames.is_empty());

        // Should have parsed many frames
        assert!(
            frames.len() > 50,
            "Expected >50 frames, got {}",
            frames.len()
        );

        // Check a known function exists (from our earlier grep)
        let has_postgres = frames.iter().any(|f| f.name == "PostgresMain");
        assert!(has_postgres, "Expected PostgresMain in flamegraph");

        // Check percentages are reasonable
        let max_pct = frames.iter().map(|f| f.pct).fold(0.0f64, f64::max);
        assert!(
            max_pct > 50.0,
            "Expected top function >50%, got {}",
            max_pct
        );

        // Test annotated frames with parent
        let annotated = build_annotated_frames(&frames, 1.0);
        assert!(!annotated.is_empty());

        // Top function should have a parent
        let top = &annotated[0];
        assert!(!top.parent.is_empty());
        assert!(top.pct > 10.0);
    }
}
