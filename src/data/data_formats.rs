use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/**
* This module defines generalized data types of all Aperf processed data used by the
* frontend JavaScripts.
*/

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum AperfData {
    TimeSeries(TimeSeriesData),
    Text(TextData),
    KeyValue(KeyValueData),
    Graph(GraphData),
}

impl AperfData {
    pub fn get_format_name(&self) -> String {
        match self {
            AperfData::TimeSeries(_) => "time_series".to_string(),
            AperfData::Text(_) => "text".to_string(),
            AperfData::KeyValue(_) => "key_value".to_string(),
            AperfData::Graph(_) => "graph".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TimeSeriesData {
    pub metrics: HashMap<String, TimeSeriesMetric>,
    pub sorted_keys: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TimeSeriesMetric {
    pub series: Vec<Series>,
    pub metadata: HashMap<String, String>,
    pub stats: Statistics,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Series {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_name: Option<String>,
    pub time_diff: Vec<u64>,
    pub values: Vec<u64>,
}

impl Series {
    pub fn new(series_name: Option<String>) -> Self {
        Series {
            series_name,
            time_diff: Vec::new(),
            values: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Statistics {
    pub avg: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
    pub p99_9: f64,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            avg: 0.0,
            std: 0.0,
            min: 0.0,
            max: 0.0,
            p50: 0.0,
            p90: 0.0,
            p99: 0.0,
            p99_9: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TextData {
    pub lines: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct KeyValueData {
    pub key_value_groups: HashMap<String, KeyValueGroup>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyValueGroup {
    pub key_values: HashMap<String, String>,
}

impl KeyValueGroup {
    pub fn new() -> Self {
        KeyValueGroup {
            key_values: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct GraphData {
    pub graph_groups: HashMap<String, GraphGroup>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct GraphGroup {
    pub group_name: String,
    pub graphs: HashMap<String, Graph>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Graph {
    pub graph_name: String,
    pub graph_path: String,
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
