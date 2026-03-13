use crate::computations::{get_average, Statistics};
use crate::data::data_formats::{Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::TimeEnum;
use log::{debug, warn};
use std::collections::HashMap;

pub const AGGREGATE_SERIES_NAME: &str = "aggregate";

#[derive(PartialEq, Debug)]
pub enum TimeSeriesDataAggregateMode {
    Average,
    MaxSeries,
    Custom,
}

pub struct TimeSeriesDataProcessor {
    // For logging purposes
    data_name: &'static str,
    // Store the data value at the previous snapshot - for accumulative data
    prev_data_points: HashMap<String, HashMap<String, f64>>,
    // Map<metric name, Map<series name, series>> for quick access of a series
    per_metric_series: HashMap<String, HashMap<String, Series>>,
    // Map<metric name, (max value, min value)> for the eventual computation of the value range
    per_metric_value_range: HashMap<String, (f64, f64)>,
    // Indicates how the aggregate series of every metric is computed
    aggregate_mode: TimeSeriesDataAggregateMode,
    // Map<metric name, (sum, count)> - in case the aggregate series is the average of all series,
    // used to compute the average of a metric at a snapshot (will be cleared at every timestamp)
    per_metric_sum_count: HashMap<String, (f64, usize)>,
    // The initial timestamp, for the computation of every time_diff
    time_zero: Option<TimeEnum>,
    // The current time_diff to be added to every series
    cur_time_diff: u64,
    // Map<metric_name.series_name, number of decreasing accumulative data> - used to count the
    // number of unexpected decreases of accumulative data, which are to be logged as warning
    // at the end of collection
    decreasing_accumulative_data: HashMap<String, usize>,
}

impl TimeSeriesDataProcessor {
    /// The aggregate series of every metric is the average of all series values at a timestamp.
    /// Use this option when the series of a metric each represents a parallel component or device,
    /// such as a CPU or network interface.
    pub fn with_average_aggregate(data_name: &'static str) -> Self {
        Self::new(data_name, TimeSeriesDataAggregateMode::Average)
    }

    /// The aggregate series of every metric is the series with the maximum average value.
    /// Use this option when the series of a metric cover a hierarchy, where the values of
    /// a series is the sum of other series. For example, diskstats metrics include the root
    /// partition as well as all the subpartitions it contains.
    pub fn with_max_series_aggregate(data_name: &'static str) -> Self {
        Self::new(data_name, TimeSeriesDataAggregateMode::MaxSeries)
    }

    /// The aggregate series needs to be manually added with the name AGGREGATE_SERIES_NAME.
    pub fn with_custom_aggregate(data_name: &'static str) -> Self {
        Self::new(data_name, TimeSeriesDataAggregateMode::Custom)
    }

    pub fn new(data_name: &'static str, aggregate_mode: TimeSeriesDataAggregateMode) -> Self {
        TimeSeriesDataProcessor {
            data_name,
            prev_data_points: HashMap::new(),
            per_metric_series: HashMap::new(),
            per_metric_value_range: HashMap::new(),
            aggregate_mode,
            per_metric_sum_count: HashMap::new(),
            time_zero: None,
            cur_time_diff: 0,
            decreasing_accumulative_data: HashMap::new(),
        }
    }

    /// Helper function to compute the average value across all series in every metric and
    /// put the value in the aggregate series.
    fn put_average_aggregate_series_value(&mut self) {
        if self.aggregate_mode == TimeSeriesDataAggregateMode::Average {
            for (metric_name, (sum, count)) in &mut self.per_metric_sum_count {
                let aggregate_series = self
                    .per_metric_series
                    .entry(metric_name.clone())
                    .or_insert(HashMap::new())
                    .entry(AGGREGATE_SERIES_NAME.to_string())
                    .or_insert(Series::new(Some(AGGREGATE_SERIES_NAME.to_string())));
                let average = if *count > 0 {
                    *sum / *count as f64
                } else {
                    0.0
                };
                aggregate_series.values.push(average);
                aggregate_series.time_diff.push(self.cur_time_diff);
            }
            // Remove all entries instead of setting every sum and count to 0, since there is no
            // guarantee that all metrics still present at the next time_diff
            self.per_metric_sum_count.clear();
        }
    }

    /// Invoked before processing the data at the next snapshot, when all the data in the
    /// previous snapshot have been processed
    pub fn proceed_to_time(&mut self, time: TimeEnum) {
        self.put_average_aggregate_series_value();
        self.cur_time_diff = match time - *self.time_zero.get_or_insert(time) {
            TimeEnum::TimeDiff(_time_diff) => _time_diff,
            TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
        };
    }

    /// Add a noncumulative data point to the corresponding metric and series. Returns the data
    /// value if it was successfully added to the series.
    pub fn add_data_point(
        &mut self,
        metric_name: &str,
        series_name: &str,
        data_value: f64,
    ) -> Option<f64> {
        self.add_data_point_impl(metric_name, series_name, data_value, false)
    }

    /// Add an accumulative data point to the corresponding metric and series. The processor
    /// keeps track of the previous values and computes the delta as the series value. Returns
    /// the series value if the data was successfully added.
    pub fn add_accumulative_data_point(
        &mut self,
        metric_name: &str,
        series_name: &str,
        data_value: f64,
    ) -> Option<f64> {
        self.add_data_point_impl(metric_name, series_name, data_value, true)
    }

    fn add_data_point_impl(
        &mut self,
        metric_name: &str,
        series_name: &str,
        data_value: f64,
        accumulative: bool,
    ) -> Option<f64> {
        let metric_value = if accumulative {
            // Keeps track of previous snapshot value to compute the series value for
            // accumulative data. Skip the data point if the snapshot value unexpectedly decreases
            match self
                .prev_data_points
                .entry(metric_name.to_string())
                .or_insert(HashMap::new())
                .insert(series_name.to_string(), data_value)
            {
                Some(prev_value) => {
                    if prev_value > data_value {
                        // In case there are a lot of decreases, we don't want to pollute the logs
                        // so only use debug here and warn the total number of decreases at the end
                        debug!(
                            "Unexpected decreasing of accumulative data {}.{}.{}: from {} to {}",
                            self.data_name, metric_name, series_name, prev_value, data_value
                        );
                        let error_counts = self
                            .decreasing_accumulative_data
                            .entry(format!("{metric_name}.{series_name}"))
                            .or_insert(0);
                        *error_counts += 1;
                        return None;
                    }
                    data_value - prev_value
                }
                None => 0.0,
            }
        } else {
            data_value
        };

        let series = self
            .per_metric_series
            .entry(metric_name.to_string())
            .or_insert(HashMap::new())
            .entry(series_name.to_string())
            .or_insert(Series::new(Some(series_name.to_string())));
        series.time_diff.push(self.cur_time_diff);
        series.values.push(metric_value);

        // Every series value in the metric accounts for its value range
        let (min, max) = self
            .per_metric_value_range
            .entry(metric_name.to_string())
            .or_insert((metric_value, metric_value));
        *min = (*min).min(metric_value);
        *max = (*max).max(metric_value);

        if self.aggregate_mode == TimeSeriesDataAggregateMode::Average {
            // Keeps track of the sum and count of all series in every metric to
            // compute the average value at the end of current time_diff, so that
            // it can be used as the value of the aggregate series
            let (sum, count) = self
                .per_metric_sum_count
                .entry(metric_name.to_string())
                .or_insert((0.0, 0));
            *sum += metric_value;
            *count += 1;
        }

        Some(metric_value)
    }

    /// Generate the time-series data, with alphabetically sorted metric names.
    pub fn get_time_series_data(self) -> TimeSeriesData {
        self.get_time_series_data_impl(None)
    }

    /// Generate the time-series data, with the metric names sorted based on the specified order.
    pub fn get_time_series_data_with_metric_name_order(
        self,
        metric_name_order: Vec<&str>,
    ) -> TimeSeriesData {
        self.get_time_series_data_impl(Some(metric_name_order))
    }

    fn get_time_series_data_impl(mut self, metric_name_order: Option<Vec<&str>>) -> TimeSeriesData {
        self.put_average_aggregate_series_value();

        // Log any unexpected decreases of accumulative data
        for (data_key, count) in self.decreasing_accumulative_data {
            warn!(
                "{}.{}: skipped {} data points due to unexpected decrease of accumulative data.",
                self.data_name, data_key, count
            );
        }

        let mut time_series_data = TimeSeriesData::default();

        let mut all_metric_names: Vec<String> = self
            .per_metric_series
            .keys()
            .map(|key| key.clone())
            .collect();
        if let Some(metric_name_order) = metric_name_order {
            all_metric_names.sort_by_key(|metric_name| {
                metric_name_order
                    .iter()
                    .position(|&ordered_name| ordered_name == metric_name)
                    .unwrap_or(metric_name_order.len())
            });
        } else {
            all_metric_names.sort();
        }
        time_series_data.sorted_metric_names = all_metric_names;

        for (metric_name, cur_metric_series) in self.per_metric_series {
            let mut time_series_metric = TimeSeriesMetric::new(metric_name.clone());

            // Compute the metric's stats based on the aggregate mode
            if cur_metric_series.len() == 1 {
                // If there's only one series in the metric, it should be used for stats computation
                time_series_metric.stats =
                    Statistics::from_values(&cur_metric_series.values().next().unwrap().values);
            } else if self.aggregate_mode == TimeSeriesDataAggregateMode::MaxSeries {
                // For max-aggregate mode, find the series that has the maximum of average
                // and use it for stats computation
                let mut max_series = None;
                let mut max_average = 0.0;
                for series in cur_metric_series.values() {
                    let cur_series_average = get_average(&series.values);
                    if cur_series_average.is_some_and(|average| average > max_average) {
                        max_average = cur_series_average.unwrap();
                        max_series = Some(series);
                    }
                }
                if let Some(max_series) = max_series {
                    time_series_metric.stats = Statistics::from_values(&max_series.values);
                }
            } else {
                // For average-aggregate mode, the average series should already be created under
                // the AGGREGATE_SERIES_NAME;
                // For manual-aggregate mode, the caller should manually add a series under
                // the AGGREGATE_SERIES_NAME (if there are more than one series in the metric).
                if let Some(aggregate_series) = cur_metric_series.get(AGGREGATE_SERIES_NAME) {
                    time_series_metric.stats = Statistics::from_values(&aggregate_series.values);
                }
            }

            if let Some((min, max)) = self.per_metric_value_range.get(&metric_name) {
                time_series_metric.value_range = (min.floor() as u64, max.ceil() as u64);
            }

            let mut all_series: Vec<Series> = cur_metric_series.into_values().collect();
            if self.aggregate_mode == TimeSeriesDataAggregateMode::Average && all_series.len() == 2
            {
                // If there is only one actual series in the metric, there is no need to show
                // the redundant aggregate (average) series
                all_series
                    .retain(|series| series.series_name.as_deref() != Some(AGGREGATE_SERIES_NAME));
            }
            all_series.sort_by_key(|series| series.series_name.clone());
            time_series_metric.series = all_series;

            time_series_data
                .metrics
                .insert(metric_name, time_series_metric);
        }

        time_series_data
    }
}

macro_rules! time_series_data_processor_with_average_aggregate {
    () => {
        TimeSeriesDataProcessor::with_average_aggregate(
            crate::data::utils::get_data_name_from_type::<Self>(),
        )
    };
}

macro_rules! time_series_data_processor_with_max_series_aggregate {
    () => {
        TimeSeriesDataProcessor::with_max_series_aggregate(
            crate::data::utils::get_data_name_from_type::<Self>(),
        )
    };
}

macro_rules! time_series_data_processor_with_custom_aggregate {
    () => {
        TimeSeriesDataProcessor::with_custom_aggregate(
            crate::data::utils::get_data_name_from_type::<Self>(),
        )
    };
}

pub(crate) use time_series_data_processor_with_average_aggregate;
pub(crate) use time_series_data_processor_with_custom_aggregate;
pub(crate) use time_series_data_processor_with_max_series_aggregate;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    const TEST_DATA: &str = "test_data";

    fn make_times(count: usize, interval_secs: i64) -> Vec<TimeEnum> {
        let base = Utc::now();
        (0..count)
            .map(|i| TimeEnum::DateTime(base + chrono::Duration::seconds(i as i64 * interval_secs)))
            .collect()
    }

    fn data_series<'a>(ts: &'a TimeSeriesData, metric: &str) -> Vec<&'a Series> {
        let m = &ts.metrics[metric];
        let mut v: Vec<_> = m
            .series
            .iter()
            .filter(|s| s.series_name.as_deref() != Some(AGGREGATE_SERIES_NAME))
            .collect();
        v.sort_by_key(|s| s.series_name.clone());
        v
    }

    fn agg_series<'a>(ts: &'a TimeSeriesData, metric: &str) -> Option<&'a Series> {
        ts.metrics[metric]
            .series
            .iter()
            .find(|s| s.series_name.as_deref() == Some(AGGREGATE_SERIES_NAME))
    }

    // =======================================================================
    // add_data_point (non-accumulative)
    // =======================================================================

    #[test]
    fn test_non_accumulative_single_metric_single_series() {
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        for (i, t) in times.iter().enumerate() {
            p.proceed_to_time(*t);
            let ret = p.add_data_point("cpu", "core0", (i * 10) as f64);
            assert_eq!(ret, Some((i * 10) as f64));
        }

        // Also verify that zero is returned faithfully (not confused with None)
        p.proceed_to_time(make_times(4, 1)[3]);
        assert_eq!(p.add_data_point("cpu", "core0", 0.0), Some(0.0));

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "cpu");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].values, vec![0.0, 10.0, 20.0, 0.0]);
        assert_eq!(s[0].time_diff, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_non_accumulative_multiple_series() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("cpu", "core0", 50.0);
        p.add_data_point("cpu", "core1", 70.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("cpu", "core0", 60.0);
        p.add_data_point("cpu", "core1", 80.0);

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "cpu");
        assert_eq!(s.len(), 2);

        let core0 = s
            .iter()
            .find(|s| s.series_name.as_deref() == Some("core0"))
            .unwrap();
        let core1 = s
            .iter()
            .find(|s| s.series_name.as_deref() == Some("core1"))
            .unwrap();
        assert_eq!(core0.values, vec![50.0, 60.0]);
        assert_eq!(core1.values, vec![70.0, 80.0]);
    }

    // =======================================================================
    // add_accumulative_data_point
    // =======================================================================

    #[test]
    fn test_accumulative_computes_deltas() {
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        let values = [100.0, 250.0, 500.0];
        let expected_deltas = [Some(0.0), Some(150.0), Some(250.0)];
        for (i, t) in times.iter().enumerate() {
            p.proceed_to_time(*t);
            let ret = p.add_accumulative_data_point("bytes", "eth0", values[i]);
            assert_eq!(ret, expected_deltas[i]);
        }

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        assert_eq!(s[0].values, vec![0.0, 150.0, 250.0]);
    }

    #[test]
    fn test_accumulative_first_sample_is_zero() {
        let times = make_times(1, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_accumulative_data_point("bytes", "eth0", 9999.0);

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        assert_eq!(s[0].values, vec![0.0]);
    }

    #[test]
    fn test_accumulative_decreasing_counter_skipped() {
        // Decreasing accumulative counter should be silently dropped (no data point recorded),
        // but the decreased value is kept in prev_data_points so the next delta is computed
        // from the decreased value
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        assert_eq!(
            p.add_accumulative_data_point("bytes", "eth0", 1000.0),
            Some(0.0)
        );
        p.proceed_to_time(times[1]);
        assert_eq!(p.add_accumulative_data_point("bytes", "eth0", 400.0), None); // decrease → skipped, but 400 kept
        p.proceed_to_time(times[2]);
        assert_eq!(
            p.add_accumulative_data_point("bytes", "eth0", 1600.0),
            Some(1200.0)
        ); // 1600-400=1200

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        // t0: 0 (first), t1: skipped, t2: 1600-400=1200
        assert_eq!(s[0].values, vec![0.0, 1200.0]);
    }

    #[test]
    fn test_accumulative_decrease_only_affects_decreased_series() {
        // One series decreases, the other doesn't — only the decreased one is skipped
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_accumulative_data_point("bytes", "eth0", 1000.0);
        p.add_accumulative_data_point("bytes", "eth1", 2000.0);

        p.proceed_to_time(times[1]);
        p.add_accumulative_data_point("bytes", "eth0", 500.0); // decrease → skipped
        p.add_accumulative_data_point("bytes", "eth1", 2500.0); // normal

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        let eth0 = s
            .iter()
            .find(|s| s.series_name.as_deref() == Some("eth0"))
            .unwrap();
        let eth1 = s
            .iter()
            .find(|s| s.series_name.as_deref() == Some("eth1"))
            .unwrap();

        // eth0: only the first sample (0), second was skipped
        assert_eq!(eth0.values, vec![0.0]);
        // eth1: both samples recorded normally
        assert_eq!(eth1.values, vec![0.0, 500.0]);
    }

    #[test]
    fn test_accumulative_recovery_after_decrease() {
        // After a decrease, the decreased value is kept so subsequent increases compute
        // deltas from the decreased value
        let times = make_times(6, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_accumulative_data_point("bytes", "eth0", 1000.0);
        p.proceed_to_time(times[1]);
        p.add_accumulative_data_point("bytes", "eth0", 200.0); // decrease → skipped, 200 kept
        p.proceed_to_time(times[2]);
        p.add_accumulative_data_point("bytes", "eth0", 500.0); // 500-200=300
        p.proceed_to_time(times[3]);
        p.add_accumulative_data_point("bytes", "eth0", 900.0); // 900-500=400
        p.proceed_to_time(times[4]);
        p.add_accumulative_data_point("bytes", "eth0", 1200.0); // 1200-900=300
        p.proceed_to_time(times[5]);
        p.add_accumulative_data_point("bytes", "eth0", 1500.0); // 1500-1200=300

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        assert_eq!(s[0].values, vec![0.0, 300.0, 400.0, 300.0, 300.0]);
        assert_eq!(s[0].time_diff, vec![0, 2, 3, 4, 5]);
    }

    #[test]
    fn test_accumulative_multiple_series_independent_deltas() {
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        let eth0_vals = [100.0, 200.0, 350.0];
        let eth1_vals = [500.0, 800.0, 900.0];

        for (i, t) in times.iter().enumerate() {
            p.proceed_to_time(*t);
            p.add_accumulative_data_point("bytes", "eth0", eth0_vals[i]);
            p.add_accumulative_data_point("bytes", "eth1", eth1_vals[i]);
        }

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        let eth0 = s
            .iter()
            .find(|s| s.series_name.as_deref() == Some("eth0"))
            .unwrap();
        let eth1 = s
            .iter()
            .find(|s| s.series_name.as_deref() == Some("eth1"))
            .unwrap();

        assert_eq!(eth0.values, vec![0.0, 100.0, 150.0]);
        assert_eq!(eth1.values, vec![0.0, 300.0, 100.0]);
    }

    // =======================================================================
    // Average aggregate mode
    // =======================================================================

    #[test]
    fn test_average_aggregate_single_series_stripped() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("cpu", "core0", 50.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("cpu", "core0", 60.0);

        let ts = p.get_time_series_data();
        assert!(
            agg_series(&ts, "cpu").is_none(),
            "Aggregate should be stripped with 1 series"
        );
        assert_eq!(ts.metrics["cpu"].series.len(), 1);
    }

    #[test]
    fn test_average_aggregate_multiple_series_present() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("cpu", "core0", 40.0);
        p.add_data_point("cpu", "core1", 60.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("cpu", "core0", 50.0);
        p.add_data_point("cpu", "core1", 70.0);

        let ts = p.get_time_series_data();
        let agg = agg_series(&ts, "cpu").expect("Aggregate should exist with 2+ series");
        assert_eq!(agg.values, vec![50.0, 60.0]);
    }

    #[test]
    fn test_average_aggregate_three_series() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 20.0);
        p.add_data_point("m", "c", 30.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 40.0);
        p.add_data_point("m", "b", 50.0);
        p.add_data_point("m", "c", 60.0);

        let ts = p.get_time_series_data();
        let agg = agg_series(&ts, "m").unwrap();
        assert_eq!(agg.values, vec![20.0, 50.0]);
    }

    #[test]
    fn test_average_aggregate_series_appearing_later() {
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 100.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 200.0);
        p.add_data_point("m", "b", 400.0);

        p.proceed_to_time(times[2]);
        p.add_data_point("m", "a", 300.0);
        p.add_data_point("m", "b", 500.0);

        let ts = p.get_time_series_data();
        let agg = agg_series(&ts, "m").unwrap();
        assert_eq!(agg.values, vec![100.0, 300.0, 400.0]);
    }

    // =======================================================================
    // MaxSeries aggregate mode
    // =======================================================================

    #[test]
    fn test_max_series_stats_from_highest_average() {
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_max_series_aggregate(TEST_DATA);

        // Series "a" has a higher peak (1000) but lower average (334)
        // Series "b" has a lower peak (400) but higher average (400)
        // The max-series mode should pick "b" (highest average), not "a" (highest peak)
        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 1.0);
        p.add_data_point("m", "b", 400.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 1.0);
        p.add_data_point("m", "b", 400.0);

        p.proceed_to_time(times[2]);
        p.add_data_point("m", "a", 1000.0);
        p.add_data_point("m", "b", 400.0);

        let ts = p.get_time_series_data();
        let stats = &ts.metrics["m"].stats;
        // Stats should come from series "b" (highest average), not "a" (highest peak)
        assert_eq!(stats.max, 400.0);
        assert_eq!(stats.min, 400.0);
        assert_eq!(stats.avg, 400.0);
    }

    #[test]
    fn test_max_series_no_aggregate_series_in_output() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_max_series_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 20.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 30.0);
        p.add_data_point("m", "b", 40.0);

        let ts = p.get_time_series_data();
        assert!(agg_series(&ts, "m").is_none());
    }

    // =======================================================================
    // Custom aggregate mode
    // =======================================================================

    #[test]
    fn test_custom_mode_no_aggregate() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_custom_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 20.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 30.0);
        p.add_data_point("m", "b", 40.0);

        let ts = p.get_time_series_data();
        assert!(agg_series(&ts, "m").is_none());
        assert_eq!(data_series(&ts, "m").len(), 2);
    }

    // =======================================================================
    // Time handling
    // =======================================================================

    #[test]
    fn test_time_diff_computed_from_first_sample() {
        let times = make_times(4, 5);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        for t in &times {
            p.proceed_to_time(*t);
            p.add_data_point("m", "s", 1.0);
        }

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "m");
        assert_eq!(s[0].time_diff, vec![0, 5, 10, 15]);
    }

    // =======================================================================
    // Metric name ordering
    // =======================================================================

    #[test]
    fn test_default_alphabetical_ordering() {
        let times = make_times(1, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("zebra", "s", 1.0);
        p.add_data_point("alpha", "s", 2.0);
        p.add_data_point("middle", "s", 3.0);

        let ts = p.get_time_series_data();
        assert_eq!(ts.sorted_metric_names, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn test_custom_metric_name_order() {
        let times = make_times(1, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("zebra", "s", 1.0);
        p.add_data_point("alpha", "s", 2.0);
        p.add_data_point("middle", "s", 3.0);
        p.add_data_point("unknown", "s", 4.0);

        let ts = p.get_time_series_data_with_metric_name_order(vec!["middle", "zebra"]);
        assert_eq!(ts.sorted_metric_names[0], "middle");
        assert_eq!(ts.sorted_metric_names[1], "zebra");
        assert_eq!(ts.sorted_metric_names.len(), 4);
    }

    // =======================================================================
    // Value range
    // =======================================================================

    #[test]
    fn test_value_range_across_series() {
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 5.0);
        p.add_data_point("m", "b", 100.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 50.0);
        p.add_data_point("m", "b", 2.0);

        p.proceed_to_time(times[2]);
        p.add_data_point("m", "a", 75.0);
        p.add_data_point("m", "b", 80.0);

        let ts = p.get_time_series_data();
        assert_eq!(ts.metrics["m"].value_range, (2, 100));
    }

    // =======================================================================
    // Series sorting
    // =======================================================================

    #[test]
    fn test_series_sorted_by_name() {
        let times = make_times(1, 1);
        let mut p = TimeSeriesDataProcessor::with_custom_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "z_series", 1.0);
        p.add_data_point("m", "a_series", 2.0);
        p.add_data_point("m", "m_series", 3.0);

        let ts = p.get_time_series_data();
        let names: Vec<_> = ts.metrics["m"]
            .series
            .iter()
            .map(|s| s.series_name.as_deref().unwrap())
            .collect();
        assert_eq!(names, vec!["a_series", "m_series", "z_series"]);
    }

    // =======================================================================
    // Empty / no data
    // =======================================================================

    #[test]
    fn test_no_data_points() {
        let p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);
        let ts = p.get_time_series_data();
        assert!(ts.metrics.is_empty());
        assert!(ts.sorted_metric_names.is_empty());
    }

    // =======================================================================
    // Multiple metrics
    // =======================================================================

    #[test]
    fn test_multiple_metrics_independent() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("cpu", "core0", 50.0);
        p.add_accumulative_data_point("bytes", "eth0", 1000.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("cpu", "core0", 60.0);
        p.add_accumulative_data_point("bytes", "eth0", 1500.0);

        let ts = p.get_time_series_data();
        assert_eq!(ts.metrics.len(), 2);

        let cpu = data_series(&ts, "cpu");
        assert_eq!(cpu[0].values, vec![50.0, 60.0]);

        let bytes = data_series(&ts, "bytes");
        assert_eq!(bytes[0].values, vec![0.0, 500.0]);
    }

    // =======================================================================
    // Statistics
    // =======================================================================

    #[test]
    fn test_stats_computed_from_aggregate_in_average_mode() {
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 30.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 20.0);
        p.add_data_point("m", "b", 40.0);

        p.proceed_to_time(times[2]);
        p.add_data_point("m", "a", 30.0);
        p.add_data_point("m", "b", 50.0);

        let ts = p.get_time_series_data();
        // Aggregate values: [20, 30, 40]
        let stats = &ts.metrics["m"].stats;
        assert_eq!(stats.min, 20.0);
        assert_eq!(stats.max, 40.0);
        assert_eq!(stats.avg, 30.0);
    }

    #[test]
    fn test_stats_from_single_series_directly() {
        // New behavior: single-series metrics compute stats from that series, not aggregate
        let times = make_times(3, 1);
        let mut p = TimeSeriesDataProcessor::with_average_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "only_series", 10.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "only_series", 40.0);
        p.proceed_to_time(times[2]);
        p.add_data_point("m", "only_series", 20.0);

        let ts = p.get_time_series_data();
        let stats = &ts.metrics["m"].stats;
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 40.0);
        // avg of [10, 40, 20] = 23.33...
        assert!((stats.avg - 70.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_stats_single_series_in_max_series_mode() {
        // With only 1 series, stats should come from that series regardless of mode
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_max_series_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "only", 100.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "only", 200.0);

        let ts = p.get_time_series_data();
        let stats = &ts.metrics["m"].stats;
        assert_eq!(stats.min, 100.0);
        assert_eq!(stats.max, 200.0);
    }

    #[test]
    fn test_stats_single_series_in_custom_mode() {
        let times = make_times(2, 1);
        let mut p = TimeSeriesDataProcessor::with_custom_aggregate(TEST_DATA);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "only", 5.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "only", 15.0);

        let ts = p.get_time_series_data();
        let stats = &ts.metrics["m"].stats;
        assert_eq!(stats.min, 5.0);
        assert_eq!(stats.max, 15.0);
    }
}
