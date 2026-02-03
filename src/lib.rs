#[macro_use]
extern crate lazy_static;

pub mod analytics;
pub mod completions;
pub mod computations;
pub mod data;
#[cfg(target_os = "linux")]
pub mod pmu;
#[cfg(target_os = "linux")]
pub mod record;
pub mod report;
pub mod utils;
pub mod visualizer;

use crate::analytics::{AnalyticalEngine, DataFindings};
use crate::data::aperf_runlog::AperfRunlog;
use crate::utils::get_data_name_from_type;
use crate::visualizer::DataVisualizer;
use anyhow::Result;
use chrono::prelude::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
#[cfg(target_os = "linux")]
use {
    crate::data::aperf_stats::AperfStat,
    data::TimeEnum,
    flate2::{write::GzEncoder, Compression},
    nix::poll::{poll, PollFd, PollFlags, PollTimeout},
    nix::sys::{
        signal,
        signalfd::{SfdFlags, SigSet, SignalFd},
    },
    std::os::unix::io::AsFd,
    std::{process, time},
    timerfd::{SetTimeFlags, TimerFd, TimerState},
};

pub const APERF_FILE_FORMAT: &str = "bin";

#[cfg(target_os = "windows")]
pub const APERF_TMP: &str = "C:\\Temp";

#[cfg(target_os = "macos")]
pub const APERF_TMP: &str = "/tmp";

#[cfg(target_os = "linux")]
pub const APERF_TMP: &str = "/tmp";

lazy_static! {
    pub static ref APERF_RUNLOG: &'static str = get_data_name_from_type::<AperfRunlog>();
}

#[derive(Error, Debug)]
pub enum PDError {
    #[error("Error initializing logger")]
    LoggerInitError,

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

    #[error("{} data is not available for run {}", .0, .1)]
    DataUnavailableError(String, String),

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

    #[error("Multiple runs with the same name: {0}")]
    DuplicateRunNames(String),

    #[error("The run {0:?} does not exist.")]
    RunNotFound(PathBuf),

    #[error("The report {0} already exists in current directory.")]
    ReportExists(String),

    #[error("The directory within the archive does not have the same name as the archive: {0}")]
    ArchiveDirectoryInvalidName(String),

    #[error("Invalid directory {0:?}")]
    InvalidDirectory(PathBuf),

    #[error("Invalid archive {0:?}")]
    InvalidArchive(PathBuf),

    #[error("Invalid verbose option")]
    InvalidVerboseOption,

    #[error("All processes collection error")]
    CollectorAllProcessError,

    #[error("Could not get the total number of online CPUs with sysconf")]
    CollectorPMUCPUError,

    #[error("Generating report from other reports. Name must be given.")]
    VisualizerReportFromReportNoNameError,

    #[error("File not found {}", .0)]
    VisualizerFileNotFound(String),

    #[error("Custom PMU config file not provided.")]
    PMUCustomFileNotFound,

    #[error("PMU config file is invalid.")]
    PMUFileInvalid,

    #[error("Run data not available")]
    InvalidRunData,

    #[error("Error getting Meminfo values for {}", .0)]
    VisualizerMeminfoValueGetError(String),

    #[error("Dependency error: {}", .0)]
    DependencyError(String),
}

#[macro_export]
macro_rules! noop {
    () => {};
}

#[cfg(target_os = "linux")]
#[allow(missing_docs)]
pub struct PerformanceData {
    pub collectors: HashMap<String, data::DataType>,
    pub init_params: InitParams,
    pub aperf_stats_path: PathBuf,
    pub aperf_stats_handle: Option<fs::File>,
}

#[cfg(target_os = "linux")]
impl PerformanceData {
    pub fn new(init_params: InitParams) -> Self {
        PerformanceData {
            collectors: HashMap::new(),
            init_params,
            aperf_stats_path: PathBuf::new(),
            aperf_stats_handle: None,
        }
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

        self.aperf_stats_path = PathBuf::from(self.init_params.dir_name.clone()).join(format!(
            "{}.{}",
            get_data_name_from_type::<AperfStat>(),
            APERF_FILE_FORMAT
        ));
        self.aperf_stats_handle = Some(
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(self.aperf_stats_path.clone())
                .expect("Could not create aperf-stats file"),
        );

        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.init_data_type(&self.init_params)?;
        }
        Ok(())
    }

    pub fn prepare_data_collectors(&mut self) -> Result<()> {
        let mut remove_entries: Vec<String> = Vec::new();

        for (name, datatype) in self.collectors.iter_mut() {
            if datatype.is_static {
                continue;
            }

            match datatype.prepare_data_collector() {
                Err(e) => {
                    if datatype.is_profile_option {
                        error!("{}", e.to_string());
                        error!("Aperf exiting...");
                        process::exit(1);
                    }
                    error!(
                        "Excluding {} from collection. Error msg: {}",
                        name,
                        e.to_string()
                    );
                    remove_entries.push(name.clone());
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
        let mut aperf_collect_data = AperfStat::new();
        let mut current = time::Instant::now();
        let end = current + time::Duration::from_secs(self.init_params.period);

        // TimerFd
        let mut tfd = TimerFd::new()?;
        tfd.set_state(
            TimerState::Periodic {
                current: time::Duration::from_secs(self.init_params.interval),
                interval: time::Duration::from_secs(self.init_params.interval),
            },
            SetTimeFlags::Default,
        );
        let timer_pollfd = PollFd::new(tfd.as_fd(), PollFlags::POLLIN);

        // SignalFd
        let mut mask = SigSet::empty();
        mask.add(signal::SIGINT);
        mask.add(signal::SIGTERM);
        mask.thread_block()?;
        let sfd = SignalFd::with_flags(&mask, SfdFlags::SFD_NONBLOCK)?;
        let signal_pollfd = PollFd::new(sfd.as_fd(), PollFlags::POLLIN);

        let mut poll_fds = [timer_pollfd, signal_pollfd];
        let mut datatype_signal = signal::SIGTERM;

        while current <= end {
            aperf_collect_data.time = TimeEnum::DateTime(Utc::now());
            aperf_collect_data.data = HashMap::new();
            if poll(&mut poll_fds, PollTimeout::NONE)? <= 0 {
                error!("Poll error.");
            }
            if let Some(ev) = poll_fds[0].revents() {
                if ev.contains(PollFlags::POLLIN) {
                    let ret = tfd.read();
                    if ret > 1 {
                        error!("Missed {} interval(s)", ret - 1);
                    }
                    debug!("Time elapsed: {:?}", start.elapsed());
                    current += time::Duration::from_secs(ret * self.init_params.interval);
                    for (name, datatype) in self.collectors.iter_mut() {
                        if datatype.is_static {
                            continue;
                        }

                        datatype.collector_params.elapsed_time = start.elapsed().as_secs();

                        aperf_collect_data.measure(
                            name.clone() + "-collect",
                            || -> Result<()> {
                                datatype.collect_data()?;
                                Ok(())
                            },
                        )?;
                        aperf_collect_data.measure(name.clone() + "-print", || -> Result<()> {
                            datatype.write_to_file()?;
                            Ok(())
                        })?;
                    }
                    let data_collection_time = time::Instant::now() - current;
                    aperf_collect_data
                        .data
                        .insert("aperf".to_string(), data_collection_time.as_micros() as u64);
                    debug!("Collection time: {:?}", data_collection_time);
                    bincode::serialize_into(
                        self.aperf_stats_handle.as_ref().unwrap(),
                        &aperf_collect_data,
                    )?;
                }
            }
            if let Some(ev) = poll_fds[1].revents() {
                if ev.contains(PollFlags::POLLIN) {
                    if let Ok(Some(siginfo)) = sfd.read_signal() {
                        if siginfo.ssi_signo == signal::SIGINT as u32 {
                            info!("Caught SIGINT. Exiting...");
                            datatype_signal = signal::SIGINT;
                        } else if siginfo.ssi_signo == signal::SIGTERM as u32 {
                            info!("Caught SIGTERM. Exiting...");
                        } else {
                            panic!("Caught an unknown signal: {}", siginfo.ssi_signo);
                        }
                        break;
                    }
                }
            }
        }
        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.set_signal(datatype_signal);
            datatype.finish_data_collection()?;
        }
        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.after_data_collection()?;
        }
        tfd.set_state(TimerState::Disarmed, SetTimeFlags::Default);
        Ok(())
    }

    pub fn end(&mut self) -> Result<()> {
        let dst_path = PathBuf::from(&self.init_params.dir_name).join(*APERF_RUNLOG);
        fs::copy(&self.init_params.runlog, dst_path)?;

        // All activities in the record folder should be complete before this.
        self.create_data_archive()?;
        Ok(())
    }

    pub fn create_data_archive(&mut self) -> Result<()> {
        let dir_name = Path::new(&self.init_params.dir_name).file_name().unwrap();
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

#[cfg(target_os = "linux")]
impl Default for PerformanceData {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

pub fn get_file(dir: &PathBuf, name: String) -> Result<(PathBuf, fs::File)> {
    for path in fs::read_dir(dir.clone())? {
        let file_name = path?.file_name().into_string().unwrap();
        if file_name.starts_with(&name) {
            let file_path = dir.join(file_name.clone());
            let file = fs::OpenOptions::new()
                .read(true)
                .open(file_path.clone())
                .expect("Could not open file");
            // file_name = file_path.to_str().unwrap().to_string();
            return Ok((file_path, file));
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

#[derive(Default)]
pub struct VisualizationData {
    pub visualizers: HashMap<String, DataVisualizer>,
}

impl VisualizationData {
    pub fn new() -> Self {
        VisualizationData {
            visualizers: HashMap::new(),
        }
    }

    pub fn init_visualizers(
        &mut self,
        run_data_dir: PathBuf,
        tmp_dir: &Path,
        report_dir: &Path,
    ) -> Result<String> {
        let run_name = data::utils::no_tar_gz_file_name(&run_data_dir).unwrap();
        let visualizers_len = self.visualizers.len();
        let mut error_count = 0;

        for data_visualizer in self.visualizers.values_mut() {
            if let Err(e) = data_visualizer.init_visualizer(
                run_data_dir.clone(),
                run_name.clone(),
                tmp_dir,
                report_dir,
            ) {
                debug!("{:#?}", e);
                error_count += 1;
            }
        }

        /* Works if a new type of visualizer is introduced but data not present */
        if error_count == visualizers_len {
            return Err(PDError::InvalidRunData.into());
        }
        Ok(run_name.clone())
    }

    pub fn add_visualizer(&mut self, data_visualizer: DataVisualizer) {
        self.visualizers
            .insert(data_visualizer.data_name.to_string(), data_visualizer);
    }

    pub fn process_raw_data(&mut self, name: String) -> Result<()> {
        for data_visualizer in self.visualizers.values_mut() {
            if let Err(e) = data_visualizer.process_raw_data(name.clone()) {
                error!(
                    "Error while processing {} raw data: {:#?}",
                    data_visualizer.data_name, e
                );
            }
        }
        Ok(())
    }

    pub fn run_analytics(&mut self) -> HashMap<String, DataFindings> {
        let mut analytical_engine = AnalyticalEngine::default();
        for (data_name, data_visualizer) in &self.visualizers {
            analytical_engine.add_data_rules(
                data_name.clone(),
                data_visualizer.data.get_analytical_rules(),
            );
            analytical_engine
                .add_processed_data(data_name.clone(), &data_visualizer.processed_data);
        }

        info!("Running analytical rules");
        analytical_engine.run();

        analytical_engine.findings
    }

    pub fn is_data_available(&self, run_name: &String, visualizer_name: &str) -> bool {
        self.visualizers
            .get(visualizer_name)
            .is_some_and(|visualizer| {
                visualizer
                    .data_available
                    .get(run_name)
                    .is_some_and(|&value| value)
            })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InitParams {
    pub time_now: DateTime<Utc>,
    pub time_str: String,
    pub dir_name: String,
    pub period: u64,
    pub profile: HashMap<String, String>,
    pub pmu_config: Option<PathBuf>,
    pub interval: u64,
    pub run_name: String,
    pub collector_version: String,
    pub commit_sha_short: String,
    pub tmp_dir: PathBuf,
    pub runlog: PathBuf,
    pub perf_frequency: u32,
    pub hotline_frequency: u32,
    pub num_to_report: u32,
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
            profile: HashMap::new(),
            pmu_config: Option::None,
            interval: 0,
            run_name,
            collector_version,
            commit_sha_short,
            tmp_dir: PathBuf::from(APERF_TMP),
            runlog: PathBuf::new(),
            perf_frequency: 99,
            hotline_frequency: 1000,
            num_to_report: 5000,
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
    #[cfg(target_os = "linux")]
    use {
        super::{InitParams, PerformanceData, APERF_FILE_FORMAT},
        std::fs,
        std::path::Path,
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_performance_data_new() {
        let pd: PerformanceData = Default::default();

        let dir_name = format!(
            "./aperf_{}",
            pd.init_params.time_now.format("%Y-%m-%d_%H_%M_%S")
        );
        assert!(pd.collectors.is_empty());
        assert_eq!(pd.init_params.dir_name, dir_name);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_performance_data_dir_creation() {
        let mut params = InitParams::new("".to_string());
        params.dir_name = format!("./performance_data_dir_creation_{}", params.time_str);

        let mut pd = PerformanceData::new(params.clone());
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
