pub mod cpu_utilization;
pub mod vmstat;

use crate::{InitParams, PDResult};
use cpu_utilization::CpuUtilization;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use std::fs::{File, OpenOptions};
use vmstat::Vmstat;

pub struct DataType {
    pub data: Data,
    pub file_handle: Option<File>,
    pub file_name: String,
    pub full_path: String,
    pub dir_name: String,
}

impl DataType {
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

data!(CpuUtilization, Vmstat);

pub trait CollectData {
    fn collect_data(&mut self) -> PDResult;
}

#[cfg(test)]
mod tests {
    use super::cpu_utilization::CpuUtilization;
    use super::Data;
    use super::DataType;
    use crate::InitParams;
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
}
