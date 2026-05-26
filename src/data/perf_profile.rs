use crate::data::common::data_formats::{AperfData, Profiler, TextData};
use crate::data::{Data, ProcessData};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    crate::profiling::perf::parser::build_perf_profiler_data,
    crate::PDError,
    chrono::Utc,
    log::{debug, error, warn},
    nix::{sys::signal, unistd, unistd::Pid},
    std::io::Write,
    std::process::{Command, Stdio},
    std::{process::Child, sync::Mutex},
};

pub const PERF_TOP_FUNCTIONS_FILE_NAME: &str = "top_functions";

#[cfg(target_os = "linux")]
lazy_static! {
    pub static ref PERF_CHILD: Mutex<Option<Child>> = Mutex::new(None);
    pub static ref PROFILE_START_TIME_MS: Mutex<i64> = Mutex::new(0);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfProfileRaw {
    pub data: String,
}

#[cfg(target_os = "linux")]
impl PerfProfileRaw {
    pub fn new() -> Self {
        PerfProfileRaw {
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for PerfProfileRaw {
    fn prepare_data_collector(&mut self, params: &CollectorParams) -> Result<()> {
        let is_root = unistd::geteuid().is_root();

        // Check kernel configs for the following perf command
        if !is_root {
            //  Check kernel.perf_event_paranoid
            // -1: Allow use of almost all events by all users (mmap perf_event_open for hotline)
            //  0: Allow CPU event Data
            //          Disallow ftrace function tracepoint by users without CAP_SYS_ADMIN
            //          Disallow raw tracepoint access by users without CAP_SYS_ADMIN
            //  1: Allow Kernel Profiling (perf will fail, but generate 0 byte perf.data)
            //          Disallow CPU event access by users without CAP_SYS_ADMIN
            //  2: Disallow everything (perf will fail, but generate 0 byte perf.data)
            //          Disallow kernel profiling by users without CAP_SYS_ADMIN
            let paranoid_value = fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")?
                .trim()
                .parse::<i32>()
                .unwrap_or(4);
            if paranoid_value > 0 {
                warn!("kernel.perf_event_paranoid is not <=0, which disallows access to CPU event data. Run `sudo sysctl -w kernel.perf_event_paranoid=-1`");
            }

            //  Check kernel.kptr_restrict
            //  0: the address is hashed before printing. (This is the equivalent to %p.)
            //  1: (symbols skewed if not root) kernel pointers replaced with 0's unless the user has CAP_SYSLOG.
            //  2: (symbols may be skewed, perf may fail even with root on older perf) kernel pointers replaced with 0's regardless of privileges.
            let kptr_value = fs::read_to_string("/proc/sys/kernel/kptr_restrict")?
                .trim()
                .parse::<i32>()
                .unwrap_or(-1);
            if kptr_value > 0 {
                warn!("kernel.kptr_restrict is not 0, which may result in missing kernel symbols. Run `sudo sysctl -w kernel.kptr_restrict=0`");
            }
        }

        *PROFILE_START_TIME_MS.lock().unwrap() = Utc::now().timestamp_millis();
        match Command::new("perf")
            .stdout(Stdio::null())
            .args([
                "record",
                "-a",
                "-q",
                "-g",
                "-k",
                "1",
                "-F",
                &params.perf_frequency.to_string(),
                "-e",
                "cpu-clock:pppH",
                "-o",
                &params.data_file_path.display().to_string(),
                "--",
                "sleep",
                &params.collection_time.to_string(),
            ])
            .spawn()
        {
            Err(e) => Err(PDError::DependencyError(format!(
                "Skipping Perf profile collection due to: {}",
                e
            ))
            .into()),
            Ok(child) => {
                debug!("Recording Perf profiling data.");
                *PERF_CHILD.lock().unwrap() = Some(child);
                Ok(())
            }
        }
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        Ok(())
    }

    fn finish_data_collection(&mut self, params: &CollectorParams) -> Result<()> {
        let mut child = PERF_CHILD.lock().unwrap();
        match child.as_ref() {
            None => return Ok(()),
            Some(_) => {}
        }

        signal::kill(
            Pid::from_raw(child.as_mut().unwrap().id() as i32),
            params.signal,
        )?;

        debug!("Waiting for perf profile collection to complete...");
        match child.as_mut().unwrap().wait() {
            Err(e) => {
                error!("'perf' did not exit successfully: {}", e);
                return Ok(());
            }
            Ok(_) => debug!("'perf record' executed successfully."),
        }

        let event_out_path_buf = params.data_dir.join("parsed_perf_data.out");
        let events_out_path = if params.save_profile_events {
            Some(event_out_path_buf.as_path())
        } else {
            None
        };

        // Parse raw Perf profile and build ProfilingData
        let perf_profiler_data = build_perf_profiler_data(
            &params.data_file_path,
            *PROFILE_START_TIME_MS.lock().unwrap(),
            events_out_path,
        );
        let perf_profiler_data_path = params
            .data_dir
            .join(format!("{}-perf-profiler-data.json", params.run_name));
        if let Ok(json) = serde_json::to_string(&perf_profiler_data) {
            fs::write(&perf_profiler_data_path, json)?;
        }

        let mut top_functions_file =
            fs::File::create(params.data_dir.join(PERF_TOP_FUNCTIONS_FILE_NAME))?;

        let out = Command::new("perf")
            .args([
                "report",
                "--stdio",
                "--percent-limit",
                "1",
                "-i",
                &params.data_file_path.display().to_string(),
            ])
            .output();

        match out {
            Err(e) => {
                let out = format!("Skipped processing profiling data due to : {}", e);
                error!("{}", out);
                write!(top_functions_file, "{}", out)?;
            }
            Ok(v) => {
                let mut top_functions = "No data collected";
                if !v.stdout.is_empty() {
                    top_functions = std::str::from_utf8(&v.stdout)?;
                }
                write!(top_functions_file, "{}", top_functions)?;
            }
        }
        Ok(())
    }

    fn is_profile() -> bool {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfProfile;

impl PerfProfile {
    pub fn new() -> Self {
        PerfProfile
    }
}

impl ProcessData for PerfProfile {
    fn process_raw_data(
        &mut self,
        params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        // Deserialize the ProfilerData generated at the end of record.
        // TODO: build ProfilingData from the ProfilerData and return it to replace top_function data.
        let perf_profiler_data_path = params
            .data_dir
            .join(format!("{}-perf-profiler-data.json", params.run_name));
        let json_string = fs::read_to_string(perf_profiler_data_path)?;
        let _perf_profiler_data = serde_json::from_str::<Profiler>(&json_string)?;

        let mut text_data = TextData::default();
        let file_loc = params.data_dir.join(PERF_TOP_FUNCTIONS_FILE_NAME);
        if file_loc.exists() {
            text_data.lines = fs::read_to_string(&file_loc)?
                .split('\n')
                .map(|x| x.to_string())
                .collect();
        }
        Ok(AperfData::Text(text_data))
    }
}
