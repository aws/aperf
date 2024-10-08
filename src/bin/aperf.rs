use anyhow::Result;
use aperf_lib::record::{record, Record};
use aperf_lib::report::{report, Report};
use aperf_lib::{PDError, APERF_TMP};
use clap::{Parser, Subcommand};
use env_logger::Builder;
use log::LevelFilter;
use std::{fs, os::unix::fs::PermissionsExt};
use tempfile::Builder as TempBuilder;

#[derive(Parser)]
#[command(author, about, long_about = None)]
#[command(name = "aperf")]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")"))]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Show debug messages. Use -vv for more verbose messages.
    #[clap(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Temporary directory for intermediate files.
    #[clap(short, long, value_parser, default_value_t = APERF_TMP.to_string(), global = true)]
    tmp_dir: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Collect performance data.
    Record(Record),

    /// Generate an HTML report based on the data collected.
    Report(Report),
}

fn init_logger(verbose: u8) -> Result<()> {
    let level = match verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Trace,
        _ => return Err(PDError::InvalidVerboseOption.into()),
    };
    Builder::new().filter_level(level).init();
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let tmp_dir = TempBuilder::new()
        .prefix("aperf-tmp-")
        .tempdir_in(&cli.tmp_dir)?;
    fs::set_permissions(&tmp_dir, fs::Permissions::from_mode(0o1777))?;
    let tmp_dir_path_buf = tmp_dir.path().to_path_buf();

    init_logger(cli.verbose)?;

    match cli.command {
        Commands::Record(r) => record(&r, &tmp_dir_path_buf),
        Commands::Report(r) => report(&r, &tmp_dir_path_buf),
    }?;
    fs::remove_dir_all(tmp_dir_path_buf)?;
    Ok(())
}
