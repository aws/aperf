use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
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
                let mut series_name = datatype.get(1).unwrap_or(&metric_name).to_string();
                // Make the series name easier to understand - since it's essentially to write the
                // collected data to disk
                if series_name == "print" {
                    series_name = "write".to_string();
                }

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
                // sort the series first to make the order consistent across different runs
                metric
                    .series
                    .sort_by(|a, b| a.series_name.cmp(&b.series_name));

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
}
