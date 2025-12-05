use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::{CollectData, CollectorParams, Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

#[cfg(test)]
mod tests {
    use super::VmstatRaw;
    use crate::data::{CollectData, CollectorParams};

    #[test]
    fn test_collect_data() {
        let mut vmstat = VmstatRaw::new();
        let params = CollectorParams::new();

        vmstat.collect_data(&params).unwrap();
        assert!(!vmstat.data.is_empty());
    }
}
