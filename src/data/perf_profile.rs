use crate::data::data_formats::{AperfData, TextData};
use crate::data::{CollectData, CollectorParams, Data, ProcessData};
use crate::visualizer::ReportParams;
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
pub struct PerfProfile;

impl PerfProfile {
    pub fn new() -> Self {
        PerfProfile
    }
}

impl ProcessData for PerfProfile {
    fn process_raw_data(
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
}
