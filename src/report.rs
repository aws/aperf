use crate::analytics::BASE_RUN_NAME;
use crate::data::utils::no_tar_gz_file_name;
use crate::data::JS_DIR;
use crate::{data, PDError, VisualizationData};
use anyhow::Result;
use chrono::Utc;
use clap::Args;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use log::info;
use serde::Serialize;
use std::collections::HashSet;
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
    /// The list of paths to the run archives (run data tar files)
    run_archive_paths: Vec<PathBuf>,
    /// The list of paths to the run data directory (where data are available for read and processing)
    run_dir_paths: Vec<PathBuf>,
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
        if self.run_names.contains(&run_name) {
            return Err(PDError::DuplicateRunNames(run_name).into());
        }

        self.run_names.push(run_name);
        self.run_archive_paths.push(run_archive_path);
        self.run_dir_paths.push(run_dir_path);

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

    check_duplicate_runs(&report.run)?;

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

    generate_report_files(report_dir_path, runs_info, tmp_dir);

    Ok(())
}

/// Checks if the list of runs include duplicates
pub fn check_duplicate_runs(run_args: &Vec<String>) -> Result<()> {
    let mut unique_runs: HashSet<String> = HashSet::new();

    for run in run_args {
        let run_name = no_tar_gz_file_name(&PathBuf::from(run))
            .ok_or(PDError::InvalidArchive(PathBuf::from(run)))?;
        if !unique_runs.insert(run_name.clone()) {
            return Err(PDError::DuplicateRunNames(run_name).into());
        }
    }

    Ok(())
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

    let archive_file = File::open(&archive_path)?;
    let gz_decoder = GzDecoder::new(archive_file);
    let mut tar = tar::Archive::new(gz_decoder);
    tar.unpack(tmp_dir)?;
    let dir_name = match no_tar_gz_file_name(&archive_path) {
        Some(dir_name) => dir_name,
        None => return Err(PDError::InvalidArchive(archive_path.clone()).into()),
    };
    let extracted_dir_path = tmp_dir.join(&dir_name);
    if !extracted_dir_path.exists() {
        return Err(PDError::ArchiveDirectoryInvalidName(dir_name).into());
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
    for run_data_dir in runs_info.run_dir_paths {
        let run_name = visualization_data
            .init_visualizers(run_data_dir.clone(), tmp_dir, &report_dir)
            .unwrap();
        visualization_data.process_raw_data(run_name).unwrap();
    }

    let analytical_findings = visualization_data.run_analytics();

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
        let out_data = serde_json::to_string(&visualizer.processed_data).unwrap();
        write!(
            processed_data_js_file,
            "processed_{}_data = {}\n\n",
            data_name, out_data
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
    for run_archive_source_path in runs_info.run_archive_paths {
        let run_archive_dest_path =
            report_run_archives_dir.join(run_archive_source_path.file_name().unwrap());
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
