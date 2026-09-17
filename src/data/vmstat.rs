use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_custom_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::data_processing::ReportParams;
use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};
use std::fs::File;
#[cfg(target_os = "linux")]
use {
    crate::data::common::utils::{
        open_files_in_dir, per_hugepage_size_dir_suffix, per_hugepage_size_dirs,
        read_open_virtual_file, read_virtual_file, sysfs_active_mode,
    },
    crate::data::common::THP_DIR,
    crate::data::CollectData,
    crate::data_collection::InitParams,
    chrono::prelude::*,
    log::{debug, warn},
    std::collections::HashMap,
    std::path::Path,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct VmstatRaw {
    // Held all open file handlers for the per-size mTHP stat files.
    #[serde(skip)]
    pub thp_stats_files: Vec<(String, File)>,
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl VmstatRaw {
    pub fn new() -> Self {
        VmstatRaw {
            thp_stats_files: Vec::new(),
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

// Read PMD size from hpage_pmd_size config file, to distinguish the mthp size that
// is already covered by /proc/vmstat.
#[cfg(target_os = "linux")]
fn get_pmd_size(thp_dir: &Path) -> Result<u64> {
    Ok(read_virtual_file(thp_dir.join("hpage_pmd_size"))?
        .trim()
        .parse::<u64>()?)
}

// Check if a hugepage size is enabled by reading the enabled config file, which might
// inherit from the top-level config. Only "always" and "madvise" mean enabled.
#[cfg(target_os = "linux")]
fn is_hugepage_size_enabled(hugepage_size_dir: &Path, top_level_mode: &Option<String>) -> bool {
    match read_virtual_file(hugepage_size_dir.join("enabled")) {
        Ok(enabled) => {
            let mut mode = sysfs_active_mode(&enabled);
            if mode == Some("inherit") {
                mode = top_level_mode.as_deref();
            }

            match mode {
                Some("never") | None => false,
                _ => true,
            }
        }
        _ => false,
    }
}

#[cfg(target_os = "linux")]
impl CollectData for VmstatRaw {
    fn prepare_data_collector(&mut self, _init_params: &InitParams) -> Result<()> {
        let thp_dir = Path::new(THP_DIR);
        // These config values help us decide whether the stats of a hugepage size need
        // to be collected.
        let pmd_size = match get_pmd_size(thp_dir) {
            Ok(pmd_size) => pmd_size,
            Err(e) => {
                warn!(
                    "Failed to read hpage_pmd_size, skipping per-size mTHP stat collections: {e}"
                );
                return Ok(());
            }
        };
        let top_level_mode = read_virtual_file(thp_dir.join("enabled"))
            .ok()
            .and_then(|value| sysfs_active_mode(&value).map(str::to_string));

        // The per-size mTHP stats and the corresponding metric name prefix in vmstat.
        // For example, anon_fault_alloc of hugepages-64kB eventually becomes thp_fault_alloc_64kB.
        let mthp_per_size_stats = HashMap::from([
            ("anon_fault_alloc", "thp_fault_alloc"),
            ("anon_fault_fallback", "thp_fault_fallback"),
            ("nr_anon", "nr_anon_transparent_hugepages"),
            ("nr_anon_partially_mapped", "nr_anon_partially_mapped"),
        ]);

        // Collect and open all per-size mthp counter files.
        let stat_names: Vec<&str> = mthp_per_size_stats.keys().copied().collect();
        for hugepage_size_dir in per_hugepage_size_dirs(thp_dir) {
            if !is_hugepage_size_enabled(&hugepage_size_dir, &top_level_mode) {
                continue;
            }

            let Some(size_suffix) = per_hugepage_size_dir_suffix(&hugepage_size_dir) else {
                continue;
            };
            let Some(hugepage_size) = size_suffix
                .strip_suffix("kB")
                .and_then(|size_str| size_str.parse::<u64>().ok())
                .and_then(|size| Some(size * 1024))
            else {
                continue;
            };

            // If the current hugepage size is the PMD size, all stats are already covered by vmstat,
            // except for nr_anon_partially_mapped
            let stats_to_collect: &[&str] = if hugepage_size == pmd_size {
                &["nr_anon_partially_mapped"]
            } else {
                &stat_names
            };

            for (stat_name, file) in
                open_files_in_dir(&hugepage_size_dir.join("stats"), stats_to_collect)
            {
                if let Some(metric_name) = mthp_per_size_stats.get(stat_name.as_str()) {
                    self.thp_stats_files
                        .push((format!("{metric_name}_{size_suffix}"), file));
                }
            }
        }

        Ok(())
    }

    fn collect_data(&mut self, _init_params: &InitParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = read_virtual_file("/proc/vmstat")?;

        // Append all per-size mthp stat values.
        for (metric_name, file) in self.thp_stats_files.iter_mut() {
            match read_open_virtual_file(file) {
                Ok(value) => {
                    self.data
                        .push_str(&format!("{metric_name} {}\n", value.trim()));
                }
                Err(e) => debug!("Could not read the value of {metric_name}: {e}"),
            }
        }

        Ok(())
    }
}

/// Most "nr_" fields in /proc/vmstat report an instantaneous level, but these ones are cumulative.
const CUMULATIVE_NR_METRICS: [&str; 11] = [
    "nr_dirtied",
    "nr_foll_pin_acquired",
    "nr_foll_pin_released",
    "nr_throttled_written",
    "nr_tlb_local_flush_all",
    "nr_tlb_local_flush_one",
    "nr_tlb_remote_flush",
    "nr_tlb_remote_flush_received",
    "nr_vmscan_immediate_reclaim",
    "nr_vmscan_write",
    "nr_written",
];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vmstat;

impl Vmstat {
    pub fn new() -> Self {
        Vmstat
    }
}

impl ProcessData for Vmstat {
    fn process_raw_data(
        &mut self,
        report_params: &ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor =
            time_series_data_processor_with_custom_aggregate!(report_params.collection_start);

        time_series_data_processor.register_derived_metric("pgminorfault", "pgfault - pgmajfault");
        time_series_data_processor
            .register_derived_metric("dirty_utilization", "nr_dirty / nr_dirty_threshold * 100");

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::VmstatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            for line in raw_value.data.lines() {
                let mut split = line.split_whitespace();
                let name = match split.next() {
                    Some(n) => n,
                    None => {
                        error!("Failed to extract name from vmstat line: {}", line);
                        continue;
                    }
                };
                let val_str = match split.next() {
                    Some(v) => v,
                    None => {
                        error!("Failed to extract value from vmstat line: {}", line);
                        continue;
                    }
                };
                let Ok(val) = val_str.parse::<i64>() else {
                    error!("Failed to parse the value from vmstat line: {}", line);
                    continue;
                };

                if name.starts_with("nr_") && !CUMULATIVE_NR_METRICS.contains(&name) {
                    time_series_data_processor.add_data_point(&name, "values", val as f64);
                } else {
                    time_series_data_processor
                        .add_accumulative_data_point(&name, "values", val as f64);
                }
            }
        }

        let time_series_data = time_series_data_processor.get_time_series_data();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {super::VmstatRaw, crate::data::CollectData, crate::data_collection::InitParams};

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut vmstat = VmstatRaw::new();
        let params = InitParams::default();

        vmstat.prepare_data_collector(&params).unwrap();
        vmstat.collect_data(&params).unwrap();
        assert!(!vmstat.data.is_empty());

        // Every synthetic per-size THP stats line must parse like a /proc/vmstat line.
        for (metric_name, _) in &vmstat.thp_stats_files {
            let line = vmstat
                .data
                .lines()
                .find(|line| line.split_whitespace().next() == Some(metric_name.as_str()))
                .unwrap_or_else(|| panic!("missing synthetic line for {metric_name}"));
            let value = line.split_whitespace().nth(1).unwrap_or("");
            assert!(
                value.parse::<i64>().is_ok(),
                "unparsable synthetic line: {line}"
            );
        }
    }
}
