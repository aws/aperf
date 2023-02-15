use anyhow::Result;
use std::env;
use std::process::Command;
use vergen::{Config, ShaKind, vergen};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");
    let status = Command::new("npm")
        .arg("install")
        .arg("--loglevel")
        .arg("verbose")
        .spawn()?
        .wait()?;
    if ! status.success() {
        std::process::exit(1);
    }

    let jsdir = format!("{}/js", env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-env=JS_DIR={}", jsdir);
    println!("cargo:rerun-if-changed=src/bin/html_files/");
    let status = Command::new("npm")
        .arg("exec")
        .arg("--")
        .arg("tsc")
        .arg("-p")
        .arg("src/bin/html_files/")
        .arg("--outDir")
        .arg(jsdir)
        .spawn()?
        .wait()?;
    if ! status.success() {
        std::process::exit(1);
    }

    let mut config = Config::default();
    *config.git_mut().sha_kind_mut() = ShaKind::Short;
    vergen(config)
}
