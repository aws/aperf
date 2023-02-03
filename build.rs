use anyhow::Result;
use std::process::Command;
use vergen::{Config, ShaKind, vergen};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");
    let status = Command::new("npm")
        .arg("install")
        .spawn()?
        .wait()?;
    if ! status.success() {
        std::process::exit(1);
    }

    println!("cargo:rerun-if-changed=src/bin/html_files/");
    let status = Command::new("npm")
        .arg("exec")
        .arg("--")
        .arg("tsc")
        .arg("-b")
        .arg("src/bin/html_files/")
        .arg("--verbose")
        .spawn()?
        .wait()?;
    if ! status.success() {
        std::process::exit(1);
    }

    let mut config = Config::default();
    *config.git_mut().sha_kind_mut() = ShaKind::Short;
    vergen(config)
}
