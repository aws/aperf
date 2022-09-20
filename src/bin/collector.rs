#[macro_use]
extern crate log;

use anyhow::Result;
use env_logger::Env;
use clap::Parser;
use performance_data::{InitParams, PERFORMANCE_DATA};

#[derive(Parser, Debug)]
#[clap(author, about, long_about = None)]
#[clap(name = "performance-data-collector")]
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
    run_name: String,
}

fn init_logger() {
    let env = Env::default().filter("PDA_LOG_LEVEL");

    env_logger::init_from_env(env);
}

fn start_collection_serial() -> Result<()> {
    info!("Collecting data serially...");
    PERFORMANCE_DATA.lock().unwrap().collect_data_serial()?;
    Ok(())
}

fn collect_static_data() -> Result<()> {
    info!("Collecting data only once...");
    PERFORMANCE_DATA.lock().unwrap().collect_static_data()?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut params = InitParams::new();

    /* Initialize logging system */
    init_logger();

    params.period = args.period;
    params.interval = args.interval;
    params.run_name = args.run_name;
    PERFORMANCE_DATA.lock().unwrap().set_params(params);
    PERFORMANCE_DATA.lock().unwrap().init_collectors()?;
    info!("Starting Performance Data collection:");
    collect_static_data()?;
    start_collection_serial()?;
    Ok(())
}
