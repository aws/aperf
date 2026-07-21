#[macro_use]
extern crate lazy_static;

pub mod analytics;
pub mod completions;
pub mod computations;
pub mod data;
pub mod data_collection;
pub mod data_processing;
#[cfg(target_os = "linux")]
pub mod pmu;
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
use thiserror::Error;
#[cfg(target_os = "linux")]
use {
    crate::data::{aperf_stats::AperfStat, common::utils::CpuInfo},
    chrono::Utc,
    log::error,
    std::cell::RefCell,
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

#[cfg(target_os = "linux")]
thread_local! {
    static APERF_STATS_COLLECTOR: RefCell<AperfStatsCollector> = RefCell::new(AperfStatsCollector::new());
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
pub fn aperf_stats_measure<F>(stat_name: String, func: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    APERF_STATS_COLLECTOR.with(|aperf_stats_collector| {
        aperf_stats_collector.borrow_mut().measure(stat_name, func)
    })?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn aperf_stats_add(stat_key: String, stat_value: u64) {
    APERF_STATS_COLLECTOR.with(|aperf_stats_collector| {
        aperf_stats_collector
            .borrow_mut()
            .add_stat(stat_key, stat_value);
    });
}

#[cfg(target_os = "linux")]
pub fn aperf_stats_flush() -> Result<()> {
    APERF_STATS_COLLECTOR
        .with(|aperf_stats_collector| aperf_stats_collector.borrow_mut().flush())?;

    Ok(())
}

/// Encapsulate all logics of collecting and writing APerf stats.
/// The collected stats will be saved in memory in time order and be written to
/// disk when flush() is called.
#[cfg(target_os = "linux")]
pub struct AperfStatsCollector {
    cur_aperf_stats: AperfStat,
    time_series_aperf_stats: Vec<AperfStat>,
    run_data_dir: Option<PathBuf>,
}

#[cfg(target_os = "linux")]
impl AperfStatsCollector {
    pub fn new() -> Self {
        Self {
            cur_aperf_stats: AperfStat::new(),
            time_series_aperf_stats: Vec::new(),
            run_data_dir: None,
        }
    }

    pub fn initialize(&mut self, run_data_dir: PathBuf) {
        self.run_data_dir = Some(run_data_dir);
    }

    /// Check current time and if at next second, save the current stats and
    /// proceed with a new empty stats.
    /// If we get to a point where the stats is big and we want to limit APerf's
    /// memory usage, we can also flush here.
    fn update_time_series(&mut self) {
        let cur_time = TimeEnum::DateTime(Utc::now());
        let cur_time_diff = match cur_time - self.cur_aperf_stats.time {
            TimeEnum::TimeDiff(time_diff) => time_diff,
            _ => return,
        };
        if cur_time_diff >= 1 {
            self.proceed_to_next_stats(cur_time);
        }
    }

    /// Save current stats and proceed to the next new stats.
    fn proceed_to_next_stats(&mut self, next_stats_time: TimeEnum) {
        let cur_aperf_stats = std::mem::replace(
            &mut self.cur_aperf_stats,
            AperfStat::for_time(next_stats_time),
        );
        self.time_series_aperf_stats.push(cur_aperf_stats);
    }

    /// Measure the wall-clock time of executing a function and save as a stat.
    pub fn measure<F>(&mut self, stat_name: String, func: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        self.update_time_series();

        self.cur_aperf_stats.measure(stat_name, func)
    }

    /// Save a stat.
    pub fn add_stat(&mut self, stat_key: String, stat_value: u64) {
        self.update_time_series();

        self.cur_aperf_stats.data.insert(stat_key, stat_value);
    }

    /// Write all saved stats to disk file.
    pub fn flush(&mut self) -> Result<()> {
        if self.run_data_dir.is_none() {
            bail!("Failed to flush APerf stat since the run data directory path is uninitialized.");
        }

        let aperf_stats_file_path = data_file_path(
            get_data_name_from_type::<AperfStat>(),
            self.run_data_dir.as_ref().unwrap(),
        );
        let mut aperf_stats_file = match fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&aperf_stats_file_path)
        {
            Ok(aperf_stats_file) => aperf_stats_file,
            Err(e) => bail!(
                "Failed to create APerf Stats file at {}: {:?}",
                aperf_stats_file_path.display(),
                e
            ),
        };

        for aperf_stats in &self.time_series_aperf_stats {
            bincode::serialize_into(&mut aperf_stats_file, aperf_stats)?;
        }
        if !self.cur_aperf_stats.data.is_empty() {
            bincode::serialize_into(&mut aperf_stats_file, &self.cur_aperf_stats)?;
        }

        self.time_series_aperf_stats.clear();
        self.cur_aperf_stats = AperfStat::new();

        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for AperfStatsCollector {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            error!("Failed to flush APerf stats on drop: {e}");
        }
    }
}

#[cfg(test)]
mod test {
    use super::find_file;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

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
