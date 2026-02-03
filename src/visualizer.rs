use crate::data::data_formats::{AperfData, DataFormat, ProcessedData};
use crate::utils::{combine_value_ranges, topological_sort};
use crate::{data::Data, data::ReportData, get_file};
use anyhow::Result;
use log::{debug, error};
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
}

impl ReportParams {
    pub fn new() -> Self {
        ReportParams {
            data_dir: PathBuf::new(),
            tmp_dir: PathBuf::new(),
            report_dir: PathBuf::new(),
            run_name: String::new(),
            data_file_path: PathBuf::new(),
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
        run_data_dir: PathBuf,
        run_name: String,
        tmp_dir: &Path,
        report_dir: &Path,
    ) -> Result<()> {
        let (file_path, file) =
            get_file(&run_data_dir, self.data_name.to_string()).or_else(|e| {
                // Backward compatibility: if file is not found using the data's name,
                // see if files with compatible names exist
                for compatible_name in self.data.compatible_filenames() {
                    match get_file(&run_data_dir, String::from(compatible_name)) {
                        Ok(compatible_file) => {
                            debug!(
                                "Data file {} not found, use compatible file name {}",
                                self.data_name, compatible_name
                            );
                            return Ok(compatible_file);
                        }
                        Err(_) => {}
                    }
                }
                self.data_available.insert(run_name.clone(), false);
                Err(e)
            })?;
        self.report_params.data_dir = run_data_dir;
        self.report_params.tmp_dir = tmp_dir.to_path_buf();
        self.report_params.report_dir = report_dir.to_path_buf();
        self.report_params.run_name = run_name.clone();
        self.report_params.data_file_path = file_path;
        self.file_handle = Some(file);
        self.data_available.insert(run_name, true);
        Ok(())
    }

    pub fn process_raw_data(&mut self, run_name: String) -> Result<()> {
        if !self.data_available.get(&run_name).unwrap() {
            debug!("Raw data unavailable for: {}", self.data_name);
            return Ok(());
        }
        debug!("Processing raw {} data in {}", self.data_name, run_name);

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
        self.processed_data.runs.insert(run_name, processed_data);

        Ok(())
    }

    /// After the raw data across all runs are processed, run additional data-processing logics
    /// which require all processed data to be available
    pub fn post_process_data(&mut self) {
        match self.processed_data.data_format {
            DataFormat::TimeSeries => post_process_time_series_data(&mut self.processed_data),
            _ => return,
        }
    }
}

/// Run post-processing logics for TimeSeriesData:
/// - Consolidate the value_ranges across different runs for TimeSeriesData, so that metric
///   graphs could have the same y-axis
/// - Consolidate sorted_metric_names across different runs, so that the frontend can know
///   what metric graphs to render as well the order of rendering
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

    // Update the TimeSeriesData with the cross-run sorted metric names and value ranges
    for aperf_data in processed_data.runs.values_mut() {
        let time_series_data = match aperf_data {
            AperfData::TimeSeries(time_series_data) => time_series_data,
            _ => {
                error!("Data post-processing running into unexpected data format (expected TimeSeriesData)");
                return;
            }
        };
        time_series_data.sorted_metric_names = sorted_metric_names.clone();
        for (metric_name, time_series_metric) in &mut time_series_data.metrics {
            if let Some(combined_value_range) = per_metric_combined_value_range.get(metric_name) {
                time_series_metric.value_range = combined_value_range.to_owned();
            }
        }
    }
}
