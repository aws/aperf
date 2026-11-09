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

    if env::var("CARGO_CFG_TARGET_OS")? == "linux" {
        compile_bpf_programs()?;
        println!("Building with eBPF (off-CPU profiling) support.");
    }
    Ok(())
}

fn compile_bpf_programs() -> Result<()> {
    use std::path::PathBuf;

    const PROG_DIR: &str = "src/ebpf/programs";
    const INCLUDE_DIR: &str = "src/ebpf/include";

    println!("cargo:rerun-if-changed={PROG_DIR}");
    println!("cargo:rerun-if-changed={INCLUDE_DIR}");
    println!("cargo:rerun-if-env-changed=CLANG");
    println!("cargo:rerun-if-env-changed=LIBBPF_INCLUDE_DIR");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    let candidates: Vec<String> = match env::var("CLANG") {
        Ok(clang) => vec![clang],
        Err(_) => {
            let mut candidates: Vec<String> = ["TARGET", "HOST"]
                .iter()
                .filter_map(|var| env::var(var).ok())
                .map(|triple| format!("{triple}-clang"))
                .chain(["clang".to_string()])
                .collect();
            candidates.dedup();
            candidates
        }
    };
    let Some((clang, version)) = candidates.iter().find_map(|candidate| {
        let output = Command::new(candidate).arg("--version").output().ok()?;
        output.status.success().then(|| {
            (
                candidate,
                String::from_utf8_lossy(&output.stdout).into_owned(),
            )
        })
    }) else {
        anyhow::bail!(
            "building eBPF needs clang with the BPF target (tried: {}); install clang or set CLANG",
            candidates.join(", ")
        );
    };
    let major = version
        .split("clang version ")
        .nth(1)
        .and_then(|rest| rest.split('.').next())
        .and_then(|major| major.parse::<u32>().ok());
    if !matches!(major, Some(10..)) {
        anyhow::bail!(
            "building eBPF needs clang 10 or newer; {clang} reports: {}",
            version.lines().next().unwrap_or_default()
        );
    }

    let header_dirs: Vec<PathBuf> = match env::var_os("LIBBPF_INCLUDE_DIR") {
        Some(dir) => vec![PathBuf::from(dir)],
        None => ["/usr/include", "/usr/local/include"]
            .map(PathBuf::from)
            .to_vec(),
    };
    let Some(libbpf_dir) = header_dirs
        .iter()
        .find(|dir| dir.join("bpf/bpf_helpers.h").is_file())
    else {
        anyhow::bail!(
            "building eBPF needs the libbpf headers (bpf/bpf_helpers.h; tried: {}); install \
             libbpf-devel (AL2023/Fedora) or libbpf-dev (Debian/Ubuntu), or set LIBBPF_INCLUDE_DIR",
            header_dirs
                .iter()
                .map(|dir| dir.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    };
    println!(
        "cargo:rerun-if-changed={}",
        libbpf_dir.join("bpf").display()
    );

    let shim = out_dir.join("libbpf-include");
    let shim_bpf = shim.join("bpf");
    std::fs::create_dir_all(&shim)?;
    if shim_bpf.symlink_metadata().is_ok() {
        std::fs::remove_file(&shim_bpf)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(libbpf_dir.join("bpf"), &shim_bpf)?;

    let mut sources: Vec<String> = std::fs::read_dir(manifest_dir.join(PROG_DIR))?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|file_name| file_name.ends_with(".bpf.c"))
        .collect();
    sources.sort();

    for file_name in sources {
        let src = format!("{PROG_DIR}/{file_name}");
        let obj = out_dir.join(file_name.replace(".bpf.c", ".bpf.o"));
        let status = Command::new(clang)
            .current_dir(&manifest_dir)
            .args([
                "-O2",
                "-g",
                "-target",
                "bpfel",
                "-mcpu=v3",
                "-nostdinc",
                "-Wall",
                "-fdebug-compilation-dir=.",
            ])
            .arg(format!(
                "-ffile-prefix-map={}=libbpf-include",
                shim.display()
            ))
            .args(["-I", INCLUDE_DIR, "-I"])
            .arg(&shim)
            .args(["-c", &src, "-o"])
            .arg(&obj)
            .status()?;
        if !status.success() {
            anyhow::bail!("`{clang}` failed to compile {src} ({status})");
        }
    }
    Ok(())
}
