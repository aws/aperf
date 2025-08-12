extern crate ctor;

use crate::data::{CollectData, CollectorParams, ProcessedData};
use crate::utils::DataMetrics;
use crate::visualizer::GetData;
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(feature = "hotline")]
use {
    crate::visualizer::ReportParams,
    crate::{
        data::{Data, DataType},
        visualizer::DataVisualizer,
        PERFORMANCE_DATA, VISUALIZATION_DATA,
    },
    ctor::ctor,
    libc::{_exit, fork, geteuid, killpg, setpgid, waitpid, SIGTERM},
    log::{info, warn},
    std::path::Path,
    std::{
        env,
        ffi::CString,
        fs,
        os::raw::{c_char, c_int},
        panic,
    },
};

#[cfg(feature = "hotline")]
extern "C" {
    fn hotline(argc: c_int, argv: *const *const i8) -> c_int;
    fn deserialize_maps(argc: c_int, argv: *const *const i8) -> c_int;
}

pub static HOTLINE_FILE_NAME: &str = "hotline_profile";

#[cfg(feature = "hotline")]
pub fn check_preconditions() -> Result<bool> {
    let mut all_conditions_met = true;

    // Check root privileges
    let euid = unsafe { geteuid() } == 0;
    if !euid {
        warn!("Not running with root privileges. Please run with sudo.");
        all_conditions_met = false;
    }

    // Check KPTI status
    let cmdline = fs::read_to_string("/proc/cmdline")?;
    let kpti_off = cmdline.contains("kpti=off");
    if !kpti_off {
        warn!("KPTI is not disabled. Add 'kpti=off' to GRUB_CMDLINE_LINUX_DEFAULT and reboot.");
        all_conditions_met = false;
    }

    // Check kptr_restrict
    let kptr_value = fs::read_to_string("/proc/sys/kernel/kptr_restrict")?
        .trim()
        .parse::<i32>()
        .unwrap_or(-1);
    if kptr_value != 0 {
        warn!(
            "kptr_restrict is not set to 0. Run: echo 0 | sudo tee /proc/sys/kernel/kptr_restrict"
        );
        all_conditions_met = false;
    }

    // Check perf_event_paranoid
    let paranoid_value = fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")?
        .trim()
        .parse::<i32>()
        .unwrap_or(4);
    if paranoid_value != -1 {
        warn!("perf_event_paranoid is not set to -1. Run: echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid");
        all_conditions_met = false;
    }

    // Check /proc/kallsyms readable
    let kallsyms_readable = match fs::metadata("/proc/kallsyms") {
        Ok(metadata) => metadata.permissions().readonly(),
        Err(_) => false,
    };
    if !kallsyms_readable {
        warn!("/proc/kallsyms is not readable. Run: sudo chmod +r /proc/kallsyms");
        all_conditions_met = false;
    }

    if !all_conditions_met {
        Ok(false)
    } else {
        info!("Starting hotline");
        Ok(true)
    }
}

#[cfg(feature = "hotline")]
pub mod hotline_reports {
    use super::ReportParams;
    use std::error::Error;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    pub struct ReportConfig<'a> {
        pub table_id: &'a str,
        pub filename: &'a str,
    }

    pub const REPORT_CONFIGS: [ReportConfig; 5] = [
        ReportConfig {
            table_id: "completion_node",
            filename: "hotline_lat_map_completion_report.csv",
        },
        ReportConfig {
            table_id: "execution_latency",
            filename: "hotline_lat_map_exec_report.csv",
        },
        ReportConfig {
            table_id: "issue_latency",
            filename: "hotline_lat_map_issue_report.csv",
        },
        ReportConfig {
            table_id: "translation_latency",
            filename: "hotline_lat_map_translation_report.csv",
        },
        ReportConfig {
            table_id: "branch",
            filename: "hotline_bmiss_map.csv",
        },
    ];

    pub fn generate_html_files(params: &ReportParams) -> Result<(), Box<dyn Error>> {
        // First, create the CSS file
        for config in REPORT_CONFIGS {
            let csv_string = std::fs::read_to_string(format!(
                "{}/{}",
                params.data_dir.display(),
                config.filename
            ))?;
            let table_html = csv_to_html::convert(&csv_string, &b',', &true);

            // Use relative path to CSS file
            let full_html = format!(
                r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="UTF-8">
                    <meta name="viewport" content="width=device-width, initial-scale=1.0">
                    <link rel="stylesheet" href="../../index.css">
                </head>
                <body>
                    {}
                </body>
                </html>"#,
                table_html
            );

            let output_path = Path::new(&params.report_dir)
                .join("data")
                .join("js")
                .join(format!("{}_{}.html", params.run_name, config.table_id));
            let mut file = File::create(output_path)?;
            file.write_all(full_html.as_bytes())?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HotlineRaw {
    pid: i32,
    launched: bool,
}

impl HotlineRaw {
    fn new() -> Self {
        HotlineRaw {
            pid: 0,
            launched: false,
        }
    }
}

impl CollectData for HotlineRaw {
    #[cfg(feature = "hotline")]
    fn prepare_data_collector(&mut self, params: &CollectorParams) -> Result<()> {
        match check_preconditions() {
            Ok(false) => {
                warn!("Skipping Hotline.");
                self.launched = false;
                return Ok(());
            }
            Err(e) => {
                warn!("Failed to check preconditions: {}", e);
                self.launched = false;
                return Ok(());
            }
            _ => {}
        }

        let args = vec![
            CString::new("hotline").unwrap(),
            CString::new("--wakeup_period").unwrap(),
            CString::new(params.interval.to_string()).unwrap(),
            CString::new("--hotline_frequency").unwrap(),
            CString::new(params.hotline_frequency.to_string()).unwrap(),
            CString::new("--timeout").unwrap(),
            CString::new(params.collection_time.to_string()).unwrap(),
            CString::new("--data_dir").unwrap(),
            CString::new(params.data_dir.to_str().unwrap()).unwrap(),
        ];

        let argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();

        unsafe {
            match fork() {
                -1 => {
                    eprintln!("Fork failed");
                    Ok(()) // Return Ok even if fork fails
                }
                0 => {
                    // Child process
                    if setpgid(0, 0) == -1 {
                        eprintln!("Failed to set process group");
                        _exit(1);
                    }

                    // Setup signal handlers
                    let mut sigset = std::mem::MaybeUninit::uninit();
                    libc::sigemptyset(sigset.as_mut_ptr());
                    libc::sigaddset(sigset.as_mut_ptr(), SIGTERM);
                    libc::sigprocmask(libc::SIG_UNBLOCK, sigset.as_ptr(), std::ptr::null_mut());

                    let result = hotline(args.len() as c_int, argv.as_ptr() as *const *const i8);
                    _exit(result);
                }
                pid => {
                    // Parent process
                    self.pid = pid;
                    self.launched = true;
                    Ok(())
                }
            }
        }
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "hotline")]
    fn finish_data_collection(&mut self, params: &CollectorParams) -> Result<()> {
        if !self.launched {
            return Ok(());
        }

        unsafe {
            // Send SIGTERM to the process group
            if killpg(self.pid, SIGTERM) == -1 {
                let err = std::io::Error::last_os_error();
                eprintln!("Warning: Failed to kill process group: {}", err);
            }

            // Wait for the child process to finish
            let mut status: c_int = 0;
            match waitpid(self.pid, &mut status, 0) {
                -1 => {
                    let err = std::io::Error::last_os_error();
                    eprintln!("Warning: Failed to wait for child process: {}", err);
                }
                _ => {
                    if !libc::WIFEXITED(status) && !libc::WIFSIGNALED(status) {
                        return Err(anyhow::anyhow!("Child process terminated abnormally"));
                    }
                }
            }
        }

        // Child process has been completely killed. Now we can process the serialized data.
        let args = vec![
            CString::new("hotline").unwrap(),
            CString::new("--num_to_report").unwrap(),
            CString::new(params.num_to_report.to_string()).unwrap(),
            CString::new("--data_dir").unwrap(),
            CString::new(params.data_dir.to_str().unwrap()).unwrap(),
        ];

        let argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();

        // Run deserialize_maps in a child process
        unsafe {
            match fork() {
                -1 => {
                    eprintln!("Fork failed");
                }
                0 => {
                    // Child process
                    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                        deserialize_maps(args.len() as c_int, argv.as_ptr() as *mut *const i8);
                    }));
                    match result {
                        Ok(_) => _exit(0),
                        Err(_) => _exit(1),
                    }
                }
                pid => {
                    // Parent process
                    let mut status: c_int = 0;
                    if waitpid(pid, &mut status, 0) == -1 {
                        eprintln!("Failed to wait for deserialize_maps process");
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hotline {
    pub generated_files: Vec<String>,
}

impl Hotline {
    pub fn new() -> Self {
        Hotline {
            generated_files: vec![],
        }
    }
}

impl GetData for Hotline {
    #[cfg(feature = "hotline")]
    fn custom_raw_data_parser(&mut self, params: ReportParams) -> Result<Vec<ProcessedData>> {
        use crate::data::hotline::hotline_reports::REPORT_CONFIGS;

        match hotline_reports::generate_html_files(&params) {
            Ok(_) => (),
            Err(e) => eprintln!("Warning: Failed to generate HTML tables: {}", e),
        }

        let mut hotline_data = Hotline::new();
        hotline_data.generated_files = Vec::new();

        for config in REPORT_CONFIGS.iter() {
            let file_path = format!("{}/{}", params.data_dir.display(), config.filename);
            if Path::new(&file_path).exists() {
                hotline_data
                    .generated_files
                    .push(config.table_id.to_string());
            }
        }

        let data = ProcessedData::Hotline(hotline_data.clone());
        Ok(vec![data])
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["values".to_string()])
    }

    fn get_data(
        &mut self,
        buffer: Vec<ProcessedData>,
        _query: String,
        _metrics: &mut DataMetrics,
    ) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::Hotline(ref value) => {
                    values.extend(value.generated_files.clone());
                }
                _ => unreachable!(),
            }
        }

        Ok(serde_json::to_string(&values)?)
    }
}

#[ctor]
#[cfg(feature = "hotline")]
fn init_hotline_profile() {
    let hotline_raw = HotlineRaw::new();
    let file_name = HOTLINE_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::HotlineRaw(hotline_raw.clone()),
        file_name.clone(),
        false,
    );
    let hotline_profile = Hotline::new();
    let js_file_name = file_name.clone() + ".js";
    let mut dv = DataVisualizer::new(
        ProcessedData::Hotline(hotline_profile.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/hotline.js")).to_string(),
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
