use crate::data::constants::*;
use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::{add_metrics, get_data_name_from_type, DataMetrics, Metric};
use crate::visualizer::{GetData, GraphLimitType, GraphMetadata};
use anyhow::Result;
use chrono::prelude::*;
use log::{error, trace};
use procfs;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

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
pub struct Diskstat {
    pub name: String,
    pub stat: HashMap<String, u64>,
}

impl Diskstat {
    fn new(name: String) -> Self {
        Diskstat {
            name,
            stat: HashMap::new(),
        }
    }

    fn add(&mut self, key: String, value: u64) {
        self.stat.insert(key, value);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Diskstats {
    pub time: TimeEnum,
    pub disk_stats: Vec<Diskstat>,
}

impl Diskstats {
    pub fn new() -> Self {
        Diskstats {
            time: TimeEnum::DateTime(Utc::now()),
            disk_stats: Vec::new(),
        }
    }

    fn set_data(&mut self, data: Vec<Diskstat>) {
        self.disk_stats = data;
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }
}

#[derive(Debug, Display, EnumString, EnumIter)]
pub enum DiskstatKeys {
    Reads,
    #[strum(serialize = "Reads Merged")]
    Merged,
    #[strum(serialize = "Sectors Read (1 sector = 512 bytes)")]
    SectorsRead,
    #[strum(serialize = "Time Reading")]
    TimeReading,
    Writes,
    #[strum(serialize = "Writes Merged")]
    WritesMerged,
    #[strum(serialize = "Sectors Written (1 sector = 512 bytes)")]
    SectorsWritten,
    #[strum(serialize = "Time Writing")]
    TimeWriting,
    #[strum(serialize = "In Progress")]
    InProgress,
    #[strum(serialize = "Time In Progress")]
    TimeInProgress,
    #[strum(serialize = "Weighted Time In Progress")]
    WeightedTimeInProgress,
    Discards,
    #[strum(serialize = "Discards Merged")]
    DiscardsMerged,
    #[strum(serialize = "Sectors Discarded (1 sector = 512 bytes)")]
    SectorsDiscarded,
    #[strum(serialize = "Time Discarding")]
    TimeDiscarding,
    Flushes,
    #[strum(serialize = "Time Flushing")]
    TimeFlushing,
}

fn process_collected_raw_data(buffer: Data) -> Result<ProcessedData> {
    let raw_value = match buffer {
        Data::DiskstatsRaw(ref value) => value,
        _ => panic!("Invalid Data type in raw file"),
    };
    let mut diskstats = Diskstats::new();
    diskstats.set_time(raw_value.time);
    let mut data = Vec::<Diskstat>::new();
    let reader = BufReader::new(raw_value.data.as_bytes());
    for line in reader.lines() {
        let line = line?;
        let mut s = line.split_whitespace();

        let _major = s.next().unwrap().parse::<i32>();
        let _minor = s.next().unwrap().parse::<i32>();
        let name = s.next().unwrap().to_string();
        let mut diskstat = Diskstat::new(name);

        let reads = s.next().unwrap().parse::<u64>();
        let merged = s.next().unwrap().parse::<u64>();
        let sectors_read = s.next().unwrap().parse::<u64>();
        let time_reading = s.next().unwrap().parse::<u64>();
        let writes = s.next().unwrap().parse::<u64>();
        let writes_merged = s.next().unwrap().parse::<u64>();
        let sectors_written = s.next().unwrap().parse::<u64>();
        let time_writing = s.next().unwrap().parse::<u64>();
        let in_progress = s.next().unwrap().parse::<u64>();
        let time_in_progress = s.next().unwrap().parse::<u64>();
        let weighted_time_in_progress = s.next().unwrap().parse::<u64>();
        // Following since kernel 4.18
        let discards = s.next().and_then(|s| s.parse::<u64>().ok());
        let discards_merged = s.next().and_then(|s| s.parse::<u64>().ok());
        let sectors_discarded = s.next().and_then(|s| s.parse::<u64>().ok());
        let time_discarding = s.next().and_then(|s| s.parse::<u64>().ok());
        // Following since kernel 5.5
        let flushes = s.next().and_then(|s| s.parse::<u64>().ok());
        let time_flushing = s.next().and_then(|s| s.parse::<u64>().ok());

        diskstat.add(DiskstatKeys::Reads.to_string(), reads?);
        diskstat.add(DiskstatKeys::Merged.to_string(), merged?);
        diskstat.add(DiskstatKeys::SectorsRead.to_string(), sectors_read?);
        diskstat.add(DiskstatKeys::TimeReading.to_string(), time_reading?);
        diskstat.add(DiskstatKeys::Writes.to_string(), writes?);
        diskstat.add(DiskstatKeys::WritesMerged.to_string(), writes_merged?);
        diskstat.add(DiskstatKeys::SectorsWritten.to_string(), sectors_written?);
        diskstat.add(DiskstatKeys::TimeWriting.to_string(), time_writing?);
        diskstat.add(DiskstatKeys::InProgress.to_string(), in_progress?);
        diskstat.add(DiskstatKeys::TimeInProgress.to_string(), time_in_progress?);
        diskstat.add(
            DiskstatKeys::WeightedTimeInProgress.to_string(),
            weighted_time_in_progress?,
        );
        diskstat.add(
            DiskstatKeys::Discards.to_string(),
            discards.unwrap_or_default(),
        );
        diskstat.add(
            DiskstatKeys::DiscardsMerged.to_string(),
            discards_merged.unwrap_or_default(),
        );
        diskstat.add(
            DiskstatKeys::SectorsDiscarded.to_string(),
            sectors_discarded.unwrap_or_default(),
        );
        diskstat.add(
            DiskstatKeys::TimeDiscarding.to_string(),
            time_discarding.unwrap_or_default(),
        );
        diskstat.add(
            DiskstatKeys::Flushes.to_string(),
            flushes.unwrap_or_default(),
        );
        diskstat.add(
            DiskstatKeys::TimeFlushing.to_string(),
            time_flushing.unwrap_or_default(),
        );

        data.push(diskstat);
    }
    diskstats.set_data(data);
    let processed_data = ProcessedData::Diskstats(diskstats.clone());
    Ok(processed_data)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskValue {
    pub time: TimeEnum,
    pub value: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskValues {
    pub name: String,
    pub values: Vec<DiskValue>,
}

impl DiskValues {
    fn new(name: String) -> Self {
        DiskValues {
            name,
            values: Vec::new(),
        }
    }
}

fn get_disk_names(value: Diskstats) -> Vec<String> {
    let mut names = Vec::new();
    for disk in value.disk_stats {
        names.push(disk.name);
    }
    names
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndDiskValues {
    pub data: Vec<DiskValues>,
    pub metadata: GraphMetadata,
}

fn get_values(values: Vec<Diskstats>, key: String, metrics: &mut DataMetrics) -> Result<String> {
    let mut ev: BTreeMap<String, DiskValues> = BTreeMap::new();
    let mut metric = Metric::new(key.clone());
    let disk_names = get_disk_names(values[0].clone());
    let mut metadata = GraphMetadata::new();
    let mut factor = FACTOR_OF_ONE;
    let mut mult_factor = MULT_FACTOR_OF_ONE;
    for name in disk_names {
        let dv = DiskValues::new(name.clone());
        ev.insert(name, dv);
    }

    if key.contains("Time") {
        factor = TIME_S_FACTOR;
    }
    if key.contains("Sectors") {
        mult_factor = MULT_SECTORS;
        factor = KB_FACTOR;
    }
    let time_zero = values[0].time;
    let mut prev_data = values[0].clone();
    for v in values {
        let mut prev_value: HashMap<String, u64> = HashMap::new();
        for disk in &prev_data.disk_stats {
            prev_value.insert(disk.name.clone(), *disk.stat.get(&key.clone()).unwrap());
        }
        for disk in &v.disk_stats {
            /*
             * The only counter to go to zero.
             * See https://www.kernel.org/doc/html/latest/admin-guide/iostats.html Field #9
             */
            let stat_value = if key == "In Progress" {
                *disk.stat.get(&key.clone()).unwrap() as f64
            } else {
                (*disk.stat.get(&key.clone()).unwrap() as i64
                    - *prev_value.get(&disk.name).unwrap_or(&0) as i64) as f64
                    * mult_factor as f64
                    / factor as f64
            };
            let dv = DiskValue {
                time: (v.time - time_zero),
                value: stat_value,
            };
            metadata.update_limits(GraphLimitType::F64(stat_value));
            metric.insert_value(stat_value);

            if !ev.contains_key(&disk.name) {
                ev.insert(disk.name.clone(), DiskValues::new(disk.name.clone()));
            }
            let dvs = ev.get_mut(&disk.name).unwrap();
            dvs.values.push(dv);
        }
        prev_data = v.clone();
    }
    let end_values = EndDiskValues {
        data: ev.into_values().collect(),
        metadata,
    };
    add_metrics(
        key,
        &mut metric,
        metrics,
        get_data_name_from_type::<Diskstats>().to_string(),
    )?;
    Ok(serde_json::to_string(&end_values)?)
}

fn get_keys() -> Result<String> {
    let mut end_values: Vec<String> = Vec::new();
    for key in DiskstatKeys::iter() {
        end_values.push(key.to_string());
    }
    Ok(serde_json::to_string(&end_values)?)
}

// TODO: ------------------------------------------------------------------------------------------
//       Below are the new implementation to process diskstats into uniform data format. Remove
//       the original for the migration.
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
// TODO: ------------------------------------------------------------------------------------------

impl GetData for Diskstats {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["disk_stats"]
    }

    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        process_collected_raw_data(buffer)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["keys".to_string(), "values".to_string()])
    }

    fn get_data(
        &mut self,
        buffer: Vec<ProcessedData>,
        query: String,
        metrics: &mut DataMetrics,
    ) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::Diskstats(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => get_keys(),
            "values" => {
                let (_, key) = &param[2];
                get_values(values, key.to_string(), metrics)
            }
            _ => panic!("Unsupported API"),
        }
    }

    fn process_raw_data_new(&mut self, raw_data: Vec<Data>) -> Result<AperfData> {
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
            let mut dist_stat_metric = TimeSeriesMetric::new(disk_stat_key.to_string());
            dist_stat_metric.series = per_device_series.into_values().collect();
            // For diskstats there is no easy way to compute or find the aggregate metric, so to assign
            // stats to a metric, we use the stats of the series with the largest avg value
            let mut max_avg = 0.0;
            // finding the max and min of all stats help us define the metric graph's range
            let mut max: f64 = 0.0;
            let mut min: f64 = f64::MAX;
            for device_series in &dist_stat_metric.series {
                let device_series_stats = Statistics::from_values(&device_series.values);
                max = max.max(device_series_stats.max);
                min = min.min(device_series_stats.min);
                if device_series_stats.avg > max_avg {
                    max_avg = device_series_stats.avg;
                    dist_stat_metric.stats = device_series_stats;
                }
            }
            dist_stat_metric.value_range = (min.floor() as u64, max.ceil() as u64);
            time_series_data
                .metrics
                .insert(disk_stat_key.to_string(), dist_stat_metric);
        }
        time_series_data.sorted_metric_names = DiskStatKey::iter()
            .map(|disk_stat_key| disk_stat_key.to_string())
            .collect();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    use super::{DiskstatKeys, Diskstats, DiskstatsRaw, EndDiskValues};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    #[test]
    fn test_collect_data() {
        let mut diskstats = DiskstatsRaw::new();
        let params = CollectorParams::new();

        diskstats.collect_data(&params).unwrap();
        assert!(!diskstats.data.is_empty());
    }

    #[test]
    fn test_keys() {
        let mut stat = DiskstatsRaw::new();
        let mut key_map = HashMap::new();
        for key in DiskstatKeys::iter() {
            key_map.insert(key.to_string(), 0);
        }
        let params = CollectorParams::new();

        stat.collect_data(&params).unwrap();
        let processed_stat = Diskstats::new()
            .process_raw_data(Data::DiskstatsRaw(stat))
            .unwrap();
        let disk_stat = match processed_stat {
            ProcessedData::Diskstats(value) => value,
            _ => unreachable!("Invalid data type in processed data"),
        };
        let keys: Vec<String> = disk_stat.disk_stats[0].stat.clone().into_keys().collect();
        for key in keys {
            assert!(key_map.contains_key(&key));
            let value = key_map.get(&key).unwrap() + 1;
            key_map.insert(key, value);
        }
        let mut values: Vec<u64> = key_map.into_values().collect();
        values.dedup();
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn test_get_data_keys() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut diskstat = DiskstatsRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        diskstat.collect_data(&params).unwrap();
        buffer.push(Data::DiskstatsRaw(diskstat));
        processed_buffer.push(
            Diskstats::new()
                .process_raw_data(buffer[0].clone())
                .unwrap(),
        );
        let json = Diskstats::new()
            .get_data(
                processed_buffer,
                "run=test&get=keys".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<String> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_get_data_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut diskstat_zero = DiskstatsRaw::new();
        let mut diskstat_one = DiskstatsRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        diskstat_zero.collect_data(&params).unwrap();
        diskstat_one.collect_data(&params).unwrap();
        buffer.push(Data::DiskstatsRaw(diskstat_zero));
        buffer.push(Data::DiskstatsRaw(diskstat_one));
        for buf in buffer {
            processed_buffer.push(Diskstats::new().process_raw_data(buf).unwrap());
        }
        let json = Diskstats::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&key=Reads".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let data: EndDiskValues = serde_json::from_str(&json).unwrap();
        assert!(!data.data[0].name.is_empty());
        assert!(!data.data[0].values.is_empty());
    }
}
