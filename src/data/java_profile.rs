use crate::data::data_formats::{AperfData, Graph, GraphData, GraphGroup};
use crate::data::{CollectData, CollectorParams, Data, ProcessData};
use crate::utils::get_data_name_from_type;
use crate::visualizer::ReportParams;
use crate::PDError;
use anyhow::Result;
use log::{debug, error, trace};
use nix::{sys::signal, unistd::Pid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::{fs, fs::File};

const PROFILE_METRICS: &[&str] = &["cpu", "alloc", "wall"];

lazy_static! {
    pub static ref ASPROF_CHILDREN: Mutex<Vec<Child>> = Mutex::new(Vec::new());
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaProfileRaw {
    process_map: HashMap<String, Vec<String>>,
}

impl Default for JavaProfileRaw {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaProfileRaw {
    pub fn new() -> Self {
        JavaProfileRaw {
            process_map: HashMap::new(),
        }
    }

    fn launch_asprof(&self, jids: Vec<String>, params: &CollectorParams) -> Result<()> {
        for jid in &jids {
            match Command::new("asprof")
                .args([
                    "-d",
                    &(params.collection_time - params.elapsed_time).to_string(),
                    "-o",
                    "jfr",
                    "-e",
                    "cpu",
                    "--alloc",
                    "2m",
                    "--wall",
                    "100ms",
                    "--cstack",
                    "vm",
                    "-F",
                    "vtable",
                    "-f",
                    &params
                        .tmp_dir
                        .join(format!("{}-java-profile-{}.jfr", params.run_name, jid))
                        .to_string_lossy(),
                    jid.as_str(),
                ])
                .spawn()
            {
                Err(e) => {
                    return Err(PDError::DependencyError(format!(
                        "'asprof' command failed. {}",
                        e
                    ))
                    .into());
                }
                Ok(child) => {
                    debug!(
                        "Recording asprof profiling data for '{}' with PID, {}.",
                        self.process_map
                            .get(jid.as_str())
                            .unwrap_or(&vec![String::from("JVM")])[0],
                        jid
                    );
                    ASPROF_CHILDREN.lock().unwrap().push(child);
                }
            }
        }
        Ok(())
    }

    fn get_jids(&mut self, arg: &str) -> Vec<String> {
        let mut jids: Vec<String> = Vec::new();
        for (key, value) in self.process_map.clone().into_iter() {
            if arg == value[0] || arg == key {
                jids.push(key);
            }
        }
        jids
    }

    fn update_process_map(&mut self) -> Result<String, PDError> {
        debug!("Running jps (may incur utilization spike)...");
        let jps_cmd = Command::new("jps").output();
        /*
        Output of jps:
        lvmid [ classname | JARfilename | "Unknown"]
        lvmid [ classname | JARfilename | "Unknown"]
        .
        .
        lvmid [ classname | JARfilename | "Unknown"]
        */
        match jps_cmd {
            Ok(jps_out) => {
                let jps_str = String::from_utf8(jps_out.stdout).unwrap_or_default();
                let jps: Vec<&str> = jps_str.split_whitespace().collect();
                for i in (0..jps.len()).step_by(2) {
                    if jps[i + 1] != "Jps" {
                        self.process_map
                            .insert(String::from(jps[i]), vec![String::from(jps[i + 1])]);
                    }
                }
                Ok(jps_str)
            }
            Err(e) => Err(PDError::DependencyError(format!(
                "Jps command failed. {}",
                e
            ))),
        }
    }

    fn launch_pgrep(&mut self) -> Result<Vec<String>, PDError> {
        let pgrep_cmd = Command::new("pgrep").arg("java").output();
        match pgrep_cmd {
            Ok(pgrep_out) => {
                let pgrep_str = String::from_utf8(pgrep_out.stdout).unwrap();
                return Ok(pgrep_str
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect());
            }
            Err(e) => Err(PDError::DependencyError(format!(
                "pgrep command failed. {}",
                e
            ))),
        }
    }
}

impl CollectData for JavaProfileRaw {
    fn prepare_data_collector(&mut self, params: &CollectorParams) -> Result<()> {
        let mut jids: Vec<String> = Vec::new();
        let pgrep: Vec<String> = self.launch_pgrep()?;
        for pid in pgrep {
            if self.process_map.contains_key(&pid) {
                continue;
            }
            self.process_map
                .insert(pid.clone(), vec![String::from("Could not resolve name!")]);
        }

        let jps_str = self.update_process_map()?;
        let jps: Vec<&str> = jps_str.split_whitespace().collect();

        let jprofile_value = params.profile.get(get_data_name_from_type::<Self>());
        if let Some(value) = jprofile_value {
            match value.as_str() {
                "jps" => {
                    jids = self.process_map.clone().into_keys().collect();
                    debug!("Jps will be run if new JVM is started during aperf record to resolve process names.",);
                }
                _ => {
                    let args: Vec<&str> = value.split(',').collect();
                    for arg in args {
                        if !jps.contains(&arg) {
                            error!("No JVM with name/PID '{}'.", arg);
                            continue;
                        }
                        jids.append(&mut self.get_jids(arg));
                    }
                }
            }
        }
        jids.sort();
        jids.dedup();
        self.launch_asprof(jids, params)
    }

    fn collect_data(&mut self, params: &CollectorParams) -> Result<()> {
        let jprofile = params
            .profile
            .get(get_data_name_from_type::<Self>())
            .unwrap()
            .as_str();
        if jprofile != "jps" {
            return Ok(());
        }

        let pgrep_pids: Vec<String> = self.launch_pgrep()?;

        let mut jids: Vec<String> = Vec::new();
        for pid in pgrep_pids {
            if self.process_map.contains_key(pid.as_str()) {
                continue;
            }
            self.process_map
                .insert(pid.clone(), vec![String::from("Could not resolve name!")]);
            jids.push(pid.clone());
        }

        if jids.is_empty() || params.elapsed_time >= params.collection_time {
            return Ok(());
        }

        self.update_process_map()?;
        self.launch_asprof(jids, params)
    }

    fn finish_data_collection(&mut self, params: &CollectorParams) -> Result<()> {
        for child in ASPROF_CHILDREN.lock().unwrap().iter() {
            signal::kill(Pid::from_raw(child.id() as i32), params.signal)?;
        }

        trace!("Waiting for asprof profile collection to complete...");
        while ASPROF_CHILDREN.lock().unwrap().len() > 0 {
            match ASPROF_CHILDREN.lock().unwrap().pop().unwrap().wait() {
                Err(e) => {
                    error!("'asprof' did not exit successfully: {}", e);
                    return Ok(());
                }
                Ok(_) => trace!("'asprof' executed successfully."),
            }
        }

        let data_dir = params.data_dir.clone();
        for key in self.process_map.keys() {
            let jfr_path = params
                .tmp_dir
                .join(format!("{}-java-profile-{}.jfr", params.run_name, key));

            if fs::exists(&jfr_path).expect("Can't check existence of jfr file") {
                for metric in PROFILE_METRICS {
                    let html_path = data_dir.join(format!(
                        "{}-java-profile-{}-{}.html",
                        params.run_name, key, metric
                    ));

                    match Command::new("jfrconv")
                        .args([
                            &format!("--{metric}"),
                            "-o",
                            "heatmap",
                            &jfr_path.to_string_lossy(),
                            html_path.to_str().unwrap(),
                        ])
                        .output()
                    {
                        Err(e) => {
                            error!(
                                "'jfrconv' command failed for {} with metric {}: {}",
                                key, metric, e
                            );
                        }
                        Ok(output) => {
                            if !output.status.success() {
                                error!(
                                    "'jfrconv' failed for {} with metric {}: {}",
                                    key,
                                    metric,
                                    String::from_utf8_lossy(&output.stderr)
                                );
                            } else {
                                trace!(
                                    "Successfully converted JFR to {} heatmap for {}",
                                    metric,
                                    key
                                );
                            }
                        }
                    }
                }

                let jfr_dest =
                    data_dir.join(format!("{}-java-profile-{}.jfr", params.run_name, key));
                fs::copy(&jfr_path, jfr_dest).ok();
            }
        }

        let mut jps_map = File::create(
            data_dir
                .clone()
                .join(format!("{}-jps-map.json", params.run_name)),
        )?;
        write!(jps_map, "{}", serde_json::to_string(&self.process_map)?)?;

        Ok(())
    }

    fn is_java_profile() -> bool {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaProfile;

impl JavaProfile {
    pub fn new() -> Self {
        JavaProfile
    }
}

impl ProcessData for JavaProfile {
    fn process_raw_data(
        &mut self,
        params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut graph_data = GraphData::default();

        let processes_loc = params
            .data_dir
            .join(format!("{}-jps-map.json", params.run_name));
        let processes_json =
            fs::read_to_string(processes_loc.to_str().unwrap()).unwrap_or_default();
        let process_map: HashMap<String, Vec<String>> =
            serde_json::from_str(&processes_json).unwrap_or(HashMap::new());

        let mut profile_metrics = Vec::from(PROFILE_METRICS);
        profile_metrics.push("legacy");
        for metric in profile_metrics {
            let mut graph_group = GraphGroup::default();
            graph_group.group_name = String::from(metric);

            for (process, process_names) in &process_map {
                let filename = if metric == "legacy" {
                    // backward compatibility - to support previous versions where java profile
                    // generates a single flamegraph
                    format!("{}-java-flamegraph-{}.html", params.run_name, process)
                } else {
                    format!(
                        "{}-java-profile-{}-{}.html",
                        params.run_name, process, metric
                    )
                };

                let relative_path = PathBuf::from("data/js");
                if let Some(file_size) = copy_file_to_report_data(
                    &filename,
                    &params.data_dir,
                    &params.report_dir.join(relative_path.clone()),
                ) {
                    let graph_name = format!(
                        "JVM: {}, PID: {} ({})",
                        process_names.first().map_or("unknown", |s| s.as_str()),
                        process,
                        metric
                    );
                    graph_group.graphs.insert(
                        graph_name.clone(),
                        Graph::new(
                            graph_name,
                            relative_path
                                .join(filename)
                                .into_os_string()
                                .into_string()
                                .unwrap(),
                            Some(file_size),
                        ),
                    );
                }
            }

            graph_data
                .graph_groups
                .insert(String::from(metric), graph_group);
        }

        Ok(AperfData::Graph(graph_data))
    }
}

fn copy_file_to_report_data(
    filename: &String,
    src_dir: &PathBuf,
    dest_dir: &PathBuf,
) -> Option<u64> {
    let src_path = src_dir.join(filename);
    let file_metadata = fs::metadata(&src_path).ok()?;
    let file_size = file_metadata.len();
    let dest_path = dest_dir.join(filename);

    fs::copy(&src_path, &dest_path).ok()?;

    Some(file_size)
}
