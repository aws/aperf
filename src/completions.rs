use anyhow::Result;
use clap::{Args, Command};
use clap_complete::{generate, Shell};
use log::{info, warn};
use std::fs::{self, File};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct SetupShellCompletions {
    /// Shell to generate completions for.
    #[clap(long, value_parser)]
    pub shell: Shell,

    /// Install the auto complete script using sudo, or specify a download path.
    #[clap(
        long,
        value_parser,
        default_missing_value = Some(""),
        value_names = &["Path including filename"],
        num_args = 0..=1
    )]
    pub install: Option<String>,
}

pub fn setup_shell_completions(args: &SetupShellCompletions, cmd: &mut Command) -> Result<()> {
    let gen = args.shell;
    let app_name = cmd.get_name();

    if let Some(custom_path) = &args.install {
        let install_path = if custom_path.is_empty() {
            match gen {
                Shell::Bash => {
                    let path = PathBuf::from("/usr/share/bash-completion/completions");
                    fs::create_dir_all(&path)?;
                    path.join(app_name)
                }
                Shell::Zsh => {
                    let path = PathBuf::from("/usr/local/share/zsh/site-functions");
                    fs::create_dir_all(&path)?;
                    path.join(format!("_{}", app_name))
                }
                _ => {
                    warn!("Could not install automatically!");
                    PathBuf::from(format!("{}_completions.sh", app_name))
                }
            }
        } else {
            // Custom path specified
            PathBuf::from(custom_path)
        };

        let mut file = File::create(&install_path)?;
        generate(gen, cmd, app_name.to_string(), &mut file);
        info!("Completion script generated to {}", install_path.display());
    } else {
        let file_name = format!("{}_completions.sh", app_name);
        let mut file = File::create(&file_name)?;
        generate(gen, cmd, app_name.to_string(), &mut file);
        info!(
            "Auto complete script generated! Enable by running `source {}` or installing",
            file_name
        );
    }
    Ok(())
}
