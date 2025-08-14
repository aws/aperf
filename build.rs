use anyhow::Result;
use std::env;
use std::process::Command;

fn main() -> Result<()> {
    let _ = vergen::EmitBuilder::builder().git_sha(true).emit();

    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");

    match Command::new("npm").arg("install").spawn() {
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
        println!("Failed to compile typescript.");
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
