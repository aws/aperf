use crate::computations::{serialize_f64_vec_fixed2, Statistics};
use crate::profiling::{Profile, BUCKET_WIDTH_MS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::Display;

/// This module defines generalized data types of all Aperf processed data used by the analytical
/// engines and frontend JavaScripts. Before introducing a new data type, ensure that it can be
/// processed into one of the formats defined here.

/// The identifier of the data format, which is used by the frontend to easily
/// recognize and parse the processed data.
#[derive(Serialize, Deserialize, Debug, Display, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DataFormat {
    TimeSeries,
    Text,
    KeyValue,
    Profile,
    Graph,
    Unknown,
}

/// The struct holding processed data across all runs for a data type
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessedData {
    pub data_name: String,
    pub data_format: DataFormat,
    pub runs: HashMap<String, AperfData>,
}

impl ProcessedData {
    pub fn new(data_name: String) -> Self {
        ProcessedData {
            data_name,
            data_format: DataFormat::Unknown,
            runs: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum AperfData {
    TimeSeries(TimeSeriesData),
    Text(TextData),
    KeyValue(KeyValueData),
    Profile(ProfilingData),
    Graph(GraphData),
}

impl AperfData {
    pub fn get_format_name(&self) -> DataFormat {
        match self {
            AperfData::TimeSeries(_) => DataFormat::TimeSeries,
            AperfData::Text(_) => DataFormat::Text,
            AperfData::KeyValue(_) => DataFormat::KeyValue,
            AperfData::Profile(_) => DataFormat::Profile,
            AperfData::Graph(_) => DataFormat::Graph,
        }
    }
}

// --------------------------------------- TIME-SERIES DATA ----------------------------------------
/// Data types falling into this format collect system metrics periodically during the recording
/// run and produce time series graphs in the report. Every data type contains multiple metrics,
/// and every metric could contain multiple time series. The report renders every metric as a
/// graph and plots every time series in the metric as a line.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TimeSeriesData {
    /// A map from the metric name to the metric's contents.
    pub metrics: HashMap<String, TimeSeriesMetric>,
    /// A list of all metric names to provide ordering for the graphs in the frontend.
    #[serde(default)]
    pub sorted_metric_names: Vec<String>,
}

/// Contents of a metric, which is to be rendered as a graph in the report.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeSeriesMetric {
    /// Name of the metric.
    pub metric_name: String,
    /// A list of all time series included in the metric.
    pub series: Vec<Series>,
    /// For quick access of the series responsible for the statistic computation of the metric.
    /// Skip serialization since it is not used by the frontend.
    #[serde(skip)]
    pub stats_series_idx: usize,
    /// The minimum and maximum data point values across all series. It offloads the computation
    /// from frontend and help decide the y-axis range of the graphs.
    pub value_range: (u64, u64),
    /// The statistics of the time series included in the metric. If there are multiple time
    /// series, use the statistics of the aggregate one.
    pub stats: Statistics,
}

impl TimeSeriesMetric {
    pub fn new(metric_name: String) -> Self {
        TimeSeriesMetric {
            metric_name,
            series: Default::default(),
            stats_series_idx: 0,
            value_range: Default::default(),
            stats: Default::default(),
        }
    }
}

/// Contents of a time series, which is to be rendered as a line in the graph.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Series {
    /// The name of the series.
    pub series_name: String,
    /// The list of all time (x-axis) values.
    pub time_diff: Vec<u64>,
    /// The list of all data (y-axis) values.
    #[serde(serialize_with = "serialize_f64_vec_fixed2")]
    pub values: Vec<f64>,
    /// Indicate whether the series is aggregate.
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub is_aggregate: bool,
}

impl Series {
    pub fn new(series_name: String) -> Self {
        Series {
            series_name,
            time_diff: Vec::new(),
            values: Vec::new(),
            is_aggregate: false,
        }
    }
}

// allow skipping serializing a bool field if it's false
fn is_false(value: &bool) -> bool {
    !(*value)
}

// ---------------------------------------- KEY-VALUE DATA -----------------------------------------
/// Data types falling into this format collect information about the system once per recording
/// run, and the format of the collected data is key-value pairs. The report presents every pair
/// and check for differences in values with the same key across different runs. In some data
/// types the keys are categorized into different groups.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KeyValueData {
    /// A map from a group name to the key-value pairs within the group. If the data type is
    /// not grouped, map all key-value pairs to an empty string.
    pub key_value_groups: HashMap<String, KeyValueGroup>,
}

/// All key-value pairs within the group.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KeyValueGroup {
    pub key_values: HashMap<String, String>,
}

// ------------------------------------------- TEXT DATA -------------------------------------------
/// Data types falling into this format produce human-readable, formatted, texts, which can be
/// displayed in the report directly.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TextData {
    /// All lines of the text content.
    pub lines: Vec<String>,
}

// ------------------------------------------ GRAPH DATA -------------------------------------------
/// TODO: Temporary format only used by hotline to ship pre-rendered HTML/SVG tables to the frontend.
///
/// Data types falling into this format produce one or more HTML or SVG files at the end of a
/// recording run, which are to be rendered through IFrame in the report. The graphs can be
/// categorized into different groups, so that only one group of graphs are shown in the report
/// at a time.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct GraphData {
    pub graph_groups: Vec<GraphGroup>,
}

/// Contents of a graph group, which contains all graphs to be displayed together.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct GraphGroup {
    /// Name of the graph group.
    pub group_name: String,
    /// A map from graph names to all graphs within the group.
    pub graphs: HashMap<String, Graph>,
}

impl GraphGroup {
    pub fn new(graph_group_name: &str) -> Self {
        GraphGroup {
            group_name: graph_group_name.to_string(),
            graphs: HashMap::new(),
        }
    }
}

/// Information about a graph.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Graph {
    /// The name of the graph.
    pub graph_name: String,
    /// The relative path to graph (value of the IFrame's src attribute).
    pub graph_path: String,
    /// The size of the graph, which can be used for graph ordering in the report.
    pub graph_size: Option<u64>,
}

impl Graph {
    pub fn new(graph_name: String, graph_path: String, graph_size: Option<u64>) -> Self {
        Graph {
            graph_name,
            graph_path,
            graph_size,
        }
    }
}

// ---------------------------------------- PROFILING DATA ------------------------------------------
/// Data types falling into this format collect profiling data from one or more profiled
/// targets (e.g. JVMs for java profiling, and system for perf profiling). Each target is
/// represented by a [`Profiler`] holding metadata and a map of [`Profile`]s keyed by profiling
/// type (e.g. "cpu", "wall", "allocation"). Each [`Profile`] carries:
///     - Time bucketed sample data with total counts for the entire profile
///     - Calling context tree used to analyze a selected time range
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProfilingData {
    /// Map from profiler name to its profiler data
    pub profilers: HashMap<String, Profiler>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profiler {
    /// Start time of the profile in milliseconds since epoch
    pub start_time_ms: i64,
    /// Duration of each block in milliseconds
    pub block_width_ms: u64,
    /// Additional metadata (e.g., source, architecture, JVM version)
    pub metadata: KeyValueData,
    /// Profiling type (e.g., "cpu", "wall", "allocation") -> Profile
    pub profiles: HashMap<String, Profile>,
}

impl Default for Profiler {
    fn default() -> Self {
        Profiler {
            start_time_ms: 0,
            block_width_ms: BUCKET_WIDTH_MS,
            metadata: KeyValueData::default(),
            profiles: HashMap::new(),
        }
    }
}

impl Profiler {
    pub fn new(start_time_ms: i64) -> Self {
        Profiler {
            start_time_ms,
            block_width_ms: BUCKET_WIDTH_MS,
            metadata: KeyValueData::default(),
            profiles: HashMap::new(),
        }
    }
}
