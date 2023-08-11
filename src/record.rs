use anyhow::Result;
use clap::Args;
use crate::{InitParams, PERFORMANCE_DATA};
use log::{debug, error, info};

#[derive(Args, Debug)]
pub struct Record {
    /// Name of the run.
    #[clap(short, long, value_parser)]
    run_name: Option<String>,

    /// Interval (in seconds) at which performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 1)]
    interval: u64,

    /// Time (in seconds) for which the performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 10)]
    period: u64,

    /// Gather profiling data using the 'perf' binary.
    #[clap(long, value_parser, default_value_t = false)]
    profile: bool,
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

fn collect_static_data() -> Result<()>  {
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
        None => {},
    }
    let mut params = InitParams::new(run_name);
    params.period = record.period;
    params.profile = record.profile;
    params.interval = record.interval;

    PERFORMANCE_DATA.lock().unwrap().set_params(params);
    PERFORMANCE_DATA.lock().unwrap().init_collectors()?;
    info!("Starting Data collection...");
    prepare_data_collectors()?;
    collect_static_data()?;
    start_collection_serial()?;
    info!("Data collection complete.");
    Ok(())
}
