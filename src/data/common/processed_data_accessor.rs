use crate::computations::{f64_to_fixed_2, Statistics};
use crate::data::common::data_formats::{
    AperfData, DataFormat, KeyValueData, ProcessedData, Series, TimeSeriesData, TimeSeriesMetric,
};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Write;

/// This struct is created to allow for accessing the processed data within the specified
/// time range, without modifying the original processed data. Such design allow APerf to
/// hold only one copy of the processed data, and they can be accessed for any time range
/// with minimal performance cost.
#[derive(Debug)]
pub struct ProcessedDataAccessor {
    /// The start of the time range for every run data
    per_run_from_time: HashMap<String, u64>,
    /// The end of the time range for every run data
    per_run_to_time: HashMap<String, u64>,
    /// The cache for a metric's stat within the time range
    /// (since stat computation is costly).
    time_series_metric_stat_cache: HashMap<(String, String), Statistics>,
}

impl ProcessedDataAccessor {
    pub fn new() -> Self {
        ProcessedDataAccessor {
            per_run_from_time: HashMap::new(),
            per_run_to_time: HashMap::new(),
            time_series_metric_stat_cache: HashMap::new(),
        }
    }

    pub fn from_time_ranges(
        per_run_from_time: HashMap<String, u64>,
        per_run_to_time: HashMap<String, u64>,
    ) -> Self {
        ProcessedDataAccessor {
            per_run_from_time,
            per_run_to_time,
            time_series_metric_stat_cache: HashMap::new(),
        }
    }

    /// Returns the iterator of all series values within a time-series metric. If the metric
    /// does not exist, returns an empty iterator.
    pub fn time_series_metric_values_iterator<'a>(
        &self,
        processed_data: &'a ProcessedData,
        run_name: &str,
        metric_name: &str,
    ) -> impl Iterator<Item = &'a f64> {
        let from_time = self.per_run_from_time.get(run_name).copied();
        let to_time = self.per_run_to_time.get(run_name).copied();
        let metric_name = metric_name.to_string();
        get_time_series_data(processed_data, run_name)
            .into_iter()
            .flat_map(move |time_series_data| {
                time_series_data
                    .metrics
                    .get(&metric_name)
                    .into_iter()
                    .flat_map(move |metric| {
                        metric.series.iter().flat_map(move |series| {
                            let (start_idx, end_idx) =
                                compute_time_diff_index(&series.time_diff, from_time, to_time);
                            series.values[start_idx..end_idx].iter()
                        })
                    })
            })
    }

    /// Returns the stat of a time-series metric within the time range. The computed stat
    /// will be cached.
    pub fn time_series_metric_stats(
        &mut self,
        processed_data: &ProcessedData,
        run_name: &str,
        metric_name: &str,
    ) -> Option<Statistics> {
        let time_series_data = get_time_series_data(processed_data, run_name)?;
        Some(self.get_or_compute_time_series_metric_stats(
            time_series_data.metrics.get(metric_name)?,
            run_name,
        ))
    }

    /// Returns the list of metric names in the run that matches the pattern.
    pub fn time_series_matched_metric_names<'a>(
        &self,
        processed_data: &'a ProcessedData,
        run_name: &str,
        pattern: &str,
    ) -> Vec<&'a str> {
        let re = match Regex::new(pattern) {
            Ok(re) => re,
            Err(_) => return Vec::new(),
        };

        if let Some(time_series_data) = get_time_series_data(processed_data, run_name) {
            time_series_data
                .metrics
                .keys()
                .filter(|&key| re.is_match(key))
                .map(|key| key.as_str())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns the value of the corresponding key in the key-value data.
    pub fn key_value_value_by_key<'a>(
        &self,
        processed_data: &'a ProcessedData,
        run_name: &str,
        key: &str,
    ) -> Option<&'a str> {
        let key_value_data = get_key_value_data(processed_data, run_name)?;
        for key_value_group in key_value_data.key_value_groups.values() {
            if let Some(value) = key_value_group.key_values.get(key) {
                return Some(value);
            }
        }
        None
    }

    /// Serializes a processed data into JSON string.
    pub fn json_string(&mut self, processed_data: &ProcessedData) -> String {
        match processed_data.data_format {
            DataFormat::TimeSeries => self.time_series_data_json_string(processed_data),
            DataFormat::KeyValue => self.key_value_data_json_string(processed_data),
            DataFormat::Text => self.text_data_json_string(processed_data),
            DataFormat::Graph => self.graph_data_json_string(processed_data),
            DataFormat::Unknown => serde_json::to_string(processed_data).unwrap(),
        }
    }

    /// Serializes the processed time-series data within the time range into JSON string.
    pub fn time_series_data_json_string(&mut self, processed_data: &ProcessedData) -> String {
        let mut buf = String::new();
        buf.push_str("{\"data_name\":");
        write!(
            buf,
            "{}",
            serde_json::to_string(&processed_data.data_name).unwrap()
        )
        .unwrap();
        buf.push_str(",\"data_format\":");
        write!(
            buf,
            "{}",
            serde_json::to_string(&processed_data.data_format).unwrap()
        )
        .unwrap();
        buf.push_str(",\"runs\":{");
        let mut first_run = true;
        for run_name in processed_data.runs.keys() {
            if !first_run {
                buf.push(',');
            }
            first_run = false;

            write!(buf, "{}:", serde_json::to_string(run_name).unwrap()).unwrap();

            if let Some(time_series_data) = get_time_series_data(processed_data, run_name) {
                let from_time = self.per_run_from_time.get(run_name).copied();
                let to_time = self.per_run_to_time.get(run_name).copied();

                buf.push_str("{\"metrics\":{");
                let mut first_metric = true;
                for (metric_name, time_series_metric) in &time_series_data.metrics {
                    if !first_metric {
                        buf.push(',');
                    }
                    first_metric = false;
                    write!(buf, "{}:", serde_json::to_string(metric_name).unwrap()).unwrap();
                    let stats =
                        self.get_or_compute_time_series_metric_stats(time_series_metric, run_name);
                    write_time_series_metric_json_string(
                        &mut buf,
                        time_series_metric,
                        stats,
                        from_time,
                        to_time,
                    );
                }

                buf.push_str("},\"sorted_metric_names\":");
                write!(
                    buf,
                    "{}",
                    serde_json::to_string(&time_series_data.sorted_metric_names).unwrap()
                )
                .unwrap();
                buf.push('}');
            }
        }

        buf.push_str("}}");
        buf
    }

    /// Serializes the data of a time-series metric within the time range into JSON string.
    pub fn time_series_metric_json_string(
        &mut self,
        time_series_metric: &TimeSeriesMetric,
        run_name: &str,
    ) -> String {
        let mut buf = String::new();
        let stats = self.get_or_compute_time_series_metric_stats(time_series_metric, run_name);
        write_time_series_metric_json_string(
            &mut buf,
            time_series_metric,
            stats,
            self.per_run_from_time.get(run_name).copied(),
            self.per_run_to_time.get(run_name).copied(),
        );
        buf
    }

    /// Serializes the processed key-value data into JSON string.
    pub fn key_value_data_json_string(&mut self, processed_data: &ProcessedData) -> String {
        serde_json::to_string(processed_data).unwrap()
    }

    /// Serializes the processed text data into JSON string.
    pub fn text_data_json_string(&mut self, processed_data: &ProcessedData) -> String {
        serde_json::to_string(processed_data).unwrap()
    }

    /// Serializes the processed graph data into JSON string.
    pub fn graph_data_json_string(&mut self, processed_data: &ProcessedData) -> String {
        serde_json::to_string(processed_data).unwrap()
    }

    /// Retrieves the stat of a time-series metric within the time range from the cache, or,
    /// if it does not exist in the cache, computes the stat and stores it in the cache.
    fn get_or_compute_time_series_metric_stats(
        &mut self,
        time_series_metric: &TimeSeriesMetric,
        run_name: &str,
    ) -> Statistics {
        let cache_key = (run_name.to_string(), time_series_metric.metric_name.clone());
        if let Some(stats) = self.time_series_metric_stat_cache.get(&cache_key) {
            return stats.clone();
        }
        let from_time = self.per_run_from_time.get(run_name).copied();
        let to_time = self.per_run_to_time.get(run_name).copied();
        let stats = if let Some(stats_series) = time_series_metric
            .series
            .get(time_series_metric.stats_series_idx)
        {
            let (start_idx, end_idx) =
                compute_time_diff_index(&stats_series.time_diff, from_time, to_time);
            Statistics::from_values(&stats_series.values[start_idx..end_idx])
        } else {
            Statistics::default()
        };
        self.time_series_metric_stat_cache
            .insert(cache_key, stats.clone());
        stats
    }
}

/// Writes a time-series metric into JSON string, with the series values within the time range.
fn write_time_series_metric_json_string(
    buf: &mut String,
    time_series_metric: &TimeSeriesMetric,
    stats: Statistics,
    from_time: Option<u64>,
    to_time: Option<u64>,
) {
    buf.push_str("{\"metric_name\":");
    write!(
        buf,
        "{}",
        serde_json::to_string(&time_series_metric.metric_name).unwrap()
    )
    .unwrap();

    buf.push_str(",\"series\":[");
    let mut first_series = true;
    for series in &time_series_metric.series {
        if !first_series {
            buf.push(',');
        }
        first_series = false;
        write_series_json_string(buf, series, from_time, to_time);
    }
    buf.push(']');

    write!(
        buf,
        ",\"value_range\":[{},{}]",
        itoa::Buffer::new().format(time_series_metric.value_range.0),
        itoa::Buffer::new().format(time_series_metric.value_range.1)
    )
    .unwrap();

    buf.push_str(",\"stats\":");
    write!(buf, "{}", serde_json::to_string(&stats).unwrap()).unwrap();

    buf.push('}');
}

/// Writes a single time-series series into JSON string, with time_diff and values within the time range.
fn write_series_json_string(
    buf: &mut String,
    series: &Series,
    from_time: Option<u64>,
    to_time: Option<u64>,
) {
    let (start, end) = compute_time_diff_index(&series.time_diff, from_time, to_time);
    // Uses itoa/ryu for fast int/float number formatting
    let mut itoa_buf = itoa::Buffer::new();
    let mut ryu_buf = ryu::Buffer::new();

    buf.push('{');

    buf.push_str("\"series_name\":");
    write!(
        buf,
        "{}",
        serde_json::to_string(&series.series_name).unwrap()
    )
    .unwrap();
    buf.push(',');

    buf.push_str("\"time_diff\":[");
    for (i, &t) in series.time_diff[start..end].iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        buf.push_str(itoa_buf.format(t));
    }
    buf.push(']');

    buf.push_str(",\"values\":[");
    for (i, &v) in series.values[start..end].iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        buf.push_str(ryu_buf.format(f64_to_fixed_2(v)));
    }
    buf.push(']');

    if series.is_aggregate {
        buf.push_str(",\"is_aggregate\":true");
    }

    buf.push('}');
}

/// Retrieves the time-series data of a run from the processed data.
fn get_time_series_data<'a>(
    processed_data: &'a ProcessedData,
    run_name: &str,
) -> Option<&'a TimeSeriesData> {
    match processed_data.runs.get(run_name) {
        Some(AperfData::TimeSeries(ts)) => Some(ts),
        _ => None,
    }
}

/// Retrieves the key-value data of a run from the processed data.
fn get_key_value_data<'a>(
    processed_data: &'a ProcessedData,
    run_name: &str,
) -> Option<&'a KeyValueData> {
    match processed_data.runs.get(run_name) {
        Some(AperfData::KeyValue(kv)) => Some(kv),
        _ => None,
    }
}

/// Locate the start and end index of a series' time_diff (and values) based on
/// the specified time range.
fn compute_time_diff_index(
    time_diff: &[u64],
    from_time: Option<u64>,
    to_time: Option<u64>,
) -> (usize, usize) {
    let start_idx = if let Some(from) = from_time {
        time_diff.partition_point(|t| *t < from)
    } else {
        0
    };
    let end_idx = if let Some(to) = to_time {
        time_diff.partition_point(|t| *t <= to)
    } else {
        time_diff.len()
    };
    (start_idx, end_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_series(name: &str, time_diff: Vec<u64>, values: Vec<f64>) -> Series {
        Series {
            series_name: name.to_string(),
            time_diff,
            values,
            is_aggregate: false,
        }
    }

    fn make_series_with_stats_idx(metrics: Vec<(&str, Vec<Series>, usize)>) -> ProcessedData {
        let mut pd = ProcessedData::new("test".to_string());
        pd.data_format = DataFormat::TimeSeries;
        let mut ts = TimeSeriesData::default();
        for (name, series_list, stats_idx) in metrics {
            let mut metric = TimeSeriesMetric::new(name.to_string());
            metric.stats_series_idx = stats_idx;
            metric.series = series_list;
            ts.metrics.insert(name.to_string(), metric);
            ts.sorted_metric_names.push(name.to_string());
        }
        pd.runs
            .insert("run1".to_string(), AperfData::TimeSeries(ts));
        pd
    }

    fn make_processed_data(metrics: Vec<(&str, Vec<Series>)>) -> ProcessedData {
        make_series_with_stats_idx(
            metrics
                .into_iter()
                .map(|(name, series)| (name, series, 0))
                .collect(),
        )
    }

    // ---- compute_time_diff_index tests ----

    #[test]
    fn test_index_no_bounds() {
        assert_eq!(
            compute_time_diff_index(&[0, 10, 20, 30, 40], None, None),
            (0, 5)
        );
    }

    #[test]
    fn test_index_with_from() {
        assert_eq!(
            compute_time_diff_index(&[0, 10, 20, 30, 40], Some(15), None),
            (2, 5)
        );
    }

    #[test]
    fn test_index_with_to() {
        assert_eq!(
            compute_time_diff_index(&[0, 10, 20, 30, 40], None, Some(25)),
            (0, 3)
        );
    }

    #[test]
    fn test_index_with_both() {
        assert_eq!(
            compute_time_diff_index(&[0, 10, 20, 30, 40], Some(10), Some(30)),
            (1, 4)
        );
    }

    #[test]
    fn test_index_exact_match() {
        assert_eq!(
            compute_time_diff_index(&[0, 10, 20, 30, 40], Some(20), Some(20)),
            (2, 3)
        );
    }

    #[test]
    fn test_index_empty() {
        assert_eq!(compute_time_diff_index(&[], Some(5), Some(10)), (0, 0));
    }

    #[test]
    fn test_index_from_beyond_end() {
        assert_eq!(
            compute_time_diff_index(&[0, 10, 20], Some(100), None),
            (3, 3)
        );
    }

    #[test]
    fn test_index_to_before_start() {
        assert_eq!(
            compute_time_diff_index(&[10, 20, 30], None, Some(5)),
            (0, 0)
        );
    }

    #[test]
    fn test_index_from_equals_first_element() {
        // from == first element → should include it
        assert_eq!(
            compute_time_diff_index(&[10, 20, 30], Some(10), None),
            (0, 3)
        );
    }

    #[test]
    fn test_index_to_equals_last_element() {
        // to == last element → should include it
        assert_eq!(
            compute_time_diff_index(&[10, 20, 30], None, Some(30)),
            (0, 3)
        );
    }

    #[test]
    fn test_index_single_element_in_range() {
        assert_eq!(compute_time_diff_index(&[42], Some(40), Some(50)), (0, 1));
    }

    #[test]
    fn test_index_single_element_out_of_range() {
        assert_eq!(compute_time_diff_index(&[42], Some(50), Some(60)), (1, 1));
    }

    #[test]
    fn test_index_duplicate_values() {
        // time_diff with duplicates — all matching values should be included
        assert_eq!(
            compute_time_diff_index(&[10, 10, 20, 20, 30], Some(10), Some(20)),
            (0, 4)
        );
    }

    #[test]
    fn test_index_from_none_to_none() {
        // Both None → full range (same as test_index_no_bounds but explicit)
        assert_eq!(compute_time_diff_index(&[5, 10, 15], None, None), (0, 3));
    }

    #[test]
    fn test_index_from_equals_to_not_in_array() {
        // from == to but value doesn't exist in array → empty range
        assert_eq!(
            compute_time_diff_index(&[10, 20, 30], Some(15), Some(15)),
            (1, 1)
        );
    }

    #[test]
    fn test_index_contiguous_range() {
        // Typical real-world case: second-by-second time_diff
        let td: Vec<u64> = (0..100).collect();
        assert_eq!(compute_time_diff_index(&td, Some(25), Some(74)), (25, 75));
    }

    // ---- iterator tests ----

    #[test]
    fn test_metric_values_no_time_range() {
        let pd = make_processed_data(vec![(
            "cpu",
            vec![make_series(
                "total",
                vec![0, 10, 20],
                vec![10.0, 20.0, 30.0],
            )],
        )]);
        let accessor = ProcessedDataAccessor::new();
        let values: Vec<&f64> = accessor
            .time_series_metric_values_iterator(&pd, "run1", "cpu")
            .collect();
        assert_eq!(values, vec![&10.0, &20.0, &30.0]);
    }

    #[test]
    fn test_metric_values_with_from_time() {
        let pd = make_processed_data(vec![(
            "cpu",
            vec![make_series(
                "total",
                vec![0, 10, 20],
                vec![10.0, 20.0, 30.0],
            )],
        )]);
        let accessor = ProcessedDataAccessor::from_time_ranges(
            HashMap::from([("run1".to_string(), 10)]),
            HashMap::new(),
        );
        let values: Vec<&f64> = accessor
            .time_series_metric_values_iterator(&pd, "run1", "cpu")
            .collect();
        assert_eq!(values, vec![&20.0, &30.0]);
    }

    #[test]
    fn test_metric_values_multiple_series() {
        let pd = make_processed_data(vec![(
            "cpu",
            vec![
                make_series("core0", vec![0, 10, 20], vec![1.0, 2.0, 3.0]),
                make_series("core1", vec![0, 10, 20], vec![4.0, 5.0, 6.0]),
            ],
        )]);
        let accessor = ProcessedDataAccessor::from_time_ranges(
            HashMap::new(),
            HashMap::from([("run1".to_string(), 10)]),
        );
        let values: Vec<&f64> = accessor
            .time_series_metric_values_iterator(&pd, "run1", "cpu")
            .collect();
        assert_eq!(values, vec![&1.0, &2.0, &4.0, &5.0]);
    }

    #[test]
    fn test_metric_values_missing() {
        let pd = make_processed_data(vec![]);
        let accessor = ProcessedDataAccessor::new();
        assert!(accessor
            .time_series_metric_values_iterator(&pd, "run1", "nonexistent")
            .collect::<Vec<_>>()
            .is_empty());
        assert!(accessor
            .time_series_metric_values_iterator(&pd, "run2", "cpu")
            .collect::<Vec<_>>()
            .is_empty());
    }

    // ---- stats tests ----

    #[test]
    fn test_metric_stat_basic() {
        let pd = make_series_with_stats_idx(vec![(
            "cpu",
            vec![make_series(
                "total",
                vec![0, 10, 20],
                vec![10.0, 20.0, 30.0],
            )],
            0,
        )]);
        let mut accessor = ProcessedDataAccessor::new();
        let stats = accessor
            .time_series_metric_stats(&pd, "run1", "cpu")
            .unwrap();
        assert_eq!(stats.avg, 20.0);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 30.0);
    }

    #[test]
    fn test_metric_stat_with_time_range() {
        let pd = make_series_with_stats_idx(vec![(
            "cpu",
            vec![make_series(
                "total",
                vec![0, 10, 20, 30],
                vec![10.0, 20.0, 30.0, 40.0],
            )],
            0,
        )]);
        let mut accessor = ProcessedDataAccessor::from_time_ranges(
            HashMap::from([("run1".to_string(), 10)]),
            HashMap::from([("run1".to_string(), 20)]),
        );
        let stats = accessor
            .time_series_metric_stats(&pd, "run1", "cpu")
            .unwrap();
        assert_eq!(stats.avg, 25.0);
    }

    #[test]
    fn test_metric_stat_uses_stats_series_idx() {
        let pd = make_series_with_stats_idx(vec![(
            "cpu",
            vec![
                make_series("core0", vec![0, 10, 20], vec![1.0, 2.0, 3.0]),
                make_series("aggregate", vec![0, 10, 20], vec![100.0, 200.0, 300.0]),
            ],
            1,
        )]);
        let mut accessor = ProcessedDataAccessor::new();
        let stats = accessor
            .time_series_metric_stats(&pd, "run1", "cpu")
            .unwrap();
        assert_eq!(stats.avg, 200.0);
    }

    #[test]
    fn test_metric_stat_caching() {
        let pd = make_series_with_stats_idx(vec![(
            "cpu",
            vec![make_series(
                "total",
                vec![0, 10, 20],
                vec![10.0, 20.0, 30.0],
            )],
            0,
        )]);
        let mut accessor = ProcessedDataAccessor::new();
        let _ = accessor.time_series_metric_stats(&pd, "run1", "cpu");
        assert!(accessor
            .time_series_metric_stat_cache
            .contains_key(&("run1".to_string(), "cpu".to_string())));
    }

    #[test]
    fn test_metric_stat_missing() {
        let pd = make_processed_data(vec![]);
        let mut accessor = ProcessedDataAccessor::new();
        assert!(accessor
            .time_series_metric_stats(&pd, "run1", "cpu")
            .is_none());
    }

    // ---- write_series tests ----

    #[test]
    fn test_write_series_no_range() {
        let series = make_series("total", vec![0, 10, 20], vec![1.5, 2.7, 3.223]);
        let mut buf = String::new();
        write_series_json_string(&mut buf, &series, None, None);
        let v: serde_json::Value = serde_json::from_str(&buf).unwrap();
        assert_eq!(v["series_name"], "total");
        assert_eq!(v["time_diff"], serde_json::json!([0, 10, 20]));
        assert_eq!(v["values"], serde_json::json!([1.5, 2.7, 3.22]));
        assert!(v.get("is_aggregate").is_none());
    }

    #[test]
    fn test_write_series_with_time_range() {
        let series = make_series("s1", vec![0, 10, 20, 30], vec![1.0, 2.0, 3.0, 4.0]);
        let mut buf = String::new();
        write_series_json_string(&mut buf, &series, Some(10), Some(20));
        let v: serde_json::Value = serde_json::from_str(&buf).unwrap();
        assert_eq!(v["time_diff"], serde_json::json!([10, 20]));
        let values: Vec<f64> = v["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        assert_eq!(values, vec![2.0, 3.0]);
    }

    #[test]
    fn test_write_series_aggregate() {
        let series = Series {
            series_name: "avg".to_string(),
            time_diff: vec![0],
            values: vec![5.0],
            is_aggregate: true,
        };
        let mut buf = String::new();
        write_series_json_string(&mut buf, &series, None, None);
        let v: serde_json::Value = serde_json::from_str(&buf).unwrap();
        assert_eq!(v["is_aggregate"], true);
    }

    // ---- time_series_metric_json_string tests ----

    #[test]
    fn test_metric_json_string_no_range() {
        let mut metric = TimeSeriesMetric::new("cpu".to_string());
        metric.series = vec![make_series(
            "total",
            vec![0, 10, 20],
            vec![10.0, 20.0, 30.0],
        )];
        let mut accessor = ProcessedDataAccessor::new();
        let json = accessor.time_series_metric_json_string(&metric, "run1");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["metric_name"], "cpu");
        assert_eq!(v["series"][0]["time_diff"], serde_json::json!([0, 10, 20]));
    }

    #[test]
    fn test_metric_json_string_with_range() {
        let mut metric = TimeSeriesMetric::new("cpu".to_string());
        metric.series = vec![make_series(
            "total",
            vec![0, 10, 20, 30],
            vec![10.0, 20.0, 30.0, 40.0],
        )];
        let mut accessor = ProcessedDataAccessor::from_time_ranges(
            HashMap::from([("run1".to_string(), 10)]),
            HashMap::from([("run1".to_string(), 20)]),
        );
        let json = accessor.time_series_metric_json_string(&metric, "run1");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["series"][0]["time_diff"], serde_json::json!([10, 20]));
        assert_eq!(v["stats"]["avg"], 25.0);
    }

    #[test]
    fn test_metric_json_string_caches_stats() {
        let mut metric = TimeSeriesMetric::new("cpu".to_string());
        metric.series = vec![make_series(
            "total",
            vec![0, 10, 20],
            vec![10.0, 20.0, 30.0],
        )];
        let mut accessor = ProcessedDataAccessor::new();
        let _ = accessor.time_series_metric_json_string(&metric, "run1");
        assert!(accessor
            .time_series_metric_stat_cache
            .contains_key(&("run1".to_string(), "cpu".to_string())));
    }

    // ---- time_series_data_json_string tests ----

    #[test]
    fn test_ts_data_json_no_range() {
        let pd = make_processed_data(vec![(
            "cpu",
            vec![make_series(
                "total",
                vec![0, 10, 20],
                vec![10.0, 20.0, 30.0],
            )],
        )]);
        let mut accessor = ProcessedDataAccessor::new();
        let json = accessor.time_series_data_json_string(&pd);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["data_name"], "test");
        assert_eq!(v["runs"]["run1"]["metrics"]["cpu"]["metric_name"], "cpu");
    }

    #[test]
    fn test_ts_data_json_with_range() {
        let pd = make_processed_data(vec![(
            "cpu",
            vec![make_series(
                "total",
                vec![0, 10, 20, 30],
                vec![10.0, 20.0, 30.0, 40.0],
            )],
        )]);
        let mut accessor = ProcessedDataAccessor::from_time_ranges(
            HashMap::from([("run1".to_string(), 10)]),
            HashMap::from([("run1".to_string(), 20)]),
        );
        let json = accessor.time_series_data_json_string(&pd);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let values: Vec<f64> = v["runs"]["run1"]["metrics"]["cpu"]["series"][0]["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        assert_eq!(values, vec![20.0, 30.0]);
    }

    // ---- json_string dispatch tests ----

    #[test]
    fn test_json_string_dispatches_time_series() {
        let pd = make_processed_data(vec![(
            "cpu",
            vec![make_series("total", vec![0, 10], vec![1.0, 2.0])],
        )]);
        let mut accessor = ProcessedDataAccessor::new();
        let json = accessor.json_string(&pd);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["runs"]["run1"]["metrics"]["cpu"].is_object());
    }

    #[test]
    fn test_json_string_dispatches_key_value() {
        use crate::data::common::data_formats::KeyValueGroup;
        let mut pd = ProcessedData::new("sysctl".to_string());
        pd.data_format = DataFormat::KeyValue;
        let mut group = KeyValueGroup::default();
        group
            .key_values
            .insert("key1".to_string(), "val1".to_string());
        let mut kv = KeyValueData::default();
        kv.key_value_groups.insert("group1".to_string(), group);
        pd.runs.insert("run1".to_string(), AperfData::KeyValue(kv));

        let mut accessor = ProcessedDataAccessor::new();
        let json = accessor.json_string(&pd);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["data_format"], "key_value");
        assert_eq!(
            v["runs"]["run1"]["key_value_groups"]["group1"]["key_values"]["key1"],
            "val1"
        );
    }

    #[test]
    fn test_from_time_ranges() {
        let accessor = ProcessedDataAccessor::from_time_ranges(
            HashMap::from([("run1".to_string(), 10)]),
            HashMap::from([("run1".to_string(), 30)]),
        );
        assert_eq!(accessor.per_run_from_time.get("run1"), Some(&10));
        assert_eq!(accessor.per_run_to_time.get("run1"), Some(&30));
    }
}
