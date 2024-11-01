use anyhow::Result;
use std::env;
use std::process::Command;
use vergen_gitcl::{Emitter, GitclBuilder};

fn main() -> Result<()> {
    let gitcl = GitclBuilder::default().sha(true).build()?;
    Emitter::default().add_instructions(&gitcl)?.emit()?;

    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");
    let status = Command::new("npm").arg("install").spawn()?.wait()?;
    if !status.success() {
        std::process::exit(1);
    }
    let jsdir = format!("{}/js", env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-env=JS_DIR={}", jsdir);
    println!("cargo:rerun-if-changed=src/html_files/");
    let status = Command::new("npm")
        .arg("exec")
        .arg("--")
        .arg("tsc")
        .arg("-p")
        .arg("src/html_files/")
        .arg("--outDir")
        .arg(jsdir)
        .spawn()?
        .wait()?;
    if !status.success() {
        std::process::exit(1);
    }
    Ok(())
}
