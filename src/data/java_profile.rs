extern crate ctor;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData};
use crate::visualizer::{DataVisualizer, GetData};
use crate::{PDError, PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use ctor::ctor;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::{fs, fs::File};

pub static JAVA_PROFILE_FILE_NAME: &str = "java_profile";

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

    fn launch_asprof(&self, jids: Vec<String>, params: CollectorParams) -> Result<()> {
        let data_dir = PathBuf::from(params.data_dir.clone());
        for jid in &jids {
            let mut html_loc = data_dir.clone();
            html_loc.push(format!("{}-java-flamegraph-{}.html", params.run_name, jid));

            match Command::new("asprof")
                .args([
                    "-d",
                    &(params.collection_time - params.elapsed_time).to_string(),
                    "-f",
                    html_loc.to_str().unwrap(),
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
            if arg == value[0] {
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
    fn prepare_data_collector(&mut self, params: CollectorParams) -> Result<()> {
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

        let jprofile_value = params.profile.get(JAVA_PROFILE_FILE_NAME);
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
                        jids = self.get_jids(arg);
                    }
                }
            }
        }

        self.launch_asprof(jids, params.clone())
    }

    fn collect_data(&mut self, params: &CollectorParams) -> Result<()> {
        let jprofile = params.profile.get(JAVA_PROFILE_FILE_NAME).unwrap().as_str();
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
        self.launch_asprof(jids, params.clone())
    }

    fn finish_data_collection(&mut self, params: CollectorParams) -> Result<()> {
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

        let data_dir = PathBuf::from(params.data_dir.clone());
        let mut jps_map = File::create(
            data_dir
                .clone()
                .join(format!("{}-jps-map.json", params.run_name)),
        )?;
        write!(jps_map, "{}", serde_json::to_string(&self.process_map)?)?;

        Ok(())
    }

    fn after_data_collection(&mut self, _params: CollectorParams) -> Result<()> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaProfile {
    pub data: String,
}

impl JavaProfile {
    pub fn new() -> Self {
        JavaProfile {
            data: String::new(),
        }
    }
}

impl Default for JavaProfile {
    fn default() -> Self {
        Self::new()
    }
}

impl GetData for JavaProfile {
    fn custom_raw_data_parser(
        &mut self,
        params: crate::visualizer::ReportParams,
    ) -> Result<Vec<ProcessedData>> {
        let processes_loc = params
            .data_dir
            .join(format!("{}-jps-map.json", params.run_name));
        let processes_json =
            fs::read_to_string(processes_loc.to_str().unwrap()).unwrap_or_default();
        let mut process_map: HashMap<String, Vec<String>> =
            serde_json::from_str(&processes_json).unwrap_or(HashMap::new());
        let process_list: Vec<String> = process_map.clone().into_keys().collect();

        for process in process_list {
            let mut fg_loc = params.report_dir.clone();
            fg_loc.push(format!(
                "data/js/{}-java-flamegraph-{}.html",
                params.run_name, process
            ));
            let mut html_loc = params.data_dir.clone();
            html_loc.push(format!(
                "{}-java-flamegraph-{}.html",
                params.run_name, process
            ));
            let html = fs::read_to_string(html_loc.to_str().unwrap())
                .unwrap_or(String::from("No data collected."));
            let mut fg_file = File::create(fg_loc.clone())?;
            write!(fg_file, "{}", html)?;

            process_map
                .entry(process)
                .and_modify(|v| v.push(html.len().to_string()));
        }

        let mut java_profile_data = JavaProfile::new();
        java_profile_data.data = serde_json::to_string(&process_map)?;
        let processed_data = vec![ProcessedData::JavaProfile(java_profile_data)];
        Ok(processed_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(&mut self, buffer: Vec<ProcessedData>, query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::JavaProfile(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "values" => Ok(values[0].data.to_string()),
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_java_profile() {
    let java_profile_raw: JavaProfileRaw = JavaProfileRaw::new();
    let file_name = JAVA_PROFILE_FILE_NAME.to_string();
    let mut dt = DataType::new(
        Data::JavaProfileRaw(java_profile_raw.clone()),
        file_name.clone(),
        false,
    );
    dt.is_profile_option();

    let java_profile = JavaProfile::new();
    let mut dv = DataVisualizer::new(
        ProcessedData::JavaProfile(java_profile),
        file_name.clone(),
        String::new(),
        String::new(),
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
        .add_visualizer(file_name.clone(), dv);
}
