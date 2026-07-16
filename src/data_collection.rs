use crate::data::TimeEnum;
use crate::PDError;
use crate::{APERF_TMP, GROUPED_PMU_MODE};
use anyhow::Result;
use chrono::prelude::*;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
#[cfg(target_os = "linux")]
use {
    crate::data::processes::ProcessesRaw,
    crate::data::Data,
    crate::{aperf_runlog_file_path, data_file_path, get_data_name_from_type},
    crate::{aperf_stats_add, aperf_stats_measure, aperf_stats_proceed_to_next_stats},
    nix::poll::{poll, PollFd, PollFlags, PollTimeout},
    nix::sys::{
        signal,
        signalfd::{SfdFlags, SigSet, SignalFd},
    },
    std::fs::{File, OpenOptions},
    std::os::unix::io::AsFd,
    std::time,
    timerfd::{SetTimeFlags, TimerFd, TimerState},
};

#[cfg(target_os = "linux")]
pub struct DataCollectionEngine {
    init_params: InitParams,
    data_collectors: HashMap<String, DataCollector>,
}

#[cfg(target_os = "linux")]
impl DataCollectionEngine {
    pub fn new(init_params: InitParams) -> Self {
        DataCollectionEngine {
            init_params,
            data_collectors: HashMap::new(),
        }
    }

    pub fn add_data_collector(&mut self, data_name: &'static str, data: Data) {
        // Ignore dummy data type.
        if matches!(data, Data::FlamegraphRaw(_)) {
            return;
        }

        self.data_collectors.insert(
            data_name.to_string(),
            DataCollector::new(data_name, data, &self.init_params.run_data_dir),
        );
    }

    pub fn prepare_data_collectors(&mut self) -> Result<()> {
        let mut remove_entries: Vec<String> = Vec::new();

        // Prepare non-profile collectors first (e.g. perf_stat which can take significant
        // time on large machines), then profile collectors that launch subprocesses
        // (perf_profile, java_profile) so they start as close to collect_data_serial as
        // possible and stay in sync with the collection period.
        for is_profile_pass in [false, true] {
            for (data_name, data_collector) in self.data_collectors.iter_mut() {
                if data_collector.is_static() || data_collector.is_profile() != is_profile_pass {
                    continue;
                }

                // We cannot compute the exact end time since the prepare time of each
                // data varies, so re-compute it after every data preparation.
                self.init_params.expected_end_time =
                    Instant::now() + time::Duration::from_secs(self.init_params.period);

                if let Err(e) = data_collector.prepare_data_collector(&self.init_params) {
                    if data_collector.is_profile() {
                        panic!("{}", e.to_string());
                    }
                    let msg = format!(
                        "Excluding {} from collection, data preparation failed: {:?}",
                        data_name, e
                    );
                    if matches!(
                        e.downcast_ref::<PDError>(),
                        Some(PDError::IgnoredDataPreparationError(_))
                    ) {
                        debug!("{}", msg);
                    } else {
                        error!("{}", msg);
                    }
                    remove_entries.push(data_name.clone());
                }
            }
        }

        for key in remove_entries {
            self.data_collectors.remove_entry(&key);
        }

        Ok(())
    }

    pub fn collect_static_data(&mut self) -> Result<()> {
        for data_collector in self.data_collectors.values_mut() {
            if !data_collector.is_static() {
                continue;
            }
            data_collector.collect_data(&self.init_params)?;
            data_collector.write_to_file()?;
        }

        Ok(())
    }

    pub fn collect_data_serial(&mut self) -> Result<()> {
        let start_time = time::Instant::now();
        let collection_start_time = TimeEnum::DateTime(Utc::now());
        self.init_params.collection_start = Some(collection_start_time);
        aperf_stats_proceed_to_next_stats(collection_start_time);
        let end_time = start_time + time::Duration::from_secs(self.init_params.period);
        self.init_params.expected_end_time = end_time;

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
        let mut end_signal = String::new();

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

                    for data_collector in self.data_collectors.values_mut() {
                        if data_collector.is_static() {
                            continue;
                        }
                        data_collector.collect_data(&self.init_params)?;
                        data_collector.write_to_file()?;
                    }
                    let cur_collection_end = time::Instant::now();

                    let cur_collection_time = cur_collection_end - cur_collection_start;
                    aperf_stats_add(
                        "aperf-collect".to_string(),
                        cur_collection_time.as_micros() as u64,
                    );
                    debug!("Collection time: {:?}", cur_collection_time);

                    current_time = cur_collection_end;
                }
            }

            if let Some(ev) = poll_fds[1].revents() {
                if ev.contains(PollFlags::POLLIN) {
                    if let Ok(Some(siginfo)) = sfd.read_signal() {
                        if siginfo.ssi_signo == signal::SIGINT as u32 {
                            info!("Caught SIGINT. Exiting...");
                            end_signal = signal::SIGINT.to_string();
                        } else if siginfo.ssi_signo == signal::SIGTERM as u32 {
                            end_signal = signal::SIGTERM.to_string();
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
        self.init_params.end_signal = end_signal;

        tfd.set_state(TimerState::Disarmed, SetTimeFlags::Default);

        Ok(())
    }

    pub fn finish_data_collection(&mut self) -> Result<()> {
        for data_collector in self.data_collectors.values_mut() {
            data_collector.finish_data_collection(&self.init_params)?;
        }

        fs::copy(
            &self.init_params.runlog,
            aperf_runlog_file_path(&self.init_params.run_data_dir),
        )?;

        // Persist meta_data once, at the end of the recording. This captures the final
        // InitParams that can be read during report generation.
        if let Err(e) = self.init_params.save_to_json() {
            error!("Failed to save run metadata: {e}");
        }

        // Conduct another round of processes data collection to collect APerf performance
        // data during the finish stage.
        if let Some(processes_data_collector) = self
            .data_collectors
            .get_mut(get_data_name_from_type::<ProcessesRaw>())
        {
            processes_data_collector.collect_data(&self.init_params)?;
            processes_data_collector.write_to_file()?;
        };

        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub struct DataCollector {
    pub data_name: &'static str,
    pub data: Data,
    pub data_file: File,
}

#[cfg(target_os = "linux")]
impl DataCollector {
    pub fn new(data_name: &'static str, data: Data, run_data_dir: &PathBuf) -> Self {
        let data_file_path = data_file_path(data_name, run_data_dir);
        let data_file = match OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open(&data_file_path)
        {
            Ok(data_file) => data_file,
            Err(e) => panic!(
                "Failed to create data file at {}: {:?}",
                data_file_path.display(),
                e
            ),
        };

        DataCollector {
            data_name,
            data,
            data_file,
        }
    }

    pub fn is_static(&self) -> bool {
        self.data.is_static()
    }

    pub fn is_profile(&self) -> bool {
        self.data.is_profile()
    }

    pub fn prepare_data_collector(&mut self, init_params: &InitParams) -> Result<()> {
        aperf_stats_measure(format!("{}-prepare", self.data_name), || -> Result<()> {
            self.data.prepare_data_collector(init_params)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn collect_data(&mut self, init_params: &InitParams) -> Result<()> {
        let aperf_stat_name = format!(
            "{}-{}collect",
            self.data_name,
            if self.is_static() { "static_" } else { "" }
        );
        aperf_stats_measure(aperf_stat_name, || -> Result<()> {
            self.data.collect_data(init_params)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn write_to_file(&mut self) -> Result<()> {
        let aperf_stat_name = format!(
            "{}-{}write",
            self.data_name,
            if self.is_static() { "static_" } else { "" }
        );
        aperf_stats_measure(aperf_stat_name, || -> Result<()> {
            bincode::serialize_into(&mut self.data_file, &self.data)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn finish_data_collection(&mut self, init_params: &InitParams) -> Result<()> {
        aperf_stats_measure(format!("{}-finish", self.data_name), || -> Result<()> {
            self.data.finish_data_collection(init_params)?;
            Ok(())
        })?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct InitParams {
    pub run_name: String,
    pub run_data_dir: PathBuf,
    pub period: u64,
    pub profile: HashMap<String, String>,
    pub pmu_config: Option<PathBuf>,
    /// Whether the collection of PMU counters is "grouped" or
    /// "ungrouped". An empty string means a legacy run before
    /// PMU config revamp.
    #[serde(default)]
    pub pmu_counter_mode: String,
    pub interval: u64,
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
    /// The PIDs of all processes launched during data collection.
    pub sub_process_pids: HashSet<u32>,
    /// The signal that ends the collection. An empty string means the collection
    /// followed the specified period and ended naturally.
    pub end_signal: String,
    /// The expected end time of the collection, accessed and used by certain data
    /// types to compute the duration of launched external tools.
    #[serde(skip)]
    pub expected_end_time: Instant,
}

impl InitParams {
    pub fn new(run_name: String, run_data_dir: PathBuf) -> Self {
        InitParams {
            run_name,
            run_data_dir,
            period: 0,
            profile: HashMap::new(),
            pmu_config: Option::None,
            pmu_counter_mode: GROUPED_PMU_MODE.to_string(),
            interval: 0,
            collector_version: env!("CARGO_PKG_VERSION").to_string(),
            collector_commit_sha: env!("VERGEN_GIT_SHA").to_string(),
            tmp_dir: PathBuf::from(APERF_TMP),
            runlog: PathBuf::new(),
            perf_frequency: 99,
            save_profile_events: false,
            hotline_frequency: 1000,
            num_to_report: 5000,
            collection_start: None,
            collection_end: None,
            pid: Some(std::process::id()),
            sub_process_pids: HashSet::new(),
            end_signal: String::new(),
            expected_end_time: Instant::now(),
        }
    }

    pub fn save_to_json(&self) -> Result<()> {
        fs::write(
            self.run_data_dir.join(Self::json_file_name()),
            serde_json::to_string(self)?,
        )?;

        Ok(())
    }

    pub fn from_json(run_data_dir: &PathBuf) -> Result<Self> {
        let json_str = fs::read_to_string(run_data_dir.join(Self::json_file_name()))?;

        Ok(serde_json::from_str::<Self>(&json_str)?)
    }

    fn json_file_name() -> &'static str {
        "metadata.json"
    }
}

impl Default for InitParams {
    fn default() -> Self {
        Self::new("".to_string(), PathBuf::from(""))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::{DataCollectionEngine, DataCollector, InitParams},
        crate::data::cpu_utilization::CpuUtilizationRaw,
        crate::data::Data,
        crate::data_file_path,
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_data_collection_engine_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut params = InitParams::default();
        params.run_data_dir = temp_dir.path().to_path_buf();
        let run_data_dir = params.run_data_dir.clone();

        let data_collection_engine = DataCollectionEngine::new(params);

        assert!(data_collection_engine.data_collectors.is_empty());
        assert_eq!(
            data_collection_engine.init_params.run_data_dir,
            run_data_dir
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_data_collector_init() {
        let temp_dir = tempfile::tempdir().unwrap();
        let run_data_dir = temp_dir.path().to_path_buf();

        // Constructing a DataCollector creates and opens the data file.
        let data = CpuUtilizationRaw::new();
        let _dc = DataCollector::new(
            "cpu_utilization",
            Data::CpuUtilizationRaw(data),
            &run_data_dir,
        );

        let expected_path = data_file_path("cpu_utilization", &run_data_dir);
        assert!(expected_path.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_write() {
        let temp_dir = tempfile::tempdir().unwrap();
        let run_data_dir = temp_dir.path().to_path_buf();

        let data = CpuUtilizationRaw::new();
        let mut dc = DataCollector::new(
            "cpu_utilization",
            Data::CpuUtilizationRaw(data),
            &run_data_dir,
        );

        let data_file_path = data_file_path("cpu_utilization", &run_data_dir);
        assert!(data_file_path.exists());

        dc.write_to_file().unwrap();

        // Re-open the file to read back what was serialized (the collector's own handle is in
        // append mode).
        let read_handle = std::fs::File::open(&data_file_path).unwrap();
        loop {
            match bincode::deserialize_from::<_, Data>(&read_handle) {
                Ok(v) => match v {
                    Data::CpuUtilizationRaw(ref value) => assert!(value.data.is_empty()),
                    _ => unreachable!(),
                },
                Err(e) => match *e {
                    bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break
                    }
                    _ => unreachable!(),
                },
            };
        }
    }
}
