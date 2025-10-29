use crate::data::data_formats::{AperfData, TextData};
use crate::data::{Data, ProcessedData};
use crate::utils::DataMetrics;
use crate::visualizer::{GetData, ReportParams};
use anyhow::Result;
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
    pub fn new() -> Self {
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

    fn process_raw_data_new(
        &mut self,
        params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut text_data = TextData::default();

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
            text_data.lines.push(line.clone());
            line.clear();
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
                ProcessedData::AperfRunlog(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        Ok(serde_json::to_string(&values)?)
    }

    fn has_custom_raw_data_parser() -> bool {
        true
    }
}
