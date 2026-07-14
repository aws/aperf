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
use crate::data::aperf_stats::AperfStat;
use anyhow::{bail, Result};
use regex::Regex;
use std::fs::{self, File};
use std::path::PathBuf;
use thiserror::Error;

pub const APERF_FILE_FORMAT: &str = "bin";

#[cfg(target_os = "windows")]
pub const APERF_TMP: &str = "C:\\Temp";

#[cfg(target_os = "macos")]
pub const APERF_TMP: &str = "/tmp";

#[cfg(target_os = "linux")]
pub const APERF_TMP: &str = "/tmp";

pub const GROUPED_PMU_MODE: &str = "grouped";
pub const UNGROUPED_PMU_MODE: &str = "ungrouped";

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

pub struct AperfStatsWriter {
    aperf_stats_file: File,
}
impl AperfStatsWriter {
    pub fn new(run_data_dir: &PathBuf) -> Result<Self> {
        let aperf_stats_file_path =
            data_file_path(get_data_name_from_type::<AperfStat>(), run_data_dir);
        let aperf_stats_file = match fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&aperf_stats_file_path)
        {
            Ok(aperf_stats_file) => aperf_stats_file,
            Err(e) => bail!(
                "Failed to create APerf Stats file at {}: {:?}",
                aperf_stats_file_path.display(),
                e
            ),
        };

        Ok(Self { aperf_stats_file })
    }

    pub fn write(&mut self, aperf_stats: &AperfStat) -> Result<()> {
        bincode::serialize_into(&mut self.aperf_stats_file, aperf_stats)?;

        Ok(())
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
