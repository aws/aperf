use crate::{PDError, VISUALIZATION_DATA};
use anyhow::Result;
use clap::Args;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Args, Debug)]
pub struct Report {
    /// Run data to be visualized. Can be a directory or a tarball.
    #[clap(short, long, value_parser, required = true, num_args = 1..)]
    pub run: Vec<String>,

    /// Report name.
    #[clap(short, long, value_parser)]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Api {
    name: String,
    runs: Vec<Run>,
}

impl Api {
    fn new(name: String) -> Self {
        Api {
            name,
            runs: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Run {
    name: String,
    keys: Vec<String>,
    key_values: HashMap<String, String>,
}

impl Run {
    fn new(name: String) -> Self {
        Run {
            name,
            keys: Vec::new(),
            key_values: HashMap::new(),
        }
    }
}

pub fn form_and_copy_archive(loc: PathBuf, report_name: &Path, tmp_dir: &Path) -> Result<()> {
    if loc.is_dir() {
        let dir_name = loc.file_name().unwrap().to_str().unwrap().to_string();

        /* Create a temp archive */
        let archive_name = format!("{}.tar.gz", &dir_name);
        let archive_path = tmp_dir.join(&archive_name);
        let archive_dst = report_name.join(format!("data/archive/{}", archive_name));
        {
            let tar_gz = fs::File::create(&archive_path)?;
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all(&dir_name, &loc)?;
        }

        /* Copy archive to aperf_report */
        fs::copy(&archive_path, archive_dst)?;
        return Ok(());
    }
    if infer::get_from_path(&loc)?.unwrap().mime_type() == "application/gzip" {
        let file_name = loc.file_name().unwrap().to_str().unwrap().to_string();

        /* Copy archive to aperf_report */
        let archive_dst = report_name.join(format!("data/archive/{}", file_name));
        fs::copy(loc, archive_dst)?;
        return Ok(());
    }
    Err(PDError::RecordNotArchiveOrDirectory.into())
}

pub fn is_report_dir(dir: PathBuf) -> Option<PathBuf> {
    if dir.join("index.css").exists()
        && dir.join("index.html").exists()
        && dir.join("index.js").exists()
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
        let dir_name = crate::data::utils::notargz_file_name(path.clone())?;
        if dir_names.contains(&dir_name) {
            error!("Cannot process two runs with the same name");
            return Ok(());
        }
        dir_names.push(dir_name);
        dir_paths.push(path.to_str().unwrap().to_string());
    }

    let mut report_name = PathBuf::new();
    match &report.name {
        Some(n) => report_name.push(crate::data::utils::notargz_string_name(n.to_string())?),
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

    info!("Creating APerf report...");
    let ico = include_bytes!("html_files/favicon.ico");
    let configure = include_bytes!("html_files/configure.png");
    let index_html = include_str!("html_files/index.html");
    let index_css = include_str!("html_files/index.css");
    let index_js = include_str!(concat!(env!("JS_DIR"), "/index.js"));
    let utils_js = include_str!(concat!(env!("JS_DIR"), "/utils.js"));
    let analytics_js = include_str!(concat!(env!("JS_DIR"), "/analytics.js"));
    let plotly_js = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/node_modules/plotly.js/dist/plotly.min.js"
    ));
    let configure_js = include_str!(concat!(env!("JS_DIR"), "/configure.js"));
    let run_names = dir_names.clone();

    fs::create_dir_all(report_name.join("images"))?;
    fs::create_dir_all(report_name.join("js"))?;
    fs::create_dir_all(report_name.join("data/archive"))?;
    fs::create_dir_all(report_name.join("data/js"))?;

    /* Generate/copy the archives of the collected data into aperf_report */
    for dir in &data_dirs {
        form_and_copy_archive(dir.to_path_buf(), &report_name, tmp_dir)?;
    }
    /* Generate base HTML, JS files */
    let mut ico_file = File::create(report_name.join("images/favicon.ico"))?;
    let mut configure_file = File::create(report_name.join("images/configure.png"))?;
    let mut index_html_file = File::create(report_name.join("index.html"))?;
    let mut index_css_file = File::create(report_name.join("index.css"))?;
    let mut index_js_file = File::create(report_name.join("index.js"))?;
    let mut analytics_js_file = File::create(report_name.join("js/analytics.js"))?;
    let mut utils_js_file = File::create(report_name.join("js/utils.js"))?;
    let mut plotly_js_file = File::create(report_name.join("js/plotly.js"))?;
    let mut configure_js_file = File::create(report_name.join("js/configure.js"))?;
    ico_file.write_all(ico)?;
    configure_file.write_all(configure)?;
    write!(index_html_file, "{}", index_html)?;
    write!(index_css_file, "{}", index_css)?;
    write!(index_js_file, "{}", index_js)?;
    write!(analytics_js_file, "{}", analytics_js)?;
    write!(utils_js_file, "{}", utils_js)?;
    write!(plotly_js_file, "{}", plotly_js)?;
    write!(configure_js_file, "{}", configure_js)?;

    let mut visualizer = VISUALIZATION_DATA.lock().unwrap();

    /* Init visualizers */
    for dir in dir_paths {
        let name = visualizer.init_visualizers(dir.to_owned(), tmp_dir, &report_name)?;
        visualizer.unpack_data(name)?;
    }

    /* Generate visualizer JS files */
    for (name, file) in visualizer.get_all_js_files()? {
        let mut created_file = File::create(report_name.join(format!("js/{}", name)))?;
        write!(created_file, "{}", file)?;
    }

    /* Generate run.js */
    let out_loc = report_name.join("data/js/runs.js");
    let mut runs_file = File::create(out_loc)?;
    write!(
        runs_file,
        "runs_raw = {}",
        serde_json::to_string(&run_names)?
    )?;
    let visualizer_names = visualizer.get_visualizer_names()?;

    /* Get visualizer data */
    for name in visualizer_names {
        let api_name = visualizer.get_api(name.clone())?;
        let calls = visualizer.get_calls(api_name.clone())?;
        let mut api = Api::new(name.clone());
        for run_name in &run_names {
            let mut run = Run::new(run_name.clone());
            if !visualizer.has_data(api_name.clone(), run_name.to_string())? {
                run.key_values
                    .insert("nodata".to_string(), "No data collected".to_string());
                api.runs.push(run);
                continue;
            }
            for call in &calls {
                if call == "keys" {
                    let data = visualizer.get_data(
                        run_name,
                        &api_name,
                        format!("run={}&get={}", run_name, call),
                    )?;
                    run.keys = serde_json::from_str::<Vec<String>>(&data)?.clone();
                }
                if call == "values" {
                    if !run.keys.is_empty() {
                        for key in &run.keys {
                            let data = visualizer.get_data(
                                run_name,
                                &api_name,
                                format!("run={}&get=values&key={}", run_name, key),
                            )?;
                            run.key_values.insert(key.clone(), data.clone());
                        }
                    } else {
                        let data = visualizer.get_data(
                            run_name,
                            &api_name,
                            format!("run={}&get=values", run_name),
                        )?;
                        run.key_values.insert(call.clone(), data.clone());
                    }
                }
            }
            api.runs.push(run);
        }
        let out_loc = report_name.join(format!("data/js/{}.js", api_name));
        let mut out_file = File::create(out_loc)?;
        let out_data = serde_json::to_string(&api)?;
        let str_out_data = format!("{}_raw_data = {}", api.name.clone(), out_data.clone());
        write!(out_file, "{}", str_out_data)?;
    }
    let out_analytics = report_name.join("data/js/analytics.js");
    let mut out_file = File::create(out_analytics)?;
    let stats = visualizer.get_analytics()?;
    let str_out_stats = format!("raw_analytics = {}", stats);
    write!(out_file, "{}", str_out_stats)?;
    /* Generate aperf_report.tar.gz */
    info!("Generating {}", report_name_tgz.display());
    let tar_gz = File::create(&report_name_tgz)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    let report_stem = report_name
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    tar.append_dir_all(&report_stem, &report_name)?;
    Ok(())
}
