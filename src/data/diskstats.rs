use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, trace};
use procfs;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskstatsRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl DiskstatsRaw {
    pub fn new() -> Self {
        DiskstatsRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for DiskstatsRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/diskstats")?;
        trace!("{:#?}", self.data);
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

fn get_device_disk_stat(disk_stat_key: DiskStatKey, device_disk_stat: &procfs::DiskStat) -> u64 {
    match disk_stat_key {
        DiskStatKey::Reads => device_disk_stat.reads,
        DiskStatKey::Merged => device_disk_stat.merged,
        DiskStatKey::SectorsRead => device_disk_stat.sectors_read,
        DiskStatKey::TimeReading => device_disk_stat.time_reading,
        DiskStatKey::Writes => device_disk_stat.writes,
        DiskStatKey::WritesMerged => device_disk_stat.writes_merged,
        DiskStatKey::SectorsWritten => device_disk_stat.sectors_written,
        DiskStatKey::TimeWriting => device_disk_stat.time_writing,
        DiskStatKey::InProgress => device_disk_stat.in_progress,
        DiskStatKey::TimeInProgress => device_disk_stat.time_in_progress,
        DiskStatKey::WeightedTimeInProgress => device_disk_stat.weighted_time_in_progress,
        DiskStatKey::Discards => device_disk_stat.discards.unwrap_or_default(),
        DiskStatKey::DiscardsMerged => device_disk_stat.discards_merged.unwrap_or_default(),
        DiskStatKey::SectorsDiscarded => device_disk_stat.sectors_discarded.unwrap_or_default(),
        DiskStatKey::TimeDiscarding => device_disk_stat.time_discarding.unwrap_or_default(),
        DiskStatKey::Flushes => device_disk_stat.flushes.unwrap_or_default(),
        DiskStatKey::TimeFlushing => device_disk_stat.time_flushing.unwrap_or_default(),
    }
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
        let mut time_series_data = TimeSeriesData::default();

        // (*Most of) the disk stats are accumulated since last boot, so memorize the previous
        // stats to compute the delta as the series value
        let mut prev_disk_stats: HashMap<String, procfs::DiskStat> = HashMap::new();
        // initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;

        // For every disk stat metric, maintain an ordered map between devices and their series in the
        // metric, so that at the end of processing, all device series can be added to the corresponding
        // metric sorted by the device name
        let mut per_disk_stat_per_device_series: HashMap<DiskStatKey, BTreeMap<String, Series>> =
            HashMap::new();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::DiskstatsRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            for device_line in raw_value.data.lines() {
                let device_disk_stats = match procfs::DiskStat::from_line(device_line) {
                    Ok(device_disk_stats) => device_disk_stats,
                    Err(proc_error) => {
                        error!("Error parsing diskstats: {}", proc_error);
                        continue;
                    }
                };
                let device = &device_disk_stats.name;
                let prev_device_disk_stats = prev_disk_stats
                    .entry(device.clone())
                    .or_insert(device_disk_stats.clone());

                for disk_stat_key in DiskStatKey::iter() {
                    let device_disk_stat = get_device_disk_stat(disk_stat_key, &device_disk_stats);
                    let prev_device_disk_stat =
                        get_device_disk_stat(disk_stat_key, &prev_device_disk_stats);
                    let device_disk_stat_value = match disk_stat_key {
                        // in_progress is the only disk stat that goes to zero
                        // See https://www.kernel.org/doc/Documentation/iostats.txt
                        DiskStatKey::InProgress => device_disk_stat as f64,
                        // We present sectors metric as the number of kilobytes. Since one sector is 512 bytes,
                        // the series value is (num of sectors) * 512 / 1024
                        DiskStatKey::SectorsRead
                        | DiskStatKey::SectorsWritten
                        | DiskStatKey::SectorsDiscarded => {
                            (device_disk_stat - prev_device_disk_stat) as f64 / 2.0
                        }
                        // The rests are simply delta between two timestamps
                        _ => (device_disk_stat - prev_device_disk_stat) as f64,
                    };

                    let per_device_series = per_disk_stat_per_device_series
                        .entry(disk_stat_key)
                        .or_insert(BTreeMap::new());
                    let device_series = per_device_series
                        .entry(device.clone())
                        .or_insert(Series::new(Some(device.clone())));
                    device_series.time_diff.push(time_diff);
                    device_series.values.push(device_disk_stat_value);
                }

                prev_disk_stats.insert(device.clone(), device_disk_stats.clone());
            }
        }

        // Put device series into disk stat metrics
        for (disk_stat_key, per_device_series) in per_disk_stat_per_device_series {
            let mut disk_stat_metric = TimeSeriesMetric::new(disk_stat_key.to_string());
            disk_stat_metric.series = per_device_series.into_values().collect();
            // For diskstats there is no easy way to compute or find the aggregate metric, so to assign
            // stats to a metric, we use the stats of the series with the largest avg value
            let mut max_avg = 0.0;
            // finding the max and min of all stats help us define the metric graph's range
            let mut max: f64 = 0.0;
            let mut min: f64 = f64::MAX;
            for device_series in &disk_stat_metric.series {
                let device_series_stats = Statistics::from_values(&device_series.values);
                max = max.max(device_series_stats.max);
                min = min.min(device_series_stats.min);
                if device_series_stats.avg > max_avg {
                    max_avg = device_series_stats.avg;
                    disk_stat_metric.stats = device_series_stats;
                }
            }
            disk_stat_metric.value_range = (min.floor() as u64, max.ceil() as u64);
            time_series_data
                .metrics
                .insert(disk_stat_key.to_string(), disk_stat_metric);
        }
        time_series_data.sorted_metric_names = DiskStatKey::iter()
            .map(|disk_stat_key| disk_stat_key.to_string())
            .collect();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    use super::DiskstatsRaw;
    use crate::data::{CollectData, CollectorParams};

    #[test]
    fn test_collect_data() {
        let mut diskstats = DiskstatsRaw::new();
        let params = CollectorParams::new();

        diskstats.collect_data(&params).unwrap();
        assert!(!diskstats.data.is_empty());
    }
}
