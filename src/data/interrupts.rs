use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::utils::{get_aggregate_cpu_series_name, get_cpu_series_name};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::prelude::*,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptDataRaw {
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl InterruptDataRaw {
    pub fn new() -> Self {
        InterruptDataRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for InterruptDataRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/interrupts")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptData;

impl InterruptData {
    pub fn new() -> Self {
        InterruptData
    }
}

#[derive(Clone)]
struct Interrupt {
    pub interrupt_name: String,
    pub interrupt_info: String,
    pub per_cpu_values: Vec<u64>,
    pub average_value: f64,
}

impl Interrupt {
    fn new(interrupt_name: String) -> Self {
        Interrupt {
            interrupt_name,
            interrupt_info: String::new(),
            per_cpu_values: Vec::new(),
            average_value: 0.0,
        }
    }
}

/// Process the raw contents of a /proc/interrupts file. For every line of interrupt data
/// parse and create an Interrupt object.
fn parse_raw_interrupt_data(raw_interrupt_data: &String) -> Vec<Interrupt> {
    let mut processed_interrupt_data: Vec<Interrupt> = Vec::new();

    let mut raw_interrupt_lines = raw_interrupt_data.lines();
    // Get the number of CPUs:
    let cpu_lines = raw_interrupt_lines.next().unwrap_or_default();
    let num_cpus: usize = cpu_lines.split_whitespace().count();

    // process every line except for the first line, which is a line of CPUs as column header
    for raw_interrupt_line in raw_interrupt_data.lines().skip(1) {
        let mut raw_columns = raw_interrupt_line.split_whitespace();

        let interrupt_name = match raw_columns.next() {
            Some(first_item) => first_item.trim_end_matches(":").to_string(),
            None => continue,
        };

        let mut interrupt = Interrupt::new(interrupt_name.clone());
        let mut interrupt_info_items: Vec<String> = Vec::new();
        let mut cpu_value_sum: u64 = 0;

        // process every CPU's value
        for _i in 0..num_cpus {
            match raw_columns.next() {
                Some(raw_column) => {
                    if let Ok(cpu_value) = raw_column.parse::<u64>() {
                        interrupt.per_cpu_values.push(cpu_value);
                        cpu_value_sum += cpu_value;
                    }
                }
                None => break,
            }
        }
        // store the remaining items as the interrupt info
        for raw_column in raw_columns {
            interrupt_info_items.push(raw_column.to_string());
        }

        interrupt.interrupt_info = interrupt_info_items.join(" ");
        // The MIS and ERR interrupts do not have per CPU counts
        if is_interrupt_name_mis_err(&interrupt_name) {
            interrupt.per_cpu_values.clear();
            interrupt.average_value = cpu_value_sum as f64;
        } else {
            interrupt.average_value = cpu_value_sum as f64 / interrupt.per_cpu_values.len() as f64;
        }

        processed_interrupt_data.push(interrupt);
    }

    processed_interrupt_data
}

/// Generate the name of the interrupt metric based on the interrupt name, number, and info.
fn get_interrupt_metric_name(interrupt: &Interrupt) -> String {
    match interrupt.interrupt_name.parse::<u64>() {
        Ok(_interrupt_number) => format!("({})", interrupt.interrupt_info),
        Err(_) => {
            if interrupt.interrupt_info.is_empty() {
                interrupt.interrupt_name.clone()
            } else {
                format!(
                    "{} ({})",
                    interrupt.interrupt_name, interrupt.interrupt_info
                )
            }
        }
    }
}

/// Check if the interrupt name is the special interrupt MIS or ERR
fn is_interrupt_name_mis_err(interrupt_name: &String) -> bool {
    interrupt_name.to_uppercase() == "MIS" || interrupt_name.to_uppercase() == "ERR"
}

impl ProcessData for InterruptData {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();
        // the aggregate series to be inserted into all interrupt metrics
        let mut per_interrupt_aggregate_series: HashMap<String, Series> = HashMap::new();

        // The /proc/interrupts data are cumulative, so memorize the previous data
        // to compute the delta as the series values
        let mut prev_per_interrupt_data: HashMap<String, Interrupt> = HashMap::new();
        // Initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;
        // Keep track of the largest series value for each metric to compute its value range
        let mut per_interrupt_max_value: HashMap<String, u64> = HashMap::new();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::InterruptDataRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            let per_interrupt_data = parse_raw_interrupt_data(&raw_value.data);
            for interrupt in per_interrupt_data {
                let interrupt_metric_name = get_interrupt_metric_name(&interrupt);
                let interrupt_metric = time_series_data
                    .metrics
                    .entry(interrupt_metric_name.clone())
                    .or_insert(TimeSeriesMetric::new(interrupt_metric_name.clone()));

                let prev_interrupt = prev_per_interrupt_data
                    .get(&interrupt_metric_name)
                    .unwrap_or(&interrupt);
                let num_cpus = interrupt.per_cpu_values.len();
                // Compute the value of every CPU series
                for cpu in 0..num_cpus {
                    let cur_cpu_value =
                        interrupt.per_cpu_values[cpu] - prev_interrupt.per_cpu_values[cpu];
                    // Keep track of the maximum value for current interrupt metric, to be used
                    // as the graph's max value range
                    if let Some(max_value) = per_interrupt_max_value.get_mut(&interrupt_metric_name)
                    {
                        *max_value = (*max_value).max(cur_cpu_value);
                    } else {
                        per_interrupt_max_value
                            .insert(interrupt_metric_name.clone(), cur_cpu_value);
                    }

                    if cpu >= interrupt_metric.series.len() {
                        interrupt_metric
                            .series
                            .push(Series::new(get_cpu_series_name(cpu)));
                    }
                    let cpu_series = &mut interrupt_metric.series[cpu];
                    cpu_series.time_diff.push(time_diff);
                    cpu_series.values.push(cur_cpu_value as f64);
                }
                // Compute the value of the aggregate series
                let aggregate_series = per_interrupt_aggregate_series
                    .entry(interrupt_metric_name.clone())
                    .or_insert(Series::new(get_aggregate_cpu_series_name()));
                aggregate_series.time_diff.push(time_diff);
                aggregate_series
                    .values
                    .push(interrupt.average_value - prev_interrupt.average_value);

                prev_per_interrupt_data.insert(interrupt_metric_name, interrupt.clone());
            }
        }

        // Compute the stats of every aggregate series and add them to the corresponding metric
        for (interrupt_metric_name, interrupt_metric) in &mut time_series_data.metrics {
            if let Some(aggregate_series) =
                per_interrupt_aggregate_series.get_mut(interrupt_metric_name)
            {
                let aggregate_stats = Statistics::from_values(&aggregate_series.values);
                interrupt_metric.value_range = (
                    0,
                    *per_interrupt_max_value
                        .get(interrupt_metric_name)
                        .unwrap_or(&(aggregate_stats.max.ceil() as u64)),
                );
                interrupt_metric.stats = aggregate_stats;
                aggregate_series.is_aggregate = true;
                interrupt_metric.series.push(aggregate_series.clone());
            }
        }

        // sort by highest avg
        let mut metric_names_with_avg: Vec<(String, f64)> = time_series_data
            .metrics
            .iter()
            .map(|(name, metric)| (name.clone(), metric.stats.avg))
            .collect();
        metric_names_with_avg
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        time_series_data.sorted_metric_names = metric_names_with_avg
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::InterruptDataRaw,
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut id = InterruptDataRaw::new();
        let params = CollectorParams::new();

        id.collect_data(&params).unwrap();
        assert!(!id.data.is_empty());
    }
}
