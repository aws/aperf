use crate::{data::Data, data::ProcessedData, get_file, PDError};
use anyhow::Result;
use log::debug;
use rustix::fd::AsRawFd;
use serde::{Deserialize, Serialize};
use std::fs;
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
    fn new() -> Self {
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
    pub file_name: String,
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
        file_name: String,
        js_file_name: String,
        js: String,
        api_name: String,
    ) -> Self {
        DataVisualizer {
            data,
            file_handle: None,
            run_values: HashMap::new(),
            file_name,
            js_file_name,
            js,
            api_name,
            has_custom_raw_data_parser: false,
            data_available: HashMap::new(),
            report_params: ReportParams::new(),
        }
    }

    pub fn has_custom_raw_data_parser(&mut self) {
        self.has_custom_raw_data_parser = true;
    }

    pub fn init_visualizer(
        &mut self,
        dir: String,
        name: String,
        tmp_dir: &Path,
        fin_dir: &Path,
    ) -> Result<()> {
        let file = get_file(dir.clone(), self.file_name.clone())?;
        let full_path = Path::new("/proc/self/fd").join(file.as_raw_fd().to_string());
        self.report_params.data_dir = PathBuf::from(dir.clone());
        self.report_params.tmp_dir = tmp_dir.to_path_buf();
        self.report_params.report_dir = fin_dir.to_path_buf();
        self.report_params.run_name = name.clone();
        self.report_params.data_file_path = fs::read_link(full_path).unwrap();
        self.file_handle = Some(file);
        self.run_values.insert(name.clone(), Vec::new());
        self.data_available.insert(name, true);
        Ok(())
    }

    pub fn data_not_available(&mut self, name: String) -> Result<()> {
        self.data_available.insert(name, false);
        Ok(())
    }

    pub fn process_raw_data(&mut self, name: String) -> Result<()> {
        if !self.data_available.get(&name).unwrap() {
            debug!("Raw data unavailable for: {}", self.api_name);
            return Ok(());
        }
        debug!("Processing raw data for: {}", self.api_name);
        if self.has_custom_raw_data_parser {
            self.run_values.insert(
                name.clone(),
                self.data
                    .custom_raw_data_parser(self.report_params.clone())?,
            );
            return Ok(());
        }
        let mut raw_data = Vec::new();
        loop {
            match bincode::deserialize_from::<_, Data>(self.file_handle.as_ref().unwrap()) {
                Ok(v) => raw_data.push(v),
                Err(e) => match *e {
                    // EOF
                    bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break
                    }
                    e => panic!("Error when Deserializing {} data {}", self.api_name, e),
                },
            };
        }
        let mut data = Vec::new();
        for value in raw_data {
            let processed_data = self.data.process_raw_data(value)?;
            data.push(processed_data);
        }
        self.run_values.insert(name.clone(), data);
        Ok(())
    }

    pub fn get_data(&mut self, name: String, query: String) -> Result<String> {
        if !self.data_available.get(&name).unwrap() {
            debug!("No data available for: {} query: {}", self.api_name, query);
            return Ok("No data collected".to_string());
        }
        /* Get run name from Query */
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query)?;
        let (_, run) = param[0].clone();

        let values = self
            .run_values
            .get_mut(&run)
            .ok_or(PDError::VisualizerRunValueGetError(run.to_string()))?;
        if values.is_empty() {
            return Ok("No data collected".to_string());
        }
        self.data.get_data(values.clone(), query)
    }

    pub fn get_calls(&mut self) -> Result<Vec<String>> {
        self.data.get_calls()
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
    fn get_calls(&mut self) -> Result<Vec<String>> {
        unimplemented!();
    }
    fn get_data(&mut self, _values: Vec<ProcessedData>, _query: String) -> Result<String> {
        unimplemented!();
    }
    fn process_raw_data(&mut self, _buffer: Data) -> Result<ProcessedData> {
        unimplemented!();
    }
    fn custom_raw_data_parser(&mut self, _params: ReportParams) -> Result<Vec<ProcessedData>> {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::DataVisualizer;
    use crate::data::cpu_utilization::{CpuData, CpuUtilization};
    use crate::data::{ProcessedData, TimeEnum};
    use std::path::PathBuf;

    #[test]
    fn test_unpack_data() {
        let mut dv = DataVisualizer::new(
            ProcessedData::CpuUtilization(CpuUtilization::new()),
            "cpu_utilization".to_string(),
            String::new(),
            String::new(),
            "cpu_utilization".to_string(),
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
