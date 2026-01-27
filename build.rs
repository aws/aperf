use anyhow::Result;
use std::env;
use std::process::Command;

fn main() -> Result<()> {
    let _ = vergen::EmitBuilder::builder().git_sha(true).emit();

    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    let npm_cmd = "npm.cmd";
    #[cfg(not(windows))]
    let npm_cmd = "npm";

    match Command::new(npm_cmd).arg("install").spawn() {
        Err(_proc) => {
            println!("Build requires npm, but it was not found. Please install Node >= 16.16.0.");
            std::process::exit(1);
        }
        Ok(mut child) => {
            let status = child.wait()?;
            if !status.success() {
                println!("Command \"npm install\" failed.");
                std::process::exit(1);
            }
        }
    }

    let report_frontend_dir = format!("{}/report_frontend", env::var("OUT_DIR")?);
    println!("cargo:rustc-env=JS_DIR={}", report_frontend_dir);
    println!("cargo:rerun-if-changed=src/report_frontend");
    let status = Command::new(npm_cmd)
        .arg("exec")
        .arg("--")
        .arg("webpack")
        .arg("--config")
        .arg("src/report_frontend/webpack.config.js")
        .arg("--env")
        .arg(format!("output={report_frontend_dir}"))
        .spawn()?
        .wait()?;
    if !status.success() {
        println!("Failed to compile report frontend Javascript.");
        std::process::exit(1);
    }

    #[cfg(all(feature = "hotline", not(target_arch = "aarch64")))]
    compile_error!("The 'hotline' feature is only supported on aarch64 architecture");

    #[cfg(feature = "hotline")]
    {
        println!("cargo:rerun-if-changed=src/hotline/*");
        cc::Build::new()
            .files([
                "src/hotline/bmiss_map.c",
                "src/hotline/btree.c",
                "src/hotline/config.c",
                "src/hotline/finode_map.c",
                "src/hotline/fname_binary_map.c",
                "src/hotline/fname_map.c",
                "src/hotline/hotline.c",
                "src/hotline/lat_map.c",
                "src/hotline/report.c",
                "src/hotline/sys.c",
                "src/hotline/vec.c",
                "src/hotline/tests/test_bmiss_map.c",
                "src/hotline/tests/test_config.c",
                "src/hotline/tests/test_finode_map.c",
                "src/hotline/tests/test_fname_binary_map.c",
                "src/hotline/tests/test_fname_map.c",
                "src/hotline/tests/test_lat_map.c",
                "src/hotline/tests/test.c",
            ])
            .includes(["src/hotline"])
            .flag("-Werror")
            .flag("-Wextra")
            .flag("-Wall")
            .opt_level(3)
            .compile("hotline");

        println!("cargo:rustc-link-lib=dylib=dw");
        println!("cargo:rustc-link-lib=dylib=elf");
        println!("cargo:rustc-link-lib=dylib=capstone");
        println!("cargo:rustc-link-lib=dylib=z");
        println!("cargo:rustc-link-lib=dylib=lzma");
        println!("cargo:rustc-link-lib=dylib=bz2");
        println!("cargo:rustc-link-lib=dylib=zstd");
        println!("cargo:rustc-link-lib=dylib=stdc++");

        println!("Building with Hotline support.");
    }
    Ok(())
}
