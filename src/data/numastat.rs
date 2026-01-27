use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::utils::get_aggregate_cpu_series_name;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::prelude::*,
    log::warn,
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
        let mut time_series_data = TimeSeriesData::default();
        let mut time_zero: Option<TimeEnum> = None;
        let mut prev_val_map: HashMap<String, HashMap<String, u64>> = HashMap::new();

        let mut per_numa_metric_aggregate_series: HashMap<String, Series> = HashMap::new();
        let mut per_numa_max_value: HashMap<String, u64> = HashMap::new();
        let mut per_numastat_per_node_series: HashMap<String, BTreeMap<String, Series>> =
            HashMap::new();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::NumastatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            let mut per_metric_sums: HashMap<String, (u64, u64)> = HashMap::new();
            let mut current_node: String = String::new();
            for line in raw_value.data.lines() {
                if line.ends_with(':') {
                    current_node = line.trim_end_matches(':').to_string();
                } else if !current_node.is_empty() && !line.trim().is_empty() {
                    let mut parts = line.split_whitespace();
                    if let (Some(metric_name), Some(value_str)) = (parts.next(), parts.next()) {
                        let current_value = match value_str.parse::<u64>() {
                            Ok(val) => val,
                            Err(_) => continue,
                        };

                        let prev_node_stats = prev_val_map
                            .entry(metric_name.to_string())
                            .or_insert(HashMap::new());
                        let diff_value = prev_node_stats
                            .get(&current_node)
                            .map(|&prev_value| current_value.saturating_sub(prev_value))
                            .unwrap_or(0);

                        // Keep track of the max value for each metric across all nodes
                        if let Some(max_value) = per_numa_max_value.get_mut(metric_name) {
                            *max_value = (*max_value).max(diff_value);
                        } else {
                            per_numa_max_value.insert(metric_name.to_string(), diff_value);
                        }

                        // Create per-node series
                        let metric = per_numastat_per_node_series
                            .entry(metric_name.to_string())
                            .or_insert(BTreeMap::new());
                        let series = metric
                            .entry(current_node.clone())
                            .or_insert(Series::new(Some(current_node.clone())));

                        series.time_diff.push(time_diff);
                        series.values.push(diff_value as f64);

                        // Update aggregate sums and counts
                        let (sum, count) = per_metric_sums
                            .entry(metric_name.to_string())
                            .or_insert((0, 0));
                        *sum += diff_value;
                        *count += 1;

                        prev_node_stats.insert(current_node.clone(), current_value);
                    }
                }
            }

            for (metric_name, (sum, count)) in per_metric_sums {
                let aggregate_series = per_numa_metric_aggregate_series
                    .entry(metric_name)
                    .or_insert(Series::new(get_aggregate_cpu_series_name()));
                let avg = if count > 0 {
                    sum as f64 / count as f64
                } else {
                    0.0
                };
                aggregate_series.time_diff.push(time_diff);
                aggregate_series.values.push(avg);
            }
        }

        // Compute the stats of every aggregate series and add them to the corresponding metric
        for (numa_metric_name, per_node_series) in per_numastat_per_node_series {
            let mut numa_metric = TimeSeriesMetric::new(numa_metric_name.clone());
            numa_metric.series = per_node_series.into_values().collect();

            if let Some(aggregate_series) =
                per_numa_metric_aggregate_series.get_mut(&numa_metric_name)
            {
                let aggregate_stats = Statistics::from_values(&aggregate_series.values);
                numa_metric.value_range = (
                    0,
                    *per_numa_max_value
                        .get(&numa_metric_name)
                        .unwrap_or(&(aggregate_stats.max.ceil() as u64)),
                );
                numa_metric.stats = aggregate_stats;
                aggregate_series.is_aggregate = true;
                numa_metric.series.push(aggregate_series.clone());
            }

            time_series_data
                .metrics
                .insert(numa_metric_name, numa_metric);
        }

        // Sort by numastat display order
        let preferred_order = [
            "numa_hit",
            "numa_miss",
            "numa_foreign",
            "interleave_hit",
            "local_node",
            "other_node",
        ];
        let mut sorted_metric_names: Vec<String> =
            time_series_data.metrics.keys().cloned().collect();
        sorted_metric_names.sort_by_key(|name| {
            preferred_order
                .iter()
                .position(|&order_name| order_name == name)
                .unwrap_or(preferred_order.len())
        });
        time_series_data.sorted_metric_names = sorted_metric_names;

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
