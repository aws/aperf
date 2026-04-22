use crate::data::common::data_formats::{AperfData, Profiler, ProfilingData};
use crate::data::{Data, ProcessData};
use crate::profiling::{Profile, ProfileGraph};
use crate::visualizer::ReportParams;
use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    crate::{get_file_name, PDError},
    inferno::collapse::{perf::Folder, Collapse},
    inferno::flamegraph::{self, Direction, Options},
    log::{debug, info},
    std::fs,
    std::fs::File,
    std::io::Write,
    std::process::Command,
};

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
impl FlamegraphRaw {
    pub fn new() -> Self {
        FlamegraphRaw {
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
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

        debug!("Running Perf inject...");
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
                // TODO: extract metadata from perf record and generate script -> ProfilingData
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
                        // TODO: move flamegraph generation to report phase using ProfilingData (so user specifies time range)
                        Folder::default().collapse_file(
                            Some(script_loc.clone()),
                            File::create(&collapse_loc)?,
                        )?;

                        // Generate icicle graph as default
                        let mut reverse_options = Options::default();
                        reverse_options.direction = Direction::Inverted;
                        reverse_options.reverse_stack_order = false;
                        flamegraph::from_files(
                            &mut reverse_options,
                            &[collapse_loc.to_path_buf()],
                            fg_out,
                        )?;

                        // Generate reverse icicle graph
                        reverse_options.reverse_stack_order = true;
                        flamegraph::from_files(
                            &mut reverse_options,
                            &[collapse_loc.to_path_buf()],
                            reverse_fg_out,
                        )?;

                        // Clean up intermediate files after creating flamegraphs and saving
                        for file in [&script_loc, &perf_jit_loc, &collapse_loc] {
                            fs::remove_file(file).ok();
                        }
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
pub struct Flamegraph;

impl Flamegraph {
    pub fn new() -> Self {
        Flamegraph
    }
}

impl ProcessData for Flamegraph {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["flamegraph"]
    }

    fn process_raw_data(
        &mut self,
        params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        fn copy_and_add_to_profiler(
            params: &ReportParams,
            filename: String,
            profiling_data: &mut ProfilingData,
            profiler_name: String,
            profile_name: String,
        ) {
            let source_path = params.data_dir.join(&filename);
            let relative_dest_path = PathBuf::from("data/js").join(filename);
            let dest_path = params.report_dir.join(relative_dest_path.clone());

            if source_path.exists() {
                if let Ok(_) = std::fs::copy(&source_path, &dest_path) {
                    let profiler = profiling_data
                        .profilers
                        .entry(profiler_name.clone())
                        .or_insert_with(Profiler::default);
                    profiler.profiles.insert(
                        profile_name.clone(),
                        Profile::with_graph(ProfileGraph::new(
                            format!("Kernel Profiling Flamegraph ({profiler_name})"),
                            relative_dest_path.into_os_string().into_string().unwrap(),
                            None,
                        )),
                    );
                }
            }
        }

        let mut profiling_data = ProfilingData::default();

        copy_and_add_to_profiler(
            &params,
            format!("{}-flamegraph.svg", params.run_name),
            &mut profiling_data,
            String::from("perf"),
            String::from("default"),
        );
        copy_and_add_to_profiler(
            &params,
            format!("{}-reverse-flamegraph.svg", params.run_name),
            &mut profiling_data,
            String::from("perf"),
            String::from("reverse"),
        );

        Ok(AperfData::Profile(profiling_data))
    }
}
