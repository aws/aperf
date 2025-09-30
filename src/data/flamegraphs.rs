use crate::data::{CollectData, CollectorParams, ProcessedData};
use crate::utils::DataMetrics;
use crate::visualizer::{GetData, ReportParams};
use crate::{get_file_name, PDError};
use anyhow::Result;
use inferno::collapse::perf::Folder;
use inferno::collapse::Collapse;
use inferno::flamegraph::{self, Options};
use log::{error, info, trace};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

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
    pub fn new() -> Self {
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
        let reverse_fg_out =
            File::create(data_dir.join(format!("{}-reverse-flamegraph.svg", params.run_name)))?;

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

                        // Generate reverse flamegraph
                        let mut reverse_options = Options::default();
                        reverse_options.reverse_stack_order = true;
                        flamegraph::from_files(
                            &mut reverse_options,
                            &[collapse_loc.to_path_buf()],
                            reverse_fg_out,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn is_profile() -> bool {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Flamegraph {
    pub data: String,
}

impl Flamegraph {
    pub fn new() -> Self {
        Flamegraph {
            data: String::new(),
        }
    }
}

impl GetData for Flamegraph {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["flamegraph"]
    }

    fn custom_raw_data_parser(&mut self, params: ReportParams) -> Result<Vec<ProcessedData>> {
        let mut processed_data = Vec::new();

        let file_name = format!("{}-flamegraph.svg", params.run_name);
        let fg_loc = params.data_dir.join(&file_name);
        let fg_out_relative = format!("data/js/{file_name}");
        let fg_out = params.report_dir.join(&fg_out_relative);

        /* Copy the flamegraph to the report dir */
        if fg_loc.exists() {
            std::fs::copy(&fg_loc, &fg_out)?;
            let mut flamegraph = Flamegraph::new();
            flamegraph.data = fg_out_relative;
            processed_data.push(ProcessedData::Flamegraph(flamegraph));
        }

        /* Copy the reverse flamegraph to the report dir */
        let reverse_file_name = format!("{}-reverse-flamegraph.svg", params.run_name);
        let reverse_fg_loc = params.data_dir.join(&reverse_file_name);
        let reverse_fg_out = params
            .report_dir
            .join(format!("data/js/{reverse_file_name}"));
        if reverse_fg_loc.exists() {
            std::fs::copy(&reverse_fg_loc, &reverse_fg_out)?;
        }

        Ok(processed_data)
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
        if buffer.is_empty() {
            return Ok(String::new());
        }

        match buffer[0] {
            ProcessedData::Flamegraph(ref value) => Ok(value.data.clone()),
            _ => unreachable!(),
        }
    }

    fn has_custom_raw_data_parser() -> bool {
        true
    }
}
