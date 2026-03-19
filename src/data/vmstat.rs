use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_custom_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::prelude::*,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VmstatRaw {
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl VmstatRaw {
    pub fn new() -> Self {
        VmstatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for VmstatRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/vmstat")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vmstat;

impl Vmstat {
    pub fn new() -> Self {
        Vmstat
    }
}

impl ProcessData for Vmstat {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_custom_aggregate!();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::VmstatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            for line in raw_value.data.lines() {
                let mut split = line.split_whitespace();
                let name = match split.next() {
                    Some(n) => n,
                    None => {
                        error!("Failed to extract name from vmstat line: {}", line);
                        continue;
                    }
                };
                let val_str = match split.next() {
                    Some(v) => v,
                    None => {
                        error!("Failed to extract value from vmstat line: {}", line);
                        continue;
                    }
                };
                let val = val_str.parse::<i64>()?;

                if name.contains("nr_") {
                    time_series_data_processor.add_data_point(&name, "values", val as f64);
                } else {
                    time_series_data_processor
                        .add_accumulative_data_point(&name, "values", val as f64);
                }
            }
        }

        let time_series_data = time_series_data_processor.get_time_series_data();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::VmstatRaw,
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut vmstat = VmstatRaw::new();
        let params = CollectorParams::new();

        vmstat.collect_data(&params).unwrap();
        assert!(!vmstat.data.is_empty());
    }
}
