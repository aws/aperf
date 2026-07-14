use crate::aperf_runlog_file_path;
use crate::data::common::data_formats::{AperfData, TextData};
use crate::data::{Data, ProcessData};
use crate::data_processing::ReportParams;
use anyhow::{Context, Result};
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
        report_params: &ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut text_data = TextData::default();

        let aperf_runlog_path = aperf_runlog_file_path(&report_params.run_data_dir);

        let file = fs::OpenOptions::new()
            .read(true)
            .open(&aperf_runlog_path)
            .with_context(|| {
                format!(
                    "Failed to open APerf runlog file at {}",
                    aperf_runlog_path.display()
                )
            })?;

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
