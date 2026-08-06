use crate::analytics::{AnalyticalEngine, DataFindings};
use crate::computations::Statistics;
use crate::data::aperf_stats::AperfStats;
use crate::data::common::data_formats::{
    AperfData, DataFormat, ProcessedData, Series, TimeSeriesMetric,
};
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use crate::data::common::utils::{combine_value_ranges, topological_sort};
use crate::data::processes::Processes;
use crate::data::TimeEnum;
use crate::{data::Data, data::ReportData};
use crate::{get_data_name_from_type, ProcessMetric};
use anyhow::{bail, Result};
use log::{debug, error, info};
use std::collections::{BTreeMap, HashMap};
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct ReportParams {
    pub run_name: String,
    pub tmp_dir: PathBuf,
    pub run_data_dir: PathBuf,
    pub report_dir: PathBuf,
    /// Wall-clock start of `collect_data_serial`. Used to anchor time_diff=0 in
    /// time-series data to the actual collection start rather than the first sample time.
    pub collection_start: Option<TimeEnum>,
    /// Whether the collection of PMU counters is "grouped" or "ungrouped". An empty \
    /// string means a legacy run before PMU config revamp.
    pub pmu_counter_mode: String,
    /// The PIDs of the APerf collector process itself and all long-running processes
    /// that it launched.
    pub aperf_process_pids: Vec<u32>,
    /// System page size in bytes at collection time
    pub page_size: u64,
}

impl ReportParams {
    pub fn new() -> Self {
        ReportParams {
            run_name: String::new(),
            run_data_dir: PathBuf::new(),
            tmp_dir: PathBuf::new(),
            report_dir: PathBuf::new(),
            collection_start: None,
            pmu_counter_mode: String::new(),
            aperf_process_pids: Vec::new(),
            page_size: 0,
        }
    }
}

#[derive(Default)]
pub struct DataProcessingEngine {
    per_run_report_params: HashMap<String, ReportParams>,
    data_processors: HashMap<String, DataProcessor>,
}

impl DataProcessingEngine {
    pub fn new(per_run_report_params: HashMap<String, ReportParams>) -> Self {
        DataProcessingEngine {
            per_run_report_params,
            data_processors: HashMap::new(),
        }
    }

    pub fn add_data_processor(&mut self, data_processor: DataProcessor) {
        self.data_processors
            .insert(data_processor.data_name.to_string(), data_processor);
    }

    pub fn process_raw_data(&mut self, run_name: &str) -> Result<()> {
        let report_params = self
            .per_run_report_params
            .get(run_name)
            .unwrap_or_else(|| panic!("Processing data with unexpected run name {run_name}"));

        for data_processor in self.data_processors.values_mut() {
            if let Err(e) = data_processor.process_raw_data(report_params) {
                error!(
                    "Error while processing raw {} data for run {run_name}: {:#?}",
                    data_processor.data_name, e
                );
            }
        }

        let num_unavailable_data: usize = self
            .all_data_names()
            .map(|data_name| !self.is_data_available(run_name, data_name) as usize)
            .sum();
        if num_unavailable_data == self.data_processors.len() {
            bail!("Run {run_name} is invalid - no raw data can be processed.");
        }

        Ok(())
    }

    pub fn run_analytics(
        &mut self,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) -> HashMap<String, DataFindings> {
        let mut analytical_engine = AnalyticalEngine::default();
        for (data_name, data_processor) in &mut self.data_processors {
            analytical_engine.add_data_rules(
                data_name.clone(),
                data_processor.data.get_analytical_rules(),
            );
            analytical_engine
                .add_processed_data(data_name.clone(), &mut data_processor.processed_data);
        }

        info!("Running analytical rules");
        analytical_engine.run(processed_data_accessor);

        analytical_engine.findings
    }

    pub fn is_data_available(&self, run_name: &str, data_name: &str) -> bool {
        self.data_processors
            .get(data_name)
            .is_some_and(|data_processor| data_processor.processed_data.runs.contains_key(run_name))
    }

    /// Logics to be run after all raw data has been processed.
    pub fn post_process_data(&mut self) {
        // Perform the inter-data post-processing logics first.
        copy_aperf_process_metrics_to_aperf_stats(
            &mut self.data_processors,
            &self.per_run_report_params,
        );

        // Perform the intra-data logics for each data type.
        for data_processor in self.data_processors.values_mut() {
            data_processor.post_process_data();
        }
    }

    pub fn all_data_names(&self) -> impl Iterator<Item = &String> {
        self.data_processors.keys()
    }

    pub fn get_processed_data(&self, data_name: &str) -> Option<&ProcessedData> {
        self.data_processors
            .get(data_name)
            .map(|data_processor| &data_processor.processed_data)
    }
}

/// Extract the metric of APerf process and all of its subprocesses from processes data
/// and add them to the aperf_stats data, to monitor APerf performance in the report.
fn copy_aperf_process_metrics_to_aperf_stats(
    data_processors: &mut HashMap<String, DataProcessor>,
    per_run_report_params: &HashMap<String, ReportParams>,
) {
    // Store all extracted APerf process series, in the format of
    // Map<run_name, Map<process_metric_name, Vec<process_series>>>
    let mut per_run_aperf_process_metrics: HashMap<String, HashMap<String, Vec<Series>>> =
        HashMap::new();

    let processes_data_processor = match data_processors.get(get_data_name_from_type::<Processes>())
    {
        Some(processes_data_processor) => processes_data_processor,
        None => return,
    };

    // Extract APerf process series from the metrics within processes data.
    for (run_name, cur_run_data) in &processes_data_processor.processed_data.runs {
        let aperf_process_pids = match per_run_report_params.get(run_name) {
            Some(report_params) => &report_params.aperf_process_pids,
            None => continue,
        };

        let cur_run_processes_data = match cur_run_data {
            AperfData::TimeSeries(time_series_data) => time_series_data,
            _ => continue,
        };

        for metric_name in &cur_run_processes_data.sorted_metric_names {
            let aperf_stats_metric_name = match ProcessMetric::from_str(metric_name) {
                Ok(process_metric) => {
                    // Skip metrics that are not too helpful.
                    match process_metric {
                        ProcessMetric::NumberProcesses
                        | ProcessMetric::ResidentSetSize
                        | ProcessMetric::VirtualMemorySize => continue,
                        _ => process_metric.to_aperf_stat_metric_name(),
                    }
                }
                Err(_) => continue,
            };

            // For every processes metric, locate and extract all series for APerf processes.
            if let Some(metric) = cur_run_processes_data.metrics.get(metric_name) {
                for aperf_process_pid in aperf_process_pids {
                    if let Some(aperf_process_series) = metric
                        .series
                        .iter()
                        .find(|s| s.series_name.starts_with(&format!("{aperf_process_pid}_")))
                    {
                        per_run_aperf_process_metrics
                            .entry(run_name.clone())
                            .or_default()
                            .entry(aperf_stats_metric_name.clone())
                            .or_default()
                            .push(aperf_process_series.clone());
                    }
                }
            }
        }
    }

    if per_run_aperf_process_metrics.is_empty() {
        return;
    }

    let aperf_stats_data_processor =
        match data_processors.get_mut(get_data_name_from_type::<AperfStats>()) {
            Some(aperf_stats_data_processor) => aperf_stats_data_processor,
            None => return,
        };

    // Add extracted APerf process series to APerf stats metrics.
    for (run_name, aperf_process_metrics) in per_run_aperf_process_metrics {
        if let Some(AperfData::TimeSeries(cur_run_aperf_stats_data)) = aperf_stats_data_processor
            .processed_data
            .runs
            .get_mut(&run_name)
        {
            for (aperf_stats_metric_name, all_aperf_process_series) in aperf_process_metrics {
                if !cur_run_aperf_stats_data
                    .sorted_metric_names
                    .contains(&aperf_stats_metric_name)
                {
                    cur_run_aperf_stats_data
                        .sorted_metric_names
                        .insert(0, aperf_stats_metric_name.clone());
                }

                let aperf_stats_metric = cur_run_aperf_stats_data
                    .metrics
                    .entry(aperf_stats_metric_name.clone())
                    .or_insert_with(|| TimeSeriesMetric::new(aperf_stats_metric_name.clone()));

                // To recreate the "total" series as the sum of all series within the metric,
                // first build a sorted map from time_diff to value to account for inconsistent
                // time_diffs across different series within the metric.
                let mut total_series_map: BTreeMap<u64, f64> = match aperf_stats_metric
                    .series
                    .iter()
                    .position(|series| series.is_aggregate)
                {
                    Some(total_series_idx) => {
                        let cur_total_series = aperf_stats_metric.series.remove(total_series_idx);
                        cur_total_series
                            .time_diff
                            .into_iter()
                            .zip(cur_total_series.values.into_iter())
                            .collect()
                    }
                    None => BTreeMap::new(),
                };

                for aperf_process_series in &all_aperf_process_series {
                    aperf_process_series
                        .time_diff
                        .iter()
                        .zip(aperf_process_series.values.iter())
                        .for_each(|(&time_diff, &value)| {
                            // Update value range for possible smaller values in the new series.
                            aperf_stats_metric.value_range.0 =
                                aperf_stats_metric.value_range.0.min(value.floor() as u64);
                            // Update the total series by adding the values in the new series.
                            *total_series_map.entry(time_diff).or_default() += value
                        });
                }

                for mut aperf_process_series in all_aperf_process_series {
                    // Drop pid and only keep the process name as the series name.
                    let series_name = aperf_process_series.series_name;
                    aperf_process_series.series_name = series_name
                        .split_once("_")
                        .map_or(series_name.clone(), |(_pid, process_name)| {
                            process_name.to_string()
                        });
                    aperf_process_series.is_aggregate = false;
                    aperf_stats_metric.series.insert(0, aperf_process_series);
                }
                // Create and save the new total series for the metric and recompute its stats.
                let (total_series_time_diff, total_series_values): (Vec<u64>, Vec<f64>) =
                    total_series_map.into_iter().unzip();
                aperf_stats_metric.stats = Statistics::from_values(&total_series_values);
                aperf_stats_metric.value_range.1 = aperf_stats_metric.stats.max.ceil() as u64;
                aperf_stats_metric.series.insert(
                    0,
                    Series {
                        series_name: "total".to_string(),
                        time_diff: total_series_time_diff,
                        values: total_series_values,
                        is_aggregate: true,
                    },
                );
                aperf_stats_metric.stats_series_idx = 0;
            }
        }
    }
}

pub struct DataProcessor {
    pub data_name: &'static str,
    pub data: ReportData,
    pub processed_data: ProcessedData,
}

impl DataProcessor {
    pub fn new(data_name: &'static str, data: ReportData) -> Self {
        DataProcessor {
            data_name,
            data,
            processed_data: ProcessedData::new(data_name.to_string()),
        }
    }

    pub fn process_raw_data(&mut self, report_params: &ReportParams) -> Result<()> {
        debug!(
            "Processing raw {} data of run {}",
            self.data_name, report_params.run_name
        );

        let raw_data = match self.read_raw_data(report_params) {
            Some(raw_data) => raw_data,
            None => return Ok(()),
        };

        let processed_data = self.data.process_raw_data(report_params, raw_data)?;
        self.processed_data.data_format = processed_data.get_format_name();
        self.processed_data
            .runs
            .insert(report_params.run_name.clone(), processed_data);

        Ok(())
    }

    /// Helper function to attempt to read the raw data file and deserialize them
    /// back to a vector of Data. None if the raw data file does not exist or
    /// cannot be read.
    fn read_raw_data(&self, report_params: &ReportParams) -> Option<Vec<Data>> {
        let mut raw_data = Vec::new();

        // aperf_runlog does not have a raw data file with common format but
        // it should still be processed.
        if matches!(self.data, ReportData::AperfRunlog(_)) {
            return Some(raw_data);
        }

        let (mut raw_data_file, raw_data_file_path) =
            match self.data.get_raw_data_file(&report_params.run_data_dir) {
                Ok(raw_data_file) => raw_data_file,
                Err(e) => {
                    debug!(
                        "Raw {} data unavailable in run {}: {e}",
                        self.data_name, report_params.run_name
                    );
                    return None;
                }
            };

        if let Err(e) = raw_data_file.seek(SeekFrom::Start(0)) {
            error!(
                "Failed to reset seek position to zero for raw data file {}: {e}",
                raw_data_file_path.display()
            );
            return None;
        }

        loop {
            match bincode::deserialize_from::<_, Data>(&raw_data_file) {
                Ok(v) => raw_data.push(v),
                Err(e) => match *e {
                    // EOF
                    bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break
                    }
                    // Ignore invalid enum variant errors, raw data wont be used by self.data
                    bincode::ErrorKind::Custom(ref msg)
                        if msg.contains("expected variant index") =>
                    {
                        break
                    }
                    e => {
                        error!(
                            "Error when deserializing raw {} data for run {} at {}: {}",
                            self.data_name,
                            report_params.run_name,
                            raw_data_file_path.display(),
                            e
                        );
                        break;
                    }
                },
            };
        }

        Some(raw_data)
    }

    /// After the raw data across all runs are processed, run additional data-processing logics
    /// which require all processed data to be available
    pub fn post_process_data(&mut self) {
        match self.processed_data.data_format {
            DataFormat::TimeSeries => post_process_time_series_data(&mut self.processed_data),
            DataFormat::Profile | DataFormat::Graph => {
                post_process_profiling_data(&mut self.processed_data)
            }
            _ => return,
        }
    }
}

/// Run post-processing logics for TimeSeriesData:
/// - Consolidate the value_ranges across different runs for TimeSeriesData, so that metric
///   graphs could have the same y-axis
/// - Consolidate sorted_metric_names across different runs, so that the frontend can know
///   what metric graphs to render as well the order of rendering
/// - If all values of a metric are zero, compress the time-series values to reduce
///   report data size
fn post_process_time_series_data(processed_data: &mut ProcessedData) {
    let mut per_run_sorted_metric_names: Vec<&Vec<String>> = Vec::new();
    let mut per_metric_value_ranges: HashMap<String, Vec<(u64, u64)>> = HashMap::new();

    // Collect every run's sorted metric name and every metric's value range
    for aperf_data in processed_data.runs.values() {
        match aperf_data {
            AperfData::TimeSeries(time_series_data) => {
                per_run_sorted_metric_names.push(&time_series_data.sorted_metric_names);
                for (metric_name, time_series_metric) in &time_series_data.metrics {
                    per_metric_value_ranges
                        .entry(metric_name.clone())
                        .or_insert(Vec::new())
                        .push(time_series_metric.value_range);
                }
            }
            _ => {
                error!("Data post-processing running into unexpected data format (expected TimeSeriesData)");
                return;
            }
        }
    }

    // Compute the cross-run sorted metric names and metric value range
    let sorted_metric_names = match topological_sort(&per_run_sorted_metric_names) {
        Ok(sorted_metric_names) => sorted_metric_names,
        Err(_) => {
            // If there are conflicting orders of the metric name, simply append all metric names
            let mut all_metric_names: Vec<String> = Vec::new();
            for cur_run_sorted_metric_names in per_run_sorted_metric_names {
                for metric_name in cur_run_sorted_metric_names {
                    if !all_metric_names.contains(metric_name) {
                        all_metric_names.push(metric_name.clone());
                    }
                }
            }
            all_metric_names
        }
    };
    let mut per_metric_combined_value_range: HashMap<String, (u64, u64)> = HashMap::new();
    for (metric_name, value_ranges) in per_metric_value_ranges {
        let combined_value_range = combine_value_ranges(value_ranges);
        per_metric_combined_value_range.insert(metric_name, combined_value_range);
    }

    for aperf_data in processed_data.runs.values_mut() {
        let time_series_data = match aperf_data {
            AperfData::TimeSeries(time_series_data) => time_series_data,
            _ => {
                error!("Data post-processing running into unexpected data format (expected TimeSeriesData)");
                return;
            }
        };
        // Update the TimeSeriesData with the cross-run sorted metric names
        time_series_data.sorted_metric_names = sorted_metric_names.clone();
        for (metric_name, time_series_metric) in &mut time_series_data.metrics {
            // Update every metric with cross-run combined value ranges
            if let Some(combined_value_range) = per_metric_combined_value_range.get(metric_name) {
                time_series_metric.value_range = combined_value_range.to_owned();
            }
        }
    }
}

/// Run post-processing for ProfilingData:
/// - if any run produced ProfilingData, ensure that the data_format of the ProcessedData
///   is profile (the legacy GraphData runs won't be shown in the frontend).
fn post_process_profiling_data(processed_data: &mut ProcessedData) {
    let any_profile = processed_data
        .runs
        .values()
        .any(|aperf_data| matches!(aperf_data, AperfData::Profile(_)));
    processed_data.data_format = if any_profile {
        DataFormat::Profile
    } else {
        DataFormat::Graph
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::common::data_formats::{
        GraphData, GraphGroup, ProfilingData, Series, TimeSeriesData, TimeSeriesMetric,
    };

    /// A metric named `name` with one series per `(series_name, values)`. `value_range` is
    /// derived from the min/max across all series values.
    fn make_metric(name: &str, series: &[(&str, &[f64])]) -> TimeSeriesMetric {
        let mut metric = TimeSeriesMetric::new(name.to_string());
        metric.series = series
            .iter()
            .map(|(series_name, values)| Series {
                series_name: series_name.to_string(),
                time_diff: (0..values.len() as u64).collect(),
                values: values.to_vec(),
                is_aggregate: false,
            })
            .collect();
        let all_values = series.iter().flat_map(|(_, values)| values.iter().copied());
        let min = all_values.clone().fold(f64::INFINITY, f64::min);
        let max = all_values.fold(f64::NEG_INFINITY, f64::max);
        metric.value_range = (min as u64, max as u64);
        metric
    }

    /// A single-series metric named `name` with values `[range.0, range.1]` (so `value_range`
    /// is `range`).
    fn make_ranged_metric(name: &str, range: (u64, u64)) -> TimeSeriesMetric {
        make_metric(name, &[("agg", &[range.0 as f64, range.1 as f64])])
    }

    /// A TimeSeriesData holding `metrics`, with `sorted_metric_names` in the given order.
    fn make_time_series_data(metrics: Vec<TimeSeriesMetric>) -> TimeSeriesData {
        let mut ts = TimeSeriesData::default();
        for metric in metrics {
            ts.sorted_metric_names.push(metric.metric_name.clone());
            ts.metrics.insert(metric.metric_name.clone(), metric);
        }
        ts
    }

    #[test]
    fn test_post_process_time_series_data() {
        let mut pd = ProcessedData::new("test".to_string());
        pd.data_format = DataFormat::TimeSeries;

        // run1 has metrics [a, b] with ranges (0,10) and (5,15)
        pd.runs.insert(
            "run1".to_string(),
            AperfData::TimeSeries(make_time_series_data(vec![
                make_ranged_metric("a", (0, 10)),
                make_ranged_metric("b", (5, 15)),
            ])),
        );
        // run2 has metrics [b, c] with ranges (3, 12) and (20, 30)
        pd.runs.insert(
            "run2".to_string(),
            AperfData::TimeSeries(make_time_series_data(vec![
                make_ranged_metric("b", (3, 12)),
                make_ranged_metric("c", (20, 30)),
            ])),
        );

        post_process_time_series_data(&mut pd);

        // Both runs end up with the same consolidated sorted_metric_names and value_ranges.
        for run_name in &["run1", "run2"] {
            let ts = match pd.runs.get(*run_name).unwrap() {
                AperfData::TimeSeries(ts) => ts,
                _ => panic!("expected TimeSeriesData"),
            };
            // Topological sort of [[a,b], [b,c]] yields [a,b,c].
            assert_eq!(ts.sorted_metric_names, vec!["a", "b", "c"]);
            // Per-metric ranges combined: min of starts, max of ends across the runs.
            // a only in run1: (0,10); b in both: (min(5,3), max(15,12)) = (3,15);
            // c only in run2: (20,30).
            if let Some(a) = ts.metrics.get("a") {
                assert_eq!(a.value_range, (0, 10));
            }
            assert_eq!(ts.metrics.get("b").unwrap().value_range, (3, 15));
            if let Some(c) = ts.metrics.get("c") {
                assert_eq!(c.value_range, (20, 30));
            }
        }
    }

    #[test]
    fn test_post_process_profiling_data() {
        // Branch 1: mixed Profile + Graph -> Profile wins (regardless of insertion order).
        let mut pd = ProcessedData::new("java_profile".to_string());
        pd.data_format = DataFormat::Graph;
        pd.runs
            .insert("run1".to_string(), AperfData::Graph(GraphData::default()));
        pd.runs.insert(
            "run2".to_string(),
            AperfData::Profile(ProfilingData::default()),
        );
        post_process_profiling_data(&mut pd);
        assert!(matches!(pd.data_format, DataFormat::Profile));

        // Branch 2: all runs are Graph -> ProcessedData stays Graph.
        let mut pd = ProcessedData::new("hotline".to_string());
        pd.data_format = DataFormat::Profile;
        let mut graph_data = GraphData::default();
        graph_data.graph_groups.push(GraphGroup::new("table_id_1"));
        pd.runs
            .insert("run1".to_string(), AperfData::Graph(graph_data));
        pd.runs
            .insert("run2".to_string(), AperfData::Graph(GraphData::default()));
        post_process_profiling_data(&mut pd);
        assert!(matches!(pd.data_format, DataFormat::Graph));
    }

    const APERF_PID: u32 = 4242;
    const SUB_PROCESS_PID: u32 = 4243;

    /// Build the `processes` + `aperf_stats` data processors for run "run1". The processes data
    /// has one "user_space_time" metric with a series for each name in `process_series_names`;
    /// aperf_stats starts with its own "aperf" metric.
    fn make_aperf_metric_processors(
        process_series_names: &[&str],
    ) -> HashMap<String, DataProcessor> {
        let processes_series: Vec<(&str, &[f64])> = process_series_names
            .iter()
            .map(|name| (*name, [0.0, 1.0, 2.0].as_slice()))
            .collect();

        [
            (
                get_data_name_from_type::<Processes>(),
                ReportData::Processes(Processes::new()),
                make_time_series_data(vec![make_metric("user_space_time", &processes_series)]),
            ),
            (
                get_data_name_from_type::<AperfStats>(),
                ReportData::AperfStats(AperfStats::new()),
                make_time_series_data(vec![make_metric("aperf", &[])]),
            ),
        ]
        .into_iter()
        .map(|(name, data, ts)| {
            let mut dp = DataProcessor::new(name, data);
            dp.processed_data
                .runs
                .insert("run1".to_string(), AperfData::TimeSeries(ts));
            (name.to_string(), dp)
        })
        .collect()
    }

    fn params_with_pids(aperf_process_pids: Vec<u32>) -> HashMap<String, ReportParams> {
        let mut rp = ReportParams::new();
        rp.run_name = "run1".to_string();
        rp.aperf_process_pids = aperf_process_pids;
        HashMap::from([("run1".to_string(), rp)])
    }

    /// The aperf_stats TimeSeriesData for "run1" after the copy.
    fn aperf_stats_result(data_processors: &HashMap<String, DataProcessor>) -> TimeSeriesData {
        match &data_processors[get_data_name_from_type::<AperfStats>()]
            .processed_data
            .runs["run1"]
        {
            AperfData::TimeSeries(ts) => ts.clone(),
            _ => panic!("expected TimeSeriesData"),
        }
    }

    #[test]
    fn test_copy_aperf_process_metrics_copies_series() {
        // Two APerf processes (the collector + a subprocess) and one unrelated process.
        let aperf_series = format!("{APERF_PID}_aperf");
        let subprocess_series = format!("{SUB_PROCESS_PID}_perf");
        let mut data_processors =
            make_aperf_metric_processors(&[&aperf_series, &subprocess_series, "999_other"]);

        copy_aperf_process_metrics_to_aperf_stats(
            &mut data_processors,
            &params_with_pids(vec![APERF_PID, SUB_PROCESS_PID]),
        );

        let ts = aperf_stats_result(&data_processors);
        let copied = ts
            .metrics
            .get("process_user_space_time")
            .expect("aperf process metric should be copied into aperf_stats");
        // The two copied APerf process series, plus the recreated "total" aggregate series
        // (the unrelated process series is not copied).
        assert_eq!(copied.series.len(), 3);
        let total = &copied.series[0];
        assert_eq!(total.series_name, "total");
        assert!(total.is_aggregate);
        // The total is the sum of the two copied series ([0,1,2] each) at every time_diff.
        assert_eq!(total.time_diff, vec![0, 1, 2]);
        assert_eq!(total.values, vec![0.0, 2.0, 4.0]);
        for series in &copied.series[1..] {
            assert!(!series.is_aggregate);
            // The PID prefix is dropped from copied series names, keeping the process name.
            assert!(["aperf", "perf"]
                .iter()
                .any(|name| *name == series.series_name));
            assert_eq!(series.values, vec![0.0, 1.0, 2.0]);
        }
        // The metric stats are recomputed from the total series.
        assert_eq!(copied.stats.max, 4.0);
        assert_eq!(copied.stats.avg, 2.0);
        // The value range covers the copied values and the total's max.
        assert_eq!(copied.value_range, (0, 4));
        // Inserted at the front, ahead of the pre-existing "aperf" metric.
        assert_eq!(
            ts.sorted_metric_names,
            vec!["process_user_space_time", "aperf"]
        );
    }

    #[test]
    fn test_copy_aperf_process_metrics_skips_when_pid_absent() {
        // Runs recorded by older aperf versions have no PID; nothing should be copied.
        let aperf_series = format!("{APERF_PID}_aperf");
        let mut data_processors = make_aperf_metric_processors(&[&aperf_series, "999_other"]);

        copy_aperf_process_metrics_to_aperf_stats(
            &mut data_processors,
            &params_with_pids(Vec::new()),
        );

        assert_eq!(
            aperf_stats_result(&data_processors).sorted_metric_names,
            vec!["aperf"]
        );
    }

    #[test]
    fn test_copy_aperf_process_metrics_skips_when_aperf_process_missing() {
        // PID is known, but the processes data has no series for it (e.g. aperf filtered out).
        let mut data_processors = make_aperf_metric_processors(&["999_other"]);

        copy_aperf_process_metrics_to_aperf_stats(
            &mut data_processors,
            &params_with_pids(vec![APERF_PID]),
        );

        assert_eq!(
            aperf_stats_result(&data_processors).sorted_metric_names,
            vec!["aperf"]
        );
    }
}
