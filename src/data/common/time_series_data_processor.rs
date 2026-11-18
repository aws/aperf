use crate::computations::{get_average, Statistics};
use crate::data::common::data_formats::{Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::TimeEnum;
use exmex::{Express, FlatEx};
use log::{debug, warn};
use numeric_sort::cmp;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Debug)]
pub enum TimeSeriesDataAggregateMode {
    Average,
    Sum,
    MaxSeries,
    Custom,
}

impl TimeSeriesDataAggregateMode {
    fn is_auto_generated(&self) -> bool {
        match self {
            TimeSeriesDataAggregateMode::Average | TimeSeriesDataAggregateMode::Sum => true,
            _ => false,
        }
    }
}

pub struct TimeSeriesDataProcessor {
    // For logging purposes
    data_name: &'static str,
    // Store the data value at the previous snapshot along with its time_diff - for accumulative data
    prev_data_points: HashMap<String, HashMap<String, (f64, u64)>>,
    // Map<metric name, Map<series name, series>> for quick access of a series
    per_metric_series: HashMap<String, HashMap<String, Series>>,
    // Map<metric name, (max value, min value)> for the eventual computation of the value range
    per_metric_value_range: HashMap<String, (f64, f64)>,
    // Indicates how the aggregate series of every metric is computed
    aggregate_mode: TimeSeriesDataAggregateMode,
    // Allow caller to override the name of auto-generated aggregate series
    aggregate_series_name: Option<&'static str>,
    // Map<metric name, (sum, count)> - used to compute the sum or average of a metric
    // at a snapshot (will be cleared at every timestamp)
    per_metric_sum_count: HashMap<String, (f64, usize)>,
    // The initial timestamp, for the computation of every time_diff
    time_zero: Option<TimeEnum>,
    // The time_diff of the current snapshot
    cur_time_diff: u64,
    // The time_diff of every snapshot, where the last one is the current snapshot
    time_diff: Vec<u64>,
    // Whether the first accumulative value, where there is no previous value to compute the delta,
    // should be ignored (the delta is 0) or used (the delta is the value itself).
    ignore_first_accumulative_value: bool,
    // Map<metric_name.series_name, number of decreasing accumulative data> - used to count the
    // number of unexpected decreases of accumulative data, which are to be logged as warning
    // at the end of collection
    decreasing_accumulative_data: HashMap<String, usize>,
    // Use to set every metric's value range and ignore the results collected in
    // per_metric_value_range
    fixed_value_range: Option<(u64, u64)>,
    // Whether to drop series that only contains zero values.
    drop_all_zero_series: bool,
    // Map<derived metric name, parsed metric expression> - used to store parsed expression
    // and compute values of all derived metrics.
    derived_metric_expressions: HashMap<String, FlatEx<f64>>,
}

impl TimeSeriesDataProcessor {
    pub fn new(
        data_name: &'static str,
        aggregate_mode: TimeSeriesDataAggregateMode,
        time_zero: Option<TimeEnum>,
    ) -> Self {
        TimeSeriesDataProcessor {
            data_name,
            prev_data_points: HashMap::new(),
            per_metric_series: HashMap::new(),
            per_metric_value_range: HashMap::new(),
            aggregate_mode,
            aggregate_series_name: None,
            per_metric_sum_count: HashMap::new(),
            time_zero,
            cur_time_diff: 0,
            time_diff: Vec::new(),
            ignore_first_accumulative_value: true,
            decreasing_accumulative_data: HashMap::new(),
            fixed_value_range: None,
            drop_all_zero_series: false,
            derived_metric_expressions: HashMap::new(),
        }
    }

    /// Force to use the accumulative value itself, instead of 0, when there is no
    /// previous value to compute the delta.
    pub fn use_first_accumulative_value(&mut self) {
        self.ignore_first_accumulative_value = false;
    }

    /// Drop series that only contains zero values from the metric.
    pub fn drop_all_zero_series(&mut self) {
        self.drop_all_zero_series = true;
    }

    /// Override the name of the auto-generated aggregate series
    pub fn set_aggregate_series_name(&mut self, aggregate_series_name: &'static str) {
        self.aggregate_series_name = Some(aggregate_series_name);
    }

    /// Override every metric's value range
    pub fn set_fixed_value_range(&mut self, value_range: (u64, u64)) {
        self.fixed_value_range = Some(value_range);
    }

    /// Invoked before processing the data at the next snapshot, when all the data in the
    /// previous snapshot have been processed
    pub fn proceed_to_time(&mut self, time: TimeEnum) {
        self.finalize_cur_time_diff();
        self.cur_time_diff = match time - *self.time_zero.get_or_insert(time) {
            TimeEnum::TimeDiff(_time_diff) => _time_diff,
            TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
        };
        self.time_diff.push(self.cur_time_diff);
    }

    /// Register a derived metric that will be automatically computed from the values of
    /// other metrics. Variable metric names that have nonalphanumeric characters or
    /// contain exmex keywords need to be wrapped with "{}".
    pub fn register_derived_metric(
        &mut self,
        derived_metric_name: &str,
        derived_metric_expression: &str,
    ) {
        match exmex::parse::<f64>(derived_metric_expression) {
            Ok(parsed_expression) => {
                self.derived_metric_expressions
                    .insert(derived_metric_name.to_string(), parsed_expression);
            }
            Err(e) => {
                debug!(
                    "Failed to parse derived metric expression {derived_metric_expression}: {e}"
                );
            }
        }
    }

    /// Add a noncumulative data point to the aggregate series of the corresponding metric.
    /// Returns the data value if it was successfully added to the series.
    pub fn add_aggregate_data_point(
        &mut self,
        metric_name: &str,
        series_name: &str,
        data_value: f64,
    ) -> Option<f64> {
        self.add_data_point_impl(metric_name, series_name, data_value, false, true)
    }

    /// Add a noncumulative data point to the corresponding metric and series. Returns the data
    /// value if it was successfully added to the series.
    pub fn add_data_point(
        &mut self,
        metric_name: &str,
        series_name: &str,
        data_value: f64,
    ) -> Option<f64> {
        self.add_data_point_impl(metric_name, series_name, data_value, false, false)
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
        self.add_data_point_impl(metric_name, series_name, data_value, true, false)
    }

    fn add_data_point_impl(
        &mut self,
        metric_name: &str,
        series_name: &str,
        data_value: f64,
        accumulative: bool,
        is_aggregate: bool,
    ) -> Option<f64> {
        let metric_value = if accumulative {
            match self.get_delta_and_set_previous_value(metric_name, series_name, data_value) {
                Some(delta_value) => delta_value,
                None => return None,
            }
        } else {
            data_value
        };

        if !self.add_value_to_series(metric_name, series_name, metric_value, is_aggregate) {
            return None;
        }

        // Every series value in the metric accounts for its value range
        self.update_metric_value_range(metric_name, metric_value);

        if self.aggregate_mode == TimeSeriesDataAggregateMode::Average
            || self.aggregate_mode == TimeSeriesDataAggregateMode::Sum
        {
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

    /// Adds a value to a series at the current snapshot.
    /// Returns false when it already has a value at the current snapshot
    /// and the new value is ignored.
    fn add_value_to_series(
        &mut self,
        metric_name: &str,
        series_name: &str,
        value: f64,
        is_aggregate: bool,
    ) -> bool {
        let cur_idx = self.time_diff.len() - 1;
        let series = self
            .per_metric_series
            .entry(metric_name.to_string())
            .or_default()
            .entry(series_name.to_string())
            .or_insert_with(|| Series::new(series_name.to_string()));

        if series.values.len() > cur_idx {
            return false;
        }

        series.is_aggregate = is_aggregate;
        // For snapshots in which the series had no value, NaN is used.
        series.values.resize(cur_idx, f64::NAN);
        series.values.push(value);

        true
    }

    /// Extends the value range of a metric to a value.
    fn update_metric_value_range(&mut self, metric_name: &str, metric_value: f64) {
        let (min, max) = self
            .per_metric_value_range
            .entry(metric_name.to_string())
            .or_insert_with(|| (metric_value, metric_value));
        *min = (*min).min(metric_value);
        *max = (*max).max(metric_value);
    }

    /// Checks if a metric exists in the time-series data.
    pub fn has_metric(&self, metric_name: &str) -> bool {
        self.per_metric_series.contains_key(metric_name)
    }

    /// Keeps track of previous snapshot value and compute the delta as the series value for
    /// accumulative data. Skip the data point if the snapshot value unexpectedly decreases
    /// (but it still replaces the old snapshot data)
    pub fn get_delta_and_set_previous_value(
        &mut self,
        metric_name: &str,
        series_name: &str,
        data_value: f64,
    ) -> Option<f64> {
        match self
            .prev_data_points
            .entry(metric_name.to_string())
            .or_default()
            .insert(series_name.to_string(), (data_value, self.cur_time_diff))
        {
            Some((prev_value, prev_time_diff)) => {
                if prev_value > data_value || self.cur_time_diff <= prev_time_diff {
                    // In case there are a lot of invalid data, we don't want to pollute the logs
                    // so only use debug here and warn the total number of decreases at the end.
                    debug!(
                        "Invalid computation of accumulative data delta for {}.{}.{}: from ({}, {}s) to ({}, {}s)",
                        self.data_name, metric_name, series_name, prev_value, prev_time_diff, data_value, self.cur_time_diff
                    );
                    let error_counts = self
                        .decreasing_accumulative_data
                        .entry(format!("{metric_name}.{series_name}"))
                        .or_insert(0);
                    *error_counts += 1;
                    return None;
                }
                Some((data_value - prev_value) / (self.cur_time_diff - prev_time_diff) as f64)
            }
            None => Some(if self.ignore_first_accumulative_value {
                0.0
            } else {
                data_value
            }),
        }
    }

    /// Helper function to generate the aggregate series value based on the specified
    /// aggregate mode
    fn generate_aggregate_series_values(&mut self) {
        if !self.aggregate_mode.is_auto_generated() {
            return;
        }

        let aggregate_series_name =
            self.aggregate_series_name
                .unwrap_or_else(|| match self.aggregate_mode {
                    TimeSeriesDataAggregateMode::Average => "average",
                    TimeSeriesDataAggregateMode::Sum => "sum",
                    _ => "aggregate",
                });

        // Remove all entries instead of setting every sum and count to 0, since there is no
        // guarantee that all metrics still present at the next time_diff
        for (metric_name, (sum, count)) in std::mem::take(&mut self.per_metric_sum_count) {
            let aggregate_value = match self.aggregate_mode {
                TimeSeriesDataAggregateMode::Average => {
                    if count > 0 {
                        sum / count as f64
                    } else {
                        0.0
                    }
                }
                TimeSeriesDataAggregateMode::Sum => {
                    self.update_metric_value_range(&metric_name, sum);
                    sum
                }
                _ => break,
            };
            self.add_value_to_series(&metric_name, aggregate_series_name, aggregate_value, true);
        }
    }

    // Helper function to compute the value of all registered derived metrics at current time diff.
    fn compute_derived_metric_values(&mut self) {
        // Hold all derived (metric_name, series_name, value) to avoid ownership problem.
        let mut derived_metric_series_values: Vec<(String, String, f64)> = Vec::new();

        for (derived_metric_name, derived_metric_expressions) in &self.derived_metric_expressions {
            let variable_metric_names = derived_metric_expressions.var_names();
            let mut per_series_expression_values: HashMap<String, Vec<f64>> = HashMap::new();

            // Collect the latest series values for every metric used to compute the derived metric.
            for variable_metric_name in variable_metric_names {
                let Some(cur_metric_series) = self.per_metric_series.get(variable_metric_name)
                else {
                    continue;
                };

                for (series_name, series) in cur_metric_series {
                    let Some(&value) = series.values.get(self.time_diff.len() - 1) else {
                        continue;
                    };
                    per_series_expression_values
                        .entry(series_name.clone())
                        .or_default()
                        .push(value);
                }
            }

            // Compute the derived metric value for every series.
            for (series_name, expression_values) in per_series_expression_values {
                if expression_values.len() != variable_metric_names.len() {
                    continue;
                }
                if let Ok(derived_metric_value) =
                    derived_metric_expressions.eval(&expression_values)
                {
                    if derived_metric_value.is_finite() {
                        derived_metric_series_values.push((
                            derived_metric_name.clone(),
                            series_name,
                            derived_metric_value,
                        ));
                    }
                }
            }
        }

        for (derived_metric_name, series_name, derived_metric_value) in derived_metric_series_values
        {
            self.add_data_point(&derived_metric_name, &series_name, derived_metric_value);
        }
    }

    /// Invoked before proceeding to the next time diff or before getting the processed data.
    fn finalize_cur_time_diff(&mut self) {
        self.compute_derived_metric_values();
        self.generate_aggregate_series_values();
    }

    /// Generate the time-series data, with alphabetically sorted metric names.
    pub fn get_time_series_data(self) -> TimeSeriesData {
        self.get_time_series_data_impl(None, false)
    }

    /// Generate the time-series data, with the metric names sorted based on the specified order.
    pub fn get_time_series_data_with_metric_name_order(
        self,
        metric_name_order: Vec<&str>,
    ) -> TimeSeriesData {
        self.get_time_series_data_impl(Some(metric_name_order), false)
    }

    pub fn get_time_series_data_sorted_by_average(self) -> TimeSeriesData {
        self.get_time_series_data_impl(None, true)
    }

    fn get_time_series_data_impl(
        mut self,
        metric_name_order: Option<Vec<&str>>,
        sorted_by_average: bool,
    ) -> TimeSeriesData {
        self.finalize_cur_time_diff();

        // Log any unexpected decreases of accumulative data
        for (data_key, count) in self.decreasing_accumulative_data {
            warn!(
                "{}.{}: skipped {} data points due to unexpected decrease of accumulative data or invalid time diff.",
                self.data_name, data_key, count
            );
        }

        let mut time_series_data = TimeSeriesData {
            time_diff: self.time_diff,
            ..Default::default()
        };

        for (metric_name, cur_metric_series) in self.per_metric_series {
            let mut time_series_metric = TimeSeriesMetric::new(metric_name.clone());

            let mut all_series: Vec<Series> = cur_metric_series.into_values().collect();

            // If there is only one actual series in the metric, there is no need to show the
            // redundant auto-generated aggregate series
            if self.aggregate_mode.is_auto_generated() && all_series.len() == 2 {
                all_series.retain(|series| !series.is_aggregate);
            }

            if self.drop_all_zero_series {
                all_series.retain(|series| {
                    series.is_aggregate
                        || !series
                            .values
                            .iter()
                            .all(|&value| value == 0.0 || value.is_nan())
                });
            }
            all_series.sort_by(|s1, s2| cmp(&s1.series_name, &s2.series_name));

            time_series_metric.stats_series_idx = if all_series.len() <= 1 {
                // Simplest case - if there is only one series then use it for stat computation.
                // Or if a metric has no data point, use 0 as the default value.
                0
            } else if self.aggregate_mode == TimeSeriesDataAggregateMode::MaxSeries {
                // For max-aggregate mode, the series that has the maximum of average is the
                // aggregate of the metric
                let mut max_series_idx: usize = 0;
                let mut max_average: f64 = 0.0;
                for (idx, series) in all_series.iter().enumerate() {
                    let cur_series_average = get_average(&series.values);
                    if cur_series_average.is_some_and(|average| average > max_average) {
                        max_series_idx = idx;
                        max_average = cur_series_average.unwrap();
                    }
                }
                all_series[max_series_idx].is_aggregate = true;
                max_series_idx
            } else {
                // For auto-generated aggregate mode, the average series should already be created;
                // For custom-aggregate mode, the caller should have manually created the aggregate
                // series by calling add_accumulative_data_point.
                // For both of the case above, simply find the aggregate series index using its name.
                all_series
                    .iter()
                    .position(|series| series.is_aggregate)
                    .unwrap_or_else(|| {warn!("Cannot find aggregate series in metric {}.{}. The report data might be incorrect.", self.data_name, metric_name); 0})
            };

            time_series_metric.series = all_series;

            if let Some(aggregate_series) = time_series_metric
                .series
                .get(time_series_metric.stats_series_idx)
            {
                time_series_metric.stats = Statistics::from_values(&aggregate_series.values);
            }

            if let Some(fixed_value_range) = self.fixed_value_range {
                time_series_metric.value_range = fixed_value_range;
            } else if let Some((min, max)) = self.per_metric_value_range.get(&metric_name) {
                time_series_metric.value_range = (min.floor() as u64, max.ceil() as u64);
            }

            time_series_data
                .metrics
                .insert(metric_name, time_series_metric);
        }

        let mut all_metric_names: Vec<String> = time_series_data
            .metrics
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
        } else if sorted_by_average {
            all_metric_names.sort_by(|a, b| {
                time_series_data
                    .metrics
                    .get(b)
                    .unwrap()
                    .stats
                    .avg
                    .total_cmp(&time_series_data.metrics.get(a).unwrap().stats.avg)
            })
        } else {
            all_metric_names.sort();
        }
        time_series_data.sorted_metric_names = all_metric_names;

        time_series_data
    }
}

/// Edits a time-series data built by a TimeSeriesDataProcessor.
pub struct TimeSeriesDataEditor<'a> {
    time_series_data: &'a mut TimeSeriesData,
    // The series copy started by start_series_copy and not finished yet
    copy_state: Option<SeriesCopyState<'a>>,
}

/// The state of a series copy, from start_series_copy to finish_series_copy.
struct SeriesCopyState<'a> {
    // The time-series data that series are copied from
    source: &'a TimeSeriesData,
    // The index in the time_diff of every time of the source
    source_time_indices: Vec<usize>,
    // How the aggregate series of the target metrics is computed, and its name
    aggregate_mode: TimeSeriesDataAggregateMode,
    aggregate_series_name: &'static str,
    // The metrics that series are copied into
    target_metric_names: HashSet<String>,
}

impl<'a> TimeSeriesDataEditor<'a> {
    pub fn new(time_series_data: &'a mut TimeSeriesData) -> Self {
        TimeSeriesDataEditor {
            time_series_data,
            copy_state: None,
        }
    }

    /// Starts copying series from the source. The time diff of the two time-series data are
    /// merged, and the exisintg values are re-placed based on the merged time_diff. upon
    /// copy finish, the aggregate series will be recomputed based on the aggregate mode.
    pub fn start_series_copy(
        &mut self,
        source: &'a TimeSeriesData,
        aggregate_mode: TimeSeriesDataAggregateMode,
        aggregate_series_name: &'static str,
    ) {
        assert!(
            self.copy_state.is_none(),
            "finish_series_copy must be called before another start_series_copy"
        );

        // Merge time diffs of the two time-series data.
        let old_time_diff = std::mem::take(&mut self.time_series_data.time_diff);
        let time_diff = &mut self.time_series_data.time_diff;
        // The index in the merged time_diff of every old time and of every time of the source
        let (mut old_time_indices, mut source_time_indices) = (Vec::new(), Vec::new());
        let (mut i, mut j) = (0, 0);
        while let Some(&time) = [old_time_diff.get(i), source.time_diff.get(j)]
            .into_iter()
            .flatten()
            .min()
        {
            if old_time_diff.get(i) == Some(&time) {
                old_time_indices.push(time_diff.len());
                i += 1;
            }
            if source.time_diff.get(j) == Some(&time) {
                source_time_indices.push(time_diff.len());
                j += 1;
            }
            time_diff.push(time);
        }

        // Place the existing values based on the merged time diff.
        let num_times = time_diff.len();
        for series in self
            .time_series_data
            .metrics
            .values_mut()
            .flat_map(|metric| &mut metric.series)
        {
            series.values = Self::place_values(&series.values, &old_time_indices, num_times);
        }

        self.copy_state = Some(SeriesCopyState {
            source,
            source_time_indices,
            aggregate_mode,
            aggregate_series_name,
            target_metric_names: HashSet::new(),
        });
    }

    /// Copies a series of the source into a metric, which is created if missing.
    pub fn copy_series(
        &mut self,
        source_metric_name: &str,
        source_series_name: &str,
        target_metric_name: &str,
        target_series_name: &str,
    ) {
        let copy_state = self
            .copy_state
            .as_mut()
            .expect("start_series_copy must be called first");
        let Some(source_series) =
            copy_state
                .source
                .metrics
                .get(source_metric_name)
                .and_then(|metric| {
                    metric
                        .series
                        .iter()
                        .find(|series| series.series_name == source_series_name)
                })
        else {
            return;
        };

        let values = Self::place_values(
            &source_series.values,
            &copy_state.source_time_indices,
            self.time_series_data.time_diff.len(),
        );

        if !self
            .time_series_data
            .sorted_metric_names
            .iter()
            .any(|metric_name| metric_name == target_metric_name)
        {
            self.time_series_data
                .sorted_metric_names
                .insert(0, target_metric_name.to_string());
        }

        self.time_series_data
            .metrics
            .entry(target_metric_name.to_string())
            .or_insert_with(|| TimeSeriesMetric::new(target_metric_name.to_string()))
            .series
            .insert(
                0,
                Series {
                    values,
                    ..Series::new(target_series_name.to_string())
                },
            );

        copy_state
            .target_metric_names
            .insert(target_metric_name.to_string());
    }

    /// Finishes copying series. Recompute the aggregate series, stats and value range of every
    /// metric that was copied into.
    pub fn finish_series_copy(&mut self) {
        let copy_state = self
            .copy_state
            .take()
            .expect("start_series_copy must be called first");

        // Recompute the aggregate series.
        let num_times = self.time_series_data.time_diff.len();
        for metric_name in &copy_state.target_metric_names {
            let metric = self.time_series_data.metrics.get_mut(metric_name).unwrap();
            metric.series.retain(|series| !series.is_aggregate);
            let aggregate_values: Vec<f64> = match copy_state.aggregate_mode {
                // The sum of the series that have a value at every time
                TimeSeriesDataAggregateMode::Sum => (0..num_times)
                    .map(|i| {
                        metric
                            .series
                            .iter()
                            .filter_map(|series| series.values.get(i).copied())
                            .filter(|value| !value.is_nan())
                            .reduce(|sum, value| sum + value)
                            .unwrap_or(f64::NAN)
                    })
                    .collect(),
                _ => unimplemented!("{:?} aggregate of copied series", copy_state.aggregate_mode),
            };

            metric.stats = Statistics::from_values(&aggregate_values);

            // Update the value range using the updated values.
            let (min, max) = metric
                .series
                .iter()
                .flat_map(|series| &series.values)
                .chain(&aggregate_values)
                .fold(
                    (metric.value_range.0 as f64, metric.value_range.1 as f64),
                    |(min, max), &value| (min.min(value), max.max(value)),
                );
            metric.value_range = (min.floor() as u64, max.ceil() as u64);

            metric.series.insert(
                0,
                Series {
                    values: aggregate_values,
                    is_aggregate: true,
                    ..Series::new(copy_state.aggregate_series_name.to_string())
                },
            );
            metric.stats_series_idx = 0;
        }
    }

    /// Places every value at its index in a time_diff of length num_times. NaN is used
    /// if a value doesn't exist at a time.
    fn place_values(values: &[f64], indices: &[usize], num_times: usize) -> Vec<f64> {
        let mut placed_values = vec![f64::NAN; num_times];
        for (&index, &value) in indices.iter().zip(values) {
            placed_values[index] = value;
        }
        placed_values
    }
}

/// The aggregate series of every metric is auto-generated as the average of all series values
/// at a timestamp.
macro_rules! time_series_data_processor_with_average_aggregate {
    ($time_zero:expr) => {{
        crate::data::common::time_series_data_processor::TimeSeriesDataProcessor::new(
            crate::get_data_name_from_type::<Self>(),
            crate::data::common::time_series_data_processor::TimeSeriesDataAggregateMode::Average,
            $time_zero,
        )
    }};
}

/// The aggregate series of every metric is auto-generated as the sum of all series values
/// at a timestamp.
macro_rules! time_series_data_processor_with_sum_aggregate {
    ($time_zero:expr) => {{
        crate::data::common::time_series_data_processor::TimeSeriesDataProcessor::new(
            crate::get_data_name_from_type::<Self>(),
            crate::data::common::time_series_data_processor::TimeSeriesDataAggregateMode::Sum,
            $time_zero,
        )
    }};
}

/// The aggregate series of every metric is the series with the maximum average value.
/// Use this option when the series of a metric cover a hierarchy, where the values of
/// a series is the sum of other series. For example, diskstats metrics include the root
/// partition as well as all the subpartitions it contains.
macro_rules! time_series_data_processor_with_max_series_aggregate {
    ($time_zero:expr) => {{
        crate::data::common::time_series_data_processor::TimeSeriesDataProcessor::new(
            crate::get_data_name_from_type::<Self>(),
            crate::data::common::time_series_data_processor::TimeSeriesDataAggregateMode::MaxSeries,
            $time_zero,
        )
    }};
}

/// The aggregate series needs to be manually added by calling add_aggregate_data.
macro_rules! time_series_data_processor_with_custom_aggregate {
    ($time_zero:expr) => {{
        crate::data::common::time_series_data_processor::TimeSeriesDataProcessor::new(
            crate::get_data_name_from_type::<Self>(),
            crate::data::common::time_series_data_processor::TimeSeriesDataAggregateMode::Custom,
            $time_zero,
        )
    }};
}

pub(crate) use time_series_data_processor_with_average_aggregate;
pub(crate) use time_series_data_processor_with_custom_aggregate;
pub(crate) use time_series_data_processor_with_max_series_aggregate;
pub(crate) use time_series_data_processor_with_sum_aggregate;

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

    fn make_processor(mode: TimeSeriesDataAggregateMode) -> TimeSeriesDataProcessor {
        TimeSeriesDataProcessor::new(TEST_DATA, mode, None)
    }

    fn data_series<'a>(ts: &'a TimeSeriesData, metric: &str) -> Vec<&'a Series> {
        let m = &ts.metrics[metric];
        let mut v: Vec<_> = m.series.iter().filter(|s| !s.is_aggregate).collect();
        v.sort_by_key(|s| s.series_name.clone());
        v
    }

    fn agg_series<'a>(ts: &'a TimeSeriesData, metric: &str) -> Option<&'a Series> {
        ts.metrics[metric].series.iter().find(|s| s.is_aggregate)
    }

    /// The (time, value) pairs of a series, without the times it has no value at.
    fn points(ts: &TimeSeriesData, series: &Series) -> Vec<(u64, f64)> {
        ts.time_diff
            .iter()
            .copied()
            .zip(series.values.iter().copied())
            .filter(|(_, value)| !value.is_nan())
            .collect()
    }

    // =======================================================================
    // add_data_point (non-accumulative)
    // =======================================================================

    #[test]
    fn test_non_accumulative_single_metric_single_series() {
        let times = make_times(3, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        assert_eq!(ts.time_diff, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_non_accumulative_multiple_series() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

        p.proceed_to_time(times[0]);
        p.add_data_point("cpu", "core0", 50.0);
        p.add_data_point("cpu", "core1", 70.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("cpu", "core0", 60.0);
        p.add_data_point("cpu", "core1", 80.0);

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "cpu");
        assert_eq!(s.len(), 2);

        let core0 = s.iter().find(|s| s.series_name == "core0").unwrap();
        let core1 = s.iter().find(|s| s.series_name == "core1").unwrap();
        assert_eq!(core0.values, vec![50.0, 60.0]);
        assert_eq!(core1.values, vec![70.0, 80.0]);
    }

    // =======================================================================
    // add_accumulative_data_point
    // =======================================================================

    #[test]
    fn test_accumulative_computes_deltas() {
        let times = make_times(3, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        assert_eq!(points(&ts, s[0]), vec![(0, 0.0), (2, 1200.0)]);
    }

    #[test]
    fn test_accumulative_decrease_only_affects_decreased_series() {
        // One series decreases, the other doesn't — only the decreased one is skipped
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

        p.proceed_to_time(times[0]);
        p.add_accumulative_data_point("bytes", "eth0", 1000.0);
        p.add_accumulative_data_point("bytes", "eth1", 2000.0);

        p.proceed_to_time(times[1]);
        p.add_accumulative_data_point("bytes", "eth0", 500.0); // decrease → skipped
        p.add_accumulative_data_point("bytes", "eth1", 2500.0); // normal

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        let eth0 = s.iter().find(|s| s.series_name == "eth0").unwrap();
        let eth1 = s.iter().find(|s| s.series_name == "eth1").unwrap();

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        assert_eq!(
            points(&ts, s[0]),
            vec![(0, 0.0), (2, 300.0), (3, 400.0), (4, 300.0), (5, 300.0)]
        );
    }

    #[test]
    fn test_accumulative_multiple_series_independent_deltas() {
        let times = make_times(3, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

        let eth0_vals = [100.0, 200.0, 350.0];
        let eth1_vals = [500.0, 800.0, 900.0];

        for (i, t) in times.iter().enumerate() {
            p.proceed_to_time(*t);
            p.add_accumulative_data_point("bytes", "eth0", eth0_vals[i]);
            p.add_accumulative_data_point("bytes", "eth1", eth1_vals[i]);
        }

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "bytes");
        let eth0 = s.iter().find(|s| s.series_name == "eth0").unwrap();
        let eth1 = s.iter().find(|s| s.series_name == "eth1").unwrap();

        assert_eq!(eth0.values, vec![0.0, 100.0, 150.0]);
        assert_eq!(eth1.values, vec![0.0, 300.0, 100.0]);
    }

    // =======================================================================
    // Average aggregate mode
    // =======================================================================

    #[test]
    fn test_average_aggregate_single_series_stripped() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
    // Sum aggregate mode
    // =======================================================================

    #[test]
    fn test_sum_aggregate_single_series_stripped() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Sum);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 50.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 60.0);

        let ts = p.get_time_series_data();
        assert!(
            agg_series(&ts, "m").is_none(),
            "Sum aggregate should be stripped with 1 series"
        );
        assert_eq!(ts.metrics["m"].series.len(), 1);
    }

    #[test]
    fn test_sum_aggregate_multiple_series() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Sum);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 20.0);
        p.add_data_point("m", "c", 30.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 40.0);
        p.add_data_point("m", "b", 50.0);
        p.add_data_point("m", "c", 60.0);

        let ts = p.get_time_series_data();
        let agg = agg_series(&ts, "m").expect("Sum aggregate should exist with 3 series");
        // Sum: 10+20+30=60, 40+50+60=150
        assert_eq!(agg.values, vec![60.0, 150.0]);
        assert_eq!(agg.series_name.as_str(), "sum");
        // The value range must account for the aggregate sum (max 150), not just the
        // individual series data points (max 60).
        assert_eq!(ts.metrics["m"].value_range, (10, 150));
    }

    #[test]
    fn test_sum_aggregate_stats_computed_from_aggregate() {
        let times = make_times(3, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Sum);

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
        // Sum values: [40, 60, 80]
        let stats = &ts.metrics["m"].stats;
        assert_eq!(stats.min, 40.0);
        assert_eq!(stats.max, 80.0);
        assert_eq!(stats.avg, 60.0);
    }

    // =======================================================================
    // MaxSeries aggregate mode
    // =======================================================================

    #[test]
    fn test_max_series_stats_from_highest_average() {
        let times = make_times(3, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::MaxSeries);

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
    fn test_max_series_is_the_aggregate() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::MaxSeries);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 20.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 30.0);
        p.add_data_point("m", "b", 40.0);

        // "b" has the larger average
        let ts = p.get_time_series_data();
        assert_eq!(agg_series(&ts, "m").unwrap().series_name, "b");
        let m = &ts.metrics["m"];
        assert_eq!(m.series[m.stats_series_idx].series_name, "b");
    }

    #[test]
    fn test_repeated_value_ignored() {
        for (mode, aggregate, value_range) in [
            (TimeSeriesDataAggregateMode::Average, 65.0, (30, 100)),
            (TimeSeriesDataAggregateMode::Sum, 130.0, (30, 130)),
        ] {
            let mut p = make_processor(mode);
            p.proceed_to_time(make_times(1, 1)[0]);
            p.add_data_point("m", "a", 100.0);
            // "a" already has a value at this snapshot
            p.add_data_point("m", "a", 10.0);
            p.add_data_point("m", "b", 30.0);

            let ts = p.get_time_series_data();
            let a = data_series(&ts, "m")
                .into_iter()
                .find(|series| series.series_name == "a")
                .unwrap();
            assert_eq!(a.values, vec![100.0]);
            assert_eq!(agg_series(&ts, "m").unwrap().values, vec![aggregate]);
            assert_eq!(ts.metrics["m"].value_range, value_range);
        }
    }

    // =======================================================================
    // Custom aggregate mode
    // =======================================================================

    #[test]
    fn test_custom_mode_no_aggregate() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Custom);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

        for t in &times {
            p.proceed_to_time(*t);
            p.add_data_point("m", "s", 1.0);
        }

        let ts = p.get_time_series_data();
        assert_eq!(ts.time_diff, vec![0, 5, 10, 15]);
    }

    // =======================================================================
    // Metric name ordering
    // =======================================================================

    #[test]
    fn test_default_alphabetical_ordering() {
        let times = make_times(1, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Custom);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "z_series", 1.0);
        p.add_data_point("m", "a_series", 2.0);
        p.add_data_point("m", "m_series", 3.0);

        let ts = p.get_time_series_data();
        let names: Vec<_> = ts.metrics["m"]
            .series
            .iter()
            .map(|s| s.series_name.as_str())
            .collect();
        assert_eq!(names, vec!["a_series", "m_series", "z_series"]);
    }

    // =======================================================================
    // Empty / no data
    // =======================================================================

    #[test]
    fn test_no_data_points() {
        let p = make_processor(TimeSeriesDataAggregateMode::Average);
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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::MaxSeries);

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
        let mut p = make_processor(TimeSeriesDataAggregateMode::Custom);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "only", 5.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "only", 15.0);

        let ts = p.get_time_series_data();
        let stats = &ts.metrics["m"].stats;
        assert_eq!(stats.min, 5.0);
        assert_eq!(stats.max, 15.0);
    }

    // =======================================================================
    // All-zero series
    // =======================================================================

    #[test]
    fn test_all_zero_series_dropped() {
        let times = make_times(5, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.drop_all_zero_series();

        for (i, t) in times.iter().enumerate() {
            p.proceed_to_time(*t);
            p.add_data_point("m", "s1", 0.0);
            p.add_data_point("m", "s2", (i * 2) as f64);
            p.add_data_point("zero", "s1", 0.0);
            p.add_data_point("zero", "s2", 0.0);
            if i >= 2 {
                // All zero after the times without values
                p.add_data_point("zero", "s3", 0.0);
            }
            p.add_data_point("single", "s1", 0.0);
        }

        let ts = p.get_time_series_data();
        let names: Vec<&str> = data_series(&ts, "m")
            .iter()
            .map(|s| s.series_name.as_str())
            .collect();
        assert_eq!(names, vec!["s2"]);
        // The aggregate is kept, still averages over all series, and stays the stats series
        let metric = &ts.metrics["m"];
        assert!(metric.series[metric.stats_series_idx].is_aggregate);
        assert_eq!(
            agg_series(&ts, "m").unwrap().values,
            vec![0.0, 1.0, 2.0, 3.0, 4.0]
        );
        // An all-zero metric only keeps its aggregate
        assert!(data_series(&ts, "zero").is_empty());
        assert_eq!(agg_series(&ts, "zero").unwrap().values, vec![0.0; 5]);
        // Without the redundant aggregate, a metric with one all-zero series has no series left
        assert!(ts.metrics["single"].series.is_empty());
    }

    // =======================================================================
    // Fixed value range
    // =======================================================================

    #[test]
    fn test_fixed_value_range_overrides_computed() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.set_fixed_value_range((0, 100));

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "s", 5.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "s", 50.0);

        let ts = p.get_time_series_data();
        // Should use fixed range (0, 100) instead of computed (5, 50)
        assert_eq!(ts.metrics["m"].value_range, (0, 100));
    }

    #[test]
    fn test_fixed_value_range_applies_to_all_metrics() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.set_fixed_value_range((0, 1000));

        p.proceed_to_time(times[0]);
        p.add_data_point("cpu", "s", 10.0);
        p.add_data_point("mem", "s", 500.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("cpu", "s", 20.0);
        p.add_data_point("mem", "s", 600.0);

        let ts = p.get_time_series_data();
        assert_eq!(ts.metrics["cpu"].value_range, (0, 1000));
        assert_eq!(ts.metrics["mem"].value_range, (0, 1000));
    }

    // =======================================================================
    // Sorted by average
    // =======================================================================

    #[test]
    fn test_sorted_by_average_descending() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

        p.proceed_to_time(times[0]);
        p.add_data_point("low", "s", 1.0);
        p.add_data_point("high", "s", 100.0);
        p.add_data_point("mid", "s", 50.0);

        p.proceed_to_time(times[1]);
        p.add_data_point("low", "s", 3.0);
        p.add_data_point("high", "s", 100.0);
        p.add_data_point("mid", "s", 50.0);

        let ts = p.get_time_series_data_sorted_by_average();
        assert_eq!(ts.sorted_metric_names, vec!["high", "mid", "low"]);
    }

    // =======================================================================
    // get_delta_and_set_previous_value
    // =======================================================================

    #[test]
    fn test_get_delta_first_call_returns_zero() {
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        assert_eq!(
            p.get_delta_and_set_previous_value("m", "s", 500.0),
            Some(0.0)
        );
    }

    #[test]
    fn test_get_delta_increasing_returns_delta() {
        // The delta is normalized by the elapsed time between the two values.
        let times = make_times(4, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.proceed_to_time(times[0]);
        p.get_delta_and_set_previous_value("m", "s", 100.0);
        p.proceed_to_time(times[1]);
        assert_eq!(
            p.get_delta_and_set_previous_value("m", "s", 350.0),
            Some(250.0)
        );
        // Two seconds elapsed until the next value: the delta is halved.
        p.proceed_to_time(times[3]);
        assert_eq!(
            p.get_delta_and_set_previous_value("m", "s", 550.0),
            Some(100.0)
        );
    }

    #[test]
    fn test_get_delta_same_time_diff_returns_none() {
        // A second value at the same time_diff cannot produce a rate (no time elapsed).
        let times = make_times(1, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.proceed_to_time(times[0]);
        p.get_delta_and_set_previous_value("m", "s", 100.0);
        assert_eq!(p.get_delta_and_set_previous_value("m", "s", 350.0), None);
    }

    #[test]
    fn test_get_delta_decreasing_returns_none() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.proceed_to_time(times[0]);
        p.get_delta_and_set_previous_value("m", "s", 100.0);
        p.proceed_to_time(times[1]);
        assert_eq!(p.get_delta_and_set_previous_value("m", "s", 50.0), None);
    }

    #[test]
    fn test_get_delta_after_decrease_uses_decreased_value() {
        let times = make_times(3, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.proceed_to_time(times[0]);
        p.get_delta_and_set_previous_value("m", "s", 100.0);
        p.proceed_to_time(times[1]);
        p.get_delta_and_set_previous_value("m", "s", 30.0); // decrease, but 30 is stored
        p.proceed_to_time(times[2]);
        assert_eq!(
            p.get_delta_and_set_previous_value("m", "s", 80.0),
            Some(50.0)
        ); // (80-30) / 1s
    }

    // =======================================================================
    // Aggregate series naming
    // =======================================================================

    #[test]
    fn test_average_aggregate_series_named_average() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 20.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 30.0);
        p.add_data_point("m", "b", 40.0);

        let ts = p.get_time_series_data();
        let agg = agg_series(&ts, "m").unwrap();
        assert_eq!(agg.series_name.as_str(), "average");
    }

    #[test]
    fn test_custom_aggregate_series_name_override() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.set_aggregate_series_name("total");

        p.proceed_to_time(times[0]);
        p.add_data_point("m", "a", 10.0);
        p.add_data_point("m", "b", 20.0);
        p.proceed_to_time(times[1]);
        p.add_data_point("m", "a", 30.0);
        p.add_data_point("m", "b", 40.0);

        let ts = p.get_time_series_data();
        let agg = agg_series(&ts, "m").unwrap();
        assert_eq!(agg.series_name.as_str(), "total");
    }

    // =======================================================================
    // Derived metrics
    // =======================================================================

    #[test]
    fn test_derived_metric_computed_for_every_series() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Average);
        p.register_derived_metric("used_percent", "used / total * 100");

        p.proceed_to_time(times[0]);
        p.add_data_point("used", "node0", 25.0);
        p.add_data_point("total", "node0", 100.0);
        p.add_data_point("used", "node1", 30.0);
        p.add_data_point("total", "node1", 200.0);

        // The data of the last snapshot is only finalized when the data is generated
        p.proceed_to_time(times[1]);
        p.add_data_point("used", "node0", 50.0);
        p.add_data_point("total", "node0", 100.0);
        p.add_data_point("used", "node1", 100.0);
        p.add_data_point("total", "node1", 200.0);

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "used_percent");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].series_name.as_str(), "node0");
        assert_eq!(s[0].values, vec![25.0, 50.0]);
        assert_eq!(ts.time_diff, vec![0, 1]);
        assert_eq!(s[1].series_name.as_str(), "node1");
        assert_eq!(s[1].values, vec![15.0, 50.0]);

        // The aggregate series of a derived metric aggregates the derived values, so it is
        // the average of the per-series ratios, not the ratio of the aggregate values
        let agg = agg_series(&ts, "used_percent")
            .expect("Derived metric should have an aggregate series");
        assert_eq!(agg.values, vec![20.0, 50.0]);
    }

    #[test]
    fn test_derived_metric_skips_values_it_cannot_compute() {
        let times = make_times(2, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Custom);
        p.register_derived_metric("ratio", "a / b");

        p.proceed_to_time(times[0]);
        p.add_data_point("a", "s1", 10.0);
        p.add_data_point("b", "s1", 2.0);
        // Division by zero: the non-finite value must not reach the series
        p.add_data_point("a", "s2", 10.0);
        p.add_data_point("b", "s2", 0.0);

        p.proceed_to_time(times[1]);
        // "b" has no value for "s1" at this time_diff, and the previous one must not be reused
        p.add_data_point("a", "s1", 20.0);
        p.add_data_point("a", "s2", 10.0);
        p.add_data_point("b", "s2", 5.0);

        let ts = p.get_time_series_data();
        let s = data_series(&ts, "ratio");
        assert_eq!(s.len(), 2);
        assert_eq!(points(&ts, s[0]), vec![(0, 5.0)]);
        assert_eq!(points(&ts, s[1]), vec![(1, 2.0)]);
    }

    #[test]
    fn test_derived_metric_with_braced_metric_names() {
        // "min" is an exmex operator and "(" is not a valid variable name character, so
        // both names have to be braced. The unbraced expression fails to parse and its
        // derived metric is never computed.
        let times = make_times(1, 1);
        let mut p = make_processor(TimeSeriesDataAggregateMode::Custom);
        p.register_derived_metric("unbraced", "min_free_kbytes / Active(anon)");
        p.register_derived_metric("braced", "{min_free_kbytes} / {Active(anon)}");

        p.proceed_to_time(times[0]);
        p.add_data_point("min_free_kbytes", "value", 64.0);
        p.add_data_point("Active(anon)", "value", 256.0);

        let ts = p.get_time_series_data();
        assert!(!ts.metrics.contains_key("unbraced"));
        assert_eq!(data_series(&ts, "braced")[0].values, vec![0.25]);
    }

    // =======================================================================
    // TimeSeriesDataEditor
    // =======================================================================

    #[test]
    fn test_copy_series() {
        let mut target = TimeSeriesData {
            time_diff: vec![0, 2],
            sorted_metric_names: vec!["m".to_string()],
            ..Default::default()
        };
        let mut metric = TimeSeriesMetric::new("m".to_string());
        metric.series.push(Series {
            values: vec![1.0, 2.0],
            ..Series::new("a".to_string())
        });
        target.metrics.insert("m".to_string(), metric);
        // The source has two times in the same second
        let mut source = TimeSeriesData {
            time_diff: vec![1, 2, 2],
            ..Default::default()
        };
        let mut metric = TimeSeriesMetric::new("source".to_string());
        metric.series.push(Series {
            values: vec![3.0, 4.0, 5.0],
            ..Series::new("1_b".to_string())
        });
        source.metrics.insert("source".to_string(), metric);

        let mut editor = TimeSeriesDataEditor::new(&mut target);
        editor.start_series_copy(&source, TimeSeriesDataAggregateMode::Sum, "total");
        editor.copy_series("source", "1_b", "m", "b");
        editor.copy_series("source", "1_b", "new", "b");
        editor.finish_series_copy();

        assert_eq!(target.time_diff, vec![0, 1, 2, 2]);
        assert_eq!(target.sorted_metric_names, vec!["new", "m"]);
        let m = &target.metrics["m"];
        let names: Vec<&str> = m.series.iter().map(|s| s.series_name.as_str()).collect();
        assert_eq!(names, vec!["total", "b", "a"]);
        assert_eq!(
            points(&target, &m.series[0]),
            vec![(0, 1.0), (1, 3.0), (2, 6.0), (2, 5.0)]
        );
        assert_eq!(points(&target, &m.series[2]), vec![(0, 1.0), (2, 2.0)]);
        assert_eq!(
            (m.stats_series_idx, m.stats.avg, m.value_range),
            (0, 3.75, (0, 6))
        );
        // A new metric with one series still has the aggregate
        let names: Vec<&str> = target.metrics["new"]
            .series
            .iter()
            .map(|s| s.series_name.as_str())
            .collect();
        assert_eq!(names, vec!["total", "b"]);
    }
}
