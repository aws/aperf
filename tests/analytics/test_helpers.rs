use aperf::computations::Statistics;
use aperf::data::data_formats::{
    AperfData, KeyValueData, KeyValueGroup, ProcessedData, Series, TimeSeriesData, TimeSeriesMetric,
};

/// Creates a TimeSeriesData with the specified metrics.
/// Each metric can have multiple series. The last series in each metric is marked as aggregate.
///
/// # Arguments
/// * `metrics` - Vector of (metric_name, Vec<(series_name, values)>)
///   - If series_name is None, the series has no name
///   - The last series in each metric is marked as the aggregate series
pub fn create_time_series_data_multi_series(
    metrics: Vec<(&str, Vec<(Option<&str>, Vec<f64>)>)>,
) -> TimeSeriesData {
    let mut ts_data = TimeSeriesData::default();
    for (metric_name, series_list) in metrics {
        let mut metric = TimeSeriesMetric::new(metric_name.to_string());
        let num_series = series_list.len();

        for (idx, (series_name, values)) in series_list.into_iter().enumerate() {
            let is_aggregate = idx == num_series - 1;
            let series = Series {
                series_name: series_name.map(|s| s.to_string()),
                time_diff: (0..values.len() as u64).collect(),
                values: values.clone(),
                is_aggregate,
            };

            if is_aggregate {
                metric.stats = Statistics::from_values(&values);
            }

            metric.series.push(series);
        }

        ts_data.metrics.insert(metric_name.to_string(), metric);
    }
    ts_data
}

/// Creates a TimeSeriesData with single series per metric (for backward compatibility).
/// Each metric has one aggregate series with the provided values.
///
/// # Arguments
/// * `metrics` - Vector of (metric_name, values)
pub fn create_time_series_data(metrics: Vec<(&str, Vec<f64>)>) -> TimeSeriesData {
    let multi_series: Vec<(&str, Vec<(Option<&str>, Vec<f64>)>)> = metrics
        .into_iter()
        .map(|(name, values)| (name, vec![(None, values)]))
        .collect();
    create_time_series_data_multi_series(multi_series)
}

/// Creates a KeyValueData with the specified key-value pairs.
/// All pairs are placed in a single unnamed group.
///
/// # Arguments
/// * `key_values` - Vector of (key, value) tuples
pub fn create_key_value_data(key_values: Vec<(&str, &str)>) -> KeyValueData {
    let mut kv_data = KeyValueData::default();
    let mut group = KeyValueGroup::default();
    for (key, value) in key_values {
        group.key_values.insert(key.to_string(), value.to_string());
    }
    kv_data.key_value_groups.insert(String::new(), group);
    kv_data
}

/// Creates a ProcessedData containing data from multiple runs.
///
/// # Arguments
/// * `data_name` - Name of the data type
/// * `runs` - Vector of (run_name, aperf_data) tuples
pub fn create_processed_data(data_name: &str, runs: Vec<(&str, AperfData)>) -> ProcessedData {
    let mut processed_data = ProcessedData::new(data_name.to_string());
    for (run_name, aperf_data) in runs {
        processed_data.data_format = aperf_data.get_format_name();
        processed_data.runs.insert(run_name.to_string(), aperf_data);
    }
    processed_data
}

/// Test-only extension trait for DataFindings to access private fields
pub trait DataFindingsExt {
    fn has_findings_for_run(&self, run_name: &str) -> bool;
    fn num_runs_with_findings(&self) -> usize;
}

impl DataFindingsExt for aperf::analytics::DataFindings {
    fn has_findings_for_run(&self, run_name: &str) -> bool {
        // Access via serialization to avoid exposing private fields
        let json = serde_json::to_value(self).unwrap();
        json["per_run_findings"]
            .as_object()
            .map(|obj| obj.contains_key(run_name))
            .unwrap_or(false)
    }

    fn num_runs_with_findings(&self) -> usize {
        let json = serde_json::to_value(self).unwrap();
        json["per_run_findings"]
            .as_object()
            .map(|obj| obj.len())
            .unwrap_or(0)
    }
}
