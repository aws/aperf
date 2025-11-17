use crate::data::data_formats::{AperfData, Graph, GraphData, GraphGroup};
use crate::data::{CollectData, CollectorParams, Data, ProcessData};
use crate::visualizer::ReportParams;
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
        fn copy_and_add_to_graph_group(
            params: &ReportParams,
            filename: String,
            graph_data: &mut GraphData,
            graph_group_name: String,
        ) {
            let source_path = params.data_dir.join(&filename);
            let relative_dest_path = PathBuf::from("data/js").join(filename);
            let dest_path = params.report_dir.join(relative_dest_path.clone());

            if source_path.exists() {
                if let Ok(_) = std::fs::copy(&source_path, &dest_path) {
                    let mut graph_group = GraphGroup::default();
                    graph_group.group_name = graph_group_name.clone();
                    graph_group.graphs.insert(
                        String::new(),
                        Graph::new(
                            format!("Kernel Profiling Flamegraph ({graph_group_name})"),
                            relative_dest_path.into_os_string().into_string().unwrap(),
                            None,
                        ),
                    );
                    graph_data
                        .graph_groups
                        .insert(graph_group_name.clone(), graph_group);
                }
            }
        }

        let mut graph_data = GraphData::default();

        copy_and_add_to_graph_group(
            &params,
            format!("{}-flamegraph.svg", params.run_name),
            &mut graph_data,
            String::from("default"),
        );
        copy_and_add_to_graph_group(
            &params,
            format!("{}-reverse-flamegraph.svg", params.run_name),
            &mut graph_data,
            String::from("reverse"),
        );

        Ok(AperfData::Graph(graph_data))
    }
}
