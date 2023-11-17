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
    #[clap(short, long, value_parser)]
    run: Vec<String>,

    /// Report name.
    #[clap(short, long, value_parser)]
    name: Option<String>,
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

pub static APERF_TMP: &str = "aperf_tmp";

pub fn form_and_copy_archive(loc: String, report_name: &Path) -> Result<()> {
    if Path::new(&loc).is_dir() {
        let dir_stem = Path::new(&loc)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        /* Create a temp archive */
        let archive_name = format!("{}.tar.gz", &dir_stem);
        let archive_path = format!("{}/{}", APERF_TMP, archive_name);
        let archive_dst = report_name.join(format!("data/archive/{}", archive_name));
        {
            let tar_gz = fs::File::create(&archive_path)?;
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all(&dir_stem, &loc)?;
        }

        /* Copy archive to aperf_report */
        fs::copy(&archive_path, archive_dst)?;
        return Ok(());
    }
    if infer::get_from_path(&loc)?.unwrap().mime_type() == "application/gzip" {
        let file_name = Path::new(&loc)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        /* Copy archive to aperf_report */
        let archive_dst = report_name.join(format!("data/archive/{}", file_name));
        fs::copy(loc, archive_dst)?;
        return Ok(());
    }
    Err(PDError::RecordNotArchiveOrDirectory.into())
}

pub fn get_dir(dir: String) -> Result<String> {
    /* If dir return */
    if Path::new(&dir).is_dir() {
        return Ok(dir);
    }
    /* Unpack if archive */
    if infer::get_from_path(&dir)?.unwrap().mime_type() == "application/gzip" {
        let tar_gz = File::open(&dir)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(APERF_TMP)?;
        let dir_name = Path::new(&dir)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".tar.gz")
            .ok_or(PDError::InvalidArchiveName)?;
        if !Path::new(&format!("{}/{}", APERF_TMP, dir_name)).exists() {
            return Err(PDError::ArchiveDirectoryMismatch.into());
        }
        return Ok(format!("{}/{}", APERF_TMP, dir_name));
    }
    Err(PDError::RecordNotArchiveOrDirectory.into())
}

pub fn report(report: &Report) -> Result<()> {
    let dirs: Vec<String> = report.run.clone();
    let mut dir_paths: Vec<String> = Vec::new();
    let mut dir_stems: Vec<String> = Vec::new();

    /* Create a tmp dir for aperf to work with */
    fs::create_dir_all(APERF_TMP)?;

    /* Get dir paths, stems */
    for dir in &dirs {
        let directory = get_dir(dir.to_string())?;
        let path = Path::new(&directory);
        if dir_stems.contains(&path.file_stem().unwrap().to_str().unwrap().to_string()) {
            error!("Cannot process two runs with the same name");
            return Ok(());
        }
        dir_stems.push(path.file_stem().unwrap().to_str().unwrap().to_string());
        dir_paths.push(path.to_str().unwrap().to_string());
    }

    let mut report_name = PathBuf::new();
    match &report.name {
        Some(n) => report_name.push(n),
        None => {
            /* Generate report name */
            let mut file_name = "aperf_report".to_string();
            for stem in &dir_stems {
                let name = if stem.ends_with(".tar.gz") {
                    stem.strip_suffix(".tar.gz").unwrap().to_string()
                } else {
                    stem.to_string()
                };
                file_name = format!("{}_{}", file_name, name);
            }
            report_name.push(file_name);
            info!("Report name not given. Using '{}'", report_name.display());
        }
    }
    let mut report_name_tgz = PathBuf::new();
    report_name_tgz.set_file_name(&report_name);
    report_name_tgz.set_extension("tar.gz");

    info!("Creating APerf report...");
    let ico = include_bytes!("html_files/favicon.ico");
    let index_html = include_str!("html_files/index.html");
    let index_css = include_str!("html_files/index.css");
    let index_js = include_str!(concat!(env!("JS_DIR"), "/index.js"));
    let utils_js = include_str!(concat!(env!("JS_DIR"), "/utils.js"));
    let plotly_js = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/node_modules/plotly.js/dist/plotly.min.js"
    ));
    let run_names = dir_stems.clone();

    fs::create_dir_all(report_name.join("js"))?;
    fs::create_dir_all(report_name.join("data/archive"))?;
    fs::create_dir_all(report_name.join("data/js"))?;

    /* Generate/copy the archives of the collected data into aperf_report */
    for dir in &dirs {
        form_and_copy_archive(dir.clone(), &report_name)?;
    }
    /* Generate base HTML, JS files */
    let mut ico_file = File::create(report_name.join("favicon.ico"))?;
    let mut index_html_file = File::create(report_name.join("index.html"))?;
    let mut index_css_file = File::create(report_name.join("index.css"))?;
    let mut index_js_file = File::create(report_name.join("index.js"))?;
    let mut utils_js_file = File::create(report_name.join("js/utils.js"))?;
    let mut plotly_js_file = File::create(report_name.join("js/plotly.js"))?;
    ico_file.write_all(ico)?;
    write!(index_html_file, "{}", index_html)?;
    write!(index_css_file, "{}", index_css)?;
    write!(index_js_file, "{}", index_js)?;
    write!(utils_js_file, "{}", utils_js)?;
    write!(plotly_js_file, "{}", plotly_js)?;

    let mut visualizer = VISUALIZATION_DATA.lock().unwrap();
    /* Init visualizers */
    for dir in dir_paths {
        let name = visualizer.init_visualizers(
            dir.to_owned(),
            APERF_TMP.to_string(),
            report_name.clone(),
        )?;
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
            let mut temp_keys: Vec<String> = Vec::<String>::new();
            let mut run = Run::new(run_name.clone());
            let mut keys = false;
            for call in &calls {
                let query = format!("run={}&get={}", run_name, call);
                let mut data;
                if call == "keys" {
                    data = visualizer.get_data(run_name, &api_name, query)?;
                    if data != "No data collected" {
                        temp_keys = serde_json::from_str(&data)?;
                    }
                    run.keys = temp_keys.clone();
                    keys = true;
                }
                if call == "values" {
                    if keys {
                        for key in &temp_keys {
                            let query = format!("run={}&get=values&key={}", run_name, key);
                            data = visualizer.get_data(run_name, &api_name, query)?;
                            run.key_values.insert(key.clone(), data.clone());
                        }
                    } else {
                        let query = format!("run={}&get=values", run_name);
                        data = visualizer.get_data(run_name, &api_name, query)?;
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
    /* Generate aperf_report.tar.gz */
    info!("Generating {}", report_name_tgz.display());
    let tar_gz = File::create(&report_name_tgz)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(&report_name, &report_name)?;
    fs::remove_dir_all(APERF_TMP)?;
    Ok(())
}
