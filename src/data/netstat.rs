use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct Netstat;

impl Netstat {
    pub fn new() -> Self {
        Netstat
    }
}

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

impl ProcessData for Netstat {
    fn process_raw_data(
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
    use super::NetstatRaw;
    use crate::data::{CollectData, CollectorParams};

    #[test]
    fn test_collect_data() {
        let mut netstat = NetstatRaw::new();
        let params = CollectorParams::new();

        netstat.collect_data(&params).unwrap();
        assert!(!netstat.data.is_empty());
    }
}
