use serde::{Deserialize, Serialize, Serializer};
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
    Graph(GraphData),
}

impl AperfData {
    pub fn get_format_name(&self) -> DataFormat {
        match self {
            AperfData::TimeSeries(_) => DataFormat::TimeSeries,
            AperfData::Text(_) => DataFormat::Text,
            AperfData::KeyValue(_) => DataFormat::KeyValue,
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
    pub sorted_metric_names: Vec<String>,
}

/// Contents of a metric, which is to be rendered as a graph in the report.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TimeSeriesMetric {
    /// Name of the metric.
    pub metric_name: String,
    /// A list of all time series included in the metric.
    pub series: Vec<Series>,
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
            value_range: Default::default(),
            stats: Default::default(),
        }
    }
}

/// Contents of a time series, which is to be rendered as a line in the graph.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Series {
    /// The name of the series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_name: Option<String>,
    /// The list of all time (x-axis) values.
    pub time_diff: Vec<u64>,
    /// The list of all data (y-axis) values.
    #[serde(serialize_with = "serialize_f64_vec_fixed2")]
    pub values: Vec<f64>,
    /// Indicate whether the series is aggregate.
    #[serde(skip_serializing_if = "is_false")]
    pub is_aggregate: bool,
}

impl Series {
    pub fn new(series_name: Option<String>) -> Self {
        Series {
            series_name,
            time_diff: Vec::new(),
            values: Vec::new(),
            is_aggregate: false,
        }
    }
}

/// Different statistics of the values contained in a Series
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Statistics {
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub avg: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub std: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub min: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub max: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p50: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p90: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p99: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p99_9: f64,
}

impl Statistics {
    pub fn from_values(values: &Vec<f64>) -> Self {
        let n = values.len();
        if n == 0 {
            return Self::default();
        }

        let mut sum = 0.0;
        let mut min = values[0];
        let mut max = values[0];
        for &value in values {
            sum += value;
            min = min.min(value);
            max = max.max(value);
        }
        let avg = sum / n as f64;

        let mut sum_sq_diff = 0.0;
        for &value in values {
            let diff = value - avg;
            sum_sq_diff += diff * diff;
        }
        let std = (sum_sq_diff / n as f64).sqrt();

        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.total_cmp(b));
        let p50 = sorted_values[(0.5 * n as f64).floor() as usize];
        let p90 = sorted_values[(0.9 * n as f64).floor() as usize];
        let p99 = sorted_values[(0.99 * n as f64).floor() as usize];
        let p99_9 = sorted_values[(0.999 * n as f64).floor() as usize];

        Self {
            avg,
            std,
            min,
            max,
            p50,
            p90,
            p99,
            p99_9,
        }
    }
}

// custom Serde serializations
// allow f64 values to be truncated to 2 decimal places (to save spaces)
#[derive(Serialize)]
struct Fixed2(f64);
impl Fixed2 {
    fn new(value: f64) -> Self {
        Fixed2(f64::trunc(value * 100.0) / 100.0)
    }
}
// custom serializing function for Vec<f64> to truncate all elements to 2 decimal places
fn serialize_f64_vec_fixed2<S>(values: &[f64], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    values
        .iter()
        .map(|&v| Fixed2::new(v))
        .collect::<Vec<_>>()
        .serialize(serializer)
}
// custom serializing function for f64 to truncate all elements to 2 decimal places
fn serialize_f64_fixed2<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(Fixed2::new(*value).0)
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

// ------------------------------------------ GRAPH DATA -------------------------------------------
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

// ------------------------------------------- TEXT DATA -------------------------------------------
/// Data types falling into this format produce human-readable, formatted, texts, which can be
/// displayed in the report directly.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TextData {
    /// All lines of the text content.
    pub lines: Vec<String>,
}
