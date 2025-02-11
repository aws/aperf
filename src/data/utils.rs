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

pub fn notargz_file_name(pbuf: PathBuf) -> Result<String> {
    if pbuf.file_name().is_none() {
        return Ok(String::new());
    }
    notargz_string_name(pbuf.file_name().unwrap().to_str().unwrap().to_string())
}

pub fn notargz_string_name(s: String) -> Result<String> {
    if s.ends_with(".tar.gz") {
        return Ok(s.strip_suffix(".tar.gz").unwrap().to_string());
    }
    Ok(s)
}
