#[macro_use]
extern crate lazy_static;

pub mod data;
pub mod record;
pub mod report;
pub mod visualizer;
use anyhow::Result;
use chrono::prelude::*;
use flate2::{write::GzEncoder, Compression};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::{self};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, time};
use thiserror::Error;
use timerfd::{SetTimeFlags, TimerFd, TimerState};

pub static APERF_FILE_FORMAT: &str = "bin";

#[derive(Error, Debug)]
pub enum PDError {
    #[error("Error getting JavaScript file for {}", .0)]
    VisualizerJSFileGetError(String),

    #[error("Error getting HashMap entry for {}", .0)]
    VisualizerHashMapEntryError(String),

    #[error("Error getting run values for {}", .0)]
    VisualizerRunValueGetError(String),

    #[error("Error getting Vmstat value for {}", .0)]
    VisualizerVmstatValueGetError(String),

    #[error("Error getting interrupt line count for CPU {}", .0)]
    VisualizerInterruptLineCPUCountError(String),

    #[error("Error getting Netstat value for {}", .0)]
    VisualizerNetstatValueGetError(String),

    #[error("Error getting Line Name Error")]
    CollectorLineNameError,

    #[error("Error getting Line Value Error")]
    CollectorLineValueError,

    #[error("Error getting value from Option")]
    ProcessorOptionExtractError,

    #[error("Unsupported CPU")]
    CollectorPerfUnsupportedCPU,

    #[error("Unsupported API")]
    VisualizerUnsupportedAPI,

    #[error("Visualizer Init error")]
    VisualizerInitError,

    #[error("Not an archive or directory")]
    RecordNotArchiveOrDirectory,

    #[error("Tar.gz file name and archived directory name inside mismatch")]
    ArchiveDirectoryMismatch,

    #[error("Invalid tar.gz file name")]
    InvalidArchiveName,

    #[error("Invalid verbose option")]
    InvalidVerboseOption,

    #[error("All processes collection error")]
    CollectorAllProcessError,

    #[error("File not found {}", .0)]
    VisualizerFileNotFound(String),

    #[error("Run data not available")]
    InvalidRunData,

    #[error("Error getting Meminfo values for {}", .0)]
    VisualizerMeminfoValueGetError(String),
}

lazy_static! {
    pub static ref PERFORMANCE_DATA: Mutex<PerformanceData> = Mutex::new(PerformanceData::new());
}

#[allow(missing_docs)]
pub struct PerformanceData {
    pub collectors: HashMap<String, data::DataType>,
    pub init_params: InitParams,
}

impl PerformanceData {
    pub fn new() -> Self {
        let collectors = HashMap::new();
        let init_params = InitParams::new("".to_string());

        PerformanceData {
            collectors,
            init_params,
        }
    }

    pub fn set_params(&mut self, params: InitParams) {
        self.init_params = params;
    }

    pub fn add_datatype(&mut self, name: String, dt: data::DataType) {
        self.collectors.insert(name, dt);
    }

    pub fn init_collectors(&mut self) -> Result<()> {
        fs::create_dir(self.init_params.dir_name.clone())?;

        /*
         * Create a meta_data file to hold the InitParams that was used by the collector.
         * This will help when we visualize the data and we don't have to guess these values.
         */
        let meta_data_path = format!(
            "{}/meta_data.{}",
            self.init_params.dir_name.clone(),
            APERF_FILE_FORMAT
        );
        let meta_data_handle = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(meta_data_path.clone())
            .expect("Could not create meta-data file");

        bincode::serialize_into(meta_data_handle, &self.init_params)?;

        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.init_data_type(self.init_params.clone())?;
        }
        Ok(())
    }

    pub fn prepare_data_collectors(&mut self) -> Result<()> {
        let mut remove_entries: Vec<String> = Vec::new();
        if !self.init_params.profile {
            self.collectors
                .remove(data::perf_profile::PERF_PROFILE_FILE_NAME)
                .unwrap();
            self.collectors
                .remove(data::flamegraphs::FLAMEGRAPHS_FILE_NAME)
                .unwrap();
        }

        for (_name, datatype) in self.collectors.iter_mut() {
            if datatype.is_static {
                continue;
            }
            match datatype.prepare_data_collector() {
                Err(e) => {
                    error!(
                        "Excluding {} from collection. Error msg: {}",
                        _name,
                        e.to_string()
                    );
                    remove_entries.push(_name.clone());
                }
                _ => continue,
            }
        }
        for key in remove_entries {
            self.collectors.remove_entry(&key);
        }

        Ok(())
    }

    pub fn collect_static_data(&mut self) -> Result<()> {
        for (_name, datatype) in self.collectors.iter_mut() {
            if !datatype.is_static {
                continue;
            }
            datatype.collect_data()?;
            datatype.write_to_file()?;
        }

        Ok(())
    }

    pub fn collect_data_serial(&mut self) -> Result<()> {
        let start = time::Instant::now();
        let mut current = time::Instant::now();
        let end = current + time::Duration::from_secs(self.init_params.period);

        let mut tfd = TimerFd::new().unwrap();
        tfd.set_state(
            TimerState::Periodic {
                current: time::Duration::from_secs(self.init_params.interval),
                interval: time::Duration::from_secs(self.init_params.interval),
            },
            SetTimeFlags::Default,
        );
        while current <= end {
            let ret = tfd.read();
            if ret > 1 {
                error!("Missed {} interval(s)", ret - 1);
            }
            debug!("Time elapsed: {:?}", start.elapsed());
            current += time::Duration::from_secs(ret * self.init_params.interval);
            for (_name, datatype) in self.collectors.iter_mut() {
                if datatype.is_static {
                    continue;
                }
                datatype.collect_data()?;
                datatype.write_to_file()?;
            }
            let data_collection_time = time::Instant::now() - current;
            debug!("Collection time: {:?}", data_collection_time);
        }
        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.finish_data_collection()?;
        }
        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.after_data_collection()?;
        }
        tfd.set_state(TimerState::Disarmed, SetTimeFlags::Default);
        self.create_data_archive()?;
        Ok(())
    }

    pub fn create_data_archive(&mut self) -> Result<()> {
        let dir_name = Path::new(&self.init_params.dir_name).file_stem().unwrap();
        let archive_path = format!("{}.tar.gz", self.init_params.dir_name);
        let tar_gz = fs::File::create(&archive_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(dir_name, &self.init_params.dir_name)?;
        info!(
            "Data collected in {}/, archived in {}",
            self.init_params.dir_name, archive_path
        );
        Ok(())
    }
}

impl Default for PerformanceData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_file(dir: String, name: String) -> Result<fs::File> {
    for path in fs::read_dir(dir.clone())? {
        let mut file_name = path?.file_name().into_string().unwrap();
        if file_name.contains(&name) {
            let file_path = Path::new(&dir).join(file_name.clone());
            file_name = file_path.to_str().unwrap().to_string();
            return Ok(fs::OpenOptions::new()
                .read(true)
                .open(file_name)
                .expect("Could not open file"));
        }
    }
    Err(PDError::VisualizerFileNotFound(name).into())
}

pub fn get_file_name(dir: String, name: String) -> Result<String> {
    for path in fs::read_dir(dir.clone())? {
        let file_name = path?.file_name().into_string().unwrap();
        if file_name.contains(&name) {
            return Ok(file_name);
        }
    }
    Err(PDError::VisualizerFileNotFound(name).into())
}

lazy_static! {
    pub static ref VISUALIZATION_DATA: Mutex<VisualizationData> =
        Mutex::new(VisualizationData::new());
}

#[derive(Default)]
pub struct VisualizationData {
    pub visualizers: HashMap<String, visualizer::DataVisualizer>,
    pub js_files: HashMap<String, String>,
    pub run_names: Vec<String>,
}

impl VisualizationData {
    pub fn new() -> Self {
        VisualizationData {
            visualizers: HashMap::new(),
            js_files: HashMap::new(),
            run_names: Vec::new(),
        }
    }

    pub fn init_visualizers(
        &mut self,
        dir: String,
        tmp_dir: String,
        fin_dir: PathBuf,
    ) -> Result<String> {
        let dir_path = Path::new(&dir);
        let dir_name = dir_path.file_stem().unwrap().to_str().unwrap().to_string();
        self.run_names.push(dir_name.clone());
        let visualizers_len = self.visualizers.len();
        let mut error_count = 0;

        for (_name, visualizer) in self.visualizers.iter_mut() {
            if let Err(e) = visualizer.init_visualizer(
                dir.clone(),
                dir_name.clone(),
                tmp_dir.clone(),
                fin_dir.clone(),
            ) {
                error!("{:#?}", e);
                visualizer.data_not_available(dir_name.clone())?;
                error_count += 1;
            }
        }

        /* Works if a new type of visualizer is introduced but data not present */
        if error_count == visualizers_len {
            return Err(PDError::InvalidRunData.into());
        }
        Ok(dir_name.clone())
    }

    pub fn add_visualizer(&mut self, name: String, dv: visualizer::DataVisualizer) {
        self.js_files.insert(dv.js_file_name.clone(), dv.js.clone());
        self.visualizers.insert(name, dv);
    }

    pub fn get_all_js_files(&mut self) -> Result<Vec<(String, String)>> {
        let mut ret = Vec::new();
        for (name, visualizer) in self.visualizers.iter() {
            let file = self
                .js_files
                .get(&visualizer.js_file_name)
                .ok_or(PDError::VisualizerJSFileGetError(name.to_string()))?;
            ret.push((visualizer.js_file_name.clone(), file.clone()));
        }
        Ok(ret)
    }

    pub fn get_js_file(&mut self, name: String) -> Result<&str> {
        let file = self
            .js_files
            .get_mut(&name)
            .ok_or(PDError::VisualizerJSFileGetError(name.to_string()))?;
        Ok(file)
    }

    pub fn unpack_data(&mut self, name: String) -> Result<()> {
        for (dvname, datavisualizer) in self.visualizers.iter_mut() {
            debug!("Unpacking data for: {}", dvname);
            datavisualizer.process_raw_data(name.clone())?;
        }
        Ok(())
    }

    pub fn get_api(&mut self, name: String) -> Result<String> {
        let api = self.visualizers.get(&name).unwrap().api_name.clone();
        Ok(api)
    }

    pub fn get_visualizer_names(&mut self) -> Result<Vec<String>> {
        let mut visualizer_names = Vec::new();
        for (name, _visualizer) in self.visualizers.iter() {
            visualizer_names.push(name.clone());
        }
        Ok(visualizer_names)
    }

    pub fn get_run_names(&mut self) -> Result<String> {
        Ok(serde_json::to_string(&self.run_names)?)
    }

    pub fn get_data(
        &mut self,
        run_name: &String,
        visualizer_name: &str,
        query: String,
    ) -> Result<String> {
        let visualizer = self.visualizers.get_mut(visualizer_name).ok_or(
            PDError::VisualizerHashMapEntryError(visualizer_name.to_string()),
        )?;
        visualizer.get_data(run_name.to_string(), query.clone())
    }

    pub fn get_calls(&mut self, name: String) -> Result<Vec<String>> {
        let visualizer = self.visualizers.get_mut(&name).unwrap();
        visualizer.get_calls()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InitParams {
    pub time_now: DateTime<Utc>,
    pub time_str: String,
    pub dir_name: String,
    pub period: u64,
    pub profile: bool,
    pub interval: u64,
    pub run_name: String,
    pub collector_version: String,
    pub commit_sha_short: String,
}

impl InitParams {
    pub fn new(dir: String) -> Self {
        let time_now = Utc::now();
        let time_str = time_now.format("%Y-%m-%d_%H_%M_%S").to_string();
        let mut dir_name = format!("./aperf_{}", time_str);
        let mut run_name = String::new();
        if !dir.is_empty() {
            dir_name = Path::new(&dir)
                .components()
                .as_path()
                .to_str()
                .unwrap()
                .to_string();
            run_name = dir;
        } else {
            let path = Path::new(&dir_name);
            info!(
                "No run-name given. Using {}",
                path.file_stem().unwrap().to_str().unwrap()
            );
        }
        let collector_version = env!("CARGO_PKG_VERSION").to_string();
        let commit_sha_short = env!("VERGEN_GIT_SHA").to_string();

        InitParams {
            time_now,
            time_str,
            dir_name,
            period: 0,
            profile: false,
            interval: 0,
            run_name,
            collector_version,
            commit_sha_short,
        }
    }
}

impl Default for InitParams {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{InitParams, PerformanceData, APERF_FILE_FORMAT};
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_performance_data_new() {
        let pd = PerformanceData::new();

        let dir_name = format!(
            "./aperf_{}",
            pd.init_params.time_now.format("%Y-%m-%d_%H_%M_%S")
        );
        assert!(pd.collectors.is_empty());
        assert!(pd.init_params.dir_name == dir_name);
    }

    #[test]
    fn test_performance_data_dir_creation() {
        let mut params = InitParams::new("".to_string());
        params.dir_name = format!("./performance_data_dir_creation_{}", params.time_str);

        let mut pd = PerformanceData::new();
        pd.set_params(params.clone());
        pd.init_collectors().unwrap();
        assert!(Path::new(&pd.init_params.dir_name).exists());
        let full_path = format!(
            "{}/meta_data.{}",
            params.dir_name.clone(),
            APERF_FILE_FORMAT
        );
        assert!(Path::new(&full_path).exists());
        fs::remove_dir_all(pd.init_params.dir_name).unwrap();
    }
}
