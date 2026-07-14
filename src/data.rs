pub mod aperf_runlog;
pub mod aperf_stats;
pub mod common;
pub mod cpu_utilization;
pub mod diskstats;
pub mod efa_stat;
pub mod ena_stat;
pub mod hotline;
pub mod interrupts;
pub mod java_profile;
pub mod kernel_config;
pub mod memalloc;
pub mod meminfo;
pub mod netstat;
pub mod numastat;
pub mod perf_profile;
pub mod perf_stat;
pub mod processes;
pub mod sysctl;
pub mod systeminfo;
pub mod vmstat;

use crate::analytics::AnalyticalRule;
use crate::data_processing::{DataProcessingEngine, DataProcessor, ReportParams};
use crate::{find_file, get_data_name_from_type, APERF_FILE_FORMAT};
use std::fs::File;
use std::fs::OpenOptions;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use {
    crate::data_collection::{DataCollectionEngine, InitParams},
    std::collections::HashSet,
};

use anyhow::{bail, Result};
use aperf_runlog::AperfRunlog;
use aperf_stats::AperfStat;
use chrono::prelude::*;
use common::data_formats::AperfData;
use cpu_utilization::{CpuUtilization, CpuUtilizationRaw};
use diskstats::{Diskstats, DiskstatsRaw};
use efa_stat::{EfaStat, EfaStatRaw};
use ena_stat::{EnaStat, EnaStatRaw};
use hotline::{Hotline, HotlineRaw};
use include_dir::{include_dir, Dir};
use interrupts::{InterruptData, InterruptDataRaw};
use java_profile::{JavaProfile, JavaProfileRaw};
use kernel_config::KernelConfig;
use memalloc::{MemallocData, MemallocDataRaw};
use meminfo::{MeminfoData, MeminfoDataRaw};
use netstat::{Netstat, NetstatRaw};
use numastat::{Numastat, NumastatRaw};
use perf_profile::{FlamegraphRaw, PerfProfile, PerfProfileRaw};
use perf_stat::{PerfStat, PerfStatRaw};
use processes::{Processes, ProcessesRaw};
use serde::{Deserialize, Serialize};
use std::ops::Sub;
use sysctl::SysctlData;
use systeminfo::SystemInfo;
use vmstat::{Vmstat, VmstatRaw};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum TimeEnum {
    DateTime(DateTime<Utc>),
    TimeDiff(u64),
}

impl Sub for TimeEnum {
    type Output = TimeEnum;

    fn sub(self, rhs: TimeEnum) -> TimeEnum {
        let self_time = match self {
            TimeEnum::DateTime(value) => value,
            _ => panic!("Cannot perform subtract op on TimeEnum::TimeDiff"),
        };
        let other_time = match rhs {
            TimeEnum::DateTime(value) => value,
            _ => panic!("Cannot perform subtract op on TimeEnum::TimeDiff"),
        };
        let time_diff = (self_time - other_time).num_milliseconds() as u64;
        // Round up to the nearest second
        TimeEnum::TimeDiff((time_diff + 500) / 1000)
    }
}

/// This macro expands to:
/// 1. define the Data Enum to hold all record data structs for collection
/// 2. define the function that instantiates all data structs and adds them
///    to the PerformanceData object.
/// 3. collect the names of data to be collected by default.
macro_rules! data {
    ( $( $data:ident ),* ) => {

        #[cfg(target_os = "linux")]
        lazy_static! {
            pub static ref DEFAULT_DATA_NAMES: Vec<&'static str> = get_default_data_names();
        }

        #[cfg(target_os = "linux")]
        fn get_default_data_names() -> Vec<&'static str> {
            let mut default_data_names: Vec<&'static str> = Vec::new();
            $(
                if !($data::is_perf_profile() || $data::is_java_profile()) {
                    default_data_names.push(get_data_name_from_type::<$data>());
                }
            )*

            #[cfg(not(feature = "hotline"))]
            default_data_names.retain(
                |&data_name| data_name != get_data_name_from_type::<Hotline>()
            );

            default_data_names
        }

        #[derive(Debug, Deserialize, Serialize)]
        pub enum Data {
            $(
                $data($data),
            )*
        }

        #[cfg(target_os = "linux")]
        impl Data {
            pub fn collect_data(&mut self, params: &InitParams) -> Result<()> {
                match self {
                    $(
                        Data::$data(ref mut value) => value.collect_data(&params)?,
                    )*
                }
                Ok(())
            }

            pub fn prepare_data_collector(&mut self, params: &InitParams) -> Result<()> {
                match self {
                    $(
                        Data::$data(ref mut value) => value.prepare_data_collector(params)?,
                    )*
                }
                Ok(())
            }

            pub fn finish_data_collection(&mut self, params: &InitParams) -> Result<()> {
                match self {
                    $(
                        Data::$data(ref mut value) => value.finish_data_collection(params)?,
                    )*
                }
                Ok(())
            }

            pub fn is_static(&self) -> bool {
                match self {
                    $(
                        Data::$data(_) => $data::is_static(),
                    )*
                }
            }

            pub fn is_profile(&self) -> bool {
                match self {
                    $(
                        Data::$data(_) => $data::is_perf_profile() || $data::is_java_profile(),
                    )*
                }
            }
        }

        #[cfg(target_os = "linux")]
        pub fn initialize_data_collection_engine(data_collection_engine: &mut DataCollectionEngine, data_names_to_collect: HashSet<String>, perf_profile_enabled: bool, java_profile_enabled: bool) {
            $(
                let data_name = get_data_name_from_type::<$data>();

                if $data::is_perf_profile() {
                    if perf_profile_enabled {
                        data_collection_engine.add_data_collector(data_name, Data::$data($data::new()));
                    }
                } else if $data::is_java_profile() {
                    if java_profile_enabled {
                        data_collection_engine.add_data_collector(data_name, Data::$data($data::new()));
                    }
                } else {
                    if data_names_to_collect.contains(data_name) {
                        data_collection_engine.add_data_collector(data_name, Data::$data($data::new()));
                    }
                }
            )*
        }
    }
}

/// This macro expands to:
/// 1. define the ReportData Enum to hold all report data structs for report generation
/// 2. populate the DataProcessingEngine with each report data's processor
macro_rules! report_data {
    ( $( $report_data:ident ),* ) => {
        pub static JS_DIR: Dir<'_> = include_dir!("$JS_DIR");

        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum ReportData {
            $(
                $report_data($report_data),
            )*
        }

        impl ReportData {
            pub fn compatible_filenames(&self) -> Vec<&str> {
                 match self {
                    $(
                        ReportData::$report_data(ref value) => value.compatible_filenames(),
                    )*
                }
            }

            pub fn process_raw_data(&mut self, report_params: &ReportParams, raw_data: Vec<Data>) -> Result<AperfData> {
                match self {
                    $(
                        ReportData::$report_data(ref mut value) => Ok(value.process_raw_data(report_params, raw_data)?),
                    )*
                }
            }

            pub fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
                match self {
                    $(
                        ReportData::$report_data(ref value) => value.get_analytical_rules(),
                    )*
                }
            }

            pub fn get_raw_data_file(&self, run_data_dir: &PathBuf) -> Result<(File, PathBuf)> {
                match self {
                    $(
                        ReportData::$report_data(ref value) => value.get_raw_data_file(run_data_dir),
                    )*
                }
            }
        }

        fn add_data_processor(data_processing_engine: &mut DataProcessingEngine, data_name: &'static str, report_data: ReportData) {
            let data_processor = DataProcessor::new(
                data_name,
                report_data,
            );
            data_processing_engine.add_data_processor(data_processor);
        }

        pub fn initialize_data_processing_engine(data_processing_engine: &mut DataProcessingEngine) {
            $(
                let data_name = get_data_name_from_type::<$report_data>();
                add_data_processor(data_processing_engine, data_name, ReportData::$report_data($report_data::new()));
            )*
        }
    };
}

// IMPORTANT: DO NOT MODIFY THE DATA ORDER HERE. NEW DATA SHOULD BE APPENDED TO THE END.
// The order decides each data's index within the Data enum, which is used in serialization.
// Changing the order leads to indices changes and deserialization failures for previous run data.
data!(
    CpuUtilizationRaw,
    VmstatRaw,
    DiskstatsRaw,
    SystemInfo,
    KernelConfig,
    InterruptDataRaw,
    SysctlData,
    PerfStatRaw,
    ProcessesRaw,
    MeminfoDataRaw,
    NetstatRaw,
    NumastatRaw,
    PerfProfileRaw,
    FlamegraphRaw, // Dummy one to maintain the order
    JavaProfileRaw,
    HotlineRaw,
    MemallocDataRaw,
    EnaStatRaw,
    EfaStatRaw
);

report_data!(
    CpuUtilization,
    Vmstat,
    Diskstats,
    SystemInfo,
    KernelConfig,
    InterruptData,
    SysctlData,
    PerfStat,
    Processes,
    MeminfoData,
    Netstat,
    Numastat,
    PerfProfile,
    Hotline,
    AperfStat,
    AperfRunlog,
    JavaProfile,
    MemallocData,
    EnaStat,
    EfaStat
);

#[cfg(target_os = "linux")]
pub trait CollectData {
    fn prepare_data_collector(&mut self, _init_params: &InitParams) -> Result<()> {
        Ok(())
    }

    fn collect_data(&mut self, _init_params: &InitParams) -> Result<()> {
        Ok(())
    }

    fn finish_data_collection(&mut self, _init_params: &InitParams) -> Result<()> {
        Ok(())
    }

    fn is_static() -> bool {
        false
    }

    fn is_perf_profile() -> bool {
        false
    }

    fn is_java_profile() -> bool {
        false
    }
}

pub trait ProcessData {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec![]
    }

    fn process_raw_data(
        &mut self,
        _report_params: &ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        unimplemented!();
    }

    fn get_raw_data_file(&self, run_data_dir: &PathBuf) -> Result<(File, PathBuf)>
    where
        Self: Sized,
    {
        let data_name = get_data_name_from_type::<Self>();
        let mut file_name_candidates = vec![data_name];
        file_name_candidates.extend(self.compatible_filenames());

        for file_name in file_name_candidates {
            if let Ok(filename) = find_file(
                &run_data_dir,
                &format!(
                    "^{}(_.*)?\\.{}$",
                    regex::escape(file_name),
                    APERF_FILE_FORMAT
                ),
                None,
            ) {
                let raw_data_file_path = run_data_dir.join(filename);
                let raw_data_file = OpenOptions::new().read(true).open(&raw_data_file_path)?;

                return Ok((raw_data_file, raw_data_file_path));
            }
        }

        bail!("Cannot locate raw data file for {data_name}");
    }
}

pub trait AnalyzeData {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use crate::data_collection::DataCollector;

    use super::TimeEnum;
    use chrono::prelude::*;
    #[cfg(target_os = "linux")]
    use {
        super::cpu_utilization::CpuUtilizationRaw, super::Data, crate::data_file_path, std::fs,
        std::path::PathBuf,
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_data_type_init() {
        let run_data_dir = PathBuf::from("./performance_data_init_test");
        fs::DirBuilder::new()
            .recursive(true)
            .create(&run_data_dir)
            .unwrap();

        // Constructing a DataCollector creates and opens the data file.
        let data = CpuUtilizationRaw::new();
        let _dc = DataCollector::new(
            "cpu_utilization",
            Data::CpuUtilizationRaw(data),
            &run_data_dir,
        );

        let expected_path = data_file_path("cpu_utilization", &run_data_dir);
        assert!(expected_path.exists());

        fs::remove_dir_all(&run_data_dir).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_print() {
        let run_data_dir = PathBuf::from("./performance_data_print_test");
        fs::DirBuilder::new()
            .recursive(true)
            .create(&run_data_dir)
            .unwrap();

        let data = CpuUtilizationRaw::new();
        let mut dc = DataCollector::new(
            "cpu_utilization",
            Data::CpuUtilizationRaw(data),
            &run_data_dir,
        );

        let data_file_path = data_file_path("cpu_utilization", &run_data_dir);
        assert!(data_file_path.exists());
        dc.write_to_file().unwrap();

        // Re-open the file to read back what was serialized (the collector's own handle is in
        // append mode).
        let read_handle = fs::File::open(&data_file_path).unwrap();
        loop {
            match bincode::deserialize_from::<_, Data>(&read_handle) {
                Ok(v) => match v {
                    Data::CpuUtilizationRaw(ref value) => assert!(value.data.is_empty()),
                    _ => unreachable!(),
                },
                Err(e) => match *e {
                    bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break
                    }
                    _ => unreachable!(),
                },
            };
        }
        fs::remove_dir_all(&run_data_dir).unwrap();
    }

    #[test]
    fn test_time_diff_second() {
        let time_zero = Utc::now();
        let one_second = chrono::Duration::seconds(1);
        let time_one = time_zero + one_second;

        let time_t0 = TimeEnum::DateTime(time_zero);
        let time_t1 = TimeEnum::DateTime(time_one);

        let time_diff = time_t1 - time_t0;
        match time_diff {
            TimeEnum::TimeDiff(value) => assert!(value == 1, "Time diff was expected to be 1"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_time_diff_one_milli_second() {
        let time_zero = Utc::now();
        let one_millisecond = chrono::Duration::milliseconds(1);
        let time_one = time_zero + one_millisecond;

        let time_t0 = TimeEnum::DateTime(time_zero);
        let time_t1 = TimeEnum::DateTime(time_one);

        let time_diff = time_t1 - time_t0;
        match time_diff {
            TimeEnum::TimeDiff(value) => assert!(value == 0, "Time diff was expected to be 0"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_time_diff_just_less_than_one_second() {
        let time_zero = Utc::now();
        let just_less_than_one_second = chrono::Duration::milliseconds(992);
        let time_one = time_zero + just_less_than_one_second;

        let time_t0 = TimeEnum::DateTime(time_zero);
        let time_t1 = TimeEnum::DateTime(time_one);

        let time_diff = time_t1 - time_t0;
        match time_diff {
            TimeEnum::TimeDiff(value) => assert!(value == 1, "Time diff was expected to be 1"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_time_diff_half_second() {
        let time_zero = Utc::now();
        let half_second = chrono::Duration::milliseconds(500);
        let time_one = time_zero + half_second;

        let time_t0 = TimeEnum::DateTime(time_zero);
        let time_t1 = TimeEnum::DateTime(time_one);

        let time_diff = time_t1 - time_t0;
        match time_diff {
            TimeEnum::TimeDiff(value) => assert!(value == 1, "Time diff was expected to be 1"),
            _ => unreachable!(),
        }
    }

    #[test]
    #[should_panic]
    fn test_time_diff_unsupported_sub_op() {
        let time_t0 = TimeEnum::TimeDiff(0);
        let time_t1 = TimeEnum::TimeDiff(1);
        let _diff = time_t1 - time_t0;
    }

    #[test]
    #[should_panic]
    fn test_time_diff_mixed_type_sub_op() {
        let time_t0 = TimeEnum::TimeDiff(0);
        let time_t1 = TimeEnum::DateTime(Utc::now());
        let _diff = time_t1 - time_t0;
    }
}
