use anyhow::Result;
use env_logger::Builder;
use log::LevelFilter;
use clap::{Parser, Subcommand};
use aperf_lib::record::{record, Record};
use aperf_lib::report::{report, Report};
use aperf_lib::PDError;

#[derive(Parser)]
#[command(author, about, long_about = None)]
#[command(name = "aperf")]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA_SHORT"), ")"))]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Show debug messages. Use -vv for more verbose messages.
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Collect performance data.
    Record(Record),

    /// Generate an HTML report based on the data collected.
    Report(Report),
}

fn init_logger(verbose: u8) -> Result<()> {
    let level;
    match verbose {
        0 => level = LevelFilter::Info,
        1 => level = LevelFilter::Debug,
        2 => level = LevelFilter::Trace,
        _ => return Err(PDError::InvalidVerboseOption.into()),
    }
    Builder::new().filter_level(level).init();
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logger(cli.verbose)?;

    match &cli.command {
        Commands::Record(r) => record(r),
        Commands::Report(r) => report(r),
    }?;
    Ok(())
}
