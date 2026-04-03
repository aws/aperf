use crate::analytics::BASE_RUN_NAME;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use crate::data::common::utils::no_tar_gz_file_name;
use crate::data::JS_DIR;
use crate::{data, PDError, VisualizationData};
use anyhow::{Context, Result};
use chrono::Utc;
use clap::Args;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use log::{info, warn};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Serialize)]
struct VersionInfo {
    version: &'static str,
    git_sha: &'static str,
}

impl VersionInfo {
    fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            git_sha: env!("VERGEN_GIT_SHA"),
        }
    }
}

/// Stores the information of all runs to be included in the report
#[derive(Default)]
struct RunsInfo {
    /// The list of run names (the run dir/archive file name minus the file format)
    run_names: Vec<String>,
    /// The index used to deduplicate each unique run name
    run_name_dedup_indices: HashMap<String, u8>,
    /// The map from run names to paths to the run archives (run data tar files)
    run_archive_paths: HashMap<String, PathBuf>,
    /// The map from run names to paths to the run data directory (where data are available for read and processing)
    run_dir_paths: HashMap<String, PathBuf>,
    /// The specified start time of every run's time range
    per_run_from_time: HashMap<String, i64>,
    /// The specified end time of every run's time range
    per_run_to_time: HashMap<String, i64>,
}

impl RunsInfo {
    fn new() -> Self {
        RunsInfo::default()
    }

    fn add_run(
        &mut self,
        run_name: String,
        run_archive_path: PathBuf,
        run_dir_path: PathBuf,
    ) -> Result<()> {
        let deduped_run_name = self.deduplicate_run_name(run_name);
        self.run_names.push(deduped_run_name.clone());
        self.run_archive_paths
            .insert(deduped_run_name.clone(), run_archive_path);
        self.run_dir_paths.insert(deduped_run_name, run_dir_path);

        Ok(())
    }

    fn deduplicate_run_name(&mut self, run_name: String) -> String {
        if !self.run_name_dedup_indices.contains_key(&run_name) {
            self.run_name_dedup_indices.insert(run_name.clone(), 1);
            return run_name;
        }
        // Any duplicate run name will be deduped to original run name appended with the first
        // unused index
        loop {
            let deduped_run_name = format!(
                "{}_{}",
                run_name,
                self.run_name_dedup_indices.get(&run_name).unwrap()
            );
            if self.run_name_dedup_indices.contains_key(&deduped_run_name) {
                *self.run_name_dedup_indices.get_mut(&run_name).unwrap() += 1;
            } else {
                self.run_name_dedup_indices
                    .insert(deduped_run_name.clone(), 1);
                warn!(
                    "Duplicate run names detected. Renaming run {run_name} to {deduped_run_name}."
                );
                return deduped_run_name;
            }
        }
    }

    fn process_per_run_time_range(
        &mut self,
        run_time_ranges: &Vec<(String, Option<i64>, Option<i64>)>,
    ) -> Result<()> {
        for (run_name, from_time, to_time) in run_time_ranges {
            // Empty run name means apply to all runs
            let target_runs: Vec<String> = if run_name.is_empty() {
                self.run_names.clone()
            } else {
                if !self.run_names.contains(run_name) {
                    return Err(PDError::InvalidRunTimeRangeOption(format!(
                        "The run name {} is not part of the report.",
                        run_name
                    ))
                    .into());
                }
                vec![run_name.clone()]
            };

            if let (Some(from_time), Some(to_time)) = (from_time, to_time) {
                // Quickly fail if the two bounds are of the same sign and FROM > TO
                if (*from_time ^ *to_time) >= 0 && *from_time > *to_time {
                    return Err(PDError::InvalidRunTimeRangeOption(format!(
                        "The specified from_time {} is larger than to_time {} for run {}.",
                        from_time,
                        to_time,
                        if run_name.is_empty() {
                            "all runs"
                        } else {
                            run_name
                        }
                    ))
                    .into());
                }
            }

            for target_run in &target_runs {
                if let Some(from_time) = from_time {
                    if self.per_run_from_time.contains_key(target_run) {
                        return Err(PDError::InvalidRunTimeRangeOption(format!(
                            "The time range of run {} was specified multiple times.",
                            target_run
                        ))
                        .into());
                    }
                    self.per_run_from_time
                        .insert(target_run.clone(), *from_time);
                }
                if let Some(to_time) = to_time {
                    if self.per_run_to_time.contains_key(target_run) {
                        return Err(PDError::InvalidRunTimeRangeOption(format!(
                            "The time range of run {} was specified multiple times.",
                            target_run
                        ))
                        .into());
                    }
                    self.per_run_to_time.insert(target_run.clone(), *to_time);
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Args, Debug)]
pub struct Report {
    /// The paths to the directories or archives of the recorded data to be included in the report.
    /// If multiple runs are specified, the first run is used as the base run. The data in every
    /// other run will be compared against the base run to generate statistical and analytical findings.
    #[clap(help_heading = "Basic Options", verbatim_doc_comment, short, long, value_parser, required = true, value_names = &["RUN_NAME> <RUN_NAME"], num_args = 1..)]
    pub run: Vec<String>,

    /// The directory and archive name of the report.
    #[clap(help_heading = "Basic Options", short, long, value_parser)]
    pub name: Option<String>,

    /// The time range to apply to a run in the report. All the time-series metrics, stats, and
    /// analytical findings of the run will be limited to the specified time range.
    /// ===================================
    /// Format: RUN_NAME=FROM:TO or FROM:TO
    /// ===================================
    /// where FROM and TO are in seconds from the start of the run. If no run name is specified,
    /// the time range is applied to all runs. Either bound can be omitted or negative, and it
    /// can be specified for multiple runs.
    /// Example: --time-range first_run=10:60 --time-range second_run=:30
    ///          --time-range 20:150
    ///          --time-range -10:-5
    #[clap(
        help_heading = "Basic Options",
        verbatim_doc_comment,
        long,
        value_parser = parse_time_range,
        value_name = "RUN=FROM:TO",
        allow_hyphen_values = true,
        num_args = 1
    )]
    pub time_range: Vec<(String, Option<i64>, Option<i64>)>,
}

pub fn report(report: &Report, tmp_dir: &PathBuf) -> Result<()> {
    let report_path_str = if let Some(report_name_arg) = &report.name {
        report_name_arg.clone()
    } else {
        let time_now = Utc::now();
        let time_str = time_now.format("%Y-%m-%d_%H_%M_%S").to_string();
        let default_report_name = format!("aperf_report_{}", time_str);
        info!("Report name not given. Using {}", default_report_name);
        default_report_name
    };
    let report_dir_path = PathBuf::from(&report_path_str);
    let report_archive_path = PathBuf::from(format!("{}.tar.gz", report_path_str));
    if report_dir_path.exists() || report_archive_path.exists() {
        return Err(PDError::ReportExists(report_path_str).into());
    }

    let mut runs_info = RunsInfo::new();

    for run in &report.run {
        let run_path = PathBuf::from(run);

        if !run_path.exists() {
            return Err(PDError::RunNotFound(run_path).into());
        }

        let is_run_path_dir = run_path.is_dir();
        // Extract the data if the input run path is an archive
        let extracted_dir_path = if is_run_path_dir {
            run_path.clone()
        } else {
            extract_archive(&run_path, tmp_dir)?
        };

        // If handling a report, get all run data archives in it and extract them
        if let Some(report_run_archive_paths) = get_report_run_archive_paths(&extracted_dir_path) {
            for report_run_archive_path in report_run_archive_paths {
                runs_info.add_run(
                    no_tar_gz_file_name(&report_run_archive_path).unwrap(),
                    report_run_archive_path.clone(),
                    extract_archive(&report_run_archive_path, tmp_dir)?,
                )?
            }
        }
        // If handling a data directory, create an archive to be copied into the report at the end
        else if is_run_path_dir {
            runs_info.add_run(
                no_tar_gz_file_name(&run_path).unwrap(),
                create_archive(&run_path, tmp_dir)?,
                run_path.clone(),
            )?
        }
        // If handling a data archive, use the archive directly
        else {
            runs_info.add_run(
                no_tar_gz_file_name(&run_path).unwrap(),
                run_path.clone(),
                extracted_dir_path,
            )?
        }
    }

    runs_info.process_per_run_time_range(&report.time_range)?;

    generate_report_files(report_dir_path, runs_info, tmp_dir);

    Ok(())
}

/// Used to parse the --time-range option, in the format of run_name=from_time:to_time,
/// into a tuple (run_name, from_time, to_time)
fn parse_time_range(s: &str) -> Result<(String, Option<i64>, Option<i64>), String> {
    // If there's no '=', treat the whole string as FROM:TO (applies to all runs)
    let (run_name, range) = s.split_once('=').unwrap_or(("", s));
    let (from_str, to_str) = range
        .split_once(':')
        .ok_or_else(|| format!("invalid range '{}', expected FROM:TO", range))?;

    let from = if from_str.is_empty() {
        None
    } else {
        Some(
            from_str
                .parse::<i64>()
                .map_err(|e| format!("invalid FROM value: {}", e))?,
        )
    };
    let to = if to_str.is_empty() {
        None
    } else {
        Some(
            to_str
                .parse::<i64>()
                .map_err(|e| format!("invalid TO value: {}", e))?,
        )
    };

    Ok((run_name.to_string(), from, to))
}

/// Creates (tar) an archive in the temporary directory
pub fn create_archive(dir_path: &PathBuf, tmp_dir: &PathBuf) -> Result<PathBuf> {
    if !dir_path.is_dir() {
        return Err(PDError::InvalidDirectory(dir_path.clone()).into());
    }

    let dir_name = no_tar_gz_file_name(dir_path).unwrap();
    let archive_name = format!("{}.tar.gz", &dir_name);
    let archive_path = tmp_dir.join(archive_name);

    info!("Creating archive {:?}", archive_path);
    let archive_file = File::create(&archive_path)?;
    let gz_encoder = GzEncoder::new(archive_file, Compression::default());
    let mut tar = tar::Builder::new(gz_encoder);
    tar.append_dir_all(&dir_name, &dir_path)?;

    Ok(archive_path)
}

/// Extracts (untar) an archive to the temporary directory
pub fn extract_archive(archive_path: &PathBuf, tmp_dir: &PathBuf) -> Result<PathBuf> {
    if infer::get_from_path(&archive_path)?.unwrap().mime_type() != "application/gzip" {
        return Err(PDError::InvalidArchive(archive_path.clone()).into());
    }

    info!("Extracting archive {:?}", archive_path);

    // Get the top-level directory to help locate the path after untar
    let archive_file = File::open(&archive_path)?;
    let gz_decoder = GzDecoder::new(archive_file);
    let mut tar = tar::Archive::new(gz_decoder);
    let dir_name = tar
        .entries()?
        .next()
        .context(format!("Empty tar archive {:?}", archive_path))??
        .path()?
        .components()
        .next()
        .context(format!(
            "No top-level directory in archive {:?}",
            archive_path
        ))?
        .as_os_str()
        .to_string_lossy()
        .to_string();

    // Re-open the file to unpack, since the above entries() call moved the archive's reader
    // past position 0
    let archive_file = File::open(&archive_path)?;
    let gz_decoder = GzDecoder::new(archive_file);
    let mut tar = tar::Archive::new(gz_decoder);
    tar.unpack(tmp_dir)?;

    let extracted_dir_path = tmp_dir.join(&dir_name);
    if !extracted_dir_path.exists() {
        return Err(PDError::InvalidDirectory(extracted_dir_path).into());
    }

    Ok(extracted_dir_path)
}

/// Checks if the given path contains an APerf report. If so, return the list of run archive paths
/// contained in the report.
pub fn get_report_run_archive_paths(report_path: &PathBuf) -> Option<Vec<PathBuf>> {
    let report_data_dir_path = report_path.join("data");
    let report_run_archives_dir_path = report_data_dir_path.join("archive");
    let report_runs_path = report_data_dir_path.join("js").join("runs.js");

    if !report_run_archives_dir_path.exists() || !report_runs_path.exists() {
        return None;
    }

    let runs_content = fs::read_to_string(report_runs_path).unwrap();
    let json_str = runs_content.strip_prefix("runs_raw = ").unwrap().trim();
    let runs: Vec<String> = serde_json::from_str(json_str).unwrap();

    let mut run_archive_paths = Vec::new();
    for run_name in runs {
        let run_archive_path = report_run_archives_dir_path.join(format!("{}.tar.gz", run_name));
        if run_archive_path.exists() {
            run_archive_paths.push(run_archive_path);
        }
    }

    Some(run_archive_paths)
}

/// Processes all the raw data, executes analytical rules, and produces all required report files
fn generate_report_files(report_dir: PathBuf, runs_info: RunsInfo, tmp_dir: &PathBuf) {
    {
        let mut base_run_name = BASE_RUN_NAME.lock().unwrap();
        *base_run_name = runs_info.run_names.get(0).unwrap().to_string();
    }

    info!("Creating APerf report...");
    let report_data_dir = report_dir.join("data");
    fs::create_dir_all(report_data_dir.join("archive")).unwrap();
    let processed_data_js_dir = report_data_dir.join("js");
    fs::create_dir_all(processed_data_js_dir.clone()).unwrap();

    info!("Processing collected data...");
    let mut visualization_data = VisualizationData::new();
    data::add_all_visualization_data(&mut visualization_data);
    /* Init visualizers */
    for (run_name, run_data_dir) in runs_info.run_dir_paths {
        visualization_data
            .init_visualizers(run_name.clone(), run_data_dir.clone(), tmp_dir, &report_dir)
            .unwrap();
        visualization_data.process_raw_data().unwrap();
    }

    let mut processed_data_accessor = ProcessedDataAccessor::from_time_ranges(
        runs_info.per_run_from_time,
        runs_info.per_run_to_time,
    );

    let analytical_findings = visualization_data.run_analytics(&mut processed_data_accessor);

    /* Generate run.js */
    let run_js_path = processed_data_js_dir.join("runs.js");
    let mut runs_file = File::create(run_js_path).unwrap();
    write!(
        runs_file,
        "runs_raw = {}",
        serde_json::to_string(&runs_info.run_names).unwrap()
    )
    .unwrap();

    /* Generate version.js */
    let version_js_path = processed_data_js_dir.join("version.js");
    let mut version_file = File::create(version_js_path).unwrap();
    let version_info = VersionInfo::new();
    write!(
        version_file,
        "version_info = {}",
        serde_json::to_string(&version_info).unwrap()
    )
    .unwrap();

    JS_DIR
        .extract(&report_dir)
        .expect("Failed to copy frontend files");

    info!("Writing processed data into report");
    for (data_name, visualizer) in &mut visualization_data.visualizers {
        visualizer.post_process_data();

        let processed_data_js_path = processed_data_js_dir.join(format!("{}.js", data_name));
        let mut processed_data_js_file = File::create(processed_data_js_path).unwrap();
        write!(
            processed_data_js_file,
            "processed_{}_data = {}\n\n",
            data_name,
            processed_data_accessor.json_string(&visualizer.processed_data)
        )
        .unwrap();

        if let Some(data_findings) = analytical_findings.get(data_name) {
            let out_findings = serde_json::to_string(data_findings).unwrap();
            write!(
                processed_data_js_file,
                "{}_findings = {}",
                data_name, out_findings
            )
            .unwrap();
        }
    }

    /* Copy all run archives into the report's data/archives path */
    let report_run_archives_dir = report_data_dir.join("archive");
    for (run_name, run_archive_source_path) in runs_info.run_archive_paths {
        let run_archive_dest_path = report_run_archives_dir.join(format!("{run_name}.tar.gz"));
        info!("Copying archive to {:?}", run_archive_dest_path);
        fs::copy(run_archive_source_path, run_archive_dest_path).unwrap();
    }

    let mut report_name_tgz = PathBuf::new();
    // If a user provided run name has a '.' in it, setting the extension as '.tar.gz'
    // here will overwrite the run name after the '.'. To prevent that set the filename.
    report_name_tgz.set_file_name(report_dir.to_str().unwrap().to_owned() + ".tar.gz");

    info!("Creating report archive");
    let tar_gz = File::create(&report_name_tgz).unwrap();
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    let report_stem = report_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    tar.append_dir_all(&report_stem, &report_dir).unwrap();
    info!("Report archived at {}", report_name_tgz.display());
}
