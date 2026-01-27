use anyhow::Result;
use clap::{Args, Command};
use clap_complete::{generate, Shell};
use log::{info, warn};
use std::env;
#[cfg(not(target_os = "windows"))]
use std::fs::{self, File};
use std::io;
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

    // PowerShell autocomplete script needs to be added to the profile file. To avoid introducing
    // unexpected changes to an existing file, generate the script to stdout and let the users edit
    // the profile file manually
    if gen == Shell::PowerShell {
        let powershell_profile_path = match env::var("USERPROFILE") {
            Ok(val) => {
                let mut path = PathBuf::from(val);
                path.push("Documents");
                path.push("WindowsPowerShell");
                path.push("Microsoft.PowerShell_profile.ps1");
                path.into_os_string()
                    .into_string()
                    .unwrap_or(String::from("the Powershell profile"))
            }
            Err(_) => String::from("the Powershell profile"),
        };

        info!(
            "Paste the below autocomplete script to {}",
            powershell_profile_path
        );
        generate(gen, cmd, app_name.to_string(), &mut io::stdout());
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    // On Windows don't (and can't) install anything, so always just output the autocomplete
    // script to stdio
    {
        if let Some(_) = &args.install {
            warn!("The script cannot be installed on Windows.")
        }
        info!("The auto-completion script is generated below:");
        generate(gen, cmd, app_name.to_string(), &mut io::stdout());
    }

    #[cfg(not(target_os = "windows"))]
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
        info!("The auto-completion script is generated below:");
        generate(gen, cmd, app_name.to_string(), &mut io::stdout());
    }

    Ok(())
}
