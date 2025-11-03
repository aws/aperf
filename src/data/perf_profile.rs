use crate::data::data_formats::{AperfData, TextData};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
use crate::utils::DataMetrics;
use crate::visualizer::{GetData, ReportParams};
use crate::PDError;
use anyhow::Result;
use log::{error, trace};
use nix::{sys::signal, unistd::Pid};
use serde::{Deserialize, Serialize};
use std::fs;
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    sync::Mutex,
};

pub const PERF_TOP_FUNCTIONS_FILE_NAME: &str = "top_functions";

lazy_static! {
    pub static ref PERF_CHILD: Mutex<Option<Child>> = Mutex::new(None);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfProfileRaw {
    pub data: String,
}

impl PerfProfileRaw {
    pub fn new() -> Self {
        PerfProfileRaw {
            data: String::new(),
        }
    }
}

impl CollectData for PerfProfileRaw {
    fn prepare_data_collector(&mut self, params: &CollectorParams) -> Result<()> {
        match Command::new("perf")
            .stdout(Stdio::null())
            .args([
                "record",
                "-a",
                "-q",
                "-g",
                "-k",
                "1",
                "-F",
                &params.perf_frequency.to_string(),
                "-e",
                "cpu-clock:pppH",
                "-o",
                &params.data_file_path.display().to_string(),
                "--",
                "sleep",
                &params.collection_time.to_string(),
            ])
            .spawn()
        {
            Err(e) => Err(PDError::DependencyError(format!(
                "Skipping Perf profile collection due to: {}",
                e
            ))
            .into()),
            Ok(child) => {
                trace!("Recording Perf profiling data.");
                *PERF_CHILD.lock().unwrap() = Some(child);
                Ok(())
            }
        }
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        Ok(())
    }

    fn finish_data_collection(&mut self, params: &CollectorParams) -> Result<()> {
        let mut child = PERF_CHILD.lock().unwrap();
        match child.as_ref() {
            None => return Ok(()),
            Some(_) => {}
        }

        signal::kill(
            Pid::from_raw(child.as_mut().unwrap().id() as i32),
            params.signal,
        )?;

        trace!("Waiting for perf profile collection to complete...");
        match child.as_mut().unwrap().wait() {
            Err(e) => {
                error!("'perf' did not exit successfully: {}", e);
                return Ok(());
            }
            Ok(_) => trace!("'perf record' executed successfully."),
        }
        let mut top_functions_file =
            fs::File::create(params.data_dir.join(PERF_TOP_FUNCTIONS_FILE_NAME))?;

        let out = Command::new("perf")
            .args([
                "report",
                "--stdio",
                "--percent-limit",
                "1",
                "-i",
                &params.data_file_path.display().to_string(),
            ])
            .output();

        match out {
            Err(e) => {
                let out = format!("Skipped processing profiling data due to : {}", e);
                error!("{}", out);
                write!(top_functions_file, "{}", out)?;
            }
            Ok(v) => {
                let mut top_functions = "No data collected";
                if !v.stdout.is_empty() {
                    top_functions = std::str::from_utf8(&v.stdout)?;
                }
                write!(top_functions_file, "{}", top_functions)?;
            }
        }
        Ok(())
    }

    fn is_profile() -> bool {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfProfile {
    pub data: Vec<String>,
}

impl PerfProfile {
    pub fn new() -> Self {
        PerfProfile { data: Vec::new() }
    }
}

impl GetData for PerfProfile {
    fn custom_raw_data_parser(&mut self, params: ReportParams) -> Result<Vec<ProcessedData>> {
        let mut profile = PerfProfile::new();
        let file_loc = params.data_dir.join(PERF_TOP_FUNCTIONS_FILE_NAME);
        if file_loc.exists() {
            profile.data = fs::read_to_string(&file_loc)?
                .split('\n')
                .map(|x| x.to_string())
                .collect();
        }

        let processed_data = vec![ProcessedData::PerfProfile(profile)];
        Ok(processed_data)
    }

    fn process_raw_data_new(
        &mut self,
        params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut text_data = TextData::default();
        let file_loc = params.data_dir.join(PERF_TOP_FUNCTIONS_FILE_NAME);
        if file_loc.exists() {
            text_data.lines = fs::read_to_string(&file_loc)?
                .split('\n')
                .map(|x| x.to_string())
                .collect();
        }
        Ok(AperfData::Text(text_data))
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(
        &mut self,
        buffer: Vec<ProcessedData>,
        _query: String,
        _metrics: &mut DataMetrics,
    ) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::PerfProfile(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        Ok(serde_json::to_string(&values)?)
    }

    fn has_custom_raw_data_parser() -> bool {
        true
    }
}
