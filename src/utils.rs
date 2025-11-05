use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
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

pub fn add_metrics(
    key: String,
    metric: &mut Metric,
    metrics: &mut DataMetrics,
    file: String,
) -> Result<()> {
    if let Some(m) = metrics.values.get_mut(&file) {
        m.insert(key, metric.form_stats());
    } else {
        let mut metric_map = HashMap::new();
        metric_map.insert(key, metric.form_stats());
        metrics.values.insert(file, metric_map);
    }
    Ok(())
}

pub fn get_data_name_from_type<T>() -> &'static str {
    let full_data_module_path = std::any::type_name::<T>();

    let mut data_identifier_found = false;
    let mut data_name: Option<&str> = None;
    for data_module_part in full_data_module_path.split("::") {
        if data_identifier_found {
            data_name = Some(data_module_part);
            break;
        }
        data_identifier_found = data_module_part == "data";
    }

    match data_name {
        Some(value) => value,
        None => panic!("Could not get data name"),
    }
}

/// Perform topological sort on a list of vectors and produce an ordered vector. Every input vector
/// represents the order between its contained values.
pub fn topological_sort(inputs: &Vec<&Vec<String>>) -> Result<Vec<String>> {
    let mut dependency_graph: HashMap<String, HashSet<String>> = HashMap::new();
    let mut in_degree_map: HashMap<String, u64> = HashMap::new();

    for &input in inputs {
        for (index, item) in input.iter().enumerate() {
            in_degree_map.insert(item.clone(), 0);
            if !dependency_graph.contains_key(item) {
                dependency_graph.insert(item.clone(), HashSet::new());
            }
            if index > 0 {
                let parent_dependencies = dependency_graph
                    .get_mut(input.get(index - 1).unwrap())
                    .unwrap();
                parent_dependencies.insert(item.clone());
            }
        }
    }
    for dependencies in dependency_graph.values() {
        for dependency in dependencies {
            *in_degree_map.get_mut(dependency).unwrap() += 1;
        }
    }

    let mut result: Vec<String> = Vec::new();

    let mut queue: VecDeque<String> = VecDeque::new();
    for (item, in_degree) in &in_degree_map {
        if *in_degree == 0 {
            queue.push_back(item.clone());
        }
    }

    while !queue.is_empty() {
        let cur_item = queue.pop_front().unwrap();
        result.push(cur_item.clone());
        for dependency in dependency_graph.get(&cur_item).unwrap() {
            let dependency_in_degree = in_degree_map.get_mut(dependency).unwrap();
            *dependency_in_degree -= 1;
            if *dependency_in_degree == 0 {
                queue.push_back(dependency.clone());
            }
        }
    }

    if result.len() != dependency_graph.len() {
        return Err(Error::msg(
            "Conflicting orders in inputs. Cannot perform topological sort.",
        ));
    }

    Ok(result)
}

/// Combine a list of input value ranges into one value range. The result value range's min is
/// the minimum of all value ranges' min, and its max is the maximum of all value ranges' max
pub fn combine_value_ranges(value_ranges: Vec<(u64, u64)>) -> (u64, u64) {
    if value_ranges.is_empty() {
        return (0, 0);
    }

    let mut min = value_ranges[0].0;
    let mut max = value_ranges[0].1;
    for value_range in value_ranges {
        min = min.min(value_range.0);
        max = max.max(value_range.1);
    }

    (min, max)
}

#[cfg(test)]
mod utils_test {
    use crate::utils::{combine_value_ranges, topological_sort};

    #[test]
    fn test_topological_sort_fixed_result() {
        let inputs_raw: Vec<Vec<String>> = vec![
            vec!["a", "b", "d", "g", "i", "j"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["b", "c", "d", "f", "h", "i"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["a", "d", "e", "g", "h", "j", "k"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["c", "e", "f"].iter().map(|&s| s.to_string()).collect(),
            vec!["f", "g"].iter().map(|&s| s.to_string()).collect(),
        ];

        let mut inputs: Vec<&Vec<String>> = Vec::new();
        for input_raw in &inputs_raw {
            inputs.push(input_raw);
        }

        if let Ok(output) = topological_sort(&inputs) {
            assert_eq!(
                output,
                vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"]
            );
        } else {
            panic!("Conflicting orders in inputs");
        }
    }

    #[test]
    fn test_topological_sort_multiple_result() {
        let inputs_raw: Vec<Vec<String>> = vec![
            vec!["apple", "orange", "pear"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["watermelon", "grape"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["orange", "grape", "peach", "avocado", "pear", "dragonfruit"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["apple", "peach", "pear"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["strawberry", "apple"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["apple", "watermelon"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
        ];

        let mut inputs: Vec<&Vec<String>> = Vec::new();
        for input_raw in &inputs_raw {
            inputs.push(input_raw);
        }

        let possible_outputs: Vec<Vec<String>> = vec![
            vec![
                "strawberry",
                "apple",
                "orange",
                "watermelon",
                "grape",
                "peach",
                "avocado",
                "pear",
                "dragonfruit",
            ],
            vec![
                "strawberry",
                "apple",
                "watermelon",
                "orange",
                "grape",
                "peach",
                "avocado",
                "pear",
                "dragonfruit",
            ],
        ]
        .iter()
        .map(|possible_output| possible_output.iter().map(|&s| s.to_string()).collect())
        .collect();

        if let Ok(output) = topological_sort(&inputs) {
            assert!(
                possible_outputs.iter().any(|expected| expected == &output),
                "Expected {:?} to be one of {:?}",
                output,
                possible_outputs,
            )
        } else {
            panic!("Conflicting orders in inputs");
        }
    }

    #[test]
    fn test_topological_sort_circular_dependency() {
        let inputs_raw: Vec<Vec<String>> = vec![
            vec!["IAD", "PDX", "DUB", "NRT", "SYD", "FRA"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["HKG", "DUB", "CMH", "KUL"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["BOM", "CMH", "PDX"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["SIN", "FRA"].iter().map(|&s| s.to_string()).collect(),
            vec!["ZAZ"].iter().map(|&s| s.to_string()).collect(),
        ];

        let mut inputs: Vec<&Vec<String>> = Vec::new();
        for input_raw in &inputs_raw {
            inputs.push(input_raw);
        }

        if let Ok(output) = topological_sort(&inputs) {
            panic!(
                "Expected conflicting orders in inputs, but got output {:?}",
                output
            );
        }
    }

    #[test]
    fn test_combine_value_ranges() {
        let ranges: Vec<(u64, u64)> = vec![];
        assert_eq!(combine_value_ranges(ranges), (0, 0));

        let ranges = vec![(5, 10)];
        assert_eq!(combine_value_ranges(ranges), (5, 10));

        let ranges = vec![(5, 10), (3, 8), (7, 15)];
        assert_eq!(combine_value_ranges(ranges), (3, 15));

        let ranges = vec![(1, 5), (3, 7), (4, 6)];
        assert_eq!(combine_value_ranges(ranges), (1, 7));

        let ranges = vec![(0, 5), (3, 7), (4, 6)];
        assert_eq!(combine_value_ranges(ranges), (0, 7));

        let ranges = vec![(5, u64::MAX), (3, 7), (4, 6)];
        assert_eq!(combine_value_ranges(ranges), (3, u64::MAX));

        let ranges = vec![(5, 10), (5, 15), (5, 8)];
        assert_eq!(combine_value_ranges(ranges), (5, 15));

        let ranges = vec![(5, 10), (3, 10), (7, 10)];
        assert_eq!(combine_value_ranges(ranges), (3, 10));

        let ranges = vec![(5, 5), (5, 5), (5, 5)];
        assert_eq!(combine_value_ranges(ranges), (5, 5));
    }
}
