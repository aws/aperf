use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::{
    time_series_data_processor_with_custom_aggregate, TimeSeriesDataProcessor,
};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::data_processing::ReportParams;
use anyhow::Result;
use indexmap::IndexMap;
use log::error;
use serde::{Deserialize, Serialize};
use std::fs::File;
#[cfg(target_os = "linux")]
use {
    crate::data::common::utils::{
        open_files_in_dir, per_hugepage_size_dir_suffix, per_hugepage_size_dirs,
        read_open_virtual_file, read_virtual_file,
    },
    crate::data::common::HUGETLB_DIR,
    crate::data::CollectData,
    crate::data_collection::InitParams,
    chrono::prelude::*,
    log::debug,
    std::collections::HashMap,
    std::path::Path,
};

/// Gather Meminfo raw data.
#[derive(Serialize, Deserialize, Debug)]
pub struct MeminfoDataRaw {
    // Held all open file handlers for the per-size HugeTLB pool counter files.
    #[serde(skip)]
    pub hugetlb_files: Vec<(String, File)>,
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl Default for MeminfoDataRaw {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl MeminfoDataRaw {
    pub fn new() -> Self {
        MeminfoDataRaw {
            hugetlb_files: Vec::new(),
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for MeminfoDataRaw {
    fn prepare_data_collector(&mut self, _init_params: &InitParams) -> Result<()> {
        // The per-size HugeTLB pool counters and the corresponding metric name prefix in meminfo.
        // For example, nr_hugepages of hugepages-2048kB eventually becomes HugePages_Total_2048kB.
        let hugetlb_pool_counters = HashMap::from([
            ("nr_hugepages", "HugePages_Total"),
            ("free_hugepages", "HugePages_Free"),
            ("resv_hugepages", "HugePages_Rsvd"),
            ("surplus_hugepages", "HugePages_Surp"),
        ]);

        // Collect and open all per-size hugetlb counter files.
        let counter_names: Vec<&str> = hugetlb_pool_counters.keys().copied().collect();
        for size_dir in per_hugepage_size_dirs(Path::new(HUGETLB_DIR)) {
            let Some(size) = per_hugepage_size_dir_suffix(&size_dir) else {
                continue;
            };
            for (counter_name, file) in open_files_in_dir(&size_dir, &counter_names) {
                if let Some(metric_name) = hugetlb_pool_counters.get(counter_name.as_str()) {
                    self.hugetlb_files
                        .push((format!("{metric_name}_{size}"), file));
                }
            }
        }
        Ok(())
    }

    fn collect_data(&mut self, _init_params: &InitParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = read_virtual_file("/proc/meminfo")?;

        // Append all per-size hugetlb counter values.
        for (metric_name, file) in self.hugetlb_files.iter_mut() {
            match read_open_virtual_file(file) {
                Ok(value) => {
                    self.data
                        .push_str(&format!("{metric_name}: {}\n", value.trim()));
                }
                Err(e) => debug!("Could not read the value of {metric_name}: {e}"),
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeminfoData;

impl MeminfoData {
    pub fn new() -> Self {
        MeminfoData
    }
}

/// Help function to parse a raw /proc/meminfo data into an IndexMap, where the
/// insertion order is maintained and can be used to create metric name ordering
fn parse_meminfo(raw_data: &str) -> IndexMap<String, u64> {
    let mut meminfo_map: IndexMap<String, u64> = IndexMap::new();

    for line in raw_data.lines() {
        if line.is_empty() {
            continue;
        }
        let split: Vec<&str> = line.split_whitespace().collect();

        if split.len() < 2 {
            error!("Unexpected raw meminfo data: {}", line);
            continue;
        }

        // the last character is a colon
        let metric_name = split[0][..split[0].len() - 1].to_string();

        let mut value: u64 = match split[1].parse() {
            Ok(value) => value,
            Err(_) => {
                error!("Unexpected metric value in raw data: {}", line);
                continue;
            }
        };
        let unit = split.get(2).copied().unwrap_or("");

        value = match unit {
            "KiB" | "kiB" | "kB" | "KB" => value * 1024,
            "MiB" | "miB" | "MB" | "mB" => value * 1024 * 1024,
            "GiB" | "giB" | "GB" | "gB" => value * 1024 * 1024 * 1024,
            _ => value,
        };

        meminfo_map.insert(metric_name, value);
    }

    meminfo_map
}

/// Register the derived metric for idle HugeTLB pool memory, as a percentage of total memory,
/// summed over all per-size free pages.
fn register_hugetlb_unused_memory_percent(
    processor: &mut TimeSeriesDataProcessor,
    metric_names: &[String],
) {
    let mut size_terms: Vec<String> = metric_names
        .iter()
        .filter_map(|name| name.strip_prefix("HugePages_Total_"))
        .filter_map(|size| size.strip_suffix("kB")?.parse::<u64>().ok())
        .map(|size_kb| {
            format!(
                "(HugePages_Free_{size_kb}kB - HugePages_Rsvd_{size_kb}kB) * {}",
                size_kb * 1024
            )
        })
        .collect();
    if size_terms.is_empty() {
        size_terms.push("(HugePages_Free - HugePages_Rsvd) * Hugepagesize".to_string());
    }
    let unused_bytes = size_terms.join(" + ");

    processor.register_derived_metric(
        "Hugetlb_Unused_Memory_Percent",
        &format!("({unused_bytes}) / MemTotal * 100"),
    );
}

impl ProcessData for MeminfoData {
    fn process_raw_data(
        &mut self,
        report_params: &ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor =
            time_series_data_processor_with_custom_aggregate!(report_params.collection_start);

        time_series_data_processor
            .register_derived_metric("PageTables_Memory_Percent", "PageTables / MemTotal * 100");

        let mut metric_name_order: Vec<String> = Vec::new();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::MeminfoDataRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            let meminfo = parse_meminfo(&raw_value.data);

            // Only use the metric names available in the first data to decide ordering.
            // In rare cases (if possible) where other metrics appear later, they'll be
            // placed at last
            if metric_name_order.is_empty() {
                metric_name_order = meminfo.keys().cloned().collect();
                register_hugetlb_unused_memory_percent(
                    &mut time_series_data_processor,
                    &metric_name_order,
                );
            }

            for (metric_name, value) in meminfo {
                time_series_data_processor.add_data_point(&metric_name, "value", value as f64);
            }
        }

        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(
                metric_name_order.iter().map(AsRef::as_ref).collect(),
            );
        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::MeminfoDataRaw,
        crate::data::common::{utils::per_hugepage_size_dirs, HUGETLB_DIR},
        crate::data::CollectData,
        crate::data_collection::InitParams,
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut meminfodata_raw = MeminfoDataRaw::new();
        let params = InitParams::default();

        meminfodata_raw.prepare_data_collector(&params).unwrap();
        meminfodata_raw.collect_data(&params).unwrap();
        assert!(!meminfodata_raw.data.is_empty());

        // Every synthetic per-size HugeTLB line must parse like a /proc/meminfo line.
        for (metric_name, _) in &meminfodata_raw.hugetlb_files {
            let line = meminfodata_raw
                .data
                .lines()
                .find(|line| line.starts_with(&format!("{metric_name}:")))
                .unwrap_or_else(|| panic!("missing synthetic line for {metric_name}"));
            let value = line.split_whitespace().nth(1).unwrap_or("");
            assert!(
                value.parse::<u64>().is_ok(),
                "unparsable synthetic line: {line}"
            );
        }
        if !per_hugepage_size_dirs(std::path::Path::new(HUGETLB_DIR)).is_empty() {
            assert!(meminfodata_raw
                .data
                .lines()
                .any(|line| line.starts_with("HugePages_Total_")));
        }
    }
}
