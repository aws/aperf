use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_custom_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use indexmap::IndexMap;
use log::error;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::prelude::*,
};

/// Gather Meminfo raw data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeminfoDataRaw {
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
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for MeminfoDataRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/meminfo")?;
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
fn parse_meminfo(raw_data: &String) -> IndexMap<String, u64> {
    let mut meminfo_map: IndexMap<String, u64> = IndexMap::new();

    for line in raw_data.lines() {
        if line.is_empty() {
            continue;
        }
        let split: Vec<&str> = line.split_whitespace().collect();

        if split.len() < 2 {
            error!("Unexpected raw data format: {}", line);
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

impl ProcessData for MeminfoData {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_custom_aggregate!();

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
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut meminfodata_raw = MeminfoDataRaw::new();
        let params = CollectorParams::new();

        meminfodata_raw.collect_data(&params).unwrap();
        assert!(!meminfodata_raw.data.is_empty());
    }
}
