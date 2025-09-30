use serde::{Deserialize, Serialize};

/**
* This module defines generalized data types of all Aperf processed data used by the
* frontend JavaScripts.
*/

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct HtmlData {
    pub data_type: String,
    pub graphs: Vec<HtmlDataGraph>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HtmlDataGraph {
    pub graph_name: String,
    pub graph_path: String,
    pub graph_size: Option<u64>,
}

impl HtmlDataGraph {
    pub fn new(graph_name: String, graph_path: String, graph_size: Option<u64>) -> Self {
        HtmlDataGraph {
            graph_name,
            graph_path,
            graph_size,
        }
    }
}
