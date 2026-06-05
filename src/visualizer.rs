use crate::data::common::data_formats::{AperfData, DataFormat, ProcessedData};
use crate::data::common::utils::{combine_value_ranges, find_file, topological_sort};
use crate::data::TimeEnum;
use crate::{data::Data, data::ReportData};
use anyhow::Result;
use log::{debug, error};
use std::fs;
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs::File};

#[derive(Clone, Debug)]
pub struct ReportParams {
    pub data_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub report_dir: PathBuf,
    pub run_name: String,
    pub data_file_path: PathBuf,
    /// Wall-clock start of `collect_data_serial`. Used to anchor time_diff=0 in
    /// time-series data to the actual collection start rather than the first sample time.
    pub collection_start: Option<TimeEnum>,
}

impl ReportParams {
    pub fn new() -> Self {
        ReportParams {
            data_dir: PathBuf::new(),
            tmp_dir: PathBuf::new(),
            report_dir: PathBuf::new(),
            run_name: String::new(),
            data_file_path: PathBuf::new(),
            collection_start: None,
        }
    }
}

pub struct DataVisualizer {
    pub data_name: &'static str,
    pub data: ReportData,
    pub file_handle: Option<File>,
    pub processed_data: ProcessedData,
    pub data_available: HashMap<String, bool>,
    pub report_params: ReportParams,
}

impl DataVisualizer {
    pub fn new(data_name: &'static str, data: ReportData) -> Self {
        DataVisualizer {
            data_name,
            data,
            file_handle: None,
            processed_data: ProcessedData::new(data_name.to_string()),
            data_available: HashMap::new(),
            report_params: ReportParams::new(),
        }
    }

    pub fn init_visualizer(
        &mut self,
        run_name: String,
        run_data_dir: PathBuf,
        tmp_dir: &Path,
        report_dir: &Path,
        collection_start: Option<TimeEnum>,
    ) -> Result<()> {
        self.report_params.run_name = run_name.clone();
        self.report_params.collection_start = collection_start;
        let file_path = find_file(
            &run_data_dir,
            &format!("^{}", regex::escape(self.data_name)),
            None,
        )
        .map(|filename| run_data_dir.join(filename))
        .or_else(|e| {
            // Backward compatibility: if file is not found using the data's name,
            // see if files with compatible names exist
            for compatible_name in self.data.compatible_filenames() {
                if let Ok(filename) = find_file(
                    &run_data_dir,
                    &format!("^{}", regex::escape(compatible_name)),
                    None,
                ) {
                    debug!(
                        "Data file {} not found, use compatible file name {}",
                        self.data_name, compatible_name
                    );
                    return Ok(run_data_dir.join(filename));
                }
            }
            self.data_available.insert(run_name.clone(), false);
            Err(e)
        })?;
        let file = fs::OpenOptions::new().read(true).open(&file_path)?;
        self.report_params.data_dir = run_data_dir;
        self.report_params.tmp_dir = tmp_dir.to_path_buf();
        self.report_params.report_dir = report_dir.to_path_buf();
        self.report_params.data_file_path = file_path;
        self.file_handle = Some(file);
        self.data_available.insert(run_name, true);
        Ok(())
    }

    pub fn process_raw_data(&mut self) -> Result<()> {
        debug!(
            "Processing raw {} data in {}",
            self.data_name, self.report_params.run_name
        );

        if !self
            .data_available
            .get(&self.report_params.run_name)
            .unwrap()
        {
            debug!("Raw data unavailable for: {}", self.data_name);
            return Ok(());
        }

        self.file_handle
            .as_ref()
            .unwrap()
            .seek(SeekFrom::Start(0))?;

        let mut raw_data = Vec::new();
        loop {
            match bincode::deserialize_from::<_, Data>(self.file_handle.as_ref().unwrap()) {
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
                            "Error when Deserializing {} data at {} : {}",
                            self.data_name,
                            self.report_params.data_file_path.display().to_string(),
                            e
                        );
                        break;
                    }
                },
            };
        }

        let processed_data = self
            .data
            .process_raw_data(self.report_params.clone(), raw_data)?;
        self.processed_data.data_format = processed_data.get_format_name();
        self.processed_data
            .runs
            .insert(self.report_params.run_name.clone(), processed_data);

        Ok(())
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

    fn make_metric(name: &str, value_range: (u64, u64)) -> TimeSeriesMetric {
        let mut metric = TimeSeriesMetric::new(name.to_string());
        metric.value_range = value_range;
        metric.series = vec![Series {
            series_name: "agg".to_string(),
            time_diff: vec![0, 10],
            values: vec![value_range.0 as f64, value_range.1 as f64],
            is_aggregate: true,
        }];
        metric
    }

    fn make_time_series_data(
        sorted_metric_names: Vec<&str>,
        metrics: Vec<(&str, (u64, u64))>,
    ) -> TimeSeriesData {
        let mut ts = TimeSeriesData::default();
        ts.sorted_metric_names = sorted_metric_names.into_iter().map(String::from).collect();
        for (name, value_range) in metrics {
            ts.metrics
                .insert(name.to_string(), make_metric(name, value_range));
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
            AperfData::TimeSeries(make_time_series_data(
                vec!["a", "b"],
                vec![("a", (0, 10)), ("b", (5, 15))],
            )),
        );
        // run2 has metrics [b, c] with ranges (3, 12) and (20, 30)
        pd.runs.insert(
            "run2".to_string(),
            AperfData::TimeSeries(make_time_series_data(
                vec!["b", "c"],
                vec![("b", (3, 12)), ("c", (20, 30))],
            )),
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
}
