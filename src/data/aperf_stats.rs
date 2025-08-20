use crate::data::{ProcessedData, TimeEnum};
use crate::utils::{add_metrics, get_data_name_from_type, DataMetrics, Metric};
use crate::visualizer::{GetData, GraphLimitType, GraphMetadata, ReportParams};
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerDataTypeStat {
    pub name: String,
    pub collect: Vec<DataPoint>,
    pub print: Vec<DataPoint>,
    pub metadata: GraphMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataPoint {
    pub time: TimeEnum,
    pub time_taken: u64,
}

fn get_key_data(values: Vec<AperfStat>, key: String, metrics: &mut DataMetrics) -> Result<String> {
    let mut metric = Metric::new(key.clone());
    let mut end_value = PerDataTypeStat {
        name: key.clone(),
        collect: Vec::new(),
        print: Vec::new(),
        metadata: GraphMetadata::new(),
    };
    let time_zero = &values[0].time;

    for value in &values {
        let time_now = value.time - *time_zero;
        for (k, v) in &value.data {
            if !k.contains(&key) {
                continue;
            }
            let datapoint = DataPoint {
                time: time_now,
                time_taken: *v,
            };
            metric.insert_value(*v as f64);
            end_value.metadata.update_limits(GraphLimitType::UInt64(*v));
            if k.contains(&key) {
                if k.contains("print") {
                    end_value.print.push(datapoint);
                } else {
                    end_value.collect.push(datapoint);
                }
            }
        }
    }
    add_metrics(
        key,
        &mut metric,
        metrics,
        get_data_name_from_type::<AperfStat>().to_string(),
    )?;

    Ok(serde_json::to_string(&end_value)?)
}

impl GetData for AperfStat {
    fn custom_raw_data_parser(&mut self, params: ReportParams) -> Result<Vec<ProcessedData>> {
        let mut raw_data: Vec<ProcessedData> = Vec::new();

        let file: Result<fs::File> = Ok(fs::OpenOptions::new()
            .read(true)
            .open(params.data_file_path)
            .expect("Could not open APerf Stats file"));
        loop {
            match bincode::deserialize_from::<_, AperfStat>(file.as_ref().unwrap()) {
                Ok(v) => raw_data.push(ProcessedData::AperfStat(v)),
                Err(e) => match *e {
                    // EOF
                    bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break
                    }
                    e => panic!("Error when Deserializing APerf Stats data: {}", e),
                },
            };
        }
        Ok(raw_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["keys".to_string(), "values".to_string()])
    }

    fn get_data(
        &mut self,
        buffer: Vec<ProcessedData>,
        query: String,
        metrics: &mut DataMetrics,
    ) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::AperfStat(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => {
                let mut names = Vec::new();
                names.push("aperf".to_string());
                let keys = values[0].data.keys().clone();

                for k in keys {
                    let datatype: Vec<&str> = k.split('-').collect();
                    if !names.contains(&datatype[0].to_string()) {
                        names.push(datatype[0].to_string());
                    }
                }
                Ok(serde_json::to_string(&names)?)
            }
            "values" => {
                let (_, key) = &param[2];
                get_key_data(values, key.to_string(), metrics)
            }
            _ => panic!("Unsupported API"),
        }
    }

    fn has_custom_raw_data_parser() -> bool {
        true
    }
}
