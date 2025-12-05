use crate::analytics::BASE_RUN_NAME;
use crate::data::JS_DIR;
use crate::{data, PDError, VisualizationData};
use anyhow::Result;
use clap::Args;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use log::{error, info};
use serde::Serialize;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

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

#[derive(Clone, Args, Debug)]
pub struct Report {
    /// The paths to the directories or archives of the recorded data to be included in the report.
    #[clap(help_heading = "Basic Options", short, long, value_parser, required = true, value_names = &["RUN_NAME> <RUN_NAME"], num_args = 1..)]
    pub run: Vec<String>,

    /// The directory and archive name of the report.
    #[clap(help_heading = "Basic Options", short, long, value_parser)]
    pub name: Option<String>,
}

pub fn form_and_copy_archive(loc: PathBuf, report_name: &Path, tmp_dir: &Path) -> Result<()> {
    if loc.is_dir() {
        let dir_name = loc.file_name().unwrap().to_str().unwrap().to_string();

        /* Create a temp archive */
        let archive_name = format!("{}.tar.gz", &dir_name);
        let archive_path = tmp_dir.join(&archive_name);
        let archive_dst = report_name.join(format!("data/archive/{}", archive_name));
        info!("Creating archive {}", archive_path.display());
        {
            let tar_gz = fs::File::create(&archive_path)?;
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all(&dir_name, &loc)?;
        }

        /* Copy archive to aperf_report */
        info!("Copying archive to {}", archive_dst.display());
        fs::copy(&archive_path, archive_dst)?;
        return Ok(());
    }
    if infer::get_from_path(&loc)?.unwrap().mime_type() == "application/gzip" {
        let file_name = loc.file_name().unwrap().to_str().unwrap().to_string();

        /* Copy archive to aperf_report */
        let archive_dst = report_name.join(format!("data/archive/{}", file_name));

        info!("Copying archive to {}", archive_dst.display());
        fs::copy(loc, archive_dst)?;
        return Ok(());
    }
    Err(PDError::RecordNotArchiveOrDirectory.into())
}

pub fn is_report_dir(dir: PathBuf) -> Option<PathBuf> {
    /* Legacy report detection */
    if dir.join("index.css").exists()
        && dir.join("index.html").exists()
        && dir.join("index.js").exists()
        && dir.join("data").exists()
        && dir.join("data/archive").exists()
    {
        return Some(dir.join("data/archive"));
    }

    /* New report detection */
    if dir.join("main.css").exists()
        && dir.join("index.html").exists()
        && dir.join("bundle.js").exists()
        && dir.join("data").exists()
        && dir.join("data/archive").exists()
    {
        return Some(dir.join("data/archive"));
    }

    None
}

pub fn get_report_archives(dir: PathBuf) -> Result<Vec<PathBuf>> {
    let mut archives = Vec::new();
    for entry in fs::read_dir(dir)? {
        archives.push(entry?.path());
    }
    Ok(archives)
}

pub fn get_dir(dir: PathBuf, tmp_dir: &PathBuf) -> Result<PathBuf> {
    /* If dir return */
    if dir.is_dir() {
        return Ok(dir);
    }
    /* Unpack if archive */
    if infer::get_from_path(&dir)?.unwrap().mime_type() == "application/gzip" {
        info!("Extracting {}", dir.display());
        let tar_gz = File::open(&dir)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(tmp_dir)?;
        let dir_name = dir
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".tar.gz")
            .ok_or(PDError::InvalidArchiveName)?;
        let out = tmp_dir.join(dir_name);
        if !out.exists() {
            return Err(PDError::ArchiveDirectoryMismatch.into());
        }
        return Ok(out);
    }
    Err(PDError::RecordNotArchiveOrDirectory.into())
}

pub fn report(report: &Report, tmp_dir: &PathBuf) -> Result<()> {
    let dirs: Vec<String> = report.run.clone();
    let mut pathbuf_dirs: Vec<PathBuf> = Vec::new();
    let mut data_dirs: Vec<PathBuf> = Vec::new();
    let mut dir_paths: Vec<String> = Vec::new();
    let mut dir_names: Vec<String> = Vec::new();

    /* Form PathBuf from dirs */
    for dir in &dirs {
        pathbuf_dirs.push(PathBuf::from(dir));
    }

    /* Check if dirs contains report */
    for dir in pathbuf_dirs {
        let path_dir = get_dir(dir.to_path_buf(), tmp_dir)?;
        if let Some(archive_dir) = is_report_dir(path_dir.clone()) {
            if report.name.is_none() {
                return Err(PDError::VisualizerReportFromReportNoNameError.into());
            }
            if let Ok(archives) = get_report_archives(archive_dir) {
                for path in archives {
                    data_dirs.push(path);
                }
            }
        } else {
            data_dirs.push(path_dir);
        }
    }

    /* Get dir paths, names */
    for dir in &data_dirs {
        let path = get_dir(dir.to_path_buf(), tmp_dir)?;
        let dir_name = data::utils::notargz_file_name(path.clone())?;
        if dir_names.contains(&dir_name) {
            error!("Cannot process two runs with the same name");
            return Ok(());
        }
        dir_names.push(dir_name);
        dir_paths.push(path.to_str().unwrap().to_string());
    }

    let mut report_name = PathBuf::new();
    match &report.name {
        Some(n) => report_name.push(data::utils::notargz_string_name(n.to_string())?),
        None => {
            /* Generate report name */
            let mut file_name = "aperf_report".to_string();
            for dir_name in &dir_names {
                file_name = format!("{}_{}", file_name, dir_name);
            }
            report_name.push(file_name);
            info!("Report name not given. Using '{}'", report_name.display());
        }
    }
    let mut report_name_tgz = PathBuf::new();
    // If a user provided run name has a '.' in it, setting the extension as '.tar.gz'
    // here will overwrite the run name after the '.'. To prevent that set the filename.
    report_name_tgz.set_file_name(report_name.to_str().unwrap().to_owned() + ".tar.gz");

    generate_report_files(
        report_name.clone(),
        &dir_names,
        &data_dirs,
        &dir_paths,
        tmp_dir,
    );

    Ok(())
}

fn generate_report_files(
    report_dir: PathBuf,
    run_names: &Vec<String>,
    raw_run_paths: &Vec<PathBuf>,
    run_dir_paths: &Vec<String>,
    tmp_dir: &PathBuf,
) {
    {
        let mut base_run_name = BASE_RUN_NAME.lock().unwrap();
        *base_run_name = run_names.get(0).unwrap().to_string();
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
    for run_dir in run_dir_paths {
        let name = visualization_data
            .init_visualizers(run_dir.to_owned(), tmp_dir, &report_dir)
            .unwrap();
        visualization_data.process_raw_data(name).unwrap();
    }

    let analytical_findings = visualization_data.run_analytics();

    /* Generate run.js */
    let run_js_path = processed_data_js_dir.join("runs.js");
    let mut runs_file = File::create(run_js_path).unwrap();
    write!(
        runs_file,
        "runs_raw = {}",
        serde_json::to_string(run_names).unwrap()
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

    /* Generate/copy the archives of the collected data into aperf_report */
    for dir in raw_run_paths {
        form_and_copy_archive(dir.to_path_buf(), &report_dir, tmp_dir).unwrap();
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
