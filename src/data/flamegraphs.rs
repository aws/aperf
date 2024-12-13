extern crate ctor;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData};
use crate::utils::DataMetrics;
use crate::visualizer::{DataVisualizer, GetData, ReportParams};
use crate::{get_file_name, PDError, PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use ctor::ctor;
use inferno::collapse::perf::Folder;
use inferno::collapse::Collapse;
use inferno::flamegraph::{self, Options};
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

pub static FLAMEGRAPHS_FILE_NAME: &str = "flamegraph";

fn write_msg_to_svg(mut file: File, msg: String) -> Result<()> {
    write!(
        file,
        "<svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" width=\"100%\" height=\"100%\"><text x=\"0%\" y=\"1%\">{}</text></svg>",
        msg
    )?;
    Ok(())
}

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
    fn prepare_data_collector(&mut self, _params: &CollectorParams) -> Result<()> {
        match Command::new("perf").args(["--version"]).output() {
            Err(e) => Err(PDError::DependencyError(format!("'perf' command failed. {}", e)).into()),
            _ => Ok(()),
        }
    }

    fn after_data_collection(&mut self, params: &CollectorParams) -> Result<()> {
        let data_dir = PathBuf::from(&params.data_dir);

        let file_pathbuf = data_dir.join(get_file_name(
            params.data_dir.display().to_string(),
            "perf_profile".to_string(),
        )?);

        let perf_jit_loc = data_dir.join("perf.data.jit");

        trace!("Running Perf inject...");
        let out_jit = Command::new("perf")
            .args([
                "inject",
                "-j",
                "-i",
                file_pathbuf.to_str().unwrap(),
                "-o",
                perf_jit_loc.to_str().unwrap(),
            ])
            .status();

        let fg_out = File::create(data_dir.join(format!("{}-flamegraph.svg", params.run_name)))?;

        match out_jit {
            Err(e) => {
                let out = format!("Skip processing profiling data due to: {}", e);
                error!("{}", out);
                write_msg_to_svg(fg_out, out)?;
            }
            Ok(_) => {
                info!("Creating flamegraph...");
                let script_loc = data_dir.join("script.out");
                let out = Command::new("perf")
                    .stdout(File::create(&script_loc)?)
                    .args(["script", "-f", "-i", perf_jit_loc.to_str().unwrap()])
                    .output();
                match out {
                    Err(e) => {
                        let out = format!("Did not process profiling data due to: {}", e);
                        error!("{}", out);
                        write_msg_to_svg(fg_out, out)?;
                    }
                    Ok(_) => {
                        let collapse_loc = data_dir.join("collapse.out");

                        Folder::default()
                            .collapse_file(Some(script_loc), File::create(&collapse_loc)?)?;
                        flamegraph::from_files(
                            &mut Options::default(),
                            &[collapse_loc.to_path_buf()],
                            fg_out,
                        )?;
                    }
                }
            }
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
        let processed_data = vec![ProcessedData::Flamegraph(Flamegraph::new())];

        let file_name = format!("{}-flamegraph.svg", params.run_name);
        let fg_loc = params.data_dir.join(&file_name);
        let fg_out = params.report_dir.join("data/js/".to_owned() + &file_name);

        /* Copy the flamegraph to the report dir */
        if fg_loc.exists() {
            std::fs::copy(fg_loc, fg_out)?;
        } else {
            write_msg_to_svg(
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .read(true)
                    .write(true)
                    .open(fg_out)?,
                "No data collected".to_string(),
            )?;
        }
        Ok(processed_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(
        &mut self,
        _buffer: Vec<ProcessedData>,
        _query: String,
        _metrics: &mut DataMetrics,
    ) -> Result<String> {
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
