extern crate ctor;

use anyhow::Result;
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use procfs::diskstats;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumString, EnumIter};

pub static DISKSTATS_FILE_NAME: &str = "disk_stats";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskStat {
    pub name: String,
    pub stat: HashMap<String, u64>,
}

impl DiskStat {
    fn new(name: String) -> Self {
        DiskStat {
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
    pub disk_stats: Vec<DiskStat>,
}

impl Diskstats {
    fn new() -> Self {
        Diskstats {
            time: TimeEnum::DateTime(Utc::now()),
            disk_stats: Vec::new(),
        }
    }

    fn set_data(&mut self, data: Vec<DiskStat>) {
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

impl CollectData for Diskstats {
    fn collect_data(&mut self) -> Result<()> {
        let stats = diskstats().unwrap();
        let mut data = Vec::<DiskStat>::new();
        for disk in stats {
            let mut diskstat = DiskStat::new(disk.name);
            diskstat.add(DiskstatKeys::Reads.to_string(), disk.reads);
            diskstat.add(DiskstatKeys::Merged.to_string(), disk.merged);
            diskstat.add(DiskstatKeys::SectorsRead.to_string(), disk.sectors_read);
            diskstat.add(DiskstatKeys::TimeReading.to_string(), disk.time_reading);
            diskstat.add(DiskstatKeys::Writes.to_string(), disk.writes);
            diskstat.add(DiskstatKeys::WritesMerged.to_string(), disk.writes_merged);
            diskstat.add(DiskstatKeys::SectorsWritten.to_string(), disk.sectors_written);
            diskstat.add(DiskstatKeys::TimeWriting.to_string(), disk.time_writing);
            diskstat.add(DiskstatKeys::InProgress.to_string(), disk.in_progress);
            diskstat.add(DiskstatKeys::TimeInProgress.to_string(), disk.time_in_progress);
            diskstat.add(DiskstatKeys::WeightedTimeInProgress.to_string(), disk.weighted_time_in_progress);
            // Following since kernel 4.18
            diskstat.add(DiskstatKeys::Discards.to_string(), disk.discards.unwrap_or_default());
            diskstat.add(DiskstatKeys::DiscardsMerged.to_string(), disk.discards_merged.unwrap_or_default());
            diskstat.add(DiskstatKeys::SectorsDiscarded.to_string(), disk.sectors_discarded.unwrap_or_default());
            diskstat.add(DiskstatKeys::TimeDiscarding.to_string(), disk.time_discarding.unwrap_or_default());
            // Following since kernel 5.5
            diskstat.add(DiskstatKeys::Flushes.to_string(), disk.flushes.unwrap_or_default());
            diskstat.add(DiskstatKeys::TimeFlushing.to_string(), disk.time_flushing.unwrap_or_default());

            data.push(diskstat);
        }

        debug!("Diskstats:\n{:#?}", self);
        self.set_data(data);
        self.set_time(TimeEnum::DateTime(Utc::now()));
        Ok(())
    }
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
    fn get_data(&mut self, buffer: Vec<Data>, query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                Data::Diskstats(ref value) => values.push(value.clone()),
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
    let diskstats = Diskstats::new();
    let file_name = DISKSTATS_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::Diskstats(diskstats.clone()),
        file_name.clone(),
        false
    );
    let js_file_name = file_name.clone() + &".js".to_string();
    let dv = DataVisualizer::new(
        Data::Diskstats(diskstats),
        file_name.clone(),
        js_file_name,
        include_str!("../bin/html_files/js/disk_stats.js").to_string(),
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
    use super::{Diskstats, DiskstatKeys, DiskValues, Key};
    use crate::data::{CollectData, Data};
    use crate::visualizer::GetData;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    #[test]
    fn test_collect_data() {
        let mut diskstats = Diskstats::new();

        assert!(diskstats.collect_data().unwrap() == ());
        assert!(diskstats.disk_stats.len() != 0);
    }

    #[test]
    fn test_keys() {
        let mut stat = Diskstats::new();
        let mut key_map = HashMap::new();
        for key in DiskstatKeys::iter() {
            key_map.insert(key.to_string(), 0);
        }
        stat.collect_data().unwrap();
        let keys: Vec<String> = stat.disk_stats[0].stat.clone().into_keys().collect();
        for key in keys {
            assert!(key_map.contains_key(&key));
            let value = key_map.get(&key).unwrap();
            key_map.insert(key, value+1);
        }
        let mut values: Vec<u64> = key_map.into_values().collect();
        values.dedup();
        assert!(values.len() == 1);
    }

    #[test]
    fn test_get_data_keys() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut diskstat = Diskstats::new();

        diskstat.collect_data().unwrap();
        buffer.push(Data::Diskstats(diskstat));
        let json = Diskstats::new().get_data(buffer, "run=test&get=keys".to_string()).unwrap();
        let values: Vec<Key> = serde_json::from_str(&json).unwrap();
        assert!(values.len() > 0);
    }

    #[test]
    fn test_get_data_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut diskstat_zero = Diskstats::new();
        let mut diskstat_one = Diskstats::new();

        diskstat_zero.collect_data().unwrap();
        diskstat_one.collect_data().unwrap();
        buffer.push(Data::Diskstats(diskstat_zero));
        buffer.push(Data::Diskstats(diskstat_one));
        let json = Diskstats::new().get_data(buffer, "run=test&get=values&key=Reads".to_string()).unwrap();
        let values: Vec<DiskValues> = serde_json::from_str(&json).unwrap();
        assert!(values[0].name != "");
        assert!(values[0].values.len() > 0);
    }
}
