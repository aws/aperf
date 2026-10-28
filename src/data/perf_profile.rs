use crate::data::common::data_formats::{
    AperfData, GraphData, GraphGroup, Profiler, ProfilingData,
};
use crate::data::common::utils::copy_graph_and_update_graph_data;
use crate::data::{Data, ProcessData};
use crate::data_processing::ReportParams;
use crate::find_file;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use {
    crate::data::common::utils::get_sub_process_duration_seconds,
    crate::data::CollectData,
    crate::data_collection::InitParams,
    crate::profiling::perf::parser::build_perf_profiler_data,
    crate::run_command,
    crate::PDError,
    chrono::Utc,
    log::{debug, error, warn},
    nix::{sys::signal, unistd, unistd::Pid},
    std::process::Stdio,
    std::str::FromStr,
    std::{process::Child, sync::Mutex},
};

// Dummy struct used to maintain the Data enum order after the flamegraph
// data type was removed. This is to avoid deserialization failure and
// maintain backward compatibility.
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
    fn is_static() -> bool {
        true
    }
}

#[cfg(target_os = "linux")]
lazy_static! {
    pub static ref PERF_CHILD: Mutex<Option<Child>> = Mutex::new(None);
    pub static ref PROFILE_START_TIME_MS: Mutex<i64> = Mutex::new(0);
}

fn raw_perf_profile_path(run_data_dir: &PathBuf, profile_type: &str) -> PathBuf {
    run_data_dir.join(format!("raw_perf_{profile_type}_profile"))
}

fn raw_perf_on_cpu_profile_path(run_data_dir: &PathBuf) -> PathBuf {
    raw_perf_profile_path(run_data_dir, "cpu")
}

fn perf_profiler_event_output_path(run_data_dir: &PathBuf) -> PathBuf {
    run_data_dir.join("parsed_perf_data.out")
}

fn perf_profiler_data_path(run_data_dir: &PathBuf) -> PathBuf {
    run_data_dir.join("perf_profiler_data.json")
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
    fn prepare_data_collector(&mut self, init_params: &InitParams) -> Result<()> {
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

        match run_command(
            "perf",
            [
                "record",
                "-a",
                "-q",
                "-g",
                "-k",
                "1",
                "-F",
                &init_params.perf_frequency.to_string(),
                "-e",
                "cpu-clock:pppH",
                "-o",
                &raw_perf_on_cpu_profile_path(&init_params.run_data_dir).to_string_lossy(),
                "--",
                "sleep",
                &get_sub_process_duration_seconds(init_params).to_string(),
            ],
            Stdio::null(),
            Stdio::inherit(),
        ) {
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

    fn collect_data(&mut self, _init_params: &InitParams) -> Result<()> {
        Ok(())
    }

    fn finish_data_collection(&mut self, init_params: &InitParams) -> Result<()> {
        let mut child = PERF_CHILD.lock().unwrap();
        match child.as_ref() {
            None => return Ok(()),
            Some(_) => {}
        }

        signal::kill(
            Pid::from_raw(child.as_mut().unwrap().id() as i32),
            signal::Signal::from_str(&init_params.end_signal).unwrap_or(signal::Signal::SIGTERM),
        )?;

        debug!("Waiting for perf profile collection to complete...");
        match child.as_mut().unwrap().wait() {
            Err(e) => {
                error!("'perf' did not exit successfully: {}", e);
                return Ok(());
            }
            Ok(_) => debug!("'perf record' executed successfully."),
        }

        let event_output_path_buf = perf_profiler_event_output_path(&init_params.run_data_dir);
        let event_output_path = if init_params.save_profile_events {
            Some(event_output_path_buf.as_path())
        } else {
            None
        };

        let raw_perf_data_path = raw_perf_on_cpu_profile_path(&init_params.run_data_dir);

        // Parse raw Perf profile and build ProfilingData
        let perf_profiler_data = build_perf_profiler_data(
            "cpu",
            &raw_perf_data_path,
            *PROFILE_START_TIME_MS.lock().unwrap(),
            event_output_path,
        );

        if let Ok(json) = serde_json::to_string(&perf_profiler_data) {
            fs::write(perf_profiler_data_path(&init_params.run_data_dir), json)?;
        }
        // The raw Perf profile could be huge and it becomes useless once it
        // leaves the system, so remove it from the run data, unless the profile
        // events are saved, which are checked against it.
        if !init_params.save_profile_events {
            if let Err(e) = fs::remove_file(&raw_perf_data_path) {
                error!(
                    "Failed to remove raw Perf profile at {}: {e}",
                    raw_perf_data_path.display()
                );
            }
        }

        Ok(())
    }

    fn is_perf_profile() -> bool {
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
        report_params: &ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        // Still attempt to process perf script + inferno generated flamegraphs for
        // backward compatibility
        let mut graph_data = GraphData::default();
        graph_data.graph_groups.push(GraphGroup::new("default"));
        graph_data.graph_groups.push(GraphGroup::new("reverse"));
        [false, true].iter().for_each(|&is_reverse| {
            // Match both the current (`flamegraph.svg`) and legacy (`<run>-flamegraph.svg`) naming.
            let filename = if is_reverse {
                find_file(
                    &report_params.run_data_dir,
                    r"reverse-flamegraph\.svg$",
                    None,
                )
            } else {
                find_file(
                    &report_params.run_data_dir,
                    r"flamegraph\.svg$",
                    Some(r"reverse-flamegraph\.svg$"),
                )
            };
            if let Ok(filename) = filename {
                copy_graph_and_update_graph_data(
                    &report_params.run_data_dir,
                    &report_params.report_dir,
                    &filename,
                    &report_params.run_name,
                    if is_reverse { "reverse" } else { "default" },
                    "cpu",
                    "Perf CPU Profile".to_string(),
                    &mut graph_data,
                );
            }
        });

        // Deserialize the ProfilerData generated at the end of record.
        let perf_profiler_data =
            match fs::read_to_string(perf_profiler_data_path(&report_params.run_data_dir))
                .ok()
                .and_then(|json| serde_json::from_str::<Profiler>(&json).ok())
            {
                Some(perf_profiler_data) => perf_profiler_data,
                // If ProfilerData could not be read, chaces are this run was created before the
                // introduction of ProfilingData, so fall back to the old GraphData.
                None => return Ok(AperfData::Graph(graph_data)),
            };

        let mut profiling_data = ProfilingData::default();
        profiling_data
            .profilers
            .insert(String::from("cpu"), perf_profiler_data);

        Ok(AperfData::Profile(profiling_data))
    }
}
