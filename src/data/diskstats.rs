use crate::data::common::time_series_data_processor::{
    time_series_data_processor_with_max_series_aggregate, TimeSeriesDataProcessor,
};
use crate::data::data_formats::AperfData;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::prelude::*,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskstatsRaw {
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl DiskstatsRaw {
    pub fn new() -> Self {
        DiskstatsRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for DiskstatsRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/diskstats")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Diskstats;

impl Diskstats {
    pub fn new() -> Self {
        Diskstats
    }
}

/// A helper struct that parses and holds one snapshot of /proc/diskstats data
pub struct ProcDiskStats {
    device_name: String,
    disk_stats: Vec<u64>,
}

impl ProcDiskStats {
    pub fn from_raw_data(raw_data: &str) -> Result<Self, String> {
        let split: Vec<&str> = raw_data.trim().split_whitespace().collect();

        if split.len() < 14 {
            return Err(format!("Cannot parse the raw data: {raw_data}"));
        }

        let device_name = match split.get(2) {
            Some(device_name) => device_name.to_string(),
            None => {
                return Err(format!(
                    "Cannot retrieve device name from the raw data: {raw_data}"
                ));
            }
        };

        let disk_stats: Vec<u64> = split[3..]
            .iter()
            .map(|&s| s.parse::<u64>().unwrap_or_default())
            .collect();

        Ok(ProcDiskStats {
            device_name,
            disk_stats,
        })
    }
}

#[derive(EnumIter, Display, Clone, Copy, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum DiskStatKey {
    Reads,
    Merged,
    SectorsRead,
    TimeReading,
    Writes,
    WritesMerged,
    SectorsWritten,
    TimeWriting,
    InProgress,
    TimeInProgress,
    WeightedTimeInProgress,
    Discards,
    DiscardsMerged,
    SectorsDiscarded,
    TimeDiscarding,
    Flushes,
    TimeFlushing,
}

fn get_device_disk_stat(disk_stat_key: DiskStatKey, device_disk_stat: &Vec<u64>) -> u64 {
    let index = match disk_stat_key {
        DiskStatKey::Reads => 0,
        DiskStatKey::Merged => 1,
        DiskStatKey::SectorsRead => 2,
        DiskStatKey::TimeReading => 3,
        DiskStatKey::Writes => 4,
        DiskStatKey::WritesMerged => 5,
        DiskStatKey::SectorsWritten => 6,
        DiskStatKey::TimeWriting => 7,
        DiskStatKey::InProgress => 8,
        DiskStatKey::TimeInProgress => 9,
        DiskStatKey::WeightedTimeInProgress => 10,
        DiskStatKey::Discards => 11,
        DiskStatKey::DiscardsMerged => 12,
        DiskStatKey::SectorsDiscarded => 13,
        DiskStatKey::TimeDiscarding => 14,
        DiskStatKey::Flushes => 15,
        DiskStatKey::TimeFlushing => 16,
    };

    device_disk_stat.get(index).copied().unwrap_or_default()
}

impl ProcessData for Diskstats {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["disk_stats"]
    }

    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        // For diskstats there is no easy way to compute or find the aggregate metric, so to assign
        // stats to a metric, we use the stats of the series with the largest average value
        let mut time_series_data_processor =
            time_series_data_processor_with_max_series_aggregate!();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::DiskstatsRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            for device_line in raw_value.data.lines() {
                let proc_disk_stats = match ProcDiskStats::from_raw_data(device_line) {
                    Ok(proc_disk_stats) => proc_disk_stats,
                    Err(proc_error) => {
                        error!("Error parsing diskstats: {}", proc_error);
                        continue;
                    }
                };
                for disk_stat_key in DiskStatKey::iter() {
                    let device_disk_stat =
                        get_device_disk_stat(disk_stat_key, &proc_disk_stats.disk_stats);
                    match disk_stat_key {
                        // in_progress is the only disk stat that goes to zero
                        // See https://www.kernel.org/doc/Documentation/iostats.txt
                        DiskStatKey::InProgress => time_series_data_processor.add_data_point(
                            &disk_stat_key.to_string(),
                            &proc_disk_stats.device_name,
                            device_disk_stat as f64,
                        ),
                        // The rests are simply delta between two timestamps
                        _ => time_series_data_processor.add_accumulative_data_point(
                            &disk_stat_key.to_string(),
                            &proc_disk_stats.device_name,
                            device_disk_stat as f64,
                        ),
                    };
                }
            }
        }

        let disk_stats_order: Vec<String> = DiskStatKey::iter()
            .map(|disk_stat_key| disk_stat_key.to_string())
            .collect();
        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(
                disk_stats_order.iter().map(AsRef::as_ref).collect(),
            );
        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::DiskstatsRaw,
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut diskstats = DiskstatsRaw::new();
        let params = CollectorParams::new();

        diskstats.collect_data(&params).unwrap();
        assert!(!diskstats.data.is_empty());
    }
}
