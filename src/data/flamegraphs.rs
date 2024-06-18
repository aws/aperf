extern crate ctor;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData};
use crate::visualizer::{DataVisualizer, GetData, ReportParams};
use crate::{get_file_name, PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use ctor::ctor;
use inferno::collapse::perf::Folder;
use inferno::collapse::Collapse;
use inferno::flamegraph::{self, Options};
use log::{error, trace};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::Command;

pub static FLAMEGRAPHS_FILE_NAME: &str = "flamegraph";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlamegraphRaw {
    pub data: String,
}

impl FlamegraphRaw {
    fn new() -> Self {
        FlamegraphRaw {
            data: String::new(),
        }
    }
}

impl CollectData for FlamegraphRaw {
    fn after_data_collection(&mut self, params: CollectorParams) -> Result<()> {
        let data_dir = PathBuf::from(params.data_dir.clone());

        let mut file_pathbuf = data_dir.clone();
        file_pathbuf.push(get_file_name(
            params.data_dir.clone(),
            "perf_profile".to_string(),
        )?);

        let mut perf_jit_loc = data_dir.clone();
        perf_jit_loc.push(format!("{}-perf.data.jit", params.run_name));

        println!("Running Perf inject...");
        let out_jit = Command::new("perf")
            .args([
                "inject",
                "-j",
                "-i",
                &file_pathbuf.to_str().unwrap(),
                "-o",
                perf_jit_loc.clone().to_str().unwrap(),
            ])
            .status();
        match out_jit {
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    error!("'perf' command not found.");
                } else {
                    error!("Unknown error: {}", e);
                }
                error!("Skip processing profiling data.");
            }
            Ok(_) => trace!("Perf inject successful."),
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Flamegraph {
    pub data: String,
}

impl Flamegraph {
    fn new() -> Self {
        Flamegraph {
            data: String::new(),
        }
    }
}

impl GetData for Flamegraph {
    fn custom_raw_data_parser(&mut self, params: ReportParams) -> Result<Vec<ProcessedData>> {
        /* Get the perf_profile file */
        let mut file_pathbuf = PathBuf::from(params.data_dir.clone());
        file_pathbuf.push(get_file_name(
            params.data_dir.clone(),
            "perf_profile".to_string(),
        )?);

        let _file_name = file_pathbuf.to_str().unwrap();
        let profile = Flamegraph::new();

        let mut perf_jit_loc = PathBuf::from(params.data_dir.clone());
        perf_jit_loc.push(format!("{}-perf.data.jit", params.run_name));

        /* Use APERF_TMP to generate intermediate perf files */
        let tmp_path = PathBuf::from(params.tmp_dir);

        let mut script_loc = tmp_path.clone();
        script_loc.push(format!("{}-script.out", params.run_name));

        let mut collapse_loc = tmp_path.clone();
        collapse_loc.push(format!("{}-collapse.out", params.run_name));

        let mut fg_loc = params.report_dir.clone();
        fg_loc.push(format!("data/js/{}-flamegraph.svg", params.run_name));

        let mut script_out = File::create(script_loc.clone())?;
        let collapse_out = File::create(collapse_loc.clone())?;
        let mut fg_out = File::create(fg_loc.clone())?;

        if perf_jit_loc.exists() {
            let out = Command::new("perf")
                .args(["script", "-f", "-i", perf_jit_loc.to_str().unwrap()])
                .output();
            match out {
                Err(e) => {
                    if e.kind() == ErrorKind::NotFound {
                        error!("'perf' command not found.");
                    } else {
                        error!("Unknown error: {}", e);
                    }
                    error!("Skip processing profiling data.");
                    write!(fg_out, "<svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" width=\"100%\" height=\"100%\"><text x=\"0%\" y=\"1%\">Did not process profiling data</text></svg>")?;
                }
                Ok(v) => {
                    write!(script_out, "{}", std::str::from_utf8(&v.stdout)?)?;
                    Folder::default().collapse_file(Some(script_loc), collapse_out)?;
                    fg_out = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .truncate(true)
                        .open(fg_loc)?;
                    flamegraph::from_files(
                        &mut Options::default(),
                        &[collapse_loc.to_path_buf()],
                        fg_out,
                    )?;
                }
            }
        } else {
            write!(fg_out, "<svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" width=\"100%\" height=\"100%\"><text x=\"0%\" y=\"1%\">No data collected</text></svg>")?;
        }
        let processed_data = vec![ProcessedData::Flamegraph(profile)];
        Ok(processed_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(&mut self, _buffer: Vec<ProcessedData>, _query: String) -> Result<String> {
        let values: Vec<&str> = Vec::new();
        Ok(serde_json::to_string(&values)?)
    }
}

#[ctor]
fn init_flamegraph() {
    let flamegraph_raw = FlamegraphRaw::new();
    let file_name = FLAMEGRAPHS_FILE_NAME.to_string();
    let mut dt = DataType::new(
        Data::FlamegraphRaw(flamegraph_raw.clone()),
        file_name.clone(),
        false,
    );
    dt.is_profile_option();
    let flamegraph = Flamegraph::new();
    let js_file_name = file_name.clone() + ".js";
    let mut dv = DataVisualizer::new(
        ProcessedData::Flamegraph(flamegraph.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/flamegraphs.js")).to_string(),
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
        .add_visualizer(file_name, dv);
}
