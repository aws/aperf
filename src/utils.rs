use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tdigest::TDigest;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ValueType {
    UInt64(u64),
    F64(f64),
    String(String),
    Stats(Stats),
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
pub struct Stats {
    pub p99: f64,
    pub p90: f64,
    pub mean: f64,
}

impl Stats {
    fn new() -> Self {
        Stats {
            p99: 0.0,
            p90: 0.0,
            mean: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataMetrics {
    pub run: String,
    pub values: HashMap<String, HashMap<String, ValueType>>,
}

impl DataMetrics {
    pub fn new(run: String) -> Self {
        DataMetrics {
            run,
            values: HashMap::new(),
        }
    }
}

/// Used to generate ValueType::Stats in form_stats().
#[derive(Default, Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub values: Vec<f64>,
    pub stats: Stats,
}

impl Metric {
    pub fn new(name: String) -> Self {
        Metric {
            name,
            values: Vec::new(),
            stats: Stats::new(),
        }
    }

    pub fn insert_value(&mut self, value: f64) {
        self.values.push(value)
    }

    pub fn form_stats(&mut self) -> ValueType {
        let t = TDigest::new_with_size(100);
        let t = t.merge_unsorted(self.values.clone());
        self.stats.p99 = t.estimate_quantile(0.99);
        self.stats.p90 = t.estimate_quantile(0.90);
        self.stats.mean = t.mean();
        ValueType::Stats(self.stats.clone())
    }
}
