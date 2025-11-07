use crate::data::data_formats::{AperfData, DataFormat, ReportData};
use crate::utils::{combine_value_ranges, topological_sort, DataMetrics};
use crate::{data::Data, data::ProcessedData, get_file, PDError};
use anyhow::Result;
use log::{debug, error};
use rustix::fd::AsRawFd;
use serde::{Deserialize, Serialize};
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
    pub data: ProcessedData,
    pub file_handle: Option<File>,
    pub run_values: HashMap<String, Vec<ProcessedData>>,
    pub report_data: ReportData,
    pub js_file_name: String,
    pub js: String,
    pub api_name: String,
    pub has_custom_raw_data_parser: bool,
    pub data_available: HashMap<String, bool>,
    pub report_params: ReportParams,
}

impl DataVisualizer {
    pub fn new(
        data: ProcessedData,
        api_name: String,
        js_file_name: String,
        js: String,
        has_custom_raw_data_parser: bool,
    ) -> Self {
        DataVisualizer {
            data,
            file_handle: None,
            run_values: HashMap::new(),
            report_data: ReportData::new(api_name.clone()),
            js_file_name,
            js,
            api_name,
            has_custom_raw_data_parser,
            data_available: HashMap::new(),
            report_params: ReportParams::new(),
        }
    }

    pub fn init_visualizer(
        &mut self,
        dir: String,
        name: String,
        tmp_dir: &Path,
        report_dir: &Path,
    ) -> Result<()> {
        let file = get_file(dir.clone(), self.api_name.clone()).or_else(|e| {
            // Backward compatibility: if file is not found using the data's name,
            // see if files with compatible names exist
            for compatible_name in self.data.compatible_filenames() {
                match get_file(dir.clone(), String::from(compatible_name)) {
                    Ok(compatible_file) => {
                        debug!(
                            "Data file {} not found, use compatible file name {}",
                            self.api_name, compatible_name
                        );
                        return Ok(compatible_file);
                    }
                    Err(_) => {}
                }
            }
            self.data_available.insert(name.clone(), false);
            Err(e)
        })?;
        let full_path = Path::new("/proc/self/fd").join(file.as_raw_fd().to_string());
        self.report_params.data_dir = PathBuf::from(dir.clone());
        self.report_params.tmp_dir = tmp_dir.to_path_buf();
        self.report_params.report_dir = report_dir.to_path_buf();
        self.report_params.run_name = name.clone();
        self.report_params.data_file_path = fs::read_link(full_path).unwrap();
        self.file_handle = Some(file);
        self.run_values.insert(name.clone(), Vec::new());
        self.data_available.insert(name, true);
        Ok(())
    }

    pub fn process_raw_data(&mut self, name: String) -> Result<()> {
        if !self.data_available.get(&name).unwrap() {
            debug!("Raw data unavailable for: {}", self.api_name);
            return Ok(());
        }
        debug!("Processing raw data for: {}", self.api_name);
        let mut data: Vec<ProcessedData>;

        if self.has_custom_raw_data_parser {
            data = self
                .data
                .custom_raw_data_parser(self.report_params.clone())?;
        } else {
            data = Vec::new();
            let mut raw_data = Vec::new();
            loop {
                match bincode::deserialize_from::<_, Data>(self.file_handle.as_ref().unwrap()) {
                    Ok(v) => raw_data.push(v),
                    Err(e) => match *e {
                        // EOF
                        bincode::ErrorKind::Io(e)
                            if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                        {
                            break
                        }
                        e => panic!(
                            "Error when Deserializing {} data at {} : {}",
                            self.api_name,
                            self.report_params.data_file_path.display().to_string(),
                            e
                        ),
                    },
                };
            }
            for value in raw_data {
                let processed_data = self.data.process_raw_data(value)?;
                data.push(processed_data);
            }
        }

        if data.is_empty() {
            debug!(
                "No processed data available for {} at run {}. Marking the run as unavailable.",
                self.api_name, name
            );
            self.data_available.insert(name.clone(), false);
        }

        self.run_values.insert(name.clone(), data);
        Ok(())
    }

    pub fn process_raw_data_new(&mut self, run_name: String) -> Result<()> {
        if !self.data_available.get(&run_name).unwrap() {
            debug!("Raw data unavailable for: {}", self.api_name);
            return Ok(());
        }
        debug!("Processing raw {} data in {}", self.api_name, run_name);

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
                    e => panic!(
                        "Error when Deserializing {} data at {} : {}",
                        self.api_name,
                        self.report_params.data_file_path.display().to_string(),
                        e
                    ),
                },
            };
        }

        let processed_data = self
            .data
            .process_raw_data_new(self.report_params.clone(), raw_data)?;
        self.report_data.data_format = processed_data.get_format_name();
        self.report_data.runs.insert(run_name, processed_data);

        Ok(())
    }

    /// After the raw data across all runs are processed, run additional data-processing logics
    /// which require all processed data to be available
    pub fn post_process_data(&mut self) {
        match self.report_data.data_format {
            DataFormat::TimeSeries => post_process_time_series_data(&mut self.report_data),
            _ => return,
        }
    }

    pub fn get_data(
        &mut self,
        name: String,
        query: String,
        metrics: &mut DataMetrics,
    ) -> Result<String> {
        if !self.data_available.get(&name).unwrap() {
            return Err(PDError::DataUnavailableError(
                self.api_name.clone(),
                name.clone(),
            ))?;
        }

        /* Get run name from Query */
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query)?;
        let (_, run) = param[0].clone();

        let values = self
            .run_values
            .get_mut(&run)
            .ok_or(PDError::VisualizerRunValueGetError(run.to_string()))?;

        self.data.get_data(values.clone(), query, metrics)
    }

    pub fn get_calls(&mut self) -> Result<Vec<String>> {
        self.data.get_calls()
    }
}

/// Run post-processing logics for TimeSeriesData:
/// - Consolidate the value_ranges across different runs for TimeSeriesData, so that metric
///   graphs could have the same y-axis
/// - Consolidate sorted_metric_names across different runs, so that the frontend can know
///   what metric graphs to render as well the order of rendering
fn post_process_time_series_data(report_data: &mut ReportData) {
    let mut per_run_sorted_metric_names: Vec<&Vec<String>> = Vec::new();
    let mut per_metric_value_ranges: HashMap<String, Vec<(u64, u64)>> = HashMap::new();

    // Collect every run's sorted metric name and every metric's value range
    for aperf_data in report_data.runs.values() {
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
    for aperf_data in report_data.runs.values_mut() {
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

pub enum GraphLimitType {
    UInt64(u64),
    F64(f64),
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct GraphLimits {
    pub low: u64,
    pub high: u64,
    pub init_done: bool,
}

impl GraphLimits {
    pub fn new() -> Self {
        GraphLimits {
            low: 0,
            high: 0,
            init_done: false,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct GraphMetadata {
    pub limits: GraphLimits,
}

impl GraphMetadata {
    pub fn new() -> Self {
        GraphMetadata {
            limits: GraphLimits::new(),
        }
    }

    fn update_limit_u64(&mut self, value: u64) {
        if !self.limits.init_done {
            self.limits.low = value;
            self.limits.init_done = true;
        }
        if value < self.limits.low {
            self.limits.low = value;
        }
        if value > self.limits.high {
            self.limits.high = value;
        }
    }

    fn update_limit_f64(&mut self, value: f64) {
        let value_floor = value.floor() as u64;
        let value_ceil = value.ceil() as u64;
        if !self.limits.init_done {
            self.limits.low = value_floor;
            self.limits.init_done = true;
        }
        // Set low
        if value_floor < self.limits.low {
            self.limits.low = value_floor;
        }
        if value_ceil < self.limits.low {
            self.limits.low = value_ceil;
        }
        // Set high
        if value_floor > self.limits.high {
            self.limits.high = value_floor;
        }
        if value_ceil > self.limits.high {
            self.limits.high = value_ceil;
        }
    }

    pub fn update_limits(&mut self, value: GraphLimitType) {
        match value {
            GraphLimitType::UInt64(v) => self.update_limit_u64(v),
            GraphLimitType::F64(v) => self.update_limit_f64(v),
        }
    }
}

pub trait GetData {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec![]
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        unimplemented!();
    }

    fn get_data(
        &mut self,
        _values: Vec<ProcessedData>,
        _query: String,
        _metrics: &mut DataMetrics,
    ) -> Result<String> {
        unimplemented!();
    }

    fn process_raw_data(&mut self, _buffer: Data) -> Result<ProcessedData> {
        unimplemented!();
    }

    fn process_raw_data_new(
        &mut self,
        _params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        Err(PDError::VisualizerUnsupportedAPI.into()) // TODO: remove when all are implemented
    }

    fn custom_raw_data_parser(&mut self, _params: ReportParams) -> Result<Vec<ProcessedData>> {
        unimplemented!();
    }

    fn has_custom_raw_data_parser() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::DataVisualizer;
    use crate::data::cpu_utilization::{CpuData, CpuUtilization};
    use crate::data::{ProcessedData, TimeEnum};
    use crate::utils::DataMetrics;
    use std::path::PathBuf;

    #[test]
    fn test_unpack_data() {
        let mut dv = DataVisualizer::new(
            ProcessedData::CpuUtilization(CpuUtilization::new()),
            "cpu_utilization".to_string(),
            String::new(),
            String::new(),
            false,
        );
        dv.init_visualizer(
            "tests/test-data/aperf_2023-07-26_18_37_43/".to_string(),
            "test".to_string(),
            &PathBuf::new(),
            &PathBuf::new(),
        )
        .unwrap();
        dv.process_raw_data("test".to_string()).unwrap();
        let ret = dv
            .get_data(
                "test".to_string(),
                "run=test&get=values&key=aggregate".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<CpuData> = serde_json::from_str(&ret).unwrap();
        assert!(values[0].cpu == -1);
        match values[0].time {
            TimeEnum::TimeDiff(value) => assert!(value == 0),
            _ => unreachable!(),
        }
        match values[1].time {
            TimeEnum::TimeDiff(value) => assert!(value == 1),
            _ => unreachable!(),
        }
    }
}
