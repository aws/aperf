pub mod constants;
pub mod cpu_utilization;
pub mod diskstats;
pub mod flamegraphs;
cfg_if::cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        pub mod grv_perf_events;
    } else if #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] {
        pub mod intel_perf_events;
        pub mod intel_icelake_perf_events;
        pub mod intel_sapphire_rapids_perf_events;
    }
}
pub mod interrupts;
pub mod kernel_config;
pub mod meminfodata;
pub mod netstat;
pub mod perf_profile;
pub mod perf_stat;
pub mod processes;
pub mod sysctldata;
pub mod systeminfo;
pub mod utils;
pub mod vmstat;

use crate::visualizer::{GetData, ReportParams};
use crate::{InitParams, APERF_FILE_FORMAT};
use anyhow::Result;
use chrono::prelude::*;
use cpu_utilization::{CpuUtilization, CpuUtilizationRaw};
use diskstats::{Diskstats, DiskstatsRaw};
use flamegraphs::{Flamegraph, FlamegraphRaw};
use interrupts::{InterruptData, InterruptDataRaw};
use kernel_config::KernelConfig;
use log::trace;
use meminfodata::{MeminfoData, MeminfoDataRaw};
use netstat::{Netstat, NetstatRaw};
use perf_profile::{PerfProfile, PerfProfileRaw};
use perf_stat::{PerfStat, PerfStatRaw};
use processes::{Processes, ProcessesRaw};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::ops::Sub;
use sysctldata::SysctlData;
use systeminfo::SystemInfo;
use vmstat::{Vmstat, VmstatRaw};

#[derive(Clone, Debug)]
pub struct CollectorParams {
    pub collection_time: u64,
    pub data_file_path: String,
    pub data_dir: String,
    pub run_name: String,
}

impl CollectorParams {
    fn new() -> Self {
        CollectorParams {
            collection_time: 0,
            data_file_path: String::new(),
            data_dir: String::new(),
            run_name: String::new(),
        }
    }
}

pub struct DataType {
    pub data: Data,
    pub file_handle: Option<File>,
    pub file_name: String,
    pub full_path: String,
    pub dir_name: String,
    pub is_static: bool,
    pub collector_params: CollectorParams,
}

impl DataType {
    pub fn new(data: Data, file_name: String, is_static: bool) -> Self {
        DataType {
            data,
            file_handle: None,
            file_name,
            full_path: String::new(),
            dir_name: String::new(),
            is_static,
            collector_params: CollectorParams::new(),
        }
    }

    pub fn set_file_handle(&mut self, handle: Option<File>) {
        self.file_handle = handle;
    }

    pub fn init_data_type(&mut self, param: InitParams) -> Result<()> {
        trace!("Initializing data type...");
        let name = format!(
            "{}_{}.{}",
            self.file_name, param.time_str, APERF_FILE_FORMAT
        );
        let full_path = format!("{}/{}", param.dir_name, name);

        self.file_name = name;
        self.full_path = full_path;
        self.dir_name = param.dir_name.clone();
        self.collector_params.run_name = param.dir_name.clone();
        self.collector_params.collection_time = param.period;
        self.collector_params.data_file_path = self.full_path.clone();
        self.collector_params.data_dir = param.dir_name.clone();

        self.file_handle = Some(
            OpenOptions::new()
                .read(true)
                .create(true)
                .append(true)
                .open(self.full_path.clone())
                .expect("Could not create file for data"),
        );

        Ok(())
    }

    pub fn prepare_data_collector(&mut self) -> Result<()> {
        trace!("Preparing data collector...");
        self.data
            .prepare_data_collector(self.collector_params.clone())?;
        Ok(())
    }

    pub fn collect_data(&mut self) -> Result<()> {
        trace!("Collecting Data...");
        self.data.collect_data()?;
        Ok(())
    }

    pub fn write_to_file(&mut self) -> Result<()> {
        trace!("Writing to file...");
        let file_handle = self.file_handle.as_ref().unwrap();
        bincode::serialize_into(file_handle.try_clone()?, &self.data)?;
        Ok(())
    }

    pub fn after_data_collection(&mut self) -> Result<()> {
        trace!("Running post collection actions...");
        self.data
            .after_data_collection(self.collector_params.clone())?;
        Ok(())
    }
}

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

/// Create a Data Enum
///
/// Each enum type will have a collect_data implemented for it.
macro_rules! data {
    ( $( $x:ident ),* ) => {
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum Data {
            $(
                $x($x),
            )*
        }

        impl Data {
            fn collect_data(&mut self) -> Result<()> {
                match self {
                    $(
                        Data::$x(ref mut value) => value.collect_data()?,
                    )*
                }
                Ok(())
            }

            fn prepare_data_collector(&mut self, params: CollectorParams) -> Result<()> {
                match self {
                    $(
                        Data::$x(ref mut value) => value.prepare_data_collector(params)?,
                    )*
                }
                Ok(())
            }

            fn after_data_collection(&mut self, params: CollectorParams) -> Result<()> {
                match self {
                    $(
                        Data::$x(ref mut value) => value.after_data_collection(params)?,
                    )*
                }
                Ok(())
            }
        }
    };
}

macro_rules! processed_data {
    ( $( $x:ident ),* ) => {
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum ProcessedData {
            $(
                $x($x),
            )*
        }

        impl ProcessedData {
            pub fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
                match self {
                    $(
                        ProcessedData::$x(ref mut value) => Ok(value.process_raw_data(buffer)?),
                    )*
                }
            }
            pub fn custom_raw_data_parser(&mut self, parser_params: ReportParams) -> Result<Vec<ProcessedData>> {
                match self {
                    $(
                        ProcessedData::$x(ref mut value) => Ok(value.custom_raw_data_parser(parser_params)?),
                    )*
                }
            }
            pub fn get_data(&mut self, values: Vec<ProcessedData>, query: String) -> Result<String> {
                match self {
                    $(
                        ProcessedData::$x(ref mut value) => Ok(value.get_data(values, query)?),
                    )*
                }
            }
            pub fn get_calls(&mut self) -> Result<Vec<String>> {
                match self {
                    $(
                        ProcessedData::$x(ref mut value) => Ok(value.get_calls()?),
                    )*
                }
            }
        }
    };
}

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
    PerfProfileRaw,
    FlamegraphRaw
);

processed_data!(
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
    PerfProfile,
    Flamegraph
);

macro_rules! noop {
    () => {};
}

pub trait CollectData {
    fn prepare_data_collector(&mut self, _params: CollectorParams) -> Result<()> {
        noop!();
        Ok(())
    }
    fn collect_data(&mut self) -> Result<()> {
        noop!();
        Ok(())
    }
    fn after_data_collection(&mut self, _params: CollectorParams) -> Result<()> {
        noop!();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::cpu_utilization::CpuUtilizationRaw;
    use super::{CollectorParams, Data, DataType, TimeEnum};
    use crate::InitParams;
    use chrono::prelude::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_data_type_init() {
        let mut param = InitParams::new("".to_string());
        let data = CpuUtilizationRaw::new();
        let mut dt = DataType {
            data: Data::CpuUtilizationRaw(data),
            file_handle: None,
            file_name: "cpu_utilization".to_string(),
            full_path: String::new(),
            dir_name: String::new(),
            is_static: false,
            collector_params: CollectorParams::new(),
        };

        param.dir_name = format!("./performance_data_init_test_{}", param.time_str);
        fs::DirBuilder::new()
            .recursive(true)
            .create(param.dir_name.clone())
            .unwrap();

        dt.init_data_type(param).unwrap();

        assert!(dt.file_handle.is_some());
        fs::remove_file(dt.full_path).unwrap();
        fs::remove_dir_all(dt.dir_name).unwrap();
    }

    #[test]
    fn test_print() {
        let mut param = InitParams::new("".to_string());
        let data = CpuUtilizationRaw::new();
        let mut dt = DataType {
            data: Data::CpuUtilizationRaw(data),
            file_handle: None,
            file_name: "cpu_utilization".to_string(),
            full_path: String::new(),
            dir_name: String::new(),
            is_static: false,
            collector_params: CollectorParams::new(),
        };

        param.dir_name = format!("./performance_data_print_test_{}", param.time_str);
        fs::DirBuilder::new()
            .recursive(true)
            .create(param.dir_name.clone())
            .unwrap();

        dt.init_data_type(param).unwrap();

        assert!(Path::new(&dt.full_path).exists());
        dt.write_to_file().unwrap();

        loop {
            match bincode::deserialize_from::<_, Data>(dt.file_handle.as_ref().unwrap()) {
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
        fs::remove_file(dt.full_path).unwrap();
        fs::remove_dir_all(dt.dir_name).unwrap();
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
