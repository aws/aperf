#[macro_use]
extern crate log;
use env_logger::Env;

use clap::Parser;
use performance_data::PERFORMANCE_DATA;
use performance_data::{InitParams, PDResult};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Interval (in seconds) at which performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 1)]
    interval: u64,

    /// Time (in seconds) for which the performance data is to be collected.
    #[clap(short, long, value_parser, default_value_t = 10)]
    period: u64,
}

fn init_logger() {
    let env = Env::default().filter("PDA_LOG_LEVEL");

    env_logger::init_from_env(env);
}

fn start_collection_serial() -> PDResult {
    info!("Collecting data serially...");
    PERFORMANCE_DATA.lock().unwrap().collect_data_serial()?;
    Ok(())
}

fn collect_data_once() -> PDResult {
    info!("Collecting data only once...");
    PERFORMANCE_DATA.lock().unwrap().collect_data_once()?;
    Ok(())
}

fn main() -> PDResult {
    let args = Args::parse();
    let mut params = InitParams::new();

    /* Initialize logging system */
    init_logger();

    params.period = args.period;
    params.interval = args.interval;
    PERFORMANCE_DATA.lock().unwrap().set_params(params);
    PERFORMANCE_DATA.lock().unwrap().init_collectors()?;
    info!("Starting Performance Data collection:");
    collect_data_once()?;
    start_collection_serial()?;
    Ok(())
}
