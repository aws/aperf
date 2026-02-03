use anyhow::Result;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct CpuInfo {
    pub vendor: String,
    pub model_name: String,
}

impl CpuInfo {
    fn new() -> Self {
        CpuInfo {
            vendor: String::new(),
            model_name: String::new(),
        }
    }
}

pub fn get_cpu_info() -> Result<CpuInfo> {
    let file = File::open("/proc/cpuinfo")?;
    let proc_cpuinfo = BufReader::new(file);
    let mut cpu_info = CpuInfo::new();
    for line in proc_cpuinfo.lines() {
        let info_line = line?;
        if info_line.is_empty() {
            break;
        }
        let key_value: Vec<&str> = info_line.split(':').collect();
        if key_value.len() < 2 {
            continue;
        }
        let key = key_value[0].trim().to_string();
        let value = key_value[1].trim().to_string();
        match key.as_str() {
            "vendor_id" => cpu_info.vendor = value,
            "model name" => cpu_info.model_name = value,
            _ => {}
        }
    }
    Ok(cpu_info)
}

pub fn no_tar_gz_file_name(path: &PathBuf) -> Option<String> {
    if path.file_name().is_none() {
        return None;
    }

    let file_name_str = path.file_name()?.to_str()?.to_string();

    if file_name_str.ends_with(".tar.gz") {
        return Some(file_name_str.strip_suffix(".tar.gz")?.to_string());
    }
    Some(file_name_str)
}

pub fn get_cpu_series_name(cpu: usize) -> Option<String> {
    Some(format!("CPU{cpu}"))
}

pub fn get_aggregate_cpu_series_name() -> Option<String> {
    Some("Aggregate".to_string())
}
