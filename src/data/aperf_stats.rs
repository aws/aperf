use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::{Data, ProcessedData, TimeEnum};
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
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["aperf_run_stats"]
    }

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

    fn process_raw_data_new(
        &mut self,
        params: ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();

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

        let mut time_zero: Option<TimeEnum> = None;
        // Keep track of the minimum value for each metric to set the value range
        let mut per_metric_min_value: HashMap<String, u64> = HashMap::new();

        for value in values {
            let time_diff: u64 = match value.time - *time_zero.get_or_insert(value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            for (name, stat) in value.data {
                let datatype: Vec<&str> = name.split('-').collect();
                let metric_name = datatype[0];
                let series_name = datatype.get(1).unwrap_or(&metric_name).to_string();

                let metric = time_series_data
                    .metrics
                    .entry(metric_name.to_string())
                    .or_insert(TimeSeriesMetric::new(metric_name.to_string()));

                let series = if metric_name == "aperf" {
                    if metric.series.is_empty() {
                        metric.series.push(Series::new(Some(series_name)));
                    }
                    &mut metric.series[0]
                } else {
                    match metric
                        .series
                        .iter_mut()
                        .find(|s| s.series_name == Some(series_name.clone()))
                    {
                        Some(s) => s,
                        None => {
                            metric.series.push(Series::new(Some(series_name)));
                            metric.series.last_mut().unwrap()
                        }
                    }
                };

                // Keep track of the global min value for the metric
                if let Some(min_value) = per_metric_min_value.get_mut(metric_name) {
                    *min_value = (*min_value).min(stat);
                } else {
                    per_metric_min_value.insert(metric_name.to_string(), stat);
                }

                series.values.push(stat as f64);
                series.time_diff.push(time_diff);
            }
        }

        let mut metrics_with_avg = Vec::new();

        for (metric_name, metric) in &mut time_series_data.metrics {
            let series = if metric_name == "aperf" {
                &mut metric.series[0]
            } else {
                // create new series that is the sum of all series
                let mut total_series = Series::new(Some("total".to_string()));
                if !metric.series.is_empty() {
                    let mut time_value_map: HashMap<u64, f64> = HashMap::new();

                    for series in &metric.series {
                        for (i, &time_diff) in series.time_diff.iter().enumerate() {
                            *time_value_map.entry(time_diff).or_insert(0.0) += series.values[i];
                        }
                    }

                    let mut sorted_times: Vec<_> = time_value_map.keys().cloned().collect();
                    sorted_times.sort();

                    total_series.time_diff = sorted_times.clone();
                    total_series.values =
                        sorted_times.iter().map(|&t| time_value_map[&t]).collect();
                }
                metric.series.push(total_series);
                metric.series.last_mut().unwrap()
            };

            metric.stats = Statistics::from_values(&series.values);
            metric.value_range = (
                *per_metric_min_value.get(metric_name).unwrap_or(&0),
                metric.stats.max.ceil() as u64,
            );
            metrics_with_avg.push((metric_name.clone(), metric.stats.avg));
        }

        metrics_with_avg.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        time_series_data.sorted_metric_names =
            metrics_with_avg.into_iter().map(|(name, _)| name).collect();

        Ok(AperfData::TimeSeries(time_series_data))
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
