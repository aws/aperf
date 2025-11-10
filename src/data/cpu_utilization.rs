use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::utils::{get_aggregate_cpu_series_name, get_cpu_series_name};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::{get_data_name_from_type, DataMetrics, Metric};
use crate::visualizer::{GetData, ReportParams};
use anyhow::Result;
use chrono::prelude::*;
use log::trace;
use procfs::{CpuTime, KernelStats};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Sub;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

/// Gather CPU Utilization raw data.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpuUtilizationRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl CpuUtilizationRaw {
    pub fn new() -> Self {
        CpuUtilizationRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl Default for CpuUtilizationRaw {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectData for CpuUtilizationRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/stat")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct UtilValues {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub irq: u64,
    pub softirq: u64,
    pub idle: u64,
    pub iowait: u64,
    pub steal: u64,
}

impl UtilValues {
    fn new() -> Self {
        UtilValues {
            user: 0,
            nice: 0,
            system: 0,
            irq: 0,
            softirq: 0,
            idle: 0,
            iowait: 0,
            steal: 0,
        }
    }

    fn is_less_than(self, other: UtilValues) -> bool {
        !(self.user >= other.user
            && self.nice >= other.nice
            && self.system >= other.system
            && self.irq >= other.irq
            && self.softirq >= other.softirq
            && self.idle >= other.idle
            && self.iowait >= other.iowait
            && self.steal >= other.steal)
    }
}

impl Sub for UtilValues {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            user: self.user - other.user,
            nice: self.nice - other.nice,
            system: self.system - other.system,
            irq: self.irq - other.irq,
            softirq: self.softirq - other.softirq,
            idle: self.idle - other.idle,
            iowait: self.iowait - other.iowait,
            steal: self.steal - other.steal,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CpuData {
    pub time: TimeEnum,
    pub cpu: i64,
    pub values: UtilValues,
}

impl CpuData {
    fn new() -> Self {
        CpuData {
            time: TimeEnum::DateTime(Utc::now()),
            cpu: 0,
            values: UtilValues::new(),
        }
    }

    fn set_data(&mut self, cpu: i64, cpu_time: &CpuTime) {
        self.cpu = cpu;
        self.values.user = cpu_time.user;
        self.values.nice = cpu_time.nice;
        self.values.system = cpu_time.system;
        self.values.irq = cpu_time.irq.unwrap_or_default();
        self.values.softirq = cpu_time.softirq.unwrap_or_default();
        self.values.idle = cpu_time.idle;
        self.values.iowait = cpu_time.iowait.unwrap_or_default();
        self.values.steal = cpu_time.steal.unwrap_or_default();
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpuUtilization {
    pub total: CpuData,
    pub per_cpu: Vec<CpuData>,
}

impl Default for CpuUtilization {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuUtilization {
    pub fn new() -> Self {
        CpuUtilization {
            total: CpuData::new(),
            per_cpu: Vec::<CpuData>::new(),
        }
    }

    fn set_total(&mut self, cpu: i64, total: CpuTime) {
        self.total.set_data(cpu, &total);
    }

    fn set_total_time(&mut self, time: DateTime<Utc>) {
        self.total.set_time(TimeEnum::DateTime(time));
    }

    fn add_per_cpu_data(&mut self, cpu_data: CpuData) {
        self.per_cpu.push(cpu_data);
    }
}

/// Process gathered raw data during visualization.
fn process_gathered_raw_data(buffer: Data) -> Result<ProcessedData> {
    let raw_value = match buffer {
        Data::CpuUtilizationRaw(ref value) => value,
        _ => panic!("Invalid Data type in raw file"),
    };
    let stat = KernelStats::from_reader(raw_value.data.as_bytes()).unwrap();
    let mut cpu_utilization = CpuUtilization::new();
    let time_now = match raw_value.time {
        TimeEnum::DateTime(value) => value,
        _ => panic!("Has to be datetime"),
    };

    /* Get total numbers */
    cpu_utilization.set_total(-1, stat.total);
    cpu_utilization.set_total_time(time_now);

    /* Get per_cpu numbers */
    for (i, cpu) in stat.cpu_time.iter().enumerate() {
        let mut current_cpu_data = CpuData::new();

        /* Set this CPU's data */
        current_cpu_data.set_data(i as i64, cpu);
        current_cpu_data.set_time(TimeEnum::DateTime(time_now));

        /* Push to Vec of per_cpu data */
        cpu_utilization.add_per_cpu_data(current_cpu_data);
    }
    let data = ProcessedData::CpuUtilization(cpu_utilization.clone());
    Ok(data)
}

/// Visualize processed data.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct TimeData {
    pub time: TimeEnum,
    pub value: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct UtilData {
    pub cpu: i64,
    pub data: Vec<TimeData>,
}

fn percentage(value: u64, total: u64) -> u64 {
    if total > 0 {
        return ((value as f64 / total as f64) * 100.0) as u64;
    }
    0
}

fn set_as_percent(value: UtilValues) -> UtilValues {
    let total = value.user
        + value.nice
        + value.system
        + value.irq
        + value.softirq
        + value.idle
        + value.iowait
        + value.steal;

    let mut new_values = UtilValues::new();
    new_values.user = percentage(value.user, total);
    new_values.nice = percentage(value.nice, total);
    new_values.system = percentage(value.system, total);
    new_values.irq = percentage(value.irq, total);
    new_values.softirq = percentage(value.softirq, total);
    new_values.idle = percentage(value.idle, total);
    new_values.iowait = percentage(value.iowait, total);
    new_values.steal = percentage(value.steal, total);
    new_values
}

pub fn get_aggregate_data(values: Vec<CpuData>, metrics: &mut DataMetrics) -> Result<String> {
    let mut end_values = Vec::new();
    let mut prev_cpu_data = values[0].values;
    let time_zero = values[0].time;
    let mut metric_map = HashMap::new();
    let mut user = Metric::new("User".to_string());
    let mut nice = Metric::new("Nice".to_string());
    let mut system = Metric::new("System".to_string());
    let mut irq = Metric::new("Irq".to_string());
    let mut softirq = Metric::new("SoftIrq".to_string());
    let mut idle = Metric::new("Idle".to_string());
    let mut iowait = Metric::new("Iowait".to_string());
    let mut steal = Metric::new("Steal".to_string());
    for value in values {
        let mut end_value = CpuData::new();
        let current_cpu_data = value.values;
        let current_time = value.time;

        if current_cpu_data.is_less_than(prev_cpu_data) {
            prev_cpu_data = current_cpu_data;
            continue;
        }
        end_value.cpu = value.cpu;
        end_value.values = set_as_percent(current_cpu_data - prev_cpu_data);
        user.insert_value(end_value.values.user as f64);
        nice.insert_value(end_value.values.nice as f64);
        system.insert_value(end_value.values.system as f64);
        irq.insert_value(end_value.values.irq as f64);
        softirq.insert_value(end_value.values.softirq as f64);
        idle.insert_value(end_value.values.idle as f64);
        iowait.insert_value(end_value.values.iowait as f64);
        steal.insert_value(end_value.values.steal as f64);
        end_value.set_time(current_time - time_zero);
        end_values.push(end_value);
        prev_cpu_data = current_cpu_data;
    }
    metric_map.insert("User".to_string(), user.form_stats());
    metric_map.insert("Nice".to_string(), nice.form_stats());
    metric_map.insert("System".to_string(), system.form_stats());
    metric_map.insert("Irq".to_string(), irq.form_stats());
    metric_map.insert("SoftIrq".to_string(), softirq.form_stats());
    metric_map.insert("Idle".to_string(), idle.form_stats());
    metric_map.insert("Iowait".to_string(), iowait.form_stats());
    metric_map.insert("Steal".to_string(), steal.form_stats());
    metrics
        .values
        .insert(get_data_name_from_type::<CpuData>().to_string(), metric_map);
    Ok(serde_json::to_string(&end_values)?)
}

pub fn get_cpu_values(cpu: i64, values: Vec<CpuUtilization>) -> Vec<CpuData> {
    let mut end_values = Vec::new();
    for value in values {
        for per_cpu_value in value.per_cpu {
            if cpu == per_cpu_value.cpu {
                end_values.push(per_cpu_value);
                break;
            }
        }
    }
    end_values
}

fn get_type(count: u64, values: Vec<CpuUtilization>, util_type: &str) -> Result<String> {
    let mut end_values = Vec::new();
    for i in 0..count {
        let mut util_data = UtilData {
            cpu: (i as i64),
            data: Vec::new(),
        };

        /* Get cpu 'i' values */
        let cpu_values = get_cpu_values(i as i64, values.clone());
        let time_zero = cpu_values[0].time;
        let mut prev_cpu_data = cpu_values[0].values;
        for value in cpu_values {
            let mut end_value = CpuData::new();
            let current_time = value.time;
            let current_cpu_data = value.values;

            if current_cpu_data.is_less_than(prev_cpu_data) {
                prev_cpu_data = current_cpu_data;
                continue;
            }
            end_value.cpu = value.cpu;
            end_value.values = set_as_percent(current_cpu_data - prev_cpu_data);
            end_value.set_time(current_time - time_zero);

            let value = match util_type {
                "user" => end_value.values.user,
                "nice" => end_value.values.nice,
                "system" => end_value.values.system,
                "irq" => end_value.values.irq,
                "softirq" => end_value.values.softirq,
                "idle" => end_value.values.idle,
                "iowait" => end_value.values.iowait,
                "steal" => end_value.values.steal,
                _ => panic!("Invalid util type"),
            };
            let time_data = TimeData {
                time: end_value.time,
                value,
            };
            util_data.data.push(time_data);
            prev_cpu_data = current_cpu_data;
        }
        end_values.push(util_data);
    }
    Ok(serde_json::to_string(&end_values)?)
}

// TODO: ------------------------------------------------------------------------------------------
//       Below are the new implementation to process cpu_utlization into uniform data format. Remove
//       the original for the migration.
#[derive(EnumIter, Display, Clone, Copy, Eq, Hash, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum CpuState {
    USER,
    NICE,
    SYSTEM,
    IDLE,
    IOWAIT,
    IRQ,
    SOFTIRQ,
    STEAL,
}

fn get_cpu_time(cpu_state: &CpuState, cpu_time: &CpuTime) -> u64 {
    match cpu_state {
        CpuState::USER => cpu_time.user,
        CpuState::NICE => cpu_time.nice,
        CpuState::SYSTEM => cpu_time.system,
        CpuState::IDLE => cpu_time.idle,
        CpuState::IOWAIT => cpu_time.iowait.unwrap_or_default(),
        CpuState::IRQ => cpu_time.irq.unwrap_or_default(),
        CpuState::SOFTIRQ => cpu_time.softirq.unwrap_or_default(),
        CpuState::STEAL => cpu_time.steal.unwrap_or_default(),
    }
}
// TODO: ------------------------------------------------------------------------------------------

impl GetData for CpuUtilization {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        process_gathered_raw_data(buffer)
    }

    fn process_raw_data_new(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();
        // the aggregate series of all CPU state's metrics to be included in the aggregate metric
        let mut per_cpu_state_aggregate_series: HashMap<CpuState, Series> = HashMap::new();
        // the aggregate series of total CPU utilization
        let mut aggregate_total_util_series = Series::new(Some("total".to_string()));
        aggregate_total_util_series.is_aggregate = true;

        // memorize the previous CPU time to compute delta (the raw /proc/stat file contains
        // accumulated CPU jiffies since boot time)
        let mut prev_cpu_time: Vec<CpuTime> = Vec::new();
        // initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::CpuUtilizationRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let kernel_stats = KernelStats::from_reader(raw_value.data.as_bytes())?;
            let aggregate_cpu_time = vec![kernel_stats.total];
            let all_cpu_time = kernel_stats.cpu_time;
            let num_cpus = all_cpu_time.len();

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            for (cpu, cpu_time) in all_cpu_time
                .iter()
                .chain(aggregate_cpu_time.iter())
                .enumerate()
            {
                // in the case where the current raw data is the first data point, use the current
                // CPU time as the prev time, to produce a dummy delta of 0
                if cpu >= prev_cpu_time.len() {
                    prev_cpu_time.push(cpu_time.clone());
                }

                // compute the cpu time delta for every CPU state as the numerator, and the sum of
                // all deltas as the denominator
                let mut per_cpu_state_time_delta: HashMap<CpuState, f64> = HashMap::new();
                let mut cpu_time_delta_sum = 0.0;
                for cpu_state in CpuState::iter() {
                    let time_delta = (get_cpu_time(&cpu_state, cpu_time)
                        - get_cpu_time(&cpu_state, &prev_cpu_time[cpu]))
                        as f64;
                    per_cpu_state_time_delta.insert(cpu_state, time_delta);
                    cpu_time_delta_sum += time_delta;
                }
                prev_cpu_time[cpu] = cpu_time.clone();

                // compute CPU utilization by dividing every time delta by the delta sum (times 100 to get percentage)
                // and store the result in the series values of the corresponding CPU state metric
                for cpu_state in CpuState::iter() {
                    let cpu_util = if cpu_time_delta_sum > 0.0 {
                        per_cpu_state_time_delta
                            .get(&cpu_state)
                            .copied()
                            .unwrap_or_default()
                            / cpu_time_delta_sum
                            * 100.0
                    } else {
                        0.0
                    };

                    if cpu < num_cpus {
                        // processing one of the CPUs - put the util data point in the corresponding series
                        // in the CPU state metric
                        let cpu_state_metric = time_series_data
                            .metrics
                            .entry(cpu_state.to_string())
                            .or_insert_with(|| {
                                let mut _cpu_state_metric = TimeSeriesMetric::default();
                                _cpu_state_metric.metric_name = cpu_state.to_string();
                                _cpu_state_metric.value_range = (0, 100);
                                _cpu_state_metric
                            });

                        if cpu >= cpu_state_metric.series.len() {
                            cpu_state_metric
                                .series
                                .push(Series::new(get_cpu_series_name(cpu)));
                        }
                        let cpu_series = &mut cpu_state_metric.series[cpu];
                        cpu_series.time_diff.push(time_diff);
                        cpu_series.values.push(cpu_util);
                    } else {
                        // processing the aggregate of all CPUs - put the util data point in the CPU state
                        // series to be included the aggregate metric
                        let aggregate_cpu_state_series = per_cpu_state_aggregate_series
                            .entry(cpu_state)
                            .or_insert(Series::new(Some(cpu_state.to_string())));
                        aggregate_cpu_state_series.time_diff.push(time_diff);
                        aggregate_cpu_state_series.values.push(cpu_util);
                    }
                }

                // if processing the aggregate of all CPUs, also compute the total CPU utilization
                // which is sum of per-state time minus idle time
                if cpu >= num_cpus {
                    let total_cpu_util = if cpu_time_delta_sum > 0.0 {
                        (cpu_time_delta_sum
                            - per_cpu_state_time_delta
                                .get(&CpuState::IDLE)
                                .copied()
                                .unwrap_or_default())
                            / cpu_time_delta_sum
                            * 100.0
                    } else {
                        0.0
                    };
                    aggregate_total_util_series.time_diff.push(time_diff);
                    aggregate_total_util_series.values.push(total_cpu_util);
                }
            }
        }

        // add every aggregate CPU state series to the corresponding CPU state metric and set the
        // metric's stats as computed from the aggregate series
        for (cpu_state, aggregate_cpu_state_series) in &per_cpu_state_aggregate_series {
            if let Some(cpu_state_metric) = time_series_data.metrics.get_mut(&cpu_state.to_string())
            {
                let mut cur_cpu_state_aggregate_series = aggregate_cpu_state_series.clone();
                cur_cpu_state_aggregate_series.series_name = get_aggregate_cpu_series_name();
                cur_cpu_state_aggregate_series.is_aggregate = true;
                cpu_state_metric.series.push(cur_cpu_state_aggregate_series);
                cpu_state_metric.stats =
                    Statistics::from_values(&aggregate_cpu_state_series.values);
            }
        }

        let aggregate_metric_name = "aggregate";
        if !per_cpu_state_aggregate_series.is_empty() {
            // create the aggregate metric to include the aggregate series of all CPU states as well
            // as the total CPU utilization
            let mut aggregate_metric = TimeSeriesMetric::default();
            aggregate_metric.metric_name = aggregate_metric_name.to_string();
            aggregate_metric.value_range = (0, 100);
            for cpu_state in CpuState::iter() {
                if let Some(cpu_state_aggregate_series) =
                    per_cpu_state_aggregate_series.remove(&cpu_state)
                {
                    aggregate_metric.series.push(cpu_state_aggregate_series);
                }
            }
            aggregate_metric.stats = Statistics::from_values(&aggregate_total_util_series.values);
            aggregate_metric.series.push(aggregate_total_util_series);
            time_series_data
                .metrics
                .insert(aggregate_metric_name.to_string(), aggregate_metric);
        }

        let mut sorted_metric_names = vec![aggregate_metric_name.to_string()];
        for cpu_state in CpuState::iter() {
            sorted_metric_names.push(cpu_state.to_string());
        }
        time_series_data.sorted_metric_names = sorted_metric_names;

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
                ProcessedData::CpuUtilization(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => {
                let end_values = [
                    "aggregate",
                    "user",
                    "nice",
                    "system",
                    "irq",
                    "softirq",
                    "idle",
                    "iowait",
                    "steal",
                ];
                Ok(serde_json::to_string(&end_values)?)
            }
            "values" => {
                let (_, key) = &param[2];
                if key == "aggregate" {
                    let mut temp_values = Vec::new();
                    for value in values {
                        temp_values.push(value.total);
                    }
                    get_aggregate_data(temp_values, metrics)
                } else {
                    get_type(values[0].per_cpu.len() as u64, values, key)
                }
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[cfg(test)]
mod cpu_tests {
    use super::{CpuData, CpuUtilization, CpuUtilizationRaw, UtilData};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::utils::DataMetrics;
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut cpu_utilization = CpuUtilizationRaw::new();
        let params = CollectorParams::new();

        cpu_utilization.collect_data(&params).unwrap();
        assert!(!cpu_utilization.data.is_empty());
    }

    #[test]
    fn test_get_data_aggregate_cpu() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut cpu_util_zero = CpuUtilizationRaw::new();
        let mut cpu_util_one = CpuUtilizationRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        cpu_util_zero.collect_data(&params).unwrap();
        cpu_util_one.collect_data(&params).unwrap();

        buffer.push(Data::CpuUtilizationRaw(cpu_util_zero));
        buffer.push(Data::CpuUtilizationRaw(cpu_util_one));
        for buf in buffer {
            processed_buffer.push(CpuUtilization::new().process_raw_data(buf).unwrap());
        }
        let json = CpuUtilization::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&=aggregate".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<CpuData> = serde_json::from_str(&json).unwrap();
        assert!(values[0].cpu == -1);
    }

    #[test]
    fn test_get_util_types() {
        let types = CpuUtilization::new()
            .get_data(
                Vec::new(),
                "run=test&get=keys".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<&str> = serde_json::from_str(&types).unwrap();
        for type_str in values {
            match type_str {
                "aggregate" | "user" | "nice" | "system" | "irq" | "softirq" | "idle"
                | "iowait" | "steal" => {}
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_get_user() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut cpu_util_zero = CpuUtilizationRaw::new();
        let mut cpu_util_one = CpuUtilizationRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        cpu_util_zero.collect_data(&params).unwrap();
        cpu_util_one.collect_data(&params).unwrap();

        buffer.push(Data::CpuUtilizationRaw(cpu_util_zero));
        buffer.push(Data::CpuUtilizationRaw(cpu_util_one));
        for buf in buffer {
            processed_buffer.push(CpuUtilization::new().process_raw_data(buf).unwrap());
        }
        let json = CpuUtilization::new()
            .get_data(
                processed_buffer,
                "run=test&get=values&key=user".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<UtilData> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }
}
