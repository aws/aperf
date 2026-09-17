use crate::data::common::data_formats::{AperfData, KeyValueData};
use crate::data::common::utils::sysfs_active_mode;
#[cfg(target_os = "linux")]
use crate::data::common::{HUGETLB_DIR, THP_DIR};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::data_processing::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use {
    crate::data::common::utils::{
        open_files_in_dir, per_hugepage_size_dirs, read_open_virtual_file,
    },
    crate::data::CollectData,
    crate::data_collection::InitParams,
    log::debug,
    std::path::Path,
};

/// All static memory configurations are collected here.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemSettingsRaw {
    pub time: TimeEnum,
    pub mem_settings_data: HashMap<String, String>,
}

impl MemSettingsRaw {
    pub fn new() -> Self {
        MemSettingsRaw {
            time: TimeEnum::DateTime(Utc::now()),
            mem_settings_data: HashMap::new(),
        }
    }
}

impl Default for MemSettingsRaw {
    fn default() -> Self {
        Self::new()
    }
}

/// Read the contents of all files in dir included in file_names. Return a map from
/// file path string to the content of the file, which will be the key and value
/// for the collected data.
#[cfg(target_os = "linux")]
fn read_dir_files(dir: &Path, file_names: &[&str]) -> HashMap<String, String> {
    let mut files = HashMap::new();

    for (filename, mut file) in open_files_in_dir(dir, file_names) {
        let file_path = dir.join(&filename);
        let value = match read_open_virtual_file(&mut file) {
            Ok(value) => value,
            Err(e) => {
                debug!("Could not read {}: {e}", file_path.display());
                String::new()
            }
        };
        files.insert(file_path.to_string_lossy().into_owned(), value);
    }

    files
}

/// Read files from all per-page-size sub-directories in dir.
#[cfg(target_os = "linux")]
fn read_per_page_size_dir_files(dir: &Path, file_names: &[&str]) -> HashMap<String, String> {
    let mut files = HashMap::new();

    for size_dir in per_hugepage_size_dirs(dir) {
        files.extend(read_dir_files(&size_dir, file_names));
    }

    files
}

#[cfg(target_os = "linux")]
const THP_CONFIG_FILES: &[&str] = &[
    "enabled",
    "defrag",
    "shmem_enabled",
    "use_zero_page",
    "hpage_pmd_size",
    "shrink_underused",
];
#[cfg(target_os = "linux")]
const THP_PER_SIZE_CONFIG_FILES: &[&str] = &["enabled", "shmem_enabled"];
#[cfg(target_os = "linux")]
const KHUGEPAGED_CONFIG_FILES: &[&str] = &[
    "defrag",
    "max_ptes_none",
    "max_ptes_shared",
    "max_ptes_swap",
    "pages_to_scan",
    "scan_sleep_millisecs",
    "alloc_sleep_millisecs",
];
#[cfg(target_os = "linux")]
const HUGETLB_POOL_FILES: &[&str] = &["nr_hugepages", "nr_overcommit_hugepages"];

#[cfg(target_os = "linux")]
impl CollectData for MemSettingsRaw {
    fn collect_data(&mut self, _init_params: &InitParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());

        let thp_dir_path = Path::new(THP_DIR);
        let khugepaged_dir_path = thp_dir_path.join("khugepaged");
        let hugetlb_dir_path = Path::new(HUGETLB_DIR);

        self.mem_settings_data = read_dir_files(thp_dir_path, THP_CONFIG_FILES);
        self.mem_settings_data.extend(read_per_page_size_dir_files(
            thp_dir_path,
            THP_PER_SIZE_CONFIG_FILES,
        ));
        self.mem_settings_data.extend(read_dir_files(
            &khugepaged_dir_path,
            KHUGEPAGED_CONFIG_FILES,
        ));
        self.mem_settings_data.extend(read_per_page_size_dir_files(
            hugetlb_dir_path,
            HUGETLB_POOL_FILES,
        ));

        Ok(())
    }

    fn is_static() -> bool {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemSettings;

impl MemSettings {
    pub fn new() -> Self {
        MemSettings
    }
}

/// Parse raw key into corresponding key-value group and key.
fn group_and_key(raw_key: &str) -> (&str, &str) {
    let Some(key) = raw_key.strip_prefix("/sys/kernel/mm/") else {
        return ("", raw_key);
    };
    let group = match key.split('/').next() {
        Some("transparent_hugepage") => "THP",
        Some("hugepages") => "HugeTLB",
        _ => "",
    };
    (group, key)
}

impl ProcessData for MemSettings {
    fn process_raw_data(
        &mut self,
        _report_params: &ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut key_value_data = KeyValueData::default();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::MemSettingsRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            for (raw_key, value) in &raw_value.mem_settings_data {
                let (group, key) = group_and_key(raw_key);
                key_value_data
                    .key_value_groups
                    .entry(group.to_string())
                    .or_default()
                    .key_values
                    .insert(
                        key.to_string(),
                        sysfs_active_mode(value).unwrap_or(value.trim()).to_string(),
                    );
            }
        }

        Ok(AperfData::KeyValue(key_value_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::{MemSettingsRaw, THP_DIR},
        crate::data::CollectData,
        crate::data_collection::InitParams,
        std::path::Path,
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut mem_settings = MemSettingsRaw::new();
        let params = InitParams::default();

        mem_settings.collect_data(&params).unwrap();

        for path in mem_settings.mem_settings_data.keys() {
            // The per-page-size stats/ sub-directories are out of scope.
            assert!(!path.contains("stats"));
            // All collected settings should exist.
            assert!(
                Path::new(path).exists(),
                "collected {path} which does not exist"
            );
        }
        if Path::new(THP_DIR).is_dir() {
            assert!(mem_settings
                .mem_settings_data
                .contains_key("/sys/kernel/mm/transparent_hugepage/enabled"));
        }
    }
}
