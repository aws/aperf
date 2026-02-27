use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
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
        let mut time_series_data = TimeSeriesData::default();

        // initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;

        // The list of metric names to indicate their ordering
        let mut metric_name_order: Vec<String> = Vec::new();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::MeminfoDataRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            let meminfo = parse_meminfo(&raw_value.data);

            // Only use the metric names available in the first data to decide ordering.
            // In rare cases (if possible) where other metrics appear later, they'll be
            // placed at last
            if metric_name_order.is_empty() {
                metric_name_order = meminfo.keys().cloned().collect();
            }

            for (metric_name, value) in meminfo {
                let meminfo_metric = time_series_data
                    .metrics
                    .entry(metric_name)
                    .or_insert_with_key(|meminfo_metric_name| {
                        let mut _mem_info_metric =
                            TimeSeriesMetric::new(meminfo_metric_name.clone());
                        _mem_info_metric.series.push(Series::new(None));
                        _mem_info_metric
                    });
                let meminfo_series = &mut meminfo_metric.series[0];
                meminfo_series.time_diff.push(time_diff);
                meminfo_series.values.push(value as f64);
            }
        }

        // Compute metric stats and set value range
        for meminfo_metric in time_series_data.metrics.values_mut() {
            let metric_stats = Statistics::from_values(&meminfo_metric.series[0].values);
            meminfo_metric.value_range = (
                metric_stats.min.floor() as u64,
                metric_stats.max.ceil() as u64,
            );
            meminfo_metric.stats = metric_stats;
        }

        let mut sorted_metric_names: Vec<String> =
            time_series_data.metrics.keys().cloned().collect();
        sorted_metric_names.sort_by_key(|metric_name| {
            metric_name_order
                .iter()
                .position(|ordered_name| ordered_name == metric_name)
                .unwrap_or(metric_name_order.len())
        });
        time_series_data.sorted_metric_names = sorted_metric_names;

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
