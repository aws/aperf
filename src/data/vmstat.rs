use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::{add_metrics, get_data_name_from_type, DataMetrics, Metric};
use crate::visualizer::{GetData, GraphLimitType, GraphMetadata};
use crate::PDError;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VmstatRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl VmstatRaw {
    pub fn new() -> Self {
        VmstatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for VmstatRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/vmstat")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Vmstat {
    pub time: TimeEnum,
    pub vmstat_data: HashMap<String, i64>,
}

impl Vmstat {
    pub fn new() -> Self {
        Vmstat {
            time: TimeEnum::DateTime(Utc::now()),
            vmstat_data: HashMap::new(),
        }
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }

    fn set_data(&mut self, data: HashMap<String, i64>) {
        self.vmstat_data = data;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EndVmstatData {
    pub data: Vec<VmstatEntry>,
    pub metadata: GraphMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VmstatEntry {
    pub time: TimeEnum,
    pub value: i64,
}

fn get_entry(values: Vec<Vmstat>, key: String, metrics: &mut DataMetrics) -> Result<String> {
    let mut end_values = Vec::new();
    let mut metric = Metric::new(key.clone());
    let mut metadata = GraphMetadata::new();
    let time_zero = values[0].time;
    let mut prev_vmstat = values[0].clone();
    for value in values {
        let current_vmstat = value.clone();
        let current_time = current_vmstat.time;

        let curr_data = current_vmstat.vmstat_data.clone();
        let curr_value = curr_data
            .get(&key)
            .ok_or(PDError::VisualizerVmstatValueGetError(key.to_string()))?;
        let prev_data = prev_vmstat.vmstat_data.clone();
        let prev_value = prev_data
            .get(&key)
            .ok_or(PDError::VisualizerVmstatValueGetError(key.to_string()))?;

        let mut v = *curr_value;
        if !key.contains("nr_") {
            v = *curr_value - *prev_value;
        }
        metadata.update_limits(GraphLimitType::UInt64(v as u64));
        let vmstat_entry = VmstatEntry {
            time: (current_time - time_zero),
            value: v,
        };
        metric.insert_value(v as f64);
        end_values.push(vmstat_entry);
        prev_vmstat = value.clone();
    }
    let vmstat_data = EndVmstatData {
        data: end_values,
        metadata,
    };
    add_metrics(
        key,
        &mut metric,
        metrics,
        get_data_name_from_type::<Vmstat>().to_string(),
    )?;
    Ok(serde_json::to_string(&vmstat_data)?)
}

fn get_entries(value: Vmstat) -> Result<String> {
    let mut keys: Vec<String> = value.vmstat_data.into_keys().collect();
    keys.sort();
    Ok(serde_json::to_string(&keys)?)
}

impl GetData for Vmstat {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::VmstatRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let mut vmstat = Vmstat::new();
        let reader = BufReader::new(raw_value.data.as_bytes());
        let mut map: HashMap<String, i64> = HashMap::new();
        for line in reader.lines() {
            let line = line?;
            let mut split = line.split_whitespace();
            let name = split.next().ok_or(PDError::ProcessorOptionExtractError)?;
            let val = split.next().ok_or(PDError::ProcessorOptionExtractError)?;
            map.insert(name.to_owned(), val.parse::<i64>()?);
        }
        vmstat.set_time(raw_value.time);
        vmstat.set_data(map);
        let processed_data = ProcessedData::Vmstat(vmstat);
        Ok(processed_data)
    }

    fn process_raw_data_new(&mut self, raw_data: Vec<Data>) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();

        let mut time_zero: Option<TimeEnum> = None;
        let mut prev_val_map: HashMap<String, i64> = HashMap::new();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::VmstatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };
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

                let prev_val = prev_val_map.entry(name.to_string()).or_insert(val);

                let mut v = val;
                if !name.contains("nr_") {
                    v -= *prev_val;
                }

                let metric = time_series_data
                    .metrics
                    .entry(name.to_string())
                    .or_insert_with(|| TimeSeriesMetric::new(name.to_string()));
                let series = match metric.series.get_mut(0) {
                    Some(s) => s,
                    None => {
                        metric.series.push(Series::new(None));
                        &mut metric.series[0]
                    }
                };

                *prev_val = val;
                series.values.push(v as f64);
                series.time_diff.push(time_diff);
            }
        }

        for (metric_name, metric) in &mut time_series_data.metrics {
            let series = match metric.series.get_mut(0) {
                Some(s) => s,
                None => continue,
            };
            let skip = !metric_name.contains("nr_") as usize;
            metric.stats = Statistics::from_values(&series.values[skip..].to_vec());
            metric.value_range = (
                metric.stats.min.floor() as u64,
                metric.stats.max.ceil() as u64,
            );
        }

        // The metrics should be sorted alphabetically by their names
        let mut vmstat_metric_names: Vec<String> =
            time_series_data.metrics.keys().cloned().collect();
        vmstat_metric_names.sort();
        time_series_data.sorted_metric_names = vmstat_metric_names;

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
                ProcessedData::Vmstat(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => get_entries(values[0].clone()),
            "values" => {
                let (_, key) = &param[2];
                get_entry(values, key.to_string(), metrics)
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EndVmstatData, Vmstat, VmstatRaw};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut vmstat = VmstatRaw::new();
        let params = CollectorParams::new();

        vmstat.collect_data(&params).unwrap();
        assert!(!vmstat.data.is_empty());
    }

    #[test]
    fn test_get_entries() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut vmstat = VmstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        vmstat.collect_data(&params).unwrap();
        buffer.push(Data::VmstatRaw(vmstat));
        processed_buffer.push(Vmstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Vmstat::new()
            .get_data(
                processed_buffer,
                "run=test&get=keys".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<&str> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_get_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut vmstat = VmstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        vmstat.collect_data(&params).unwrap();
        buffer.push(Data::VmstatRaw(vmstat));
        processed_buffer.push(Vmstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Vmstat::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&key=nr_dirty".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let data: EndVmstatData = serde_json::from_str(&json).unwrap();
        assert!(!data.data.is_empty());
        match data.data[0].time {
            TimeEnum::TimeDiff(value) => assert!(value == 0),
            _ => unreachable!(),
        }
    }
}
