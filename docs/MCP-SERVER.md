# APerf MCP Server

APerf includes a built-in [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server that lets AI assistants record performance data, generate reports, and analyze APerf reports interactively.

## Usage

```bash
aperf server --mcp
```

This starts the MCP server on stdio. It's designed to be launched by an MCP client (Kiro, Claude Desktop, etc.), not run manually.

## MCP Client Configuration

### Kiro (`~/.kiro/settings/mcp.json`)

```json
{
  "mcpServers": {
    "aperf-mcp": {
      "command": "/path/to/aperf",
      "args": ["server", "--mcp"],
      "disabled": false
    }
  }
}
```

Replace `/path/to/aperf` with the actual binary path.

## Available Tools

### Read-only tools (report analysis)

#### `load_report`

Load an APerf report and return metadata + available metrics. Must be called first before other analysis tools.

| Parameter | Type | Description |
|---|---|---|
| `report_path` | string | Absolute or relative path to the APerf report directory |

#### `get_metrics`

Query available metrics from the loaded report. Supports filtering.

| Parameter | Type | Description |
|---|---|---|
| `file_name` | string (optional) | JS filename, e.g. `"cpu_utilization.js"` |
| `category` | string (optional) | Data category, e.g. `"meminfo"` |

#### `get_metric_values`

Retrieve actual data values for a specific metric.

| Parameter | Type | Description |
|---|---|---|
| `metric_name` | string | Name of the metric (e.g. `"user"`, `"system"`) |
| `file_name` | string (optional) | JS filename containing the metric |
| `category` | string (optional) | Category/data_name (e.g. `"cpu_utilization"`) |
| `output_type` | string (optional) | Output format (default: `summary`, see below) |
| `cpu_ids` | string[] (optional) | Filter to specific CPUs. If omitted, averages across all. |
| `run_id` | string (optional) | Specific run ID (default: all runs) |
| `from_time` | int (optional) | Start of time range in seconds. Negative = relative to end (e.g. -60 = last 60s) |
| `to_time` | int (optional) | End of time range in seconds. Negative = relative to end (e.g. -10 = stop 10s before end) |

**Output types:**

| Type | Description | Token cost |
|------|-------------|------------|
| `summary` | Smart text with all stats (avg/min/max/std/p50/p90/p99), trend direction, spike and drop detection | Minimal (default) |
| `stats` | Aggregated statistics only (avg, std, min, max, p50, p90, p99, p99.9) | Minimal |
| `timeseries` | Full raw time-series data with timestamps and values | High |
| `compact` | Delta-encoded notation with run-length encoding | ~80% less than timeseries |
| `downsampled` | Fixed 50 buckets with min/avg/max per bucket | Bounded |

#### `get_analytical_findings`

Get analytical findings from the report's analytical engine. Findings represent rule-based detections (regressions, improvements, configuration mismatches). Sorted by absolute score (severity) descending.

| Parameter | Type | Description |
|---|---|---|
| `offset` | int (optional) | Start index for pagination, default 0 |
| `limit` | int (optional) | Max results per page, default 50 |

#### `get_statistical_findings`

Get statistical findings (metric stat deltas between runs). Computes the percentage change of each time-series metric's statistics compared to the base run (first run). Sorted by absolute delta descending. Requires a multi-run report.

| Parameter | Type | Description |
|---|---|---|
| `offset` | int (optional) | Start index for pagination, default 0 |
| `limit` | int (optional) | Max results per page, default 50 |
| `stat` | string (optional) | Filter by stat type: `avg`, `std`, `min`, `max`, `p50`, `p90`, `p99`, `p99_9` |
| `data_type` | string (optional) | Filter by data category (e.g. `cpu_utilization`, `meminfo`) |
| `min_delta_pct` | float (optional) | Minimum absolute delta percentage to include (e.g. 5.0) |

#### `get_flamegraph`

Query flamegraph data from the loaded report. Returns top functions by default (sorted by percentage), with optional regex filtering to search for specific functions. Supports normal, reverse, diff, and reverse-diff modes. Returns data for all runs by default, or specific runs if `run_id` is provided. In diff/reverse-diff mode, `run_id` must contain exactly 2 run IDs.

| Parameter | Type | Description |
|---|---|---|
| `flamegraph_type` | string (optional) | `normal` (default), `reverse`, `diff`, or `reverse-diff` |
| `run_id` | string[] (optional) | Run ID(s). Normal/reverse: omit for all runs, or list specific ones. Diff: exactly 2 IDs `[base, comparison]`, or omit for first two. |
| `limit` | int (optional) | Max functions to return per run, default 30 |
| `min_pct` | float (optional) | Minimum percentage threshold (default 0.1). In diff mode, min absolute delta % (default 0.5). |
| `filter` | string (optional) | Regex filter for function names (case-insensitive). Example: `"compact\|migrate"` |

**Examples:**

```
# Top 10 hottest functions across all runs
get_flamegraph(limit=10)

# Search for compaction-related functions
get_flamegraph(filter="compact|kcompactd|migrate")

# Reverse flamegraph for a specific run, filtered
get_flamegraph(flamegraph_type="reverse", run_id=["run1"], filter="lock")

# Diff between two runs — what got hotter/cooler
get_flamegraph(flamegraph_type="diff")

# Diff with explicit run IDs, filtered
get_flamegraph(flamegraph_type="diff", run_id=["run1", "run2"], filter="compact|writeback")

# Reverse diff
get_flamegraph(flamegraph_type="reverse-diff", run_id=["run1", "run2"])
```

### Write tools (data collection and report generation)

#### `record`

Record performance data on the current system. Runs `aperf record` as a subprocess.

| Parameter | Type | Description |
|---|---|---|
| `run_name` | string (optional) | Name of the run (default: `aperf_<timestamp>`) |
| `interval` | int (optional) | Collection interval in seconds (default: 1) |
| `period` | int (optional) | Recording duration in seconds (default: 10) |
| `profile` | bool (optional) | Enable CPU profiling using perf (default: false) |
| `perf_frequency` | int (optional) | Perf profiling frequency in Hz (default: 99) |
| `memory_allocation` | bool (optional) | Collect memory allocation data (default: false) |
| `dont_collect` | string (optional) | Comma-separated data types to skip |
| `collect_only` | string (optional) | Comma-separated data types to collect exclusively |
| `profile_java` | string (optional) | Profile JVMs — empty for all, or comma-separated PIDs/names |
| `pmu_config` | string (optional) | Path to custom PMU config file |

> Requires Linux and appropriate kernel permissions. See the main README for details.

#### `generate_report`

Generate an HTML report from one or more recorded APerf runs. For multi-run comparison, provide multiple run paths — the first run is used as the base for statistical findings.

| Parameter | Type | Description |
|---|---|---|
| `runs` | string[] | Paths to run directories or archives (at least one required) |
| `name` | string (optional) | Report name (default: `aperf_report_<timestamp>`) |

## Building

The MCP server is included in the default `aperf` build:

```bash
cargo build --release
# Binary at target/release/aperf
# Usage: aperf server --mcp
```

## Architecture

The MCP server code lives in `src/server/mcp/`:

```
src/server/
├── mod.rs              ← Server subcommand dispatch
└── mcp/
    ├── mod.rs          ← MCP entry point (tokio runtime + stdio transport)
    ├── tools.rs        ← Tool definitions (#[tool_router] / #[tool_handler])
    ├── report.rs       ← Report loading, validation, metric extraction
    ├── metadata.rs     ← systeminfo.js parsing
    └── js_parser.rs    ← JS variable prefix stripping + JSON extraction
```

MCP dependencies (`rmcp`, `schemars`) are feature-gated behind the `mcp` feature (enabled by default).

## Running Tests

```bash
# All MCP server tests
cargo test --lib server

# Specific test
cargo test --lib server::mcp::tools::tests::test_build_record_args_full
```
