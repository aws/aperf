use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_sum_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::data_processing::ReportParams;
use anyhow::{bail, Result};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time;

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

    pub fn for_time(time: TimeEnum) -> Self {
        AperfStat {
            time,
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
        report_params: &ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor =
            time_series_data_processor_with_sum_aggregate!(report_params.collection_start);
        time_series_data_processor.set_aggregate_series_name("total");

        let (raw_aperf_stats_file, _) = match self.get_raw_data_file(&report_params.run_data_dir) {
            Ok(rs) => rs,
            Err(e) => bail!("Failed to open raw APerf Stats file: {:?}", e),
        };

        let mut values = Vec::new();
        loop {
            match bincode::deserialize_from::<_, AperfStat>(&raw_aperf_stats_file) {
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

        let mut collection_started = false;
        for value in values {
            // Ignore the time diff for data before the collection started, as time_diff
            // computation would have been corrupted and these data are not time-series anyway.
            if !collection_started
                && report_params
                    .collection_start
                    .map_or(true, |collection_start_time| {
                        value.time >= collection_start_time
                    })
            {
                collection_started = true;
            }
            if collection_started {
                time_series_data_processor.proceed_to_time(value.time);
            }

            for (stat_key, stat_value) in value.data {
                let stat_key_components: Vec<&str> = stat_key.split('-').collect();
                let data_name = stat_key_components[0];
                let mut stat_name = stat_key_components.get(1).unwrap_or(&data_name).to_string();
                // Backward compatibility - the previous stat name was "print"
                if stat_name == "print" {
                    stat_name = "write".to_string();
                }

                if stat_name == "collect" || stat_name == "write" {
                    // The stats for APerf collecting time-series data, which should be
                    // processed as regular time-series data as well.
                    time_series_data_processor.add_data_point(
                        data_name,
                        &stat_name,
                        stat_value as f64,
                    );
                } else {
                    // For non-time-series stats, group them in a dummy metric by stat_name as different
                    // series by data_name.
                    time_series_data_processor.add_data_point(
                        &stat_name,
                        data_name,
                        stat_value as f64,
                    );
                }
            }
        }

        let time_series_data = time_series_data_processor.get_time_series_data_sorted_by_average();
        Ok(AperfData::TimeSeries(time_series_data))
    }
}
