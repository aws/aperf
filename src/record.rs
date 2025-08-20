use crate::data::java_profile::JavaProfile;
use crate::utils::get_data_name_from_type;
use crate::{data, InitParams, PerformanceData};
use anyhow::anyhow;
use anyhow::Result;
use clap::Args;
use log::{debug, error, info};
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct Record {
    /// Name of the run.
    #[clap(short, long, value_parser)]
    pub run_name: Option<String>,

    /// Interval (in seconds) at which performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 1)]
    pub interval: u64,

    /// Time (in seconds) for which the performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 10)]
    pub period: u64,

    /// Gather profiling data using 'perf' binary.
    #[clap(long, value_parser)]
    pub profile: bool,

    /// Frequency for perf profiling (Hz).
    #[clap(short = 'F', long, value_parser, default_value_t = 99)]
    pub perf_frequency: u32,

    /// Profile JVMs using async-profiler. Specify args using comma separated values. Profiles all JVMs if no args are provided.
    #[clap(long, value_parser, default_missing_value = Some("jps"), value_names = &["PID/Name>,<PID/Name>,...,<PID/Name"], num_args = 0..=1)]
    pub profile_java: Option<String>,

    /// Custom PMU config file to use.
    #[clap(long, value_parser)]
    pub pmu_config: Option<String>,

    #[cfg(feature = "hotline")]
    /// SPE sampling frequency, defaulted to 1kHz on Grv4.
    #[clap(long, value_parser, default_value_t = 1000)]
    pub hotline_frequency: u32,

    #[cfg(feature = "hotline")]
    /// Maximum number of report entries to process for Hotline tables
    #[clap(long, value_parser, default_value_t = 5000)]
    pub num_to_report: u32,
}

pub fn record(record: &Record, tmp_dir: &Path, runlog: &Path) -> Result<()> {
    let mut run_name = String::new();
    if record.period == 0 {
        error!("Collection period cannot be 0.");
        return Err(anyhow!("Cannot start recording with the given parameters."));
    }
    if record.interval == 0 {
        error!("Collection interval cannot be 0.");
        return Err(anyhow!("Cannot start recording with the given parameters."));
    }
    // Check if interval > period , if so give error user and exit.
    if record.interval >= record.period {
        error!("The overall recording period of {period} seconds needs to be longer than the interval of {interval} seconds.\
                Please increase the overall recording period or decrease the interval.", interval = record.interval, period =record.period);
        return Err(anyhow!("Cannot start recording with the given parameters."));
    }
    match &record.run_name {
        Some(r) => run_name = r.clone(),
        None => {}
    }
    let mut params = InitParams::new(run_name);
    params.period = record.period;
    params.interval = record.interval;
    params.tmp_dir = tmp_dir.to_path_buf();
    params.runlog = runlog.to_path_buf();
    if let Some(p) = &record.pmu_config {
        params.pmu_config = Some(PathBuf::from(p));
    }

    #[cfg(feature = "hotline")]
    {
        params.hotline_frequency = record.hotline_frequency;
        params.num_to_report = record.num_to_report;
    }

    match &record.profile_java {
        Some(j) => {
            params.profile.insert(
                String::from(get_data_name_from_type::<JavaProfile>()),
                j.clone(),
            );
        }
        None => {}
    }
    if record.profile {
        params.perf_frequency = record.perf_frequency;
    }

    let mut performance_data = PerformanceData::new(params);

    data::add_all_performance_data(
        &mut performance_data,
        record.profile,
        record
            .profile_java
            .as_ref()
            .map_or(false, |j| !j.is_empty()),
    );

    performance_data.init_collectors()?;
    info!("Starting Data collection...");

    info!("Preparing data collectors...");
    performance_data.prepare_data_collectors()?;
    debug!("Collecting static data...");
    performance_data.collect_static_data()?;
    info!("Collecting data...");
    performance_data.collect_data_serial()?;

    info!("Data collection complete.");
    performance_data.end()?;

    Ok(())
}
