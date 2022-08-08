use crate::{InitParams, PDResult};
use log::debug;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use std::fs::{File, OpenOptions};

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

#[derive(Serialize, Deserialize, Debug)]
pub enum Data {
    None,
}

impl Data {
    fn collect_data(&mut self) -> PDResult {
        match self {
            Data::None => Ok(()),
        }
    }
}

pub trait CollectData {
    fn collect_data(&mut self) -> PDResult;
}
