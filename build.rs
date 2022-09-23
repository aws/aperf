use anyhow::Result;
use std::process::Command;
use vergen::{Config, ShaKind, vergen};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=src/bin/html_files/");
    Command::new("tsc")
        .arg("-b")
        .arg("src/bin/html_files/")
        .arg("--verbose")
        .spawn()?
        .wait()?;
    let mut config = Config::default();
    *config.git_mut().sha_kind_mut() = ShaKind::Short;
    vergen(config)
}
