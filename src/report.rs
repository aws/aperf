use crate::data::data_formats::AperfData;
use crate::data::JS_DIR;
use crate::{data, PDError, VisualizationData};
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
    /// The paths to the directories or archives of the recorded data to be included in the report.
    #[clap(help_heading = "Basic Options", short, long, value_parser, required = true, value_names = &["RUN_NAME> <RUN_NAME"], num_args = 1..)]
    pub run: Vec<String>,

    /// The directory and archive name of the report.
    #[clap(help_heading = "Basic Options", short, long, value_parser)]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ReportData {
    data_name: String,
    data_format: String,
    runs: HashMap<String, AperfData>,
}

impl ReportData {
    #[cfg(feature = "new-report")]
    fn new(data_name: String) -> Self {
        ReportData {
            data_name,
            data_format: String::new(),
            runs: HashMap::new(),
        }
    }
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

    #[cfg(feature = "new-report")]
    {
        generate_report_files(
            report_name.clone(),
            &dir_names,
            &data_dirs,
            &dir_paths,
            tmp_dir,
        );
        return Ok(());
    }

    info!("Creating APerf report...");
    let ico = include_bytes!("html_files/favicon.ico");
    let configure = include_bytes!("html_files/configure.png");
    let index_html = include_str!("html_files/index.html");
    let index_css = include_str!("html_files/index.css");
    let index_js_bytes = JS_DIR.get_file("index.js").unwrap();
    let index_js = index_js_bytes.contents_utf8().unwrap();
    let utils_js_bytes = JS_DIR.get_file("utils.js").unwrap();
    let utils_js = utils_js_bytes.contents_utf8().unwrap();
    let analytics_js_bytes = JS_DIR.get_file("analytics.js").unwrap();
    let analytics_js = analytics_js_bytes.contents_utf8().unwrap();
    let configure_js_bytes = JS_DIR.get_file("configure.js").unwrap();
    let configure_js = configure_js_bytes.contents_utf8().unwrap();
    let plotly_js = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/node_modules/plotly.js/dist/plotly.min.js"
    ));
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

    let mut visualization_data = VisualizationData::new();

    data::add_all_visualization_data(&mut visualization_data);

    /* Init visualizers */
    for dir in dir_paths {
        let name = visualization_data.init_visualizers(dir.to_owned(), tmp_dir, &report_name)?;
        visualization_data.unpack_data(name.clone())?;
    }

    /* Generate visualizer JS files */
    for (name, file) in visualization_data.get_all_js_files()? {
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
    let visualizer_names = visualization_data.get_visualizer_names()?;

    /* Get visualizer data */
    for name in visualizer_names {
        let api_name = visualization_data.get_api(name.clone())?;
        let calls = visualization_data.get_calls(api_name.clone())?;
        let mut api = Api::new(name.clone());
        for run_name in &run_names {
            let mut run = Run::new(run_name.clone());

            if !visualization_data.is_data_available(run_name, &name) {
                api.runs.push(run);
                continue;
            }

            let mut temp_keys: Vec<String> = Vec::<String>::new();
            let mut keys = false;
            for call in &calls {
                let query = format!("run={}&get={}", run_name, call);
                let mut data;
                if call == "keys" {
                    data = visualization_data.get_data(run_name, &api_name, query)?;
                    temp_keys = serde_json::from_str(&data)?;
                    run.keys = temp_keys.clone();
                    keys = true;
                }
                if call == "values" {
                    if keys {
                        for key in &temp_keys {
                            let query = format!("run={}&get=values&key={}", run_name, key);
                            data =
                                visualization_data.get_data(run_name, &api_name, query.clone())?;
                            run.key_values.insert(key.clone(), data.clone());
                        }
                    } else {
                        let query = format!("run={}&get=values", run_name);
                        data = visualization_data.get_data(run_name, &api_name, query)?;
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
    let stats = visualization_data.get_analytics()?;
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

#[cfg(feature = "new-report")]
fn generate_report_files(
    report_dir: PathBuf,
    run_names: &Vec<String>,
    raw_run_paths: &Vec<PathBuf>,
    run_dir_paths: &Vec<String>,
    tmp_dir: &PathBuf,
) {
    info!("Creating APerf report...");
    let report_data_dir = report_dir.join("data");
    fs::create_dir_all(report_data_dir.join("archive")).unwrap();
    let report_data_js_dir = report_data_dir.join("js");
    fs::create_dir_all(report_data_js_dir.clone()).unwrap();

    info!("Processing collected data...");
    let mut visualization_data = VisualizationData::new();
    data::add_all_visualization_data(&mut visualization_data);
    /* Init visualizers */
    for run_dir in run_dir_paths {
        let name = visualization_data
            .init_visualizers(run_dir.to_owned(), tmp_dir, &report_dir)
            .unwrap();
        visualization_data.unpack_data_new(name).unwrap();
    }

    /* Generate run.js */
    let run_js_path = report_data_js_dir.join("runs.js");
    let mut runs_file = File::create(run_js_path).unwrap();
    write!(
        runs_file,
        "runs_raw = {}",
        serde_json::to_string(run_names).unwrap()
    )
    .unwrap();

    JS_DIR
        .extract(&report_dir)
        .expect("Failed to copy frontend files");

    let visualizer_names = visualization_data.get_visualizer_names().unwrap(); // TODO: remove after replacing old get visualizer data
    for name in visualizer_names {
        let data_name = visualization_data.get_api(name.clone()).unwrap();
        let processed_data_js_path = report_data_js_dir.join(format!("{}.js", data_name));
        let mut processed_data_js_file = File::create(processed_data_js_path).unwrap();
        let mut report_data = ReportData::new(data_name.clone());
        for run_name in run_names {
            let visualizer = visualization_data
                .visualizers
                .get_mut(&name)
                .ok_or(PDError::VisualizerHashMapEntryError(name.clone()))
                .unwrap();
            let data = match visualizer.run_values_new.get(run_name) {
                Some(data) => data,
                None => continue,
            };
            report_data.runs.insert(run_name.clone(), data.clone());
            report_data.data_format = data.get_format_name();
        }
        let out_data = serde_json::to_string(&report_data).unwrap();
        write!(
            processed_data_js_file,
            "processed_{}_data = {}",
            data_name, out_data
        )
        .unwrap();
    }

    /* Generate/copy the archives of the collected data into aperf_report */
    for dir in raw_run_paths {
        form_and_copy_archive(dir.to_path_buf(), &report_dir, tmp_dir).unwrap();
    }

    let mut report_name_tgz = PathBuf::new();
    // If a user provided run name has a '.' in it, setting the extension as '.tar.gz'
    // here will overwrite the run name after the '.'. To prevent that set the filename.
    report_name_tgz.set_file_name(report_dir.to_str().unwrap().to_owned() + ".tar.gz");

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
