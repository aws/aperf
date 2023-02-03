extern crate ctor;

use anyhow::Result;
use crate::data::{CollectData, Data, ProcessedData, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumString, EnumIter};

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
    fn collect_data(&mut self) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/diskstats")?;
        debug!("{:#?}", self.data);
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
    Merged,
    #[strum(serialize = "Sectors Read")]
    SectorsRead,
    #[strum(serialize = "Time Reading")]
    TimeReading,
    Writes,
    #[strum(serialize = "Writes Merged")]
    WritesMerged,
    #[strum(serialize = "Sectors Written")]
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
    #[strum(serialize = "Sectors Discarded")]
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
        let discards = s.next().and_then(|s| u64::from_str_radix(s, 10).ok());
        let discards_merged = s.next().and_then(|s| u64::from_str_radix(s, 10).ok());
        let sectors_discarded = s.next().and_then(|s| u64::from_str_radix(s, 10).ok());
        let time_discarding = s.next().and_then(|s| u64::from_str_radix(s, 10).ok());
        // Following since kernel 5.5
        let flushes = s.next().and_then(|s| u64::from_str_radix(s, 10).ok());
        let time_flushing = s.next().and_then(|s| u64::from_str_radix(s, 10).ok());

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
        diskstat.add(DiskstatKeys::WeightedTimeInProgress.to_string(), weighted_time_in_progress?);
        diskstat.add(DiskstatKeys::Discards.to_string(), discards.unwrap_or_default());
        diskstat.add(DiskstatKeys::DiscardsMerged.to_string(), discards_merged.unwrap_or_default());
        diskstat.add(DiskstatKeys::SectorsDiscarded.to_string(), sectors_discarded.unwrap_or_default());
        diskstat.add(DiskstatKeys::TimeDiscarding.to_string(), time_discarding.unwrap_or_default());
        diskstat.add(DiskstatKeys::Flushes.to_string(), flushes.unwrap_or_default());
        diskstat.add(DiskstatKeys::TimeFlushing.to_string(), time_flushing.unwrap_or_default());

        data.push(diskstat);
    }
    diskstats.set_data(data);
    let processed_data = ProcessedData::Diskstats(diskstats.clone());
    Ok(processed_data)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskValue {
    pub time: TimeEnum,
    pub value: u64,
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

fn get_values(values: Vec<Diskstats>, key: String) -> Result<String> {
    let mut ev: BTreeMap<String, DiskValues> = BTreeMap::new();
    let disk_names = get_disk_names(values[0].clone());
    for name in disk_names {
        let dv = DiskValues::new(name.clone());
        ev.insert(name, dv);
    }
    let time_zero = values[0].time;
    for value in values {
        for disk in value.disk_stats {
            let dv = DiskValue {
                time: (value.time - time_zero),
                value: *disk.stat.get(&key.clone()).unwrap(),
            };
            let dvs = ev.get_mut(&disk.name).unwrap();
            dvs.values.push(dv);
        }
    }
    let end_values: Vec<DiskValues> = ev.into_values().collect();
    Ok(serde_json::to_string(&end_values)?)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Key {
    pub name: String,
    pub unit: String,
}

fn get_keys() -> Result<String> {
    let mut end_values: Vec<Key> = Vec::new();
    for key in DiskstatKeys::iter() {
        let mut unit: String = "Count".to_string();
        if key.to_string().contains("Time") {
            unit = "Time (ms)".to_string();
        }
        end_values.push(Key {name: key.to_string(), unit: unit});
    }
    return Ok(serde_json::to_string(&end_values)?)
}

impl GetData for Diskstats {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        process_collected_raw_data(buffer)
    }

    fn get_data(&mut self, buffer: Vec<ProcessedData>, query: String) -> Result<String> {
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
            "keys" => return get_keys(),
            "values" => {
                let (_, key) = &param[2];
                return get_values(values, key.to_string());
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
        false
    );
    let js_file_name = file_name.clone() + &".js".to_string();
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
    use super::{Diskstats, DiskstatsRaw, DiskstatKeys, DiskValues, Key};
    use crate::data::{CollectData, Data, ProcessedData};
    use crate::visualizer::GetData;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    #[test]
    fn test_collect_data() {
        let mut diskstats = DiskstatsRaw::new();

        assert!(diskstats.collect_data().unwrap() == ());
        assert!(!diskstats.data.is_empty());
    }

    #[test]
    fn test_keys() {
        let mut stat = DiskstatsRaw::new();
        let mut key_map = HashMap::new();
        for key in DiskstatKeys::iter() {
            key_map.insert(key.to_string(), 0);
        }
        stat.collect_data().unwrap();
        let processed_stat = Diskstats::new().process_raw_data(Data::DiskstatsRaw(stat)).unwrap();
        let mut disk_stat: Diskstats = Diskstats::new();
        match processed_stat {
            ProcessedData::Diskstats(value) => disk_stat = value,
            _ => assert!(false, "Invalid data type in processed data"),
        };
        let keys: Vec<String> = disk_stat.disk_stats[0].stat.clone().into_keys().collect();
        for key in keys {
            assert!(key_map.contains_key(&key));
            let value = key_map.get(&key).unwrap() + 1;
            key_map.insert(key, value);
        }
        let mut values: Vec<u64> = key_map.into_values().collect();
        values.dedup();
        assert!(values.len() == 1);
    }

    #[test]
    fn test_get_data_keys() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut diskstat = DiskstatsRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();

        diskstat.collect_data().unwrap();
        buffer.push(Data::DiskstatsRaw(diskstat));
        processed_buffer.push(Diskstats::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Diskstats::new().get_data(processed_buffer, "run=test&get=keys".to_string()).unwrap();
        let values: Vec<Key> = serde_json::from_str(&json).unwrap();
        assert!(values.len() > 0);
    }

    #[test]
    fn test_get_data_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut diskstat_zero = DiskstatsRaw::new();
        let mut diskstat_one = DiskstatsRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();

        diskstat_zero.collect_data().unwrap();
        diskstat_one.collect_data().unwrap();
        buffer.push(Data::DiskstatsRaw(diskstat_zero));
        buffer.push(Data::DiskstatsRaw(diskstat_one));
        for buf in buffer {
            processed_buffer.push(Diskstats::new().process_raw_data(buf).unwrap());
        }
        let json = Diskstats::new().get_data(processed_buffer, "run=test&get=values&key=Reads".to_string()).unwrap();
        let values: Vec<DiskValues> = serde_json::from_str(&json).unwrap();
        assert!(values[0].name != "");
        assert!(values[0].values.len() > 0);
    }
}
