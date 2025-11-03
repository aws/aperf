use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::{add_metrics, get_data_name_from_type, DataMetrics, Metric};
use crate::visualizer::{GetData, GraphLimitType, GraphMetadata, ReportParams};
use crate::PDError;
use anyhow::Result;
use chrono::prelude::*;
use log::trace;
use procfs::Meminfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufReader;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

/// Gather Meminfo raw data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeminfoDataRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl Default for MeminfoDataRaw {
    fn default() -> Self {
        Self::new()
    }
}

impl MeminfoDataRaw {
    pub fn new() -> Self {
        MeminfoDataRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for MeminfoDataRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/meminfo")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Debug, Display, EnumString, EnumIter)]
pub enum MeminfoKeys {
    #[strum(serialize = "Mem Total")]
    MemTotal,
    #[strum(serialize = "Mem Free")]
    MemFree,
    #[strum(serialize = "Mem Available")]
    MemAvailable,
    Buffers,
    Cached,
    #[strum(serialize = "Swap Cached")]
    SwapCached,
    Active,
    Inactive,
    #[strum(serialize = "Active Anon")]
    ActiveAnon,
    #[strum(serialize = "Inactive Anon")]
    InactiveAnon,
    #[strum(serialize = "Active File")]
    ActiveFile,
    #[strum(serialize = "Inactive File")]
    InactiveFile,
    Unevictable,
    Mlocked,
    #[strum(serialize = "Mmap Copy")]
    MmapCopy,
    #[strum(serialize = "Swap Total")]
    SwapTotal,
    #[strum(serialize = "Swap Free")]
    SwapFree,
    Dirty,
    Writeback,
    #[strum(serialize = "Anon Pages")]
    AnonPages,
    Mapped,
    Shmem,
    #[strum(serialize = "K Reclaimable")]
    KReclaimable,
    Slab,
    #[strum(serialize = "S Reclaimable")]
    SReclaimable,
    #[strum(serialize = "S Unreclaim")]
    SUnreclaim,
    #[strum(serialize = "Kernel Stack")]
    KernelStack,
    #[strum(serialize = "Page Tables")]
    PageTables,
    Quicklists,
    #[strum(serialize = "NFS Unstable")]
    NfsUnstable,
    Bounce,
    #[strum(serialize = "Writeback Tmp")]
    WritebackTmp,
    #[strum(serialize = "Commit Limit")]
    CommitLimit,
    #[strum(serialize = "Committed As")]
    CommittedAs,
    #[strum(serialize = "Vmalloc Total")]
    VmallocTotal,
    #[strum(serialize = "Vmalloc Used")]
    VmallocUsed,
    #[strum(serialize = "Vmalloc Chunk")]
    VmallocChunk,
    #[strum(serialize = "Per CPU")]
    PerCpu,
    #[strum(serialize = "Hardware Corrupted")]
    HardwareCorrupted,
    #[strum(serialize = "Anon HugePages")]
    AnonHugepages,
    #[strum(serialize = "Shmem HugePages")]
    ShmemHugepages,
    #[strum(serialize = "Shmem Pmd Mapped")]
    ShmemPmdMapped,
    #[strum(serialize = "File Pmd Mapped")]
    FilePmdMapped,
    #[strum(serialize = "File Huge Pages")]
    FileHugePages,
    #[strum(serialize = "Cma Total")]
    CmaTotal,
    #[strum(serialize = "Cma Free")]
    CmaFree,
    #[strum(serialize = "HugePages_Total")]
    HugepagesTotal,
    #[strum(serialize = "HugePages_Free")]
    HugepagesFree,
    #[strum(serialize = "HugePages_Rsvd")]
    HugepagesRsvd,
    #[strum(serialize = "HugePages_Surp")]
    HugepagesSurp,
    Hugepagesize,
    Hugetlb,
    #[strum(serialize = "Direct Map 4K")]
    DirectMap4k,
    #[strum(serialize = "Direct Map 4M")]
    DirectMap4M,
    #[strum(serialize = "Direct Map 2M")]
    DirectMap2M,
    #[strum(serialize = "Direct Map 1G")]
    DirectMap1G,
}

fn get_keys() -> Result<String> {
    let mut end_values: Vec<String> = Vec::new();
    for key in MeminfoKeys::iter() {
        end_values.push(key.to_string());
    }
    Ok(serde_json::to_string(&end_values)?)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemData {
    pub name: String,
    pub values: Vec<MemEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemEntry {
    pub time: TimeEnum,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EndMemValues {
    pub data: MemData,
    pub metadata: GraphMetadata,
}

fn get_values(values: Vec<MeminfoData>, key: String, metrics: &mut DataMetrics) -> Result<String> {
    let time_zero = values[0].time;
    let mut metric = Metric::new(key.clone());
    let mut metadata = GraphMetadata::new();
    let mut end_value = MemData {
        name: key.clone(),
        values: Vec::new(),
    };
    for v in values {
        let value = v
            .data
            .get(&key)
            .ok_or(PDError::VisualizerMeminfoValueGetError(key.to_string()))?;
        /* Bytes => kB */
        let mementry = MemEntry {
            time: v.time - time_zero,
            value: *value / 1024,
        };
        metric.insert_value(mementry.value as f64);
        metadata.update_limits(GraphLimitType::UInt64(mementry.value));
        end_value.values.push(mementry);
    }
    let end_values = EndMemValues {
        data: end_value,
        metadata,
    };
    add_metrics(
        key,
        &mut metric,
        metrics,
        get_data_name_from_type::<MeminfoData>().to_string(),
    )?;
    Ok(serde_json::to_string(&end_values)?)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeminfoData {
    pub time: TimeEnum,
    pub data: HashMap<String, u64>,
}

impl MeminfoData {
    pub fn new() -> Self {
        MeminfoData {
            time: TimeEnum::DateTime(Utc::now()),
            data: HashMap::new(),
        }
    }

    fn add(&mut self, key: String, value: u64) {
        self.data.insert(key, value);
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }
}

// TODO: ------------------------------------------------------------------------------------------
//       Below are the new implementation to process meminfo into uniform data format. Remove
//       the original for the migration.

#[derive(EnumIter, Display, Clone, Copy, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum MeminfoType {
    MemTotal,
    MemFree,
    MemAvailable,
    Buffers,
    Cached,
    SwapCached,
    Active,
    Inactive,
    ActiveAnon,
    InactiveAnon,
    ActiveFile,
    InactiveFile,
    Unevictable,
    Mlocked,
    MmapCopy,
    SwapTotal,
    SwapFree,
    Dirty,
    Writeback,
    AnonPages,
    Mapped,
    Shmem,
    KReclaimable,
    Slab,
    SReclaimable,
    SUnreclaim,
    KernelStack,
    PageTables,
    Quicklists,
    NfsUnstable,
    Bounce,
    WritebackTmp,
    CommitLimit,
    CommittedAs,
    VmallocTotal,
    VmallocUsed,
    VmallocChunk,
    PerCpu,
    HardwareCorrupted,
    AnonHugepages,
    ShmemHugepages,
    ShmemPmdMapped,
    FilePmdMapped,
    FileHugePages,
    CmaTotal,
    CmaFree,
    HugepagesTotal,
    HugepagesFree,
    HugepagesRsvd,
    HugepagesSurp,
    Hugepagesize,
    Hugetlb,
    DirectMap4k,
    DirectMap4M,
    DirectMap2M,
    DirectMap1G,
}

fn get_meminfo_data(meminfo_type: MeminfoType, meminfo: &Meminfo) -> Option<u64> {
    match meminfo_type {
        MeminfoType::MemTotal => Some(meminfo.mem_total),
        MeminfoType::MemFree => Some(meminfo.mem_free),
        MeminfoType::MemAvailable => meminfo.mem_available,
        MeminfoType::Buffers => Some(meminfo.buffers),
        MeminfoType::Cached => Some(meminfo.cached),
        MeminfoType::SwapCached => Some(meminfo.swap_cached),
        MeminfoType::Active => Some(meminfo.active),
        MeminfoType::Inactive => Some(meminfo.inactive),
        MeminfoType::ActiveAnon => meminfo.active_anon,
        MeminfoType::InactiveAnon => meminfo.inactive_anon,
        MeminfoType::ActiveFile => meminfo.active_file,
        MeminfoType::InactiveFile => meminfo.inactive_file,
        MeminfoType::Unevictable => meminfo.unevictable,
        MeminfoType::Mlocked => meminfo.mlocked,
        MeminfoType::MmapCopy => meminfo.mmap_copy,
        MeminfoType::SwapTotal => Some(meminfo.swap_total),
        MeminfoType::SwapFree => Some(meminfo.swap_free),
        MeminfoType::Dirty => Some(meminfo.dirty),
        MeminfoType::Writeback => Some(meminfo.writeback),
        MeminfoType::AnonPages => meminfo.anon_pages,
        MeminfoType::Mapped => Some(meminfo.mapped),
        MeminfoType::Shmem => meminfo.shmem,
        MeminfoType::KReclaimable => meminfo.k_reclaimable,
        MeminfoType::Slab => Some(meminfo.slab),
        MeminfoType::SReclaimable => meminfo.s_reclaimable,
        MeminfoType::SUnreclaim => meminfo.s_unreclaim,
        MeminfoType::KernelStack => meminfo.kernel_stack,
        MeminfoType::PageTables => meminfo.page_tables,
        MeminfoType::Quicklists => meminfo.quicklists,
        MeminfoType::NfsUnstable => meminfo.nfs_unstable,
        MeminfoType::Bounce => meminfo.bounce,
        MeminfoType::WritebackTmp => meminfo.writeback_tmp,
        MeminfoType::CommitLimit => meminfo.commit_limit,
        MeminfoType::CommittedAs => Some(meminfo.committed_as),
        MeminfoType::VmallocTotal => Some(meminfo.vmalloc_total),
        MeminfoType::VmallocUsed => Some(meminfo.vmalloc_used),
        MeminfoType::VmallocChunk => Some(meminfo.vmalloc_chunk),
        MeminfoType::PerCpu => meminfo.per_cpu,
        MeminfoType::HardwareCorrupted => meminfo.hardware_corrupted,
        MeminfoType::AnonHugepages => meminfo.anon_hugepages,
        MeminfoType::ShmemHugepages => meminfo.shmem_hugepages,
        MeminfoType::ShmemPmdMapped => meminfo.shmem_pmd_mapped,
        MeminfoType::FilePmdMapped => meminfo.file_pmd_mapped,
        MeminfoType::FileHugePages => meminfo.file_huge_pages,
        MeminfoType::CmaTotal => meminfo.cma_total,
        MeminfoType::CmaFree => meminfo.cma_free,
        MeminfoType::HugepagesTotal => meminfo.hugepages_total,
        MeminfoType::HugepagesFree => meminfo.hugepages_free,
        MeminfoType::HugepagesRsvd => meminfo.hugepages_rsvd,
        MeminfoType::HugepagesSurp => meminfo.hugepages_surp,
        MeminfoType::Hugepagesize => meminfo.hugepagesize,
        MeminfoType::Hugetlb => meminfo.hugetlb,
        MeminfoType::DirectMap4k => meminfo.direct_map_4k,
        MeminfoType::DirectMap4M => meminfo.direct_map_4M,
        MeminfoType::DirectMap2M => meminfo.direct_map_2M,
        MeminfoType::DirectMap1G => meminfo.direct_map_1G,
    }
}

// TODO: ------------------------------------------------------------------------------------------

impl GetData for MeminfoData {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::MeminfoDataRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let reader = BufReader::new(raw_value.data.as_bytes());
        let meminfo = Meminfo::from_reader(reader)?;
        let mut meminfo_data = MeminfoData::new();
        meminfo_data.add(MeminfoKeys::MemTotal.to_string(), meminfo.mem_total);
        meminfo_data.add(MeminfoKeys::MemFree.to_string(), meminfo.mem_free);
        meminfo_data.add(
            MeminfoKeys::MemAvailable.to_string(),
            meminfo.mem_available.unwrap_or_default(),
        );
        meminfo_data.add(MeminfoKeys::Buffers.to_string(), meminfo.buffers);
        meminfo_data.add(MeminfoKeys::Cached.to_string(), meminfo.cached);
        meminfo_data.add(MeminfoKeys::SwapCached.to_string(), meminfo.swap_cached);
        meminfo_data.add(MeminfoKeys::Active.to_string(), meminfo.active);
        meminfo_data.add(MeminfoKeys::Inactive.to_string(), meminfo.inactive);
        meminfo_data.add(
            MeminfoKeys::ActiveAnon.to_string(),
            meminfo.active_anon.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::InactiveAnon.to_string(),
            meminfo.inactive_anon.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::ActiveFile.to_string(),
            meminfo.active_file.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::InactiveFile.to_string(),
            meminfo.inactive_file.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::Unevictable.to_string(),
            meminfo.unevictable.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::Mlocked.to_string(),
            meminfo.mlocked.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::MmapCopy.to_string(),
            meminfo.mmap_copy.unwrap_or_default(),
        );
        meminfo_data.add(MeminfoKeys::SwapTotal.to_string(), meminfo.swap_total);
        meminfo_data.add(MeminfoKeys::SwapFree.to_string(), meminfo.swap_free);
        meminfo_data.add(MeminfoKeys::Dirty.to_string(), meminfo.dirty);
        meminfo_data.add(MeminfoKeys::Writeback.to_string(), meminfo.writeback);
        meminfo_data.add(
            MeminfoKeys::AnonPages.to_string(),
            meminfo.anon_pages.unwrap_or_default(),
        );
        meminfo_data.add(MeminfoKeys::Mapped.to_string(), meminfo.mapped);
        meminfo_data.add(
            MeminfoKeys::Shmem.to_string(),
            meminfo.shmem.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::KReclaimable.to_string(),
            meminfo.k_reclaimable.unwrap_or_default(),
        );
        meminfo_data.add(MeminfoKeys::Slab.to_string(), meminfo.slab);
        meminfo_data.add(
            MeminfoKeys::SReclaimable.to_string(),
            meminfo.s_reclaimable.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::SUnreclaim.to_string(),
            meminfo.s_unreclaim.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::KernelStack.to_string(),
            meminfo.kernel_stack.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::PageTables.to_string(),
            meminfo.page_tables.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::Quicklists.to_string(),
            meminfo.quicklists.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::NfsUnstable.to_string(),
            meminfo.nfs_unstable.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::Bounce.to_string(),
            meminfo.bounce.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::WritebackTmp.to_string(),
            meminfo.writeback_tmp.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::CommitLimit.to_string(),
            meminfo.commit_limit.unwrap_or_default(),
        );
        meminfo_data.add(MeminfoKeys::CommittedAs.to_string(), meminfo.committed_as);
        meminfo_data.add(MeminfoKeys::VmallocTotal.to_string(), meminfo.vmalloc_total);
        meminfo_data.add(MeminfoKeys::VmallocUsed.to_string(), meminfo.vmalloc_used);
        meminfo_data.add(MeminfoKeys::VmallocChunk.to_string(), meminfo.vmalloc_chunk);
        meminfo_data.add(
            MeminfoKeys::PerCpu.to_string(),
            meminfo.per_cpu.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::HardwareCorrupted.to_string(),
            meminfo.hardware_corrupted.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::AnonHugepages.to_string(),
            meminfo.anon_hugepages.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::ShmemHugepages.to_string(),
            meminfo.shmem_hugepages.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::ShmemPmdMapped.to_string(),
            meminfo.shmem_pmd_mapped.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::FilePmdMapped.to_string(),
            meminfo.file_pmd_mapped.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::FileHugePages.to_string(),
            meminfo.file_huge_pages.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::CmaTotal.to_string(),
            meminfo.cma_total.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::CmaFree.to_string(),
            meminfo.cma_free.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::HugepagesTotal.to_string(),
            meminfo.hugepages_total.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::HugepagesFree.to_string(),
            meminfo.hugepages_free.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::HugepagesRsvd.to_string(),
            meminfo.hugepages_rsvd.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::HugepagesSurp.to_string(),
            meminfo.hugepages_surp.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::Hugepagesize.to_string(),
            meminfo.hugepagesize.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::Hugetlb.to_string(),
            meminfo.hugetlb.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::DirectMap4k.to_string(),
            meminfo.direct_map_4k.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::DirectMap4M.to_string(),
            meminfo.direct_map_4M.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::DirectMap2M.to_string(),
            meminfo.direct_map_2M.unwrap_or_default(),
        );
        meminfo_data.add(
            MeminfoKeys::DirectMap1G.to_string(),
            meminfo.direct_map_1G.unwrap_or_default(),
        );
        meminfo_data.set_time(raw_value.time);
        let processed_data = ProcessedData::MeminfoData(meminfo_data);
        Ok(processed_data)
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
                ProcessedData::MeminfoData(ref value) => values.push(value.clone()),
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

    fn process_raw_data_new(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();

        // initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::MeminfoDataRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            let meminfo = Meminfo::from_reader(raw_value.data.as_bytes())?;

            for meminfo_type in MeminfoType::iter() {
                let meminfo_data = match get_meminfo_data(meminfo_type, &meminfo) {
                    Some(meminfo_data) => meminfo_data,
                    None => continue,
                } as f64;

                let meminfo_value = match meminfo_type {
                    // These types are count and do not have any units, so use the data
                    // directly
                    MeminfoType::HugepagesTotal
                    | MeminfoType::HugepagesFree
                    | MeminfoType::HugepagesRsvd
                    | MeminfoType::HugepagesSurp => meminfo_data,
                    // All other types are in bytes, so converting it to KB
                    _ => meminfo_data / 1024.0,
                };

                let meminfo_metric = time_series_data
                    .metrics
                    .entry(meminfo_type.to_string())
                    .or_insert_with_key(|meminfo_metric_name| {
                        let mut _mem_info_metric =
                            TimeSeriesMetric::new(meminfo_metric_name.clone());
                        _mem_info_metric.series.push(Series::new(None));
                        _mem_info_metric
                    });
                let meminfo_series = &mut meminfo_metric.series[0];
                meminfo_series.time_diff.push(time_diff);
                meminfo_series.values.push(meminfo_value);
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
        time_series_data.sorted_metric_names = MeminfoType::iter()
            .map(|meminfo_type| meminfo_type.to_string())
            .collect();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    use super::{EndMemValues, MeminfoData, MeminfoDataRaw, MeminfoKeys};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    #[test]
    fn test_collect_data() {
        let mut meminfodata_raw = MeminfoDataRaw::new();
        let params = CollectorParams::new();

        meminfodata_raw.collect_data(&params).unwrap();
        assert!(!meminfodata_raw.data.is_empty());
    }

    #[test]
    fn test_keys() {
        let mut meminfodata_raw = MeminfoDataRaw::new();
        let mut key_map = HashMap::new();
        for key in MeminfoKeys::iter() {
            key_map.insert(key.to_string(), 0);
        }
        let params = CollectorParams::new();
        meminfodata_raw.collect_data(&params).unwrap();
        let processed_data = MeminfoData::new()
            .process_raw_data(Data::MeminfoDataRaw(meminfodata_raw))
            .unwrap();
        let meminfodata = match processed_data {
            ProcessedData::MeminfoData(value) => value,
            _ => unreachable!("Invalid data type in processed data"),
        };
        let keys: Vec<String> = meminfodata.data.clone().into_keys().collect();
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
        let mut buffer: Vec<Data> = Vec::new();
        let mut meminfodata_raw = MeminfoDataRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::new();
        let params = CollectorParams::new();

        meminfodata_raw.collect_data(&params).unwrap();
        buffer.push(Data::MeminfoDataRaw(meminfodata_raw));
        processed_buffer.push(
            MeminfoData::new()
                .process_raw_data(buffer[0].clone())
                .unwrap(),
        );
        let json = MeminfoData::new()
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
        let mut buffer: Vec<Data> = Vec::new();
        let mut meminfodata_raw_zero = MeminfoDataRaw::new();
        let mut meminfodata_raw_one = MeminfoDataRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::new();
        let params = CollectorParams::new();

        meminfodata_raw_zero.collect_data(&params).unwrap();
        meminfodata_raw_one.collect_data(&params).unwrap();
        buffer.push(Data::MeminfoDataRaw(meminfodata_raw_zero));
        buffer.push(Data::MeminfoDataRaw(meminfodata_raw_one));
        for buf in buffer {
            processed_buffer.push(MeminfoData::new().process_raw_data(buf).unwrap());
        }
        let json = MeminfoData::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&key=Mem Total".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let memdata: EndMemValues = serde_json::from_str(&json).unwrap();
        assert_eq!(memdata.data.name, "Mem Total");
        assert!(!memdata.data.values.is_empty());
    }
}
