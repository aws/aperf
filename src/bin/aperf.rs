use anyhow::Result;
use aperf::pmu::{custom_pmu, CustomPMU};
use aperf::record::{record, Record, RECORD_DATA_RECOMMENDATION};
use aperf::report::{report, Report};
use aperf::{PDError, APERF_RUNLOG, APERF_TMP};
use clap::{Parser, Subcommand};
use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    append::file::FileAppender,
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
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
    #[clap(help_heading = "Basic Options", short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Temporary directory for intermediate files.
    #[clap(help_heading = "Basic Options", short, long, value_parser, default_value_t = APERF_TMP.to_string(), global = true)]
    tmp_dir: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Collect performance data.
    #[command(after_help = RECORD_DATA_RECOMMENDATION.to_ascii_uppercase())]
    Record(Record),

    /// Generate an HTML report based on the data collected.
    Report(Report),

    /// Create a custom PMU configuration file for use with Aperf record.
    CustomPMU(CustomPMU),
}

fn init_logger(verbose: u8, runlog: &PathBuf) -> Result<()> {
    let level = match verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Trace,
        _ => return Err(PDError::InvalidVerboseOption.into()),
    };
    let pattern = "[{d(%Y-%m-%dT%H:%M:%SZ)} {h({l}):5.5} {M}] {m}{n}";
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build();

    let fileout = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build(runlog)?;

    let config = Config::builder()
        /* This prints only the user selected level to the console. */
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(level)))
                .build("stdout", Box::new(stdout)),
        )
        .appender(Appender::builder().build("aperflog", Box::new(fileout)))
        /* This creates a logger for our module at a default Debug level. */
        .logger(
            Logger::builder()
                .appender("aperflog")
                .appender("stdout")
                .build(env!("CARGO_CRATE_NAME"), LevelFilter::Debug),
        )
        .build(
            /* Set the Root to Warn. Underlying dependencies also print if set to debug.
             * See: https://github.com/estk/log4rs/issues/196
             */
            Root::builder().build(LevelFilter::Warn),
        )?;

    log4rs::init_config(config)?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let tmp_dir = TempBuilder::new()
        .prefix("aperf-tmp-")
        .tempdir_in(&cli.tmp_dir)?;
    fs::set_permissions(&tmp_dir, fs::Permissions::from_mode(0o1777))?;
    let tmp_dir_path_buf = tmp_dir.path().to_path_buf();
    let runlog = tmp_dir_path_buf.join(*APERF_RUNLOG);

    init_logger(cli.verbose, &runlog)?;

    match cli.command {
        Commands::Record(r) => record(&r, &tmp_dir_path_buf, &runlog),
        Commands::Report(r) => report(&r, &tmp_dir_path_buf),
        Commands::CustomPMU(r) => custom_pmu(&r),
    }?;
    fs::remove_dir_all(tmp_dir_path_buf)?;
    Ok(())
}
