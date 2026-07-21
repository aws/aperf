#![cfg(target_os = "linux")]

use crate::aperf_stats_flush;
use crate::aperf_stats_initialize;
use crate::data;
use crate::data::java_profile::JavaProfile;
use crate::data_collection::DataCollectionEngine;
use crate::data_collection::InitParams;
use crate::no_tar_gz_file_name;
use crate::{get_data_name_from_type, UNGROUPED_PMU_MODE};
use anyhow::bail;
use anyhow::Result;
use chrono::Utc;
use clap::{builder::PossibleValuesParser, ArgGroup, Args};
use flate2::write::GzEncoder;
use flate2::Compression;
use log::{debug, error, info, warn};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
#[clap(group(ArgGroup::new("customized-collection").args(&["dont_collect", "collect_only"])))]
pub struct Record {
    /// Name or path of the run.
    #[clap(help_heading = "Basic Options", short, long, value_parser)]
    pub run_name: Option<String>,

    /// Interval (in seconds) at which performance data is to be collected.
    #[clap(
        help_heading = "Basic Options",
        short,
        long,
        value_parser,
        default_value_t = 1
    )]
    pub interval: u64,

    /// Time (in seconds) for which the performance data is to be collected.
    #[clap(
        help_heading = "Basic Options",
        short,
        long,
        value_parser,
        default_value_t = 10
    )]
    pub period: u64,

    /// The list of performance data to skip collection. Cannot be used with --collect_only.
    #[clap(
        help_heading = "Data Selection",
        long,
        value_parser = PossibleValuesParser::new(data::DEFAULT_DATA_NAMES.as_slice()),
        value_names = &["Data Name>,<Data Name"],
        num_args = 1..,
        value_delimiter = ','
    )]
    pub dont_collect: Option<Vec<String>>,

    /// The list of performance data to be collected - the others will not be collected. Cannot be used with --dont_collect.
    #[clap(
        help_heading = "Data Selection",
        long,
        value_parser = PossibleValuesParser::new(data::DEFAULT_DATA_NAMES.as_slice()),
        value_names = &["Data Name>,<Data Name"],
        num_args = 1..,
        value_delimiter = ','
    )]
    pub collect_only: Option<Vec<String>>,

    /// Gather profiling data using 'perf' binary.
    #[clap(help_heading = "Profiling", long, value_parser)]
    pub profile: bool,

    /// Frequency for perf profiling (Hz).
    #[clap(
        help_heading = "Profiling",
        short = 'F',
        long,
        value_parser,
        default_value_t = 99
    )]
    pub perf_frequency: u32,

    /// Profile JVMs using async-profiler. Specify args using comma separated values. Profiles all JVMs if no args are provided.
    #[clap(
        help_heading = "Profiling",
        long, value_parser,
        default_missing_value = Some("jps"),
        value_names = &["PID/Name>,<PID/Name>,...,<PID/Name"],
        num_args = 0..=1
    )]
    pub profile_java: Option<String>,

    /// Save all profile events in the output file.
    #[clap(help_heading = "Profiling", long, value_parser, hide = true)]
    pub save_profile_events: bool,

    /// Custom PMU config file to use.
    #[clap(help_heading = "PMU Options", long, value_parser)]
    pub pmu_config: Option<String>,

    /// Avoid creating a PMU counter group for each metric defined in the
    /// PMU config. It reduces multiplexing and file descriptor usage, but
    /// counters used by a metric are no longer guaranteed to be collected
    /// together. Recommend to use the option when the total number of
    /// counters (events) to be collected is less than or equal to that of
    /// the PMU counter registers.
    #[clap(help_heading = "PMU Options", long, value_parser, verbatim_doc_comment)]
    pub ungroup_pmu_events: bool,

    #[cfg(feature = "hotline")]
    /// SPE sampling frequency, defaulted to 1kHz on Grv4.
    #[clap(
        help_heading = "Hotline Options",
        long,
        value_parser,
        default_value_t = 1000
    )]
    pub hotline_frequency: u32,

    #[cfg(feature = "hotline")]
    /// Maximum number of report entries to process for Hotline tables
    #[clap(
        help_heading = "Hotline Options",
        long,
        value_parser,
        default_value_t = 5000
    )]
    pub num_to_report: u32,
}

pub fn record(record: &Record, tmp_dir: &Path, runlog: &Path) -> Result<()> {
    if record.period == 0 {
        error!("Collection period cannot be 0.");
        bail!("Cannot start recording with the given parameters.");
    }
    if record.interval == 0 {
        error!("Collection interval cannot be 0.");
        bail!("Cannot start recording with the given parameters.");
    }
    // Check if interval > period , if so give error user and exit.
    if record.interval >= record.period {
        error!("The overall recording period of {period} seconds needs to be longer than the interval of {interval} seconds.\
                Please increase the overall recording period or decrease the interval.", interval = record.interval, period =record.period);
        bail!("Cannot start recording with the given parameters.");
    }

    // Parse and validate the provided run name or path. If it is not provided or invalid,
    // use the default name and path.
    let (run_name, run_data_dir) = match &record.run_name {
        Some(run_name_arg) => {
            // Reassemble the components to discard the trailing slash.
            let run_data_dir: PathBuf = PathBuf::from(run_name_arg).components().collect();
            match run_data_dir.file_name() {
                Some(file_name) => (file_name.to_string_lossy().into_owned(), run_data_dir),
                None => {
                    let run_name_and_dir = get_default_run_name_and_dir();
                    info!(
                        "Invalid run name or path {}. Using default path {}",
                        run_data_dir.display(),
                        run_name_and_dir.1.display()
                    );
                    run_name_and_dir
                }
            }
        }
        None => {
            let run_name_and_dir = get_default_run_name_and_dir();
            info!(
                "Run name or path not provided. Using default path {}",
                run_name_and_dir.1.display()
            );
            run_name_and_dir
        }
    };

    aperf_stats_initialize(run_data_dir.clone());

    let mut init_params = InitParams::new(run_name, run_data_dir.clone());
    init_params.period = record.period;
    init_params.interval = record.interval;
    init_params.tmp_dir = tmp_dir.to_path_buf();
    init_params.runlog = runlog.to_path_buf();
    init_params.page_size = match procfs::page_size() {
        Ok(page_size) => page_size as u64,
        Err(e) => {
            warn!(
                "Failed to read system page size, ResidentSetSize will be reported in pages: {e}"
            );
            0
        }
    };
    if let Some(p) = &record.pmu_config {
        init_params.pmu_config = Some(PathBuf::from(p));
    }
    if record.ungroup_pmu_events {
        init_params.pmu_counter_mode = UNGROUPED_PMU_MODE.to_string();
    }

    #[cfg(feature = "hotline")]
    {
        init_params.hotline_frequency = record.hotline_frequency;
        init_params.num_to_report = record.num_to_report;
    }

    match &record.profile_java {
        Some(j) => {
            init_params.profile.insert(
                String::from(get_data_name_from_type::<JavaProfile>()),
                j.clone(),
            );
        }
        None => {}
    }
    if record.profile {
        init_params.perf_frequency = record.perf_frequency;
    }
    init_params.save_profile_events = record.save_profile_events;

    if let Err(e) = fs::create_dir(&run_data_dir) {
        panic!(
            "Failed to create the run data directory at {}: {e}",
            run_data_dir.display()
        );
    }

    let mut data_collection_engine = DataCollectionEngine::new(init_params);

    data::initialize_data_collection_engine(
        &mut data_collection_engine,
        get_data_names_to_collect(&record.collect_only, &record.dont_collect),
        record.profile,
        record
            .profile_java
            .as_ref()
            .map_or(false, |j| !j.is_empty()),
    );

    info!("Starting Data collection...");

    info!("Preparing data collectors...");
    data_collection_engine.prepare_data_collectors()?;
    debug!("Collecting static data...");
    data_collection_engine.collect_static_data()?;
    info!("Collecting data...");
    data_collection_engine.collect_data_serial()?;
    info!("Finishing data collection...");
    data_collection_engine.finish_data_collection()?;
    info!("Data collection complete.");

    if let Err(e) = aperf_stats_flush() {
        error!("Failed to write APerf stats: {e}");
    }

    info!("Creating run data archive...");
    create_run_data_archive(&run_data_dir)?;

    Ok(())
}

pub const RECORD_DATA_RECOMMENDATION: &str = "we recommend to always collect as much data as possible for performance debugging, unless you are sure some data can be excluded.";

/// Compute the set of data names to be collected, based on the args collect_only and dont_collect.
/// Although not implemented here, the clap ArgGroup defined up makes sure that at most one of the
/// args is used.
fn get_data_names_to_collect(
    collect_only_arg: &Option<Vec<String>>,
    dont_collect_arg: &Option<Vec<String>>,
) -> HashSet<String> {
    if let Some(collect_only_data_names) = collect_only_arg {
        info!(
            "Since you used the --collect-only flag, please note that {}",
            RECORD_DATA_RECOMMENDATION
        );
        return collect_only_data_names
            .iter()
            .map(|data_name| data_name.clone())
            .collect();
    }

    let mut all_default_data_names_set: HashSet<String> = data::DEFAULT_DATA_NAMES
        .iter()
        .map(|&data_name| data_name.to_string())
        .collect();

    if let Some(dont_collect_data_names) = dont_collect_arg {
        for data_name in dont_collect_data_names {
            all_default_data_names_set.remove(data_name);
        }
    };

    all_default_data_names_set
}

pub fn get_default_run_name_and_dir() -> (String, PathBuf) {
    let default_run_name = format!(
        "aperf_{}",
        Utc::now().format("%Y-%m-%d_%H_%M_%S").to_string()
    );
    let default_run_data_dir = PathBuf::from(format!("./{default_run_name}"));

    (default_run_name, default_run_data_dir)
}

pub fn create_run_data_archive(run_data_dir: &PathBuf) -> Result<()> {
    let dir_name = no_tar_gz_file_name(run_data_dir).unwrap();
    let archive_path = PathBuf::from(format!("{}.tar.gz", run_data_dir.display()));
    let tar_gz = fs::File::create(&archive_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(dir_name, run_data_dir)?;
    info!(
        "Data collected in {}/ and archive available at {}",
        run_data_dir.display(),
        archive_path.display(),
    );
    Ok(())
}
