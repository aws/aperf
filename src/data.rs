pub mod cpu_utilization;
pub mod vmstat;
pub mod diskstats;
pub mod systeminfo;

use crate::{InitParams, PDResult};
use chrono::prelude::*;
use cpu_utilization::CpuUtilization;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use std::fs::{File, OpenOptions};
use vmstat::Vmstat;
use diskstats::Diskstats;
use std::ops::Sub;
use systeminfo::SystemInfo;

pub struct DataType {
    pub data: Data,
    pub file_handle: Option<File>,
    pub file_name: String,
    pub full_path: String,
    pub dir_name: String,
    pub collect_once:  bool
}

impl DataType {
    pub fn new(data: Data, file_name: String) -> Self {
        DataType {
            data: data,
            file_handle: None,
            file_name: file_name,
            full_path: String::new(),
            dir_name: String::new(),
            collect_once: false
        }
    }

    pub fn set_file_handle(&mut self, handle: Option<File>) {
        self.file_handle = handle;
    }

    pub fn init_data_type(&mut self, param: InitParams) -> PDResult {
        debug!("Initializing data type...");
        let name = format!("{}_{}.yaml", self.file_name, param.time_str);
        let full_path = format!("{}/{}", param.dir_name, name);

        self.file_name = name;
        self.full_path = full_path;
        self.dir_name = param.dir_name;

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

    pub fn collect_data(&mut self) -> PDResult {
        debug!("Collecting Data...");
        self.data.collect_data()?;
        Ok(())
    }

    pub fn print_to_file(&mut self) -> PDResult {
        debug!("Printing to YAML file...");
        let file_handle = self.file_handle.as_ref().unwrap();
        serde_yaml::to_writer(file_handle.try_clone()?, &self.data)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum TimeEnum {
    DateTime(DateTime<Utc>),
    TimeDiff(u64),
}

impl Sub for TimeEnum {
    type Output = TimeEnum;

    fn sub(self, rhs: TimeEnum) -> TimeEnum {
        let self_time;
        let other_time;
        match self {
            TimeEnum::DateTime(value) => self_time = value,
            _ => panic!("Cannot perform subtract op on TimeEnum::TimeDiff"),
        }
        match rhs {
            TimeEnum::DateTime(value) => other_time = value,
             _ => panic!("Cannot perform subtract op on TimeEnum::TimeDiff"),
        }
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
        #[derive(Serialize, Deserialize, Debug)]
        pub enum Data {
            $(
                $x($x),
            )*
        }

        impl Data {
            fn collect_data(&mut self) -> PDResult {
                match self {
                    $(
                        Data::$x(ref mut value) => value.collect_data()?,
                    )*
                }
                Ok(())
            }
        }
    };
}

data!(CpuUtilization, Vmstat, Diskstats, SystemInfo);

pub trait CollectData {
    fn collect_data(&mut self) -> PDResult;
}

#[cfg(test)]
mod tests {
    use super::cpu_utilization::CpuUtilization;
    use super::{Data, DataType, TimeEnum};
    use crate::InitParams;
    use chrono::prelude::*;
    use serde::Deserialize;
    use serde_yaml::{self};
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_data_type_init() {
        let mut param = InitParams::new();
        let data = CpuUtilization::new();
        let mut dt = DataType {
            data: Data::CpuUtilization(data),
            file_handle: None,
            file_name: "cpu_utilization".to_string(),
            full_path: String::new(),
            dir_name: String::new(),
            collect_once: false
        };

        param.dir_name = format!("./performance_data_init_test_{}", param.time_str);
        let _ret = fs::DirBuilder::new()
            .recursive(true)
            .create(param.dir_name.clone())
            .unwrap();

        dt.init_data_type(param).unwrap();

        assert!(!dt.file_handle.is_none());
        fs::remove_file(dt.full_path).unwrap();
        fs::remove_dir_all(dt.dir_name).unwrap();
    }

    #[test]
    fn test_print() {
        let mut param = InitParams::new();
        let data = CpuUtilization::new();
        let mut dt = DataType {
            data: Data::CpuUtilization(data),
            file_handle: None,
            file_name: "cpu_utilization".to_string(),
            full_path: String::new(),
            dir_name: String::new(),
            collect_once: false
        };

        param.dir_name = format!("./performance_data_print_test_{}", param.time_str);
        let _ret = fs::DirBuilder::new()
            .recursive(true)
            .create(param.dir_name.clone())
            .unwrap();

        dt.init_data_type(param).unwrap();

        assert!(Path::new(&dt.full_path).exists());
        assert!(dt.print_to_file().unwrap() == ());

        for document in serde_yaml::Deserializer::from_reader(dt.file_handle.unwrap()) {
            let v = Data::deserialize(document).expect("File read error");
            match v {
                Data::CpuUtilization(ref value) => {
                    assert!(value.total.cpu == 0);
                    assert!(value.per_cpu.is_empty());
                }
                _ => assert!(true),
            }
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
            _ => assert!(false),
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
            _ => assert!(false),
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
            _ => assert!(false),
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
            _ => assert!(false),
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
