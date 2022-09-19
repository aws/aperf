#[macro_use]
extern crate lazy_static;

pub mod data;
use chrono::prelude::*;
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use std::time;
use thiserror::Error;
use timerfd::{SetTimeFlags, TimerFd, TimerState};

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum PDError {
    #[error("File Error")]
    FileError {
        #[from]
        source: std::io::Error,
    },

    #[error("Time Error")]
    TimeError,

    #[error(transparent)]
    YAMLError(#[from] serde_yaml::Error),
}

pub type PDResult = Result<(), PDError>;

lazy_static! {
    pub static ref PERFORMANCE_DATA: Mutex<PerformanceData> = Mutex::new(PerformanceData::new());
}

#[allow(missing_docs)]
pub struct PerformanceData {
    pub collectors: HashMap<String, data::DataType>,
    pub init_params: InitParams,
}

impl PerformanceData {
    pub fn new() -> Self {
        let collectors = HashMap::new();
        let init_params = InitParams::new();

        PerformanceData {
            collectors,
            init_params,
        }
    }

    pub fn set_params(&mut self, params: InitParams) {
        self.init_params = params;
    }

    pub fn add_datatype(&mut self, name: String, dt: data::DataType) {
        self.collectors.insert(name, dt);
    }

    pub fn init_collectors(&mut self) -> PDResult {
        let _ret = fs::create_dir_all(self.init_params.dir_name.clone()).unwrap();

        for (_name, datatype) in self.collectors.iter_mut() {
            datatype.init_data_type(self.init_params.clone())?;
        }
        Ok(())
    }

    pub fn collect_static_data(&mut self) -> PDResult {
        for (_name, datatype) in self.collectors.iter_mut() {
            if !datatype.is_static {
                continue;
            }
            datatype.collect_data()?;
            datatype.print_to_file()?;
        }

        Ok(())
    }

    pub fn collect_data_serial(&mut self) -> PDResult {
        let start = time::Instant::now();
        let mut current = time::Instant::now();
        let end = current + time::Duration::from_secs(self.init_params.period);

        let mut tfd = TimerFd::new().unwrap();
        tfd.set_state(
            TimerState::Periodic {
                current: time::Duration::from_secs(self.init_params.interval),
                interval: time::Duration::from_secs(self.init_params.interval),
            },
            SetTimeFlags::Default,
        );
        while current <= end {
            let ret = tfd.read();
            if ret > 1 {
                error!("Missed {} interval(s)", ret - 1);
            }
            info!("Time elapsed: {:?}", start.elapsed());
            current += time::Duration::from_secs(ret);
            for (_name, datatype) in self.collectors.iter_mut() {
                if datatype.is_static {
                    continue;
                }
                datatype.collect_data()?;
                datatype.print_to_file()?;
            }
            let data_collection_time = time::Instant::now() - current;
            info!("Collection time: {:?}", data_collection_time);
        }
        tfd.set_state(TimerState::Disarmed, SetTimeFlags::Default);
        Ok(())
    }
}

impl Default for PerformanceData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct InitParams {
    pub time_now: DateTime<Utc>,
    pub time_str: String,
    pub dir_name: String,
    pub period: u64,
    pub interval: u64,
}

impl InitParams {
    pub fn new() -> Self {
        let time_now = Utc::now();
        let time_str = time_now.format("%Y-%m-%d_%H_%M_%S").to_string();
        let dir_name = format!("./performance_data_{}", time_str);

        InitParams {
            time_now,
            time_str,
            dir_name,
            period: 0,
            interval: 0,
        }
    }
}

impl Default for InitParams {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{InitParams, PerformanceData};
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_performance_data_new() {
        let pd = PerformanceData::new();

        let dir_name = format!(
            "./performance_data_{}",
            pd.init_params.time_now.format("%Y-%m-%d_%H_%M_%S")
        );
        assert!(pd.collectors.is_empty());
        assert!(pd.init_params.dir_name == dir_name);
    }

    #[test]
    fn test_performance_data_dir_creation() {
        let mut params = InitParams::new();
        params.dir_name = format!("./performance_data_dir_creation_{}", params.time_str);

        let mut pd = PerformanceData::new();
        pd.init_collectors().unwrap();
        assert!(Path::new(&pd.init_params.dir_name).exists());
        fs::remove_dir_all(pd.init_params.dir_name).unwrap();
    }
}
