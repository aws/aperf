#[macro_use]
extern crate log;

use anyhow::Result;
use env_logger::Env;
use clap::Parser;
use aperf::{InitParams, PERFORMANCE_DATA};

#[derive(Parser, Debug)]
#[clap(author, about, long_about = None)]
#[clap(name = "aperf-collector")]
#[clap(version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA_SHORT"), ")"))]
struct Args {
    /// Interval (in seconds) at which performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 1)]
    interval: u64,

    /// Time (in seconds) for which the performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 10)]
    period: u64,

    /// Name of the run.
    #[clap(short, long, value_parser)]
    run_name: Option<String>,
}

fn init_logger() {
    let env = Env::default().filter_or("APERF_LOG_LEVEL", "info");

    env_logger::init_from_env(env);
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
    info!("Collecting static data...");
    PERFORMANCE_DATA.lock().unwrap().collect_static_data()?;
    Ok(())
}

fn main() -> Result<()> {
    /* Initialize logging system */
    init_logger();
    info!("To see debug messages export APERF_LOG_LEVEL=[debug|trace]");

    let args = Args::parse();
    let mut run_name = String::new();
    match args.run_name {
        Some(r) => run_name = r,
        None => {},
    }
    let mut params = InitParams::new(run_name);
    params.period = args.period;
    params.interval = args.interval;

    PERFORMANCE_DATA.lock().unwrap().set_params(params);
    PERFORMANCE_DATA.lock().unwrap().init_collectors()?;
    info!("Starting Data collection...");
    prepare_data_collectors()?;
    collect_static_data()?;
    start_collection_serial()?;
    info!("Data collection complete.");
    Ok(())
}
