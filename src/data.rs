pub mod aperf_runlog;
pub mod aperf_stats;
pub mod constants;
pub mod cpu_utilization;
pub mod diskstats;
pub mod flamegraphs;
pub mod hotline;
pub mod interrupts;
pub mod java_profile;
pub mod kernel_config;
pub mod meminfo;
pub mod netstat;
pub mod perf_profile;
pub mod perf_stat;
pub mod processes;
pub mod sysctl;
pub mod systeminfo;
pub mod utils;
pub mod vmstat;

use crate::utils::{get_data_name_from_type, DataMetrics};
use crate::visualizer::{DataVisualizer, GetData, ReportParams};
use crate::{noop, InitParams, PerformanceData, VisualizationData, APERF_FILE_FORMAT};
use anyhow::Result;
use aperf_runlog::AperfRunlog;
use aperf_stats::AperfStat;
use chrono::prelude::*;
use cpu_utilization::{CpuUtilization, CpuUtilizationRaw};
use diskstats::{Diskstats, DiskstatsRaw};
use flamegraphs::{Flamegraph, FlamegraphRaw};
use hotline::{Hotline, HotlineRaw};
use include_directory::{include_directory, Dir};
use interrupts::{InterruptData, InterruptDataRaw};
use java_profile::{JavaProfile, JavaProfileRaw};
use kernel_config::KernelConfig;
use log::trace;
use meminfo::{MeminfoData, MeminfoDataRaw};
use netstat::{Netstat, NetstatRaw};
use nix::sys::{signal, signal::Signal};
use perf_profile::{PerfProfile, PerfProfileRaw};
use perf_stat::{PerfStat, PerfStatRaw};
use processes::{Processes, ProcessesRaw};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::ops::Sub;
use std::path::PathBuf;
use sysctl::SysctlData;
use systeminfo::SystemInfo;
use vmstat::{Vmstat, VmstatRaw};

#[derive(Clone, Debug)]
pub struct CollectorParams {
    pub collection_time: u64,
    pub elapsed_time: u64,
    pub data_file_path: PathBuf,
    pub data_dir: PathBuf,
    pub run_name: String,
    pub profile: HashMap<String, String>,
    pub tmp_dir: PathBuf,
    pub signal: Signal,
    pub runlog: PathBuf,
    pub pmu_config: Option<PathBuf>,
    pub perf_frequency: u32,
    pub hotline_frequency: u32,
    pub interval: u64,
    pub num_to_report: u32,
}

impl CollectorParams {
    fn new() -> Self {
        CollectorParams {
            collection_time: 0,
            elapsed_time: 0,
            data_file_path: PathBuf::new(),
            data_dir: PathBuf::new(),
            run_name: String::new(),
            profile: HashMap::new(),
            tmp_dir: PathBuf::new(),
            signal: signal::SIGTERM,
            runlog: PathBuf::new(),
            pmu_config: None,
            perf_frequency: 99,
            hotline_frequency: 1000,
            interval: 1,
            num_to_report: 5000,
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
    pub is_profile_option: bool,
    pub collector_params: CollectorParams,
}

impl DataType {
    pub fn new(data: Data, file_name: String, is_static: bool, is_profile_option: bool) -> Self {
        DataType {
            data,
            file_handle: None,
            file_name,
            full_path: String::new(),
            dir_name: String::new(),
            is_static,
            is_profile_option,
            collector_params: CollectorParams::new(),
        }
    }

    pub fn set_file_handle(&mut self, handle: Option<File>) {
        self.file_handle = handle;
    }

    pub fn set_signal(&mut self, signal: Signal) {
        self.collector_params.signal = signal;
    }

    pub fn init_data_type(&mut self, param: &InitParams) -> Result<()> {
        trace!("Initializing data type...");
        let name = format!(
            "{}_{}.{}",
            self.file_name, param.time_str, APERF_FILE_FORMAT
        );

        self.file_name = name.clone();
        self.full_path = format!("{}/{}", param.dir_name, name);
        self.dir_name = param.dir_name.clone();
        self.collector_params.run_name = param.dir_name.clone();
        self.collector_params.collection_time = param.period;
        self.collector_params.elapsed_time = 0;
        self.collector_params.data_file_path = PathBuf::from(&self.full_path);
        self.collector_params.data_dir = PathBuf::from(param.dir_name.clone());
        self.collector_params.profile = param.profile.clone();
        self.collector_params.tmp_dir = param.tmp_dir.clone();
        self.collector_params.runlog = param.runlog.clone();
        self.collector_params.pmu_config = param.pmu_config.clone();
        self.collector_params.interval = param.interval;
        self.collector_params.hotline_frequency = param.hotline_frequency;
        self.collector_params.num_to_report = param.num_to_report;

        self.file_handle = Some(
            OpenOptions::new()
                .read(true)
                .create(true)
                .append(true)
                .open(&self.full_path)
                .expect("Could not create file for data"),
        );

        Ok(())
    }

    pub fn prepare_data_collector(&mut self) -> Result<()> {
        trace!("Preparing data collector...");
        self.data.prepare_data_collector(&self.collector_params)?;
        Ok(())
    }

    pub fn collect_data(&mut self) -> Result<()> {
        trace!("Collecting Data...");
        self.data.collect_data(&self.collector_params)?;
        Ok(())
    }

    pub fn write_to_file(&mut self) -> Result<()> {
        trace!("Writing to file...");
        let file_handle = self.file_handle.as_ref().unwrap();
        bincode::serialize_into(file_handle.try_clone()?, &self.data)?;
        Ok(())
    }

    pub fn finish_data_collection(&mut self) -> Result<()> {
        trace!("Finish data collection...");
        self.data.finish_data_collection(&self.collector_params)?;
        Ok(())
    }

    pub fn after_data_collection(&mut self) -> Result<()> {
        trace!("Running post collection actions...");
        self.data.after_data_collection(&self.collector_params)?;
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

/// This macro expands to:
/// 1. define the Data Enum to hold all record data structs for collection
/// 2. define the function that instantiates all data structs and adds them
///    to the PerformanceData object.
/// 3. collect the names of data to be collected by default.
/// The main record function (aperf::record::record(&Record, &Path, &Path) -> Result<()>
/// creates the PerformanceData object and invokes the function.
macro_rules! data {
    ( $( $data:ident ),* ) => {

        lazy_static! {
            pub static ref DEFAULT_DATA_NAMES: Vec<&'static str> = get_default_data_names();
        }

        fn get_default_data_names() -> Vec<&'static str> {
            let mut default_data_names: Vec<&'static str> = Vec::new();
            $(
                if !($data::is_profile() || $data::is_java_profile()) {
                    default_data_names.push(get_data_name_from_type::<$data>());
                }
            )*

            #[cfg(not(feature = "hotline"))]
            default_data_names.retain(
                |&data_name| data_name != get_data_name_from_type::<Hotline>()
            );

            default_data_names
        }

        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum Data {
            $(
                $data($data),
            )*
        }

        impl Data {
            fn collect_data(&mut self, params: &CollectorParams) -> Result<()> {
                match self {
                    $(
                        Data::$data(ref mut value) => value.collect_data(&params)?,
                    )*
                }
                Ok(())
            }

            fn prepare_data_collector(&mut self, params: &CollectorParams) -> Result<()> {
                match self {
                    $(
                        Data::$data(ref mut value) => value.prepare_data_collector(params)?,
                    )*
                }
                Ok(())
            }

            fn finish_data_collection(&mut self, params: &CollectorParams) -> Result<()> {
                match self {
                    $(
                        Data::$data(ref mut value) => value.finish_data_collection(params)?,
                    )*
                }
                Ok(())
            }

            fn after_data_collection(&mut self, params: &CollectorParams) -> Result<()> {
                match self {
                    $(
                        Data::$data(ref mut value) => value.after_data_collection(params)?,
                    )*
                }
                Ok(())
            }
        }

        fn add_performance_data(performance_data: &mut PerformanceData, data_name: &str, data: Data, is_static: bool, is_profile_option: bool) {
            let data_type = DataType::new(
                data,
                data_name.to_string(),
                is_static,
                is_profile_option
            );
            performance_data.add_datatype(data_name.to_string(), data_type);
        }

        pub fn add_all_performance_data(performance_data: &mut PerformanceData, data_names_to_collect: HashSet<String>, profile_enabled: bool, java_profile_enabled: bool) {
            $(
                let data_name = get_data_name_from_type::<$data>();

                if $data::is_profile() {
                    if profile_enabled {
                        add_performance_data(performance_data, data_name, Data::$data($data::new()), $data::is_static(), true);
                    }
                } else if $data::is_java_profile() {
                    if java_profile_enabled {
                        add_performance_data(performance_data, data_name, Data::$data($data::new()), $data::is_static(), true);
                    }
                } else {
                    if data_names_to_collect.contains(data_name) {
                        add_performance_data(performance_data, data_name, Data::$data($data::new()), $data::is_static(), false);
                    }
                }
            )*
        }
    }
}

/// This macro expands to:
/// 1. define the ProcessedData Enum to hold all report data structs for visualization
/// 2. define the function that instantiates all data structs and adds them
///    to the VisualizationData object.
/// The main report function (aperf::report::report(&Report, &PathBuf) -> Result<()>
/// creates the VisualizationData object and invokes the function.
macro_rules! processed_data {
    ( $( $processed_data:ident ),* ) => {
        pub static JS_DIR: Dir<'_> = include_directory!("$JS_DIR");

        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum ProcessedData {
            $(
                $processed_data($processed_data),
            )*
        }

        impl ProcessedData {
            pub fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
                match self {
                    $(
                        ProcessedData::$processed_data(ref mut value) => Ok(value.process_raw_data(buffer)?),
                    )*
                }
            }

            pub fn custom_raw_data_parser(&mut self, parser_params: ReportParams) -> Result<Vec<ProcessedData>> {
                match self {
                    $(
                        ProcessedData::$processed_data(ref mut value) => Ok(value.custom_raw_data_parser(parser_params)?),
                    )*
                }
            }

            pub fn get_data(&mut self, values: Vec<ProcessedData>, query: String, metrics: &mut DataMetrics) -> Result<String> {
                match self {
                    $(
                        ProcessedData::$processed_data(ref mut value) => Ok(value.get_data(values, query, metrics)?),
                    )*
                }
            }

            pub fn get_calls(&mut self) -> Result<Vec<String>> {
                match self {
                    $(
                        ProcessedData::$processed_data(ref mut value) => Ok(value.get_calls()?),
                    )*
                }
            }
        }

        fn add_visualization_data(visualization_data: &mut VisualizationData, data_name: &str, processed_data: ProcessedData, has_custom_raw_data_parser: bool) {
            let mut js_file_name = format!("{}.js", data_name);
            let mut js_content: &str = "";

            if !JS_DIR.contains(&js_file_name) {
                trace!("Skip reading {} since it does not exist", js_file_name);
                js_file_name = String::new();
            } else {
                let js_file_bytes = JS_DIR.get_file(&js_file_name).unwrap();
                js_content = js_file_bytes.contents_utf8().unwrap();
            }

            let data_visualizer = DataVisualizer::new(
                processed_data,
                data_name.to_string(),
                js_file_name,
                js_content.to_string(),
                has_custom_raw_data_parser,
            );

            visualization_data.add_visualizer(data_name.to_string(), data_visualizer);
        }

        pub fn add_all_visualization_data(visualization_data: &mut VisualizationData) {
            $(
                let data_name = get_data_name_from_type::<$processed_data>();
                add_visualization_data(visualization_data, data_name, ProcessedData::$processed_data($processed_data::new()), $processed_data::has_custom_raw_data_parser());
            )*
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
    FlamegraphRaw,
    JavaProfileRaw,
    HotlineRaw
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
    Hotline,
    Flamegraph,
    AperfStat,
    AperfRunlog,
    JavaProfile
);

pub trait CollectData {
    fn prepare_data_collector(&mut self, _params: &CollectorParams) -> Result<()> {
        noop!();
        Ok(())
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        noop!();
        Ok(())
    }

    fn finish_data_collection(&mut self, _params: &CollectorParams) -> Result<()> {
        noop!();
        Ok(())
    }

    fn after_data_collection(&mut self, _params: &CollectorParams) -> Result<()> {
        noop!();
        Ok(())
    }

    fn is_static() -> bool {
        false
    }

    fn is_profile() -> bool {
        false
    }

    fn is_java_profile() -> bool {
        false
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
            is_profile_option: false,
            collector_params: CollectorParams::new(),
        };

        param.dir_name = format!("./performance_data_init_test_{}", param.time_str);
        fs::DirBuilder::new()
            .recursive(true)
            .create(param.dir_name.clone())
            .unwrap();

        dt.init_data_type(&param).unwrap();

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
            is_profile_option: false,
            collector_params: CollectorParams::new(),
        };

        param.dir_name = format!("./performance_data_print_test_{}", param.time_str);
        fs::DirBuilder::new()
            .recursive(true)
            .create(param.dir_name.clone())
            .unwrap();

        dt.init_data_type(&param).unwrap();

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
