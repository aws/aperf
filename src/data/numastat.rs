use crate::data::common::common_raw_data::parse_common_raw_time_series_data;
use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_average_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::prelude::*,
    std::fs,
    std::path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
lazy_static! {
    static ref NAME_PATH_MAP: HashMap<String, PathBuf> = {
        let mut name_path_map = HashMap::new();
        let node_dir = Path::new("/sys/devices/system/node");
        if !node_dir.exists() {
            warn!("No NUMA support, not collecting numastat data");
            return name_path_map; // No NUMA support
        }

        if let Ok(entries) = fs::read_dir(node_dir) {
            name_path_map.extend(
                entries
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| {
                        let path = entry.path();
                        let name_str = path.file_name()?.to_str()?;
                        if name_str.starts_with("node") && name_str[4..].chars().all(|c| c.is_ascii_digit()) {
                            let numastat_path = path.join("numastat");
                            if numastat_path.exists() {
                                Some((name_str.to_string(), numastat_path))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
            );
        }

        name_path_map
    };
}

/// Gather NUMA stats raw data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NumastatRaw {
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl NumastatRaw {
    pub fn new() -> Self {
        NumastatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for NumastatRaw {
    fn prepare_data_collector(&mut self, _params: &CollectorParams) -> Result<()> {
        let _ = &*NAME_PATH_MAP; // Force initialization before collection time
        Ok(())
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();

        // Collect NUMA stats from /sys/devices/system/node/node*/numastat
        for (node_name, numastat_path) in NAME_PATH_MAP.iter() {
            let content = fs::read_to_string(numastat_path)?;
            self.data
                .push_str(&format!("{}:\n{}\n", node_name.to_string(), content));
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Numastat;

impl Numastat {
    pub fn new() -> Self {
        Numastat
    }
}

impl ProcessData for Numastat {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_average_aggregate!();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::NumastatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            // Although the raw data was not built through CommonRawDataBuilder, its format
            // is the same
            let numa_data = parse_common_raw_time_series_data(&raw_value.data);
            for (numa_metric_name, per_node_metric_value) in numa_data {
                for (numa_node, metric_value) in per_node_metric_value {
                    time_series_data_processor.add_accumulative_data_point(
                        &numa_metric_name,
                        &numa_node,
                        metric_value,
                    );
                }
            }
        }

        // Sort by numastat display order
        let preferred_order = vec![
            "numa_hit",
            "numa_miss",
            "numa_foreign",
            "interleave_hit",
            "local_node",
            "other_node",
        ];
        let time_series_data =
            time_series_data_processor.get_time_series_data_with_metric_name_order(preferred_order);

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::NumastatRaw,
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut numastat_raw = NumastatRaw::new();
        let params = CollectorParams::new();

        // This test may fail on systems without NUMA support, which is expected
        let result = numastat_raw.collect_data(&params);
        assert!(result.is_ok());
    }
}
