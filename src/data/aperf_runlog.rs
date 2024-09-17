extern crate ctor;

use crate::visualizer::{DataVisualizer, GetData, ReportParams};
use crate::{data::ProcessedData, APERF_RUNLOG, VISUALIZATION_DATA};
use anyhow::Result;
use ctor::ctor;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, BufReader},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AperfRunlog {
    pub data: Vec<String>,
}

impl AperfRunlog {
    fn new() -> Self {
        AperfRunlog { data: Vec::new() }
    }
}

impl GetData for AperfRunlog {
    fn custom_raw_data_parser(&mut self, params: ReportParams) -> Result<Vec<ProcessedData>> {
        let mut raw_data: Vec<ProcessedData> = Vec::new();
        let mut runlog = AperfRunlog::new();

        let file = fs::OpenOptions::new()
            .read(true)
            .open(params.data_file_path)
            .expect("Could not open Aperf Runlog file");
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            if line.ends_with('\n') {
                line = line.trim_end().to_string();
            }
            runlog.data.push(line.clone());
            line.clear();
        }
        raw_data.push(ProcessedData::AperfRunlog(runlog));
        Ok(raw_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(&mut self, buffer: Vec<ProcessedData>, _query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::AperfRunlog(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        Ok(serde_json::to_string(&values)?)
    }
}

#[ctor]
fn init_aperf_runlog() {
    let file_name = APERF_RUNLOG.to_string();
    let js_file_name = file_name.clone() + ".js";
    let aperf_runlog = AperfRunlog::new();
    let mut dv = DataVisualizer::new(
        ProcessedData::AperfRunlog(aperf_runlog.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/aperf_runlog.js")).to_string(),
        file_name.clone(),
    );
    dv.has_custom_raw_data_parser();

    VISUALIZATION_DATA
        .lock()
        .unwrap()
        .add_visualizer(file_name, dv);
}
