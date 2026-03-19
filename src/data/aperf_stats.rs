use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_sum_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fs, time};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AperfStat {
    pub time: TimeEnum,
    pub name: String,
    pub data: HashMap<String, u64>,
}

impl AperfStat {
    pub fn new() -> Self {
        AperfStat {
            time: TimeEnum::DateTime(Utc::now()),
            name: String::new(),
            data: HashMap::new(),
        }
    }

    pub fn measure<F>(&mut self, name: String, mut func: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        let start_time = time::Instant::now();
        func()?;
        let func_time: u64 = (time::Instant::now() - start_time).as_micros() as u64;
        self.data.insert(name, func_time);
        Ok(())
    }
}

impl ProcessData for AperfStat {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["aperf_run_stats"]
    }

    fn process_raw_data(
        &mut self,
        params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_sum_aggregate!();
        time_series_data_processor.set_aggregate_series_name("total");

        let mut values = Vec::new();
        let file: Result<fs::File> = Ok(fs::OpenOptions::new()
            .read(true)
            .open(params.data_file_path)
            .expect("Could not open APerf Stats file"));
        loop {
            match bincode::deserialize_from::<_, AperfStat>(file.as_ref().unwrap()) {
                Ok(v) => values.push(v),
                Err(e) => match *e {
                    // EOF
                    bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break
                    }
                    e => panic!("Error when Deserializing APerf Stats data: {}", e),
                },
            };
        }

        for value in values {
            time_series_data_processor.proceed_to_time(value.time);

            for (name, stat) in value.data {
                let datatype: Vec<&str> = name.split('-').collect();
                let metric_name = datatype[0];
                let mut series_name = datatype.get(1).unwrap_or(&metric_name).to_string();
                // Make the series name easier to understand - since it's essentially to write the
                // collected data to disk
                if series_name == "print" {
                    series_name = "write".to_string();
                }

                time_series_data_processor.add_data_point(metric_name, &series_name, stat as f64);
            }
        }

        let time_series_data = time_series_data_processor.get_time_series_data_sorted_by_average();
        Ok(AperfData::TimeSeries(time_series_data))
    }
}
