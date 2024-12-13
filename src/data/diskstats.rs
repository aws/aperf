extern crate ctor;

use crate::data::constants::*;
use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData, TimeEnum};
use crate::utils::DataMetrics;
use crate::visualizer::{DataVisualizer, GetData, GraphLimitType, GraphMetadata};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

pub static DISKSTATS_FILE_NAME: &str = "disk_stats";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskstatsRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl DiskstatsRaw {
    fn new() -> Self {
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
    fn new() -> Self {
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

fn get_values(values: Vec<Diskstats>, key: String) -> Result<String> {
    let mut ev: BTreeMap<String, DiskValues> = BTreeMap::new();
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
                    - *prev_value.get(&disk.name).unwrap() as i64) as f64
                    * mult_factor as f64
                    / factor as f64
            };
            let dv = DiskValue {
                time: (v.time - time_zero),
                value: stat_value,
            };
            metadata.update_limits(GraphLimitType::F64(stat_value));
            let dvs = ev.get_mut(&disk.name).unwrap();
            dvs.values.push(dv);
        }
        prev_data = v.clone();
    }
    let end_values = EndDiskValues {
        data: ev.into_values().collect(),
        metadata,
    };
    Ok(serde_json::to_string(&end_values)?)
}

fn get_keys() -> Result<String> {
    let mut end_values: Vec<String> = Vec::new();
    for key in DiskstatKeys::iter() {
        end_values.push(key.to_string());
    }
    Ok(serde_json::to_string(&end_values)?)
}

impl GetData for Diskstats {
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
        _metrics: &mut DataMetrics,
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
                get_values(values, key.to_string())
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_diskstats() {
    let diskstats_raw = DiskstatsRaw::new();
    let file_name = DISKSTATS_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::DiskstatsRaw(diskstats_raw.clone()),
        file_name.clone(),
        false,
    );
    let js_file_name = file_name.clone() + ".js";
    let diskstats = Diskstats::new();
    let dv = DataVisualizer::new(
        ProcessedData::Diskstats(diskstats),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/disk_stats.js")).to_string(),
        file_name.clone(),
    );

    PERFORMANCE_DATA
        .lock()
        .unwrap()
        .add_datatype(file_name.clone(), dt);

    VISUALIZATION_DATA
        .lock()
        .unwrap()
        .add_visualizer(file_name.clone(), dv);
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
