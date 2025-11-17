use crate::data::data_formats::{AperfData, TextData};
use crate::data::{Data, ProcessData};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, BufReader},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AperfRunlog;

impl AperfRunlog {
    pub fn new() -> Self {
        AperfRunlog
    }
}

impl ProcessData for AperfRunlog {
    fn process_raw_data(
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
}
