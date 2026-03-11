use crate::data::data_formats::TimeSeriesMetric;
use anyhow::{Error, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

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

#[derive(Clone, Debug)]
pub struct CpuInfo {
    pub vendor: String,
    pub model_name: String,
}

impl CpuInfo {
    fn new() -> Self {
        CpuInfo {
            vendor: String::new(),
            model_name: String::new(),
        }
    }
}

pub fn get_cpu_info() -> Result<CpuInfo> {
    let file = File::open("/proc/cpuinfo")?;
    let proc_cpuinfo = BufReader::new(file);
    let mut cpu_info = CpuInfo::new();
    for line in proc_cpuinfo.lines() {
        let info_line = line?;
        if info_line.is_empty() {
            break;
        }
        let key_value: Vec<&str> = info_line.split(':').collect();
        if key_value.len() < 2 {
            continue;
        }
        let key = key_value[0].trim().to_string();
        let value = key_value[1].trim().to_string();
        match key.as_str() {
            "vendor_id" => cpu_info.vendor = value,
            "model name" => cpu_info.model_name = value,
            _ => {}
        }
    }
    Ok(cpu_info)
}

pub fn no_tar_gz_file_name(path: &PathBuf) -> Option<String> {
    if path.file_name().is_none() {
        return None;
    }

    let file_name_str = path.file_name()?.to_str()?.to_string();

    if file_name_str.ends_with(".tar.gz") {
        return Some(file_name_str.strip_suffix(".tar.gz")?.to_string());
    }
    Some(file_name_str)
}

pub fn get_cpu_series_name(cpu: usize) -> Option<String> {
    Some(format!("CPU{cpu}"))
}

pub fn get_aggregate_series_name() -> Option<String> {
    Some("Aggregate".to_string())
}

/// If a time series metric only has zero values, compress the data by only showing the
/// first and last data points of every series
pub fn compress_all_zero_time_series_metric(time_series_metric: &mut TimeSeriesMetric) {
    if time_series_metric.stats.min == 0.0 && time_series_metric.stats.max == 0.0 {
        for series in &mut time_series_metric.series {
            let time_diff_len = series.time_diff.len();
            if time_diff_len == 0 {
                continue;
            }
            let mut compressed_time_diffs: Vec<u64> = vec![series.time_diff[0]];
            let mut compressed_values: Vec<f64> = vec![0.0];
            if time_diff_len > 1 {
                compressed_time_diffs.push(series.time_diff[time_diff_len - 1]);
                compressed_values.push(0.0);
            }
            series.time_diff = compressed_time_diffs;
            series.values = compressed_values;
        }
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
    use super::{combine_value_ranges, compress_all_zero_time_series_metric, topological_sort};
    use crate::computations::Statistics;
    use crate::data::data_formats::{Series, TimeSeriesMetric};

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

    #[test]
    fn test_compress_all_zero_time_series_metric_with_zeros() {
        let mut metric = TimeSeriesMetric {
            metric_name: "test_metric".to_string(),
            series: vec![
                Series {
                    series_name: Some("series1".to_string()),
                    time_diff: vec![0, 1, 2, 3, 4],
                    values: vec![0.0, 0.0, 0.0, 0.0, 0.0],
                    is_aggregate: false,
                },
                Series {
                    series_name: Some("series2".to_string()),
                    time_diff: vec![0, 1, 2],
                    values: vec![0.0, 0.0, 0.0],
                    is_aggregate: false,
                },
            ],
            value_range: (0, 0),
            stats: Statistics {
                min: 0.0,
                max: 0.0,
                ..Default::default()
            },
        };

        compress_all_zero_time_series_metric(&mut metric);

        assert_eq!(metric.series[0].time_diff, vec![0, 4]);
        assert_eq!(metric.series[0].values, vec![0.0, 0.0]);
        assert_eq!(metric.series[1].time_diff, vec![0, 2]);
        assert_eq!(metric.series[1].values, vec![0.0, 0.0]);
    }

    #[test]
    fn test_compress_all_zero_time_series_metric_with_non_zeros() {
        let mut metric = TimeSeriesMetric {
            metric_name: "test_metric".to_string(),
            series: vec![Series {
                series_name: Some("series1".to_string()),
                time_diff: vec![0, 1, 2, 3, 4],
                values: vec![0.0, 1.0, 2.0, 3.0, 4.0],
                is_aggregate: false,
            }],
            value_range: (0, 4),
            stats: Statistics {
                min: 0.0,
                max: 4.0,
                ..Default::default()
            },
        };

        compress_all_zero_time_series_metric(&mut metric);

        assert_eq!(metric.series[0].time_diff, vec![0, 1, 2, 3, 4]);
        assert_eq!(metric.series[0].values, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_compress_all_zero_time_series_metric_single_point() {
        let mut metric = TimeSeriesMetric {
            metric_name: "test_metric".to_string(),
            series: vec![Series {
                series_name: Some("series1".to_string()),
                time_diff: vec![0],
                values: vec![0.0],
                is_aggregate: false,
            }],
            value_range: (0, 0),
            stats: Statistics {
                min: 0.0,
                max: 0.0,
                ..Default::default()
            },
        };

        compress_all_zero_time_series_metric(&mut metric);

        assert_eq!(metric.series[0].time_diff, vec![0]);
        assert_eq!(metric.series[0].values, vec![0.0]);
    }

    #[test]
    fn test_compress_all_zero_time_series_metric_empty_series() {
        let mut metric = TimeSeriesMetric {
            metric_name: "test_metric".to_string(),
            series: vec![Series {
                series_name: Some("series1".to_string()),
                time_diff: vec![],
                values: vec![],
                is_aggregate: false,
            }],
            value_range: (0, 0),
            stats: Statistics {
                min: 0.0,
                max: 0.0,
                ..Default::default()
            },
        };

        compress_all_zero_time_series_metric(&mut metric);

        assert_eq!(metric.series[0].time_diff, Vec::<u64>::new());
        assert_eq!(metric.series[0].values, Vec::<f64>::new());
    }
}
