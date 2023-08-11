extern crate ctor;

use anyhow::Result;
use crate::data::{CollectData, Data, DataType, ProcessedData, CollectorParams};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData, ReportParams};
use ctor::ctor;
use log::{trace, error};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::io::ErrorKind;

pub static PERF_PROFILE_FILE_NAME: &str = "perf_profile";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfProfileRaw {
    pub data: String,
}

impl PerfProfileRaw {
    fn new() -> Self {
        PerfProfileRaw {
            data: String::new(),
        }
    }
}

impl CollectData for PerfProfileRaw {
    fn prepare_data_collector(&mut self, params: CollectorParams) -> Result<()> {
        match Command::new("perf")
            .args(["record", "-a", "-q", "-g", "-k", "1", "-F", "99", "-e", "cpu-clock:pppH", "-o", &params.data_file_path, "--", "sleep", &params.collection_time.to_string()])
            .spawn() {
                Err(e) => {
                    if e.kind() == ErrorKind::NotFound {
                        error!("'perf' command not found.");
                    } else {
                        error!("Unknown error: {}", e);
                    }
                    error!("Skipping Perf profile collection.");
                },
                Ok(_) => trace!("Recording Perf profiling data."),
            }
        Ok(())
    }

    fn collect_data(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfProfile {
    pub data: Vec<String>,
}

impl PerfProfile {
    fn new() -> Self {
        PerfProfile {
            data: Vec::new(),
        }
    }
}

impl GetData for PerfProfile {
    fn custom_raw_data_parser(&mut self, params: ReportParams) -> Result<Vec<ProcessedData>> {
        let file_name = params.data_file_path.to_str().unwrap();
        let mut profile = PerfProfile::new();

        let out = Command::new("perf")
            .args(["report", "--stdio", "-g", "none", "--percent-limit", "1", "-i", file_name]).output()?.stdout;

        if out.len() == 0 {
            profile.data = vec!["No data collected".to_string()];
        } else {
            profile.data = std::str::from_utf8(&out)?
                .to_string()
                .split('\n')
                .map(|x| x.to_string())
                .collect();
        }


        let processed_data = vec![ProcessedData::PerfProfile(profile)];
        Ok(processed_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        let mut end_values = Vec::new();
        end_values.push("values".to_string());
        Ok(end_values)
    }

    fn get_data(&mut self, buffer: Vec<ProcessedData>, _query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::PerfProfile(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        Ok(serde_json::to_string(&values)?)
    }
}

#[ctor]
fn init_perf_profile() {
    let perf_profile_raw = PerfProfileRaw::new();
    let file_name = PERF_PROFILE_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::PerfProfileRaw(perf_profile_raw.clone()),
        file_name.clone(),
        false
    );
    let perf_profile = PerfProfile::new();
    let js_file_name = file_name.clone() + &".js".to_string();
    let mut dv = DataVisualizer::new(
        ProcessedData::PerfProfile(perf_profile.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/perf_profile.js")).to_string(),
        file_name.clone(),
    );
    dv.has_custom_raw_data_parser();

    PERFORMANCE_DATA
        .lock()
        .unwrap()
        .add_datatype(file_name.clone(), dt);

    VISUALIZATION_DATA
        .lock()
        .unwrap()
        .add_visualizer(file_name.clone(), dv);
}
