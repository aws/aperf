use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use log::trace;
use procfs::Meminfo;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeminfoData;

impl MeminfoData {
    pub fn new() -> Self {
        MeminfoData
    }
}

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

fn get_meminfo_data(meminfo_type: MeminfoType, meminfo: &procfs::Meminfo) -> Option<u64> {
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

impl ProcessData for MeminfoData {
    fn process_raw_data(
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
    use super::MeminfoDataRaw;
    use crate::data::{CollectData, CollectorParams};

    #[test]
    fn test_collect_data() {
        let mut meminfodata_raw = MeminfoDataRaw::new();
        let params = CollectorParams::new();

        meminfodata_raw.collect_data(&params).unwrap();
        assert!(!meminfodata_raw.data.is_empty());
    }
}
