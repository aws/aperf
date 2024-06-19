use crate::{data, InitParams, PERFORMANCE_DATA};
use anyhow::Result;
use clap::{Args, Subcommand};
use log::{debug, error, info};
use struct_iterable::Iterable;

#[derive(Subcommand, Debug)]
pub enum ProfilerSubCommand {
    //Use profilers
    #[clap(name = "--profile")]
    Profile(Profile),
}
#[derive(Args, Debug, Iterable)]
pub struct Profile {
    /// Gather profiling data using 'perf' binary. Automatically selected if no options are passed to '--profile'.
    #[clap(short='p', long="perf", value_parser)]
    pub perf_profile: bool,

    /// Profile JVM using async-profiler. Specify args using comma separated values. Profiles all currently running JVMs if no args are provided.
    #[clap(short='j', long="java", value_parser, default_missing_value=Some("jps"), value_names=&["PID/Name>,<PID/Name>,...,<PID/Name"], num_args=0..=1)]
    pub java_profile: Option<String>,
}

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

    /// Gather profiling data using the 'perf' binary or additional profilers.
    #[command(subcommand)]
    pub profile: Option<ProfilerSubCommand>,
}

fn prepare_data_collectors() -> Result<()> {
    info!("Preparing data collectors...");
    PERFORMANCE_DATA.lock().unwrap().prepare_data_collectors()?;
    Ok(())
}

fn start_collection_serial() -> Result<()> {
    info!("Collecting data...");
    PERFORMANCE_DATA.lock().unwrap().collect_data_serial()?;
    Ok(())
}

fn collect_static_data() -> Result<()> {
    debug!("Collecting static data...");
    PERFORMANCE_DATA.lock().unwrap().collect_static_data()?;
    Ok(())
}

pub fn record(record: &Record) -> Result<()> {
    let mut run_name = String::new();
    if record.period == 0 {
        error!("Collection period cannot be 0.");
        return Ok(());
    }
    if record.interval == 0 {
        error!("Collection interval cannot be 0.");
        return Ok(());
    }
    match &record.run_name {
        Some(r) => run_name = r.clone(),
        None => {}
    }
    let mut params = InitParams::new(run_name);
    params.period = record.period;
    params.interval = record.interval;

    match &record.profile {
        Some(ProfilerSubCommand::Profile(p)) => {
            for (field, value) in p.iter() {
                match value.downcast_ref::<Option<String>>().unwrap_or(&None) {
                    Some(v) => {
                        params.profile.insert(String::from(field), v.clone());
                    }
                    None => {}
                }
            }
            if params.profile.is_empty() || p.perf_profile {
                params.profile.insert(
                    String::from(data::perf_profile::PERF_PROFILE_FILE_NAME),
                    String::new(),
                );
                params.profile.insert(
                    String::from(data::flamegraphs::FLAMEGRAPHS_FILE_NAME),
                    String::new(),
                );
            }
        }
        None => {}
    }

    PERFORMANCE_DATA.lock().unwrap().set_params(params);
    PERFORMANCE_DATA.lock().unwrap().init_collectors()?;
    info!("Starting Data collection...");
    prepare_data_collectors()?;
    collect_static_data()?;
    start_collection_serial()?;
    info!("Data collection complete.");
    Ok(())
}
