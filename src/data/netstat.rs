use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::{add_metrics, get_data_name_from_type, DataMetrics, Metric};
use crate::visualizer::{GetData, GraphLimitType, GraphMetadata, ReportParams};
use crate::PDError;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetstatRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl NetstatRaw {
    pub fn new() -> Self {
        NetstatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for NetstatRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/net/netstat")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Netstat {
    pub time: TimeEnum,
    pub netstat_data: HashMap<String, u64>,
}

impl Netstat {
    pub fn new() -> Self {
        Netstat {
            time: TimeEnum::DateTime(Utc::now()),
            netstat_data: HashMap::new(),
        }
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }

    fn set_data(&mut self, data: HashMap<String, u64>) {
        self.netstat_data = data;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NetstatEntry {
    pub time: TimeEnum,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EndNetData {
    pub data: Vec<NetstatEntry>,
    pub metadata: GraphMetadata,
}

fn get_entry(values: Vec<Netstat>, key: String, metrics: &mut DataMetrics) -> Result<String> {
    let mut end_values = Vec::new();
    let mut metric = Metric::new(key.clone());
    let mut metadata = GraphMetadata::new();
    let time_zero = values[0].time;
    let mut prev_netstat = values[0].clone();
    for value in values {
        let current_netstat = value.clone();
        let current_time = current_netstat.time;

        let curr_data = current_netstat.netstat_data.clone();
        let curr_value = curr_data
            .get(&key)
            .ok_or(PDError::VisualizerNetstatValueGetError(key.to_string()))?;
        let prev_data = prev_netstat.netstat_data.clone();
        let prev_value = prev_data
            .get(&key)
            .ok_or(PDError::VisualizerNetstatValueGetError(key.to_string()))?;

        let netstat_entry = NetstatEntry {
            time: (current_time - time_zero),
            value: *curr_value - *prev_value,
        };
        metric.insert_value(netstat_entry.value as f64);
        metadata.update_limits(GraphLimitType::UInt64(netstat_entry.value));
        end_values.push(netstat_entry);
        prev_netstat = value.clone();
    }
    let netdata = EndNetData {
        data: end_values,
        metadata,
    };
    add_metrics(
        key,
        &mut metric,
        metrics,
        get_data_name_from_type::<Netstat>().to_string(),
    )?;
    Ok(serde_json::to_string(&netdata)?)
}

fn get_entries(value: Netstat) -> Result<String> {
    let mut keys: Vec<String> = value.netstat_data.into_keys().collect();
    keys.sort();
    Ok(serde_json::to_string(&keys)?)
}

// TODO: ------------------------------------------------------------------------------------------
//       Below are the new implementation to process netstat into uniform data format. Remove
//       the original for the migration.

fn parse_raw_netstat_data(raw_netstat_data: &String) -> Result<HashMap<String, u64>, String> {
    let mut netstat: HashMap<String, u64> = HashMap::new();

    let mut raw_netstat_lines = raw_netstat_data.lines();
    while let (Some(netstat_names_line), Some(netstat_values_line)) =
        (raw_netstat_lines.next(), raw_netstat_lines.next())
    {
        let mut netstat_names = netstat_names_line.split_whitespace();
        let mut netstat_values = netstat_values_line.split_whitespace();

        let netstat_name_prefix = netstat_names.next().ok_or(format!(
            "Malformatted netstat names line: {netstat_names_line}"
        ))?;
        // The prefix exists on the values line as well so skip it first
        netstat_values.next();

        while let Some(netstat_value) = netstat_values.next() {
            let netstat_name = netstat_names.next().ok_or(format!(
                "Missing expected netstat name in {netstat_names_line}"
            ))?;
            let parsed_netstat_value = netstat_value
                .parse::<u64>()
                .map_err(|_| format!("Invalid netstat value in {netstat_values_line}"))?;
            netstat.insert(
                netstat_name_prefix.to_string() + netstat_name,
                parsed_netstat_value,
            );
        }

        if let Some(_) = netstat_names.next() {
            return Err(format!("Excessive netstat name in {netstat_names_line}"));
        }
    }

    Ok(netstat)
}

// TODO: ------------------------------------------------------------------------------------------

impl GetData for Netstat {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let raw_value = match buffer {
            Data::NetstatRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let mut netstat = Netstat::new();
        let reader = BufReader::new(raw_value.data.as_bytes());
        let mut map: HashMap<String, u64> = HashMap::new();
        let mut lines = reader.lines();

        while let (Some(line1), Some(line2)) = (lines.next(), lines.next()) {
            let binding = line1.unwrap();
            let params: Vec<&str> = binding.split_whitespace().collect();

            let binding = line2.unwrap();
            let values: Vec<&str> = binding.split_whitespace().collect();

            if params.len() != values.len() {
                panic!("Parameter count should match value count!")
            }

            let mut param_itr = params.iter();
            let mut val_itr = values.iter();

            let tag = param_itr.next().unwrap().to_owned();
            val_itr.next();

            for param in param_itr {
                let val = val_itr.next().ok_or(PDError::ProcessorOptionExtractError)?;
                map.insert(tag.to_owned() + " " + param.to_owned(), val.parse::<u64>()?);
            }
        }

        netstat.set_time(raw_value.time);
        netstat.set_data(map);
        let processed_data = ProcessedData::Netstat(netstat);
        Ok(processed_data)
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
                ProcessedData::Netstat(ref value) => values.push(value.clone()),
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

    fn process_raw_data_new(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();

        // The /proc/net/netstat data are accumulative, so memorize all the stats of the
        // previous state
        let mut prev_netstat: HashMap<String, u64> = HashMap::new();
        // Initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::NetstatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            let netstat = match parse_raw_netstat_data(&raw_value.data) {
                Ok(netstat) => netstat,
                Err(message) => {
                    error!("{}", message);
                    continue;
                }
            };

            for (netstat_name, netstat_value) in &netstat {
                let prev_netstat_value = prev_netstat.get(netstat_name).unwrap_or(&netstat_value);

                if !time_series_data.metrics.contains_key(netstat_name) {
                    let mut netstat_metric = TimeSeriesMetric::new(netstat_name.clone());
                    netstat_metric.series.push(Series::new(None));
                    time_series_data
                        .metrics
                        .insert(netstat_name.clone(), netstat_metric);
                }
                let netstat_metric = time_series_data.metrics.get_mut(netstat_name).unwrap();
                let series = &mut netstat_metric.series[0];
                series.time_diff.push(time_diff);
                series
                    .values
                    .push((netstat_value - prev_netstat_value) as f64);
            }

            prev_netstat = netstat;
        }

        // Compute the stats of every metric and update the value range
        for net_stat_metric in time_series_data.metrics.values_mut() {
            let series = match net_stat_metric.series.get_mut(0) {
                Some(series) => {
                    // We are skipping the first element for stats computation (since it's always 0)
                    // by creating a slice from the second element. Therefore, skip the computation
                    // if the number of values in the series is less than 2
                    if series.values.len() < 2 {
                        continue;
                    }
                    series
                }
                None => continue,
            };
            let metric_stats = Statistics::from_values(&series.values[1..].to_vec());
            net_stat_metric.value_range = (
                metric_stats.min.floor() as u64,
                metric_stats.max.ceil() as u64,
            );
            net_stat_metric.stats = metric_stats;
        }
        // The metrics should be sorted alphabetically by their names
        let mut netstat_metric_names: Vec<String> =
            time_series_data.metrics.keys().cloned().collect();
        netstat_metric_names.sort();
        time_series_data.sorted_metric_names = netstat_metric_names;

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    use super::{EndNetData, Netstat, NetstatRaw};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut netstat = NetstatRaw::new();
        let params = CollectorParams::new();

        netstat.collect_data(&params).unwrap();
        assert!(!netstat.data.is_empty());
    }

    #[test]
    fn test_get_entries() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut netstat = NetstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        netstat.collect_data(&params).unwrap();
        buffer.push(Data::NetstatRaw(netstat));
        processed_buffer.push(Netstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Netstat::new()
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
        let mut netstat = NetstatRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        netstat.collect_data(&params).unwrap();
        buffer.push(Data::NetstatRaw(netstat));
        processed_buffer.push(Netstat::new().process_raw_data(buffer[0].clone()).unwrap());
        let json = Netstat::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&key=TcpExt: TCPDSACKRecv".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let data: EndNetData = serde_json::from_str(&json).unwrap();
        assert!(!data.data.is_empty());
        match data.data[0].time {
            TimeEnum::TimeDiff(value) => assert_eq!(value, 0),
            _ => unreachable!(),
        }
    }
}
