use crate::data::common::data_formats::{
    AperfData, GraphData, GraphGroup, Profiler, ProfilingData,
};
use crate::data::common::utils::copy_graph_and_update_graph_data;
#[cfg(target_os = "linux")]
use crate::data::common::utils::get_data_name_from_type;
use crate::data::{Data, ProcessData};
use crate::profiling::{jfr, Profile};
use crate::visualizer::ReportParams;
use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    crate::PDError,
    log::debug,
    nix::{sys::signal, unistd::Pid},
    std::fs::File,
    std::io::Write,
    std::process::{Child, Command},
    std::sync::Mutex,
};

const PROFILE_METRICS: &[&str] = &["cpu", "alloc", "wall"];

#[cfg(target_os = "linux")]
lazy_static! {
    pub static ref ASPROF_CHILDREN: Mutex<Vec<Child>> = Mutex::new(Vec::new());
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaProfileRaw {
    process_map: HashMap<String, Vec<String>>,
}

#[cfg(target_os = "linux")]
impl Default for JavaProfileRaw {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
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
                "jps command failed. Ensure JDK is installed to use Java profiling. Error msg: {}",
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
                "pgrep command failed. Error msg: {}",
                e
            ))),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for JavaProfileRaw {
    fn prepare_data_collector(&mut self, params: &CollectorParams) -> Result<()> {
        // Check if asprof is installed
        match Command::new("asprof").args(["--version"]).output() {
            Ok(_) => {},
            Err(e) => return Err(PDError::DependencyError(format!(
                "'asprof' command failed. Ensure it is installed and refer to DEPENDENCIES documentation for more info. Error msg: {}",
                e
            )).into()),
        }

        let mut jids: Vec<String> = Vec::new();
        let pgrep: Vec<String> = self.launch_pgrep()?;
        for pid in pgrep {
            if self.process_map.contains_key(&pid) {
                continue;
            }
            self.process_map
                .insert(pid.clone(), vec![format!("Unknown_JVM_{pid}")]);
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
                .insert(pid.clone(), vec![format!("Unknown_JVM_{pid}")]);
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

        debug!("Waiting for asprof profile collection to complete...");
        while ASPROF_CHILDREN.lock().unwrap().len() > 0 {
            match ASPROF_CHILDREN.lock().unwrap().pop().unwrap().wait() {
                Err(e) => {
                    error!("'asprof' did not exit successfully: {}", e);
                    return Ok(());
                }
                Ok(_) => debug!("'asprof' executed successfully."),
            }
        }

        let data_dir = params.data_dir.clone();
        for key in self.process_map.keys() {
            let jfr_path = params
                .tmp_dir
                .join(format!("{}-java-profile-{}.jfr", params.run_name, key));

            if fs::exists(&jfr_path).expect("Can't check existence of jfr file") {
                // Extract metadata JSON string from JFR
                let metadata_events = [
                    "jdk.ActiveRecording",
                    "jdk.ActiveSetting",
                    "jdk.CheckPoint",
                    "jdk.Metadata",
                    "jdk.JVMInformation",
                    "jdk.NativeLibrary",
                ];
                let metadata_json = match Command::new("jfr")
                    .args([
                        "print",
                        "--json",
                        "--events",
                        &metadata_events.join(","),
                        &jfr_path.to_string_lossy(),
                    ])
                    .output()
                {
                    Err(e) => {
                        error!("'jfr' metadata extraction failed for {}: {}", key, e);
                        Value::Null
                    }
                    Ok(output) => {
                        if !output.status.success() {
                            error!(
                                "'jfr' metadata extraction failed for {}: {}",
                                key,
                                String::from_utf8_lossy(&output.stderr)
                            );
                            Value::Null
                        } else {
                            serde_json::from_slice(&output.stdout).unwrap_or(Value::Null)
                        }
                    }
                };

                // Generate heatmaps for each profiling type
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
                            "--dot",
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
                                debug!(
                                    "Successfully converted JFR to {} heatmap for {}",
                                    metric, key
                                );
                            }
                        }
                    }
                }

                // Generate Profiler from JFR
                let profiler_data_path = params.data_dir.join(format!(
                    "{}-java-profile-{}-profiler-data.json",
                    params.run_name, key
                ));

                let event_out_path_buf =
                    params.data_dir.join(format!("parsed_jfr_events_{key}.out"));
                let events_out_path = if params.save_profile_events {
                    Some(event_out_path_buf.as_path())
                } else {
                    None
                };

                // TODO: Guard the new profile processing logic by the save_profile_events flag,
                //       so that the new flow is only executed in tests. Remove the guardrail
                //       after the feature is ready to launch.
                if params.save_profile_events {
                    match jfr::build_java_profiler_data(&jfr_path, events_out_path) {
                        Ok(mut profiler) => {
                            profiler.metadata = jfr::parse_jfr_metadata(&metadata_json);
                            if let Ok(json) = serde_json::to_string(&profiler) {
                                fs::write(&profiler_data_path, json).ok();
                            }
                        }
                        Err(e) => {
                            error!("Failed to build Profiler Data for {}: {}", key, e);
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
        let mut profiling_data = ProfilingData::default();
        // For backward compatibility
        let mut graph_data = GraphData::default();

        let processes_loc = params
            .data_dir
            .join(format!("{}-jps-map.json", params.run_name));
        let processes_json =
            fs::read_to_string(processes_loc.to_str().unwrap()).unwrap_or_default();
        let process_map: HashMap<String, Vec<String>> =
            serde_json::from_str(&processes_json).unwrap_or_default();

        let mut profile_metrics = Vec::from(PROFILE_METRICS);
        profile_metrics.push("legacy");
        profile_metrics.iter().for_each(|&metric| {
            graph_data.graph_groups.push(GraphGroup::new(metric));
        });

        // Track JVMs with same name
        let mut jvm_name_counts: HashMap<String, usize> = HashMap::new();
        // Stores the deduped JVM name of each PID
        let mut deduped_names: HashMap<String, String> = HashMap::new();

        for (process, process_names) in &process_map {
            let jvm_name = process_names.first().map_or("unknown", |s| s.as_str());
            let deduped_name = deduped_names.entry(process.clone()).or_insert_with(|| {
                let jvm_name_count = jvm_name_counts.entry(jvm_name.to_string()).or_insert(0);
                *jvm_name_count += 1;

                if *jvm_name_count > 1 {
                    format!("{} ({})", jvm_name, *jvm_name_count - 1)
                } else {
                    jvm_name.to_string()
                }
            });

            // Copy jfrconv-generated HTML graphs for this process to the report data dir
            // and build the GraphData (backward compatibility).
            for &metric in &profile_metrics {
                let filename = if metric == "legacy" {
                    // backward compatibility - previous versions generated a single flamegraph
                    format!("{}-java-flamegraph-{}.html", params.run_name, process)
                } else {
                    format!(
                        "{}-java-profile-{}-{}.html",
                        params.run_name, process, metric
                    )
                };
                copy_graph_and_update_graph_data(
                    &params.data_dir,
                    &params.report_dir,
                    &filename,
                    metric,
                    &deduped_name,
                    format!("({}) JVM: {}", metric, deduped_name),
                    &mut graph_data,
                );
            }

            // Deserialize the ProfilerData generated at the end of recording.
            let profiler_data_path = params.data_dir.join(format!(
                "{}-java-profile-{}-profiler-data.json",
                params.run_name, process
            ));
            let mut profiler = match fs::read_to_string(&profiler_data_path)
                .ok()
                .and_then(|json| serde_json::from_str::<Profiler>(&json).ok())
            {
                Some(profiler) => profiler,
                None => continue,
            };

            // Ensure every profiling metric has an entry so the frontend renders a
            // tab for it even when the JFR contained no events for that metric.
            for metric in PROFILE_METRICS {
                profiler
                    .profiles
                    .entry(metric.to_string())
                    .or_insert_with(Profile::new);
            }

            profiling_data
                .profilers
                .insert(deduped_name.clone(), profiler);
        }

        // If no ProfilerData was read, chances are this run was created before the
        // introduction of ProfilingData, so fall back to the old GraphData.
        if profiling_data.profilers.is_empty() {
            return Ok(AperfData::Graph(graph_data));
        }

        Ok(AperfData::Profile(profiling_data))
    }
}
