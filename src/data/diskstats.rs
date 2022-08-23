extern crate ctor;

use crate::data::{CollectData, Data, DataType};
use crate::PDResult;
use crate::PERFORMANCE_DATA;
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use procfs::diskstats;
use serde::{Deserialize, Serialize};

// Same as DiskStat from procfs/diskstats.rs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskStat {
    /// Device name
    pub name: String,

    /// Reads completed successfully
    ///
    /// This is the total number of reads completed successfully
    pub reads: u64,

    /// Reads merged
    ///
    /// The number of adjacent reads that have been merged for efficiency.
    pub merged: u64,

    /// Sectors read successfully
    ///
    /// This is the total number of sectors read successfully.
    pub sectors_read: u64,

    /// Time spent reading (ms)
    pub time_reading: u64,

    /// writes completed
    pub writes: u64,

    /// writes merged
    ///
    /// The number of adjacent writes that have been merged for efficiency.
    pub writes_merged: u64,

    /// Sectors written successfully
    pub sectors_written: u64,

    /// Time spent writing (ms)
    pub time_writing: u64,

    /// I/Os currently in progress
    pub in_progress: u64,

    /// Time spent doing I/Os (ms)
    pub time_in_progress: u64,

    /// Weighted time spent doing I/Os (ms)
    pub weighted_time_in_progress: u64,

    /// Discards completed successfully
    ///
    /// (since kernel 4.18)
    pub discards: u64,

    /// Discards merged
    pub discards_merged: u64,

    /// Sectors discarded
    pub sectors_discarded: u64,

    /// Time spent discarding
    pub time_discarding: u64,

    /// Flush requests completed successfully
    ///
    /// (since kernel 5.5)
    pub flushes: u64,

    /// Time spent flushing
    pub time_flushing: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Diskstats {
    pub time: DateTime<Utc>,
    pub disk_stats: Vec<DiskStat>,
}

impl Diskstats {
    fn new() -> Self {
        Diskstats {
            time: Utc::now(),
            disk_stats: Vec::new(),
        }
    }

    fn set_data(&mut self, data: Vec<DiskStat>) {
        self.disk_stats = data;
    }
}

impl CollectData for Diskstats {
    fn collect_data(&mut self) -> PDResult {
        let stats = diskstats().unwrap();
        let mut data = Vec::<DiskStat>::new();
        for disk in stats {
            let stat = DiskStat {
                name: disk.name,
                reads: disk.reads,
                merged: disk.merged,
                sectors_read: disk.sectors_read,
                time_reading: disk.time_reading,
                writes: disk.writes,
                writes_merged: disk.writes_merged,
                sectors_written: disk.sectors_written,
                time_writing: disk.time_writing,
                in_progress: disk.in_progress,
                time_in_progress: disk.time_in_progress,
                weighted_time_in_progress: disk.weighted_time_in_progress,
                discards: disk.discards.unwrap_or_default(),
                discards_merged: disk.discards_merged.unwrap_or_default(),
                sectors_discarded: disk.sectors_discarded.unwrap_or_default(),
                time_discarding: disk.time_discarding.unwrap_or_default(),
                flushes: disk.flushes.unwrap_or_default(),
                time_flushing: disk.time_flushing.unwrap_or_default(),
            };
            data.push(stat);
        }

        debug!("Diskstats:\n{:#?}", self);
        self.set_data(data);
        Ok(())
    }
}

#[ctor]
fn init_diskstats() {
    let disk_stats = Diskstats::new();

    let dt = DataType {
        data: Data::Diskstats(disk_stats),
        file_handle: None,
        file_name: "disk_stats".to_string(),
        dir_name: String::new(),
        full_path: String::new(),
    };
    PERFORMANCE_DATA
        .lock()
        .unwrap()
        .add_datatype("Disk Stats".to_string(), dt);
}

#[cfg(test)]
mod tests {
    use super::Diskstats;
    use crate::data::CollectData;

    #[test]
    fn test_collect_data() {
        let mut diskstats = Diskstats::new();

        assert!(diskstats.collect_data().unwrap() == ());
        assert!(diskstats.disk_stats.len() != 0);
    }
}
