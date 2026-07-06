#[macro_use]
extern crate lazy_static;

pub mod analytics;
pub mod completions;
pub mod computations;
pub mod data;
#[cfg(target_os = "linux")]
pub mod pmu;
pub mod profiling;
#[cfg(target_os = "linux")]
pub mod record;
pub mod report;
#[cfg(feature = "mcp-server")]
pub mod server;
pub mod visualizer;

use crate::analytics::{AnalyticalEngine, DataFindings};
use crate::computations::Statistics;
use crate::data::aperf_runlog::AperfRunlog;
use crate::data::aperf_stats::AperfStat;
use crate::data::common::data_formats::{AperfData, TimeSeriesMetric};
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use crate::data::processes::Processes;
use crate::data::TimeEnum;
use crate::visualizer::{DataVisualizer, ReportParams};
use anyhow::{Context, Result};
use chrono::prelude::*;
use data::common::utils::get_data_name_from_type;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
#[cfg(target_os = "linux")]
use {
    crate::data::Data,
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
    #[error("Error getting Line Name Error")]
    CollectorLineNameError,

    #[error("Error getting Line Value Error")]
    CollectorLineValueError,

    #[error("The run {0:?} does not exist.")]
    RunNotFound(PathBuf),

    #[error("The run {0:?} was specified more than once.")]
    DuplicateRunPath(PathBuf),

    #[error("The report {0} already exists in current directory.")]
    ReportExists(String),

    #[error("Invalid directory {0:?}")]
    InvalidDirectory(PathBuf),

    #[error("Invalid archive {0:?}")]
    InvalidArchive(PathBuf),

    #[error("Invalid verbose option")]
    InvalidVerboseOption,

    #[error("Invalid time-range option: {}", .0)]
    InvalidRunTimeRangeOption(String),

    #[error("Failed to detect network interfaces: {}", .0)]
    NetworkInterfaceDetectionFailure(String),

    #[error("Failed to create ioctl socket for ethtool stats collection")]
    EthToolSocketCreationFailure,

    #[error("Run data not available")]
    InvalidRunData,

    #[error("Dependency error: {}", .0)]
    DependencyError(String),

    #[error("Ignored data preparation: {}", .0)]
    IgnoredDataPreparationError(String),
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
        // Ignore dummy data type.
        if matches!(dt.data, Data::FlamegraphRaw(_)) {
            return;
        }
        self.collectors.insert(name, dt);
    }

    pub fn init_collectors(&mut self) -> Result<()> {
        fs::create_dir(self.init_params.dir_name.clone())?;

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

        // Prepare non-profile collectors first (e.g. perf_stat which can take significant
        // time on large machines), then profile collectors that launch subprocesses
        // (perf_profile, java_profile) so they start as close to collect_data_serial as
        // possible and stay in sync with the collection period.
        for is_profile_pass in [false, true] {
            for (name, datatype) in self.collectors.iter_mut() {
                if datatype.is_static || datatype.is_profile_option != is_profile_pass {
                    continue;
                }

                match datatype.prepare_data_collector() {
                    Err(e) => {
                        if datatype.is_profile_option {
                            error!("{}", e.to_string());
                            error!("Aperf exiting...");
                            process::exit(1);
                        }
                        let msg = format!(
                            "Excluding {} from collection, data preparation failed: {:?}",
                            name, e
                        );
                        if matches!(
                            e.downcast_ref::<PDError>(),
                            Some(PDError::IgnoredDataPreparationError(_))
                        ) {
                            debug!("{}", msg);
                        } else {
                            error!("{}", msg);
                        }
                        remove_entries.push(name.clone());
                    }
                    _ => continue,
                }
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
        let start_time = time::Instant::now();
        self.init_params.collection_start = Some(TimeEnum::DateTime(Utc::now()));
        let mut aperf_stat = AperfStat::new();
        let end_time = start_time + time::Duration::from_secs(self.init_params.period);

        // TimerFd
        let mut tfd = TimerFd::new()?;
        tfd.set_state(
            TimerState::Periodic {
                current: time::Duration::from_nanos(1),
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
        let mut data_type_signal = signal::SIGTERM;

        let mut current_time = start_time;

        while current_time <= end_time {
            if poll(&mut poll_fds, PollTimeout::NONE)? <= 0 {
                error!("Failed to poll timer or signal fds");
            }

            if let Some(ev) = poll_fds[0].revents() {
                if ev.contains(PollFlags::POLLIN) {
                    let ret = tfd.read();
                    if ret > 1 {
                        error!("Missed {} interval(s)", ret - 1);
                    }
                    debug!("Time elapsed: {:?}", start_time.elapsed());

                    let cur_collection_start = time::Instant::now();
                    aperf_stat.time = TimeEnum::DateTime(Utc::now());
                    aperf_stat.data = HashMap::new();

                    for (name, data_type) in self.collectors.iter_mut() {
                        if data_type.is_static {
                            continue;
                        }

                        data_type.collector_params.elapsed_time = start_time.elapsed().as_secs();

                        aperf_stat.measure(name.clone() + "-collect", || -> Result<()> {
                            data_type.collect_data()?;
                            Ok(())
                        })?;
                        aperf_stat.measure(name.clone() + "-print", || -> Result<()> {
                            data_type.write_to_file()?;
                            Ok(())
                        })?;
                    }
                    let cur_collection_end = time::Instant::now();

                    let cur_collection_time = cur_collection_end - cur_collection_start;
                    aperf_stat
                        .data
                        .insert("aperf".to_string(), cur_collection_time.as_micros() as u64);
                    debug!("Collection time: {:?}", cur_collection_time);

                    bincode::serialize_into(
                        self.aperf_stats_handle.as_ref().unwrap(),
                        &aperf_stat,
                    )?;

                    current_time = cur_collection_end;
                }
            }

            if let Some(ev) = poll_fds[1].revents() {
                if ev.contains(PollFlags::POLLIN) {
                    if let Ok(Some(siginfo)) = sfd.read_signal() {
                        if siginfo.ssi_signo == signal::SIGINT as u32 {
                            info!("Caught SIGINT. Exiting...");
                            data_type_signal = signal::SIGINT;
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

        self.init_params.collection_end = Some(TimeEnum::DateTime(Utc::now()));

        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.set_signal(data_type_signal);
            datatype.finish_data_collection()?;
        }

        tfd.set_state(TimerState::Disarmed, SetTimeFlags::Default);

        Ok(())
    }

    pub fn create_record_archive(&mut self) -> Result<()> {
        let dst_path = PathBuf::from(&self.init_params.dir_name).join(*APERF_RUNLOG);
        fs::copy(&self.init_params.runlog, dst_path)?;

        // Persist meta_data once, at the end of the recording. This captures the final
        // InitParams including collection_start/collection_end stamped by
        // collect_data_serial.
        let meta_data_path = PathBuf::from(&self.init_params.dir_name)
            .join(format!("meta_data.{}", APERF_FILE_FORMAT));
        let meta_data_handle = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&meta_data_path)?;
        bincode::serialize_into(meta_data_handle, &self.init_params)?;

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

#[derive(Default)]
pub struct VisualizationData {
    per_run_report_params: HashMap<String, ReportParams>,
    pub visualizers: HashMap<String, DataVisualizer>,
}

impl VisualizationData {
    pub fn new(per_run_report_params: HashMap<String, ReportParams>) -> Self {
        VisualizationData {
            per_run_report_params,
            visualizers: HashMap::new(),
        }
    }

    pub fn init_visualizers(&mut self, run_name: &str) -> Result<()> {
        let visualizers_len = self.visualizers.len();
        let mut error_count = 0;

        let cur_run_report_params =
            self.per_run_report_params
                .get_mut(run_name)
                .with_context(|| {
                    format!("Unrecognized run name {run_name} when initializing report data.")
                })?;

        for data_visualizer in self.visualizers.values_mut() {
            if let Err(e) = data_visualizer.init_visualizer(cur_run_report_params.clone()) {
                debug!("{:#?}", e);
                error_count += 1;
            }
        }

        /* Works if a new type of visualizer is introduced but data not present */
        if error_count == visualizers_len {
            return Err(PDError::InvalidRunData.into());
        }

        Ok(())
    }

    pub fn add_visualizer(&mut self, data_visualizer: DataVisualizer) {
        self.visualizers
            .insert(data_visualizer.data_name.to_string(), data_visualizer);
    }

    pub fn process_raw_data(&mut self) -> Result<()> {
        for data_visualizer in self.visualizers.values_mut() {
            if let Err(e) = data_visualizer.process_raw_data() {
                error!(
                    "Error while processing {} raw data: {:#?}",
                    data_visualizer.data_name, e
                );
            }
        }
        Ok(())
    }

    pub fn run_analytics(
        &mut self,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) -> HashMap<String, DataFindings> {
        let mut analytical_engine = AnalyticalEngine::default();
        for (data_name, data_visualizer) in &mut self.visualizers {
            analytical_engine.add_data_rules(
                data_name.clone(),
                data_visualizer.data.get_analytical_rules(),
            );
            analytical_engine
                .add_processed_data(data_name.clone(), &mut data_visualizer.processed_data);
        }

        info!("Running analytical rules");
        analytical_engine.run(processed_data_accessor);

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

    /// Logics to be run after all raw data has been processed.
    pub fn post_process_data(&mut self) {
        // Perform the inter-data post-processing logics first.
        copy_aperf_process_metrics_to_aperf_stats(
            &mut self.visualizers,
            &self.per_run_report_params,
        );

        // Perform the intra-data logics for each data type.
        for data_visualizer in self.visualizers.values_mut() {
            data_visualizer.post_process_data();
        }
    }
}

/// Extract the APerf process's metric from processes data and add them to the
/// aperf_stats data, to monitor APerf performance in the report.
fn copy_aperf_process_metrics_to_aperf_stats(
    visualizers: &mut HashMap<String, DataVisualizer>,
    per_run_report_params: &HashMap<String, ReportParams>,
) {
    // A map from a run name to the sorted list of APerf process metrics
    let mut per_run_aperf_process_metrics: HashMap<String, Vec<TimeSeriesMetric>> = HashMap::new();

    let processes_data_visualizer = match visualizers.get(get_data_name_from_type::<Processes>()) {
        Some(processes_data_visualizer) => processes_data_visualizer,
        None => return,
    };

    for (run_name, cur_run_data) in &processes_data_visualizer.processed_data.runs {
        let cur_run_pid = match per_run_report_params.get(run_name) {
            Some(report_params) => {
                if let Some(pid) = report_params.pid {
                    pid
                } else {
                    continue;
                }
            }
            None => continue,
        };

        let cur_run_processes_data = match cur_run_data {
            AperfData::TimeSeries(time_series_data) => time_series_data,
            _ => continue,
        };

        let aperf_series_name = format!("{cur_run_pid}_aperf");

        for metric_name in &cur_run_processes_data.sorted_metric_names {
            // For every processes metric, locate the corresponding series for the APerf process and
            // create a new dedicated metric for it.
            if let Some(metric) = cur_run_processes_data.metrics.get(metric_name) {
                if let Some(series) = metric
                    .series
                    .iter()
                    .find(|s| s.series_name == aperf_series_name)
                {
                    let aperf_process_metric_name = format!("process_{metric_name}");
                    let mut aperf_process_metric =
                        TimeSeriesMetric::new(aperf_process_metric_name.clone());
                    let stats = Statistics::from_values(&series.values);
                    aperf_process_metric.value_range =
                        (stats.min.floor() as u64, stats.max.ceil() as u64);
                    aperf_process_metric.stats = stats;
                    aperf_process_metric.series = vec![series.clone()];
                    per_run_aperf_process_metrics
                        .entry(run_name.clone())
                        .or_default()
                        .push(aperf_process_metric);
                }
            }
        }
    }

    if per_run_aperf_process_metrics.is_empty() {
        return;
    }

    let aperf_stat_data_visualizer =
        match visualizers.get_mut(get_data_name_from_type::<AperfStat>()) {
            Some(aperf_stats_visualizer) => aperf_stats_visualizer,
            None => return,
        };

    for (run_name, aperf_process_metrics) in per_run_aperf_process_metrics {
        if let Some(AperfData::TimeSeries(cur_run_aperf_stats_data)) = aperf_stat_data_visualizer
            .processed_data
            .runs
            .get_mut(&run_name)
        {
            // The APerf process metrics should be showing upfront
            let mut insert_pos = 0;
            for aperf_process_metric in aperf_process_metrics {
                cur_run_aperf_stats_data
                    .sorted_metric_names
                    .insert(insert_pos, aperf_process_metric.metric_name.clone());
                cur_run_aperf_stats_data.metrics.insert(
                    aperf_process_metric.metric_name.clone(),
                    aperf_process_metric,
                );
                insert_pos += 1;
            }
        }
    }
}

pub const GROUPED_PMU_MODE: &str = "grouped";
pub const UNGROUPED_PMU_MODE: &str = "ungrouped";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct InitParams {
    pub dir_name: String,
    pub period: u64,
    pub profile: HashMap<String, String>,
    pub pmu_config: Option<PathBuf>,
    /// Whether the collection of PMU counters is "grouped" or
    /// "ungrouped". An empty string means a legacy run before
    /// PMU config revamp.
    pub pmu_counter_mode: String,
    pub interval: u64,
    pub run_name: String,
    /// The version of APerf that performed the collection.
    pub collector_version: String,
    /// The short commit SHA of APerf that performed the collection.
    pub collector_commit_sha: String,
    pub tmp_dir: PathBuf,
    pub runlog: PathBuf,
    pub perf_frequency: u32,
    pub save_profile_events: bool,
    pub hotline_frequency: u32,
    pub num_to_report: u32,
    /// Wall-clock start of `collect_data_serial`. `None` for archives
    /// produced by versions of aperf that did not record this.
    #[serde(default)]
    pub collection_start: Option<TimeEnum>,
    /// Wall-clock end of `collect_data_serial`. `None` for archives
    /// produced by versions of aperf that did not record this.
    #[serde(default)]
    pub collection_end: Option<TimeEnum>,
    /// PID of the aperf process that performed the collection. `None`
    /// for archives produced by versions of aperf that did not record this.
    #[serde(default)]
    pub pid: Option<u32>,
}

impl InitParams {
    pub fn new(dir: String) -> Self {
        let mut dir_name = format!(
            "./aperf_{}",
            Utc::now().format("%Y-%m-%d_%H_%M_%S").to_string()
        );
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
        let collector_commit_sha = env!("VERGEN_GIT_SHA").to_string();

        InitParams {
            dir_name,
            period: 0,
            profile: HashMap::new(),
            pmu_config: Option::None,
            pmu_counter_mode: GROUPED_PMU_MODE.to_string(),
            interval: 0,
            run_name,
            collector_version,
            collector_commit_sha,
            tmp_dir: PathBuf::from(APERF_TMP),
            runlog: PathBuf::new(),
            perf_frequency: 99,
            save_profile_events: false,
            hotline_frequency: 1000,
            num_to_report: 5000,
            collection_start: None,
            collection_end: None,
            pid: Some(std::process::id()),
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
        super::{InitParams, PerformanceData},
        chrono::Utc,
        std::fs,
        std::path::Path,
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_performance_data_new() {
        let pd: PerformanceData = Default::default();

        let dir_name = format!(
            "./aperf_{}",
            Utc::now().format("%Y-%m-%d_%H_%M_%S").to_string()
        );
        assert!(pd.collectors.is_empty());
        assert_eq!(pd.init_params.dir_name, dir_name);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_performance_data_dir_creation() {
        let mut params = InitParams::new("".to_string());
        params.dir_name = format!(
            "./performance_data_dir_creation_{}",
            Utc::now().format("%Y-%m-%d_%H_%M_%S").to_string()
        );

        let mut pd = PerformanceData::new(params.clone());
        pd.init_collectors().unwrap();
        assert!(Path::new(&pd.init_params.dir_name).exists());
        fs::remove_dir_all(pd.init_params.dir_name).unwrap();
    }
}
