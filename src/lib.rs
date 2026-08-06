#[macro_use]
extern crate lazy_static;

pub mod analytics;
pub mod completions;
pub mod computations;
pub mod data;
pub mod data_collection;
pub mod data_processing;
pub mod profiling;
#[cfg(target_os = "linux")]
pub mod record;
pub mod report;
#[cfg(feature = "mcp-server")]
pub mod server;

use crate::data::aperf_runlog::AperfRunlog;
use crate::data::TimeEnum;
use anyhow::{bail, Result};
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use strum_macros::{Display, EnumIter, EnumString};
use thiserror::Error;
#[cfg(target_os = "linux")]
use {
    crate::data::{aperf_stats::AperfStatsCollector, common::utils::CpuInfo},
    log::warn,
    std::cell::RefCell,
    std::collections::HashSet,
    std::ffi::OsStr,
    std::fs::File,
    std::io::{Read, Seek, SeekFrom},
    std::os::unix::process::ExitStatusExt,
    std::path::Path,
    std::process::{Child, Command, ExitStatus, Output, Stdio},
    std::time,
};

pub const APERF_FILE_FORMAT: &str = "bin";

#[cfg(target_os = "windows")]
pub const APERF_TMP: &str = "C:\\Temp";

#[cfg(target_os = "macos")]
pub const APERF_TMP: &str = "/tmp";

#[cfg(target_os = "linux")]
pub const APERF_TMP: &str = "/tmp";

pub const GROUPED_PMU_MODE: &str = "grouped";
pub const UNGROUPED_PMU_MODE: &str = "ungrouped";

#[cfg(target_os = "linux")]
lazy_static! {
    pub static ref CPU_INFO: Result<CpuInfo> = CpuInfo::new();
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

/// Use the module name (file name) of a data as its unique identifier in APerf.
pub fn get_data_name_from_type<T>() -> &'static str {
    let full_data_module_path = std::any::type_name::<T>();

    let mut data_identifier_found = false;
    let mut data_name: Option<&str> = None;
    for data_module_part in full_data_module_path.split("::") {
        if data_identifier_found {
            data_name = Some(data_module_part);
            break;
        }
        data_identifier_found = data_module_part == "data";
    }

    match data_name {
        Some(value) => value,
        None => panic!("Could not get data name"),
    }
}

/// Returns the name of the first file in dir whose name matches the pattern regex but does
/// not match the optional exclude regex.
pub fn find_file(dir: &PathBuf, pattern: &str, exclude_pattern: Option<&str>) -> Result<String> {
    let regex = Regex::new(pattern)?;
    let exclude_regex = exclude_pattern.map(Regex::new).transpose()?;
    for entry in fs::read_dir(dir)? {
        let filename = entry?.file_name().into_string().unwrap();
        if regex.is_match(&filename)
            && !exclude_regex
                .as_ref()
                .is_some_and(|ex| ex.is_match(&filename))
        {
            return Ok(filename);
        }
    }
    match exclude_pattern {
        Some(exclude_pattern) => bail!(
            "Could not find any file matching /{pattern}/ (excluding /{exclude_pattern}/) in {}",
            dir.display()
        ),
        None => bail!(
            "Could not find any file matching /{pattern}/ in {}",
            dir.display()
        ),
    }
}

/// Extracts the file name from a path. If the file is a tar ball, ignore the "tar.gz" suffix.
pub fn no_tar_gz_file_name(path: &PathBuf) -> Option<String> {
    if path.file_name().is_none() {
        return None;
    }

    let file_name_str = path.file_name()?.to_string_lossy().into_owned();

    if file_name_str.ends_with(".tar.gz") {
        return Some(file_name_str.strip_suffix(".tar.gz")?.to_string());
    }
    Some(file_name_str)
}

pub fn data_file_path(data_name: &str, run_data_dir: &PathBuf) -> PathBuf {
    run_data_dir.join(format!("{}.{}", data_name, APERF_FILE_FORMAT))
}

pub fn aperf_runlog_file_path(run_data_dir: &PathBuf) -> PathBuf {
    run_data_dir.join(get_data_name_from_type::<AperfRunlog>())
}

#[derive(EnumIter, EnumString, Display, Clone, Copy, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum ProcessMetric {
    UserSpaceTime,
    KernelSpaceTime,
    NumberThreads,
    VirtualMemorySize,
    ResidentSetSize,
    ResidentSetSizeBytes,
    NumberProcesses,
}

impl ProcessMetric {
    fn to_aperf_stat_metric_name(&self) -> String {
        format!("process_{}", self.to_string())
    }
}

#[cfg(target_os = "linux")]
thread_local! {
    /// The singleton APerf stats to collect APerf performance metrics
    /// throughout the collection.
    static APERF_STATS_COLLECTOR: RefCell<AperfStatsCollector> = RefCell::new(AperfStatsCollector::new());
    /// PIDs of the long-running subprocesses launched via run_command, to be
    /// saved to InitParams.sub_process_pids at the end of the collection.
    static SUB_PROCESS_PIDS: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
}

#[cfg(target_os = "linux")]
pub fn aperf_stats_initialize(run_data_dir: PathBuf) {
    APERF_STATS_COLLECTOR.with(|aperf_stats_collector| {
        aperf_stats_collector.borrow_mut().initialize(run_data_dir);
    });
}

#[cfg(target_os = "linux")]
pub fn aperf_stats_proceed_to_next_stats(next_stats_time: TimeEnum) {
    APERF_STATS_COLLECTOR.with(|aperf_stats_collector| {
        aperf_stats_collector
            .borrow_mut()
            .proceed_to_next_stats(next_stats_time);
    });
}

#[cfg(target_os = "linux")]
pub fn aperf_stats_add(stat_name: String, data_name: String, stat_value: f64) {
    APERF_STATS_COLLECTOR.with(|aperf_stats_collector| {
        aperf_stats_collector
            .borrow_mut()
            .add_stat(stat_name, data_name, stat_value);
    });
}

/// Measure the wall-clock time of executing a function and add as a stat.
#[cfg(target_os = "linux")]
pub fn aperf_stats_measure<F>(stat_name: String, data_name: String, mut callback: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    let start_time = time::Instant::now();
    callback()?;
    let execution_time = (time::Instant::now() - start_time).as_micros() as f64;

    aperf_stats_add(stat_name, data_name, execution_time);

    Ok(())
}

/// Save the usage metrics of a child process as APerf stats.
#[cfg(target_os = "linux")]
pub fn aperf_stats_add_process_usage(process_name: &str, rusage: libc::rusage) {
    aperf_stats_add(
        ProcessMetric::UserSpaceTime.to_aperf_stat_metric_name(),
        process_name.to_string(),
        rusage.ru_utime.tv_sec as f64 + rusage.ru_utime.tv_usec as f64 / 1_000_000.0,
    );
    aperf_stats_add(
        ProcessMetric::KernelSpaceTime.to_aperf_stat_metric_name(),
        process_name.to_string(),
        rusage.ru_stime.tv_sec as f64 + rusage.ru_stime.tv_usec as f64 / 1_000_000.0,
    );
    aperf_stats_add(
        ProcessMetric::ResidentSetSizeBytes.to_aperf_stat_metric_name(),
        process_name.to_string(),
        rusage.ru_maxrss as f64 * 1024.0,
    );
}

#[cfg(target_os = "linux")]
pub fn aperf_stats_flush() -> Result<()> {
    APERF_STATS_COLLECTOR
        .with(|aperf_stats_collector| aperf_stats_collector.borrow_mut().flush())?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn sub_process_pids() -> HashSet<u32> {
    SUB_PROCESS_PIDS.with(|pids| pids.borrow().clone())
}

#[cfg(target_os = "linux")]
pub fn register_sub_process_pid(pid: u32) {
    SUB_PROCESS_PIDS.with(|pids| pids.borrow_mut().insert(pid));
}

/// Run a command without waiting for its completion and save its pid, so that
/// its process metrics can be later identified and extracted into APerf stats.
/// The caller is expected to own the command's stdio config, signaling, and
/// waiting.
#[cfg(target_os = "linux")]
pub fn run_command<S, I>(command: S, args: I, stdout: Stdio, stderr: Stdio) -> Result<Child>
where
    S: AsRef<OsStr>,
    I: IntoIterator,
    I::Item: AsRef<OsStr>,
{
    let child = Command::new(command)
        .args(args)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()?;

    register_sub_process_pid(child.id());

    Ok(child)
}

/// Run a command, wait for its completion, and collect its resource usage.
///
/// For the command's output streams, stderr is always written to a temp file,
/// and stdout is either piped or written to the file at stdout_path if provided.
/// With only one possible pipe, so we don't need to worry about deadlock when
/// draining with single thread.
#[cfg(target_os = "linux")]
pub fn run_command_and_wait<S, I>(
    command: S,
    args: I,
    process_name: &str,
    stdout_path: Option<&Path>,
) -> Result<Output>
where
    S: AsRef<OsStr>,
    I: IntoIterator,
    I::Item: AsRef<OsStr>,
{
    let mut command = Command::new(command);
    command.args(args);

    let stderr_file = tempfile::tempfile();
    if let Ok(stderr_file) = stderr_file.as_ref() {
        if let Ok(stderr_file_clone) = stderr_file.try_clone() {
            command.stderr(Stdio::from(stderr_file_clone));
        }
    }

    let capture_stdout = stdout_path.is_none();
    match stdout_path {
        Some(path) => command.stdout(Stdio::from(File::create(path)?)),
        None => command.stdout(Stdio::piped()),
    };

    let mut child = command.spawn()?;
    let pid = child.id() as libc::pid_t;

    let mut stdout = Vec::new();
    if capture_stdout {
        // Drain the pipe to EOF before reaping, so the subprocess can never
        // be blcoked by a full pipe while we wait for its completion.
        if let Some(pipe) = child.stdout.as_mut() {
            pipe.read_to_end(&mut stdout)?;
        }
    }

    // Reap with wait4 to also obtain the kernel's resource usage accounting
    // of the subprocess.
    let mut status: libc::c_int = 0;
    let mut rusage: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::wait4(pid, &mut status, 0, &mut rusage) } == -1 {
        warn!(
            "Failed to obtain the resource usage of command {process_name}: {}",
            std::io::Error::last_os_error()
        );
    } else {
        aperf_stats_add_process_usage(&process_name, rusage);
    }

    // Read the subprocess's stderr back from the temp file.
    let mut stderr = Vec::new();
    if let Ok(mut stderr_file) = stderr_file {
        if stderr_file.seek(SeekFrom::Start(0)).is_ok() {
            let _ = stderr_file.read_to_end(&mut stderr);
        }
    }

    Ok(Output {
        status: ExitStatus::from_raw(status),
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod test {
    use super::find_file;
    #[cfg(target_os = "linux")]
    use super::run_command_and_wait;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[cfg(target_os = "linux")]
    #[test]
    fn test_run_command_and_wait_captures_output() {
        // stdout and stderr are captured separately; exit status is reported.
        let output = run_command_and_wait(
            "sh",
            ["-c", "echo out_data; echo err_data >&2; exit 3"],
            "sh",
            None,
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "out_data\n");
        assert_eq!(String::from_utf8_lossy(&output.stderr), "err_data\n");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_run_command_and_wait_stdout_to_file() {
        // With stdout_path set, stdout goes to the file and Output.stdout is empty.
        let dir = TempDir::new().unwrap();
        let stdout_path = dir.path().join("stdout.txt");
        let output = run_command_and_wait(
            "sh",
            ["-c", "echo to_file; echo err_data >&2"],
            "sh",
            Some(&stdout_path),
        )
        .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(String::from_utf8_lossy(&output.stderr), "err_data\n");
        assert_eq!(fs::read_to_string(&stdout_path).unwrap(), "to_file\n");
    }

    #[test]
    fn test_find_file_prefix_match() {
        let dir = TempDir::new().unwrap();
        for f in &[
            "cpu_utilization.bin",
            "other_cpu_utilization.bin",
            "noise.txt",
        ] {
            fs::File::create(dir.path().join(f)).unwrap();
        }
        let path = PathBuf::from(dir.path());
        // Anchored at the start with `^`.
        assert_eq!(
            find_file(&path, "^cpu_utilization", None).unwrap(),
            "cpu_utilization.bin",
        );
        // No match returns Err.
        assert!(find_file(&path, "^missing", None).is_err());
    }

    #[test]
    fn test_find_file_suffix_match() {
        let dir = TempDir::new().unwrap();
        for f in &["data.bin", "data.bin.bak", "noise.txt"] {
            fs::File::create(dir.path().join(f)).unwrap();
        }
        let path = PathBuf::from(dir.path());
        // Anchored at the end with `$` (".bin" mid-name in "data.bin.bak" doesn't match).
        assert_eq!(find_file(&path, r"\.bin$", None).unwrap(), "data.bin");
        // No match returns Err.
        assert!(find_file(&path, r"\.missing$", None).is_err());
    }

    #[test]
    fn test_find_file_excludes_substring_collision() {
        // Regression test: the forward flamegraph lookup must not pick up
        // `reverse-flamegraph.svg`, whose name also ends in `flamegraph.svg`. Create the files
        // in both orders to defeat any reliance on directory read ordering.
        for order in [
            ["flamegraph.svg", "reverse-flamegraph.svg"],
            ["reverse-flamegraph.svg", "flamegraph.svg"],
        ] {
            let dir = TempDir::new().unwrap();
            for f in order {
                fs::File::create(dir.path().join(f)).unwrap();
            }
            let path = PathBuf::from(dir.path());
            // Forward: match `flamegraph.svg` but exclude the reverse variant.
            assert_eq!(
                find_file(
                    &path,
                    r"flamegraph\.svg$",
                    Some(r"reverse-flamegraph\.svg$")
                )
                .unwrap(),
                "flamegraph.svg",
            );
            // Reverse: matches only the reverse variant.
            assert_eq!(
                find_file(&path, r"reverse-flamegraph\.svg$", None).unwrap(),
                "reverse-flamegraph.svg",
            );
        }
    }

    #[test]
    fn test_find_file_excludes_legacy_run_prefixed_names() {
        // The same disambiguation must hold for the legacy `<run>-flamegraph.svg` naming.
        let dir = TempDir::new().unwrap();
        for f in &["myrun-flamegraph.svg", "myrun-reverse-flamegraph.svg"] {
            fs::File::create(dir.path().join(f)).unwrap();
        }
        let path = PathBuf::from(dir.path());
        assert_eq!(
            find_file(
                &path,
                r"flamegraph\.svg$",
                Some(r"reverse-flamegraph\.svg$")
            )
            .unwrap(),
            "myrun-flamegraph.svg",
        );
        assert_eq!(
            find_file(&path, r"reverse-flamegraph\.svg$", None).unwrap(),
            "myrun-reverse-flamegraph.svg",
        );
    }
}
