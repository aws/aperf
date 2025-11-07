use crate::data::data_formats::{AperfData, Series, Statistics, TimeSeriesData, TimeSeriesMetric};
use crate::data::utils::{get_aggregate_cpu_series_name, get_cpu_series_name};
use crate::data::{CollectData, CollectorParams, Data, ProcessedData, TimeEnum};
use crate::utils::DataMetrics;
use crate::visualizer::{GetData, ReportParams};
use crate::PDError;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptDataRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl InterruptDataRaw {
    pub fn new() -> Self {
        InterruptDataRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for InterruptDataRaw {
    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        self.data = std::fs::read_to_string("/proc/interrupts")?;
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InterruptLine {
    InterruptNr(u64),
    InterruptStr(String),
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptCPUData {
    pub cpu: u64,
    pub count: u64,
}

impl InterruptCPUData {
    fn new() -> Self {
        InterruptCPUData { cpu: 0, count: 0 }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptLineData {
    pub time: TimeEnum,
    pub interrupt_line: InterruptLine,
    pub interrupt_type: String,
    pub interrupt_device: String,
    pub per_cpu: Vec<InterruptCPUData>,
}

impl InterruptLineData {
    fn new() -> Self {
        InterruptLineData {
            time: TimeEnum::DateTime(Utc::now()),
            interrupt_line: InterruptLine::None,
            per_cpu: Vec::<InterruptCPUData>::new(),
            interrupt_type: String::new(),
            interrupt_device: String::new(),
        }
    }

    fn set_time(&mut self, time: TimeEnum) {
        self.time = time;
    }

    fn set_interrupt_line(&mut self, interrupt_line: InterruptLine) {
        self.interrupt_line = interrupt_line;
    }

    fn push_to_per_cpu(&mut self, per_cpu: InterruptCPUData) {
        self.per_cpu.push(per_cpu);
    }

    fn set_type(&mut self, interrupt_type: String) {
        self.interrupt_type = interrupt_type;
    }

    fn set_device(&mut self, interrupt_device: String) {
        self.interrupt_device = interrupt_device;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterruptData {
    pub interrupt_data: Vec<InterruptLineData>,
}

impl InterruptData {
    pub fn new() -> Self {
        InterruptData {
            interrupt_data: Vec::<InterruptLineData>::new(),
        }
    }

    fn set_interrupt_data(&mut self, data: Vec<InterruptLineData>) {
        self.interrupt_data = data;
    }
}

fn get_intr_line(data: &str) -> Result<InterruptLine> {
    let intr_line = &data;
    if intr_line.chars().all(char::is_numeric) {
        Ok(InterruptLine::InterruptNr(intr_line.parse::<u64>()?))
    } else {
        Ok(InterruptLine::InterruptStr(intr_line.to_string()))
    }
}

fn get_interrupt_cpu_data(data: &str, cpu: u64) -> Result<InterruptCPUData> {
    let mut cpu_data = InterruptCPUData::new();
    cpu_data.cpu = cpu;
    cpu_data.count = data.parse()?;

    Ok(cpu_data)
}

fn process_collected_raw_data(buffer: Data) -> Result<ProcessedData> {
    let mut interrupt_data = InterruptData::new();
    let raw_value = match buffer {
        Data::InterruptDataRaw(ref value) => value,
        _ => panic!("Invalid Data type in raw file"),
    };
    let mut reader = BufReader::new(raw_value.data.as_bytes());

    /* Collect the CPUs from the 1st line */
    let mut cpus_string = String::new();
    reader.read_line(&mut cpus_string)?;
    let cpus = cpus_string.split_whitespace();

    /* Create a vec to hold CPU # to be use later */
    let cpus_nr: Vec<u64> = cpus
        .into_iter()
        .map(|string| string[3..].parse::<u64>().unwrap())
        .collect();
    let cpu_count = cpus_nr.len() as u64;

    let mut interrupt_line_datas = Vec::new();
    for line in reader.lines() {
        let mut interrupt_line_data = InterruptLineData::new();
        interrupt_line_data.set_time(raw_value.time);

        let line = line?;
        let mut split = line
            .split(|c: char| c.is_whitespace() || c == ':')
            .filter(|s| !s.is_empty());

        /* Get type of interrupt line */
        let intr_line = get_intr_line(split.next().unwrap())?;
        interrupt_line_data.set_interrupt_line(intr_line.clone());

        match intr_line {
            InterruptLine::InterruptStr(ref value) => {
                /* Interrupts of type MIS/ERR are not per cpu */
                if value.to_uppercase() == "MIS" || value.to_uppercase() == "ERR" {
                    let interrupt_cpu_data = get_interrupt_cpu_data(split.next().unwrap(), 0)?;
                    interrupt_line_data.push_to_per_cpu(interrupt_cpu_data);
                    interrupt_line_data.set_type(value.to_string());
                } else {
                    /* Other named INTRs are per-cpu */
                    for cpu in 0..cpu_count {
                        let interrupt_cpu_data = get_interrupt_cpu_data(
                            split.next().unwrap(),
                            *cpus_nr.get(cpu as usize).unwrap(),
                        )?;
                        interrupt_line_data.push_to_per_cpu(interrupt_cpu_data);
                    }
                    /*
                     * Their names can contain spaces. Use this until as_str is
                     * merged and available in stable Rust.
                     * https://github.com/rust-lang/rust/issues/77998
                     *
                     * as_str - Get the remaining data as is
                     */
                    let mut type_name = Vec::<&str>::new();
                    loop {
                        let s = split.next();
                        match s {
                            Some(value) => type_name.push(value),
                            None => break,
                        }
                    }
                    interrupt_line_data.set_type(type_name.join(" ").to_string());
                }
            }
            InterruptLine::InterruptNr(_) => {
                /* Numbered interrupt lines are per-cpu */
                for cpu in 0..cpu_count {
                    let interrupt_cpu_data = get_interrupt_cpu_data(
                        split.next().unwrap(),
                        *cpus_nr.get(cpu as usize).unwrap(),
                    )?;
                    interrupt_line_data.push_to_per_cpu(interrupt_cpu_data);
                }
                /* They also contain additional information about type, edge and device name */
                let intr_type = split.next().unwrap();
                let device_name = split.last().unwrap();
                interrupt_line_data.set_type(intr_type.to_string());
                interrupt_line_data.set_device(device_name.to_string());
            }
            _ => error!("Invalid interrupt type"),
        }
        interrupt_line_datas.push(interrupt_line_data);
    }
    trace!("{:#?}", interrupt_line_datas);
    interrupt_data.set_interrupt_data(interrupt_line_datas);
    let processed_data = ProcessedData::InterruptData(interrupt_data);
    Ok(processed_data)
}

fn get_lines(value: InterruptData) -> Result<String> {
    let mut lines = Vec::new();
    for line_data in value.interrupt_data {
        let line_name = match line_data.interrupt_line {
            InterruptLine::InterruptStr(v) => v,
            InterruptLine::InterruptNr(v) => v.to_string(),
            _ => panic!("Line Nr cannot be None"),
        };
        lines.push(line_name);
    }
    Ok(serde_json::to_string(&lines)?)
}

fn get_key_data(values: Vec<InterruptData>, key: String) -> Vec<InterruptLineData> {
    let mut key_values = Vec::new();
    for value in values {
        for line_data in value.interrupt_data {
            let line_name = match line_data.interrupt_line.clone() {
                InterruptLine::InterruptStr(v) => v,
                InterruptLine::InterruptNr(v) => v.to_string(),
                _ => panic!("Can't be None"),
            };
            if line_name == key {
                key_values.push(line_data);
            }
        }
    }
    key_values
}

fn get_line_data(values: Vec<InterruptData>, key: String) -> Result<String> {
    let key_values = get_key_data(values, key);
    let mut end_values = Vec::new();
    let mut prev_data_map = HashMap::new();
    let time_zero = key_values[0].time;
    for cpu_data in &key_values[0].per_cpu {
        prev_data_map.insert(cpu_data.cpu, cpu_data.count);
    }
    for data in key_values {
        let mut end_value = data.clone();
        end_value.set_time(data.time - time_zero);
        for cpu_data in &mut end_value.per_cpu {
            cpu_data.count -= prev_data_map.get(&cpu_data.cpu).ok_or(
                PDError::VisualizerInterruptLineCPUCountError(format!("{}", cpu_data.cpu)),
            )?;
        }
        end_values.push(end_value);
        prev_data_map.clear();
        for cpu_data in data.per_cpu {
            prev_data_map.insert(cpu_data.cpu, cpu_data.count);
        }
    }
    Ok(serde_json::to_string(&end_values)?)
}

// TODO: ------------------------------------------------------------------------------------------
//       Below are the new implementation to process interrupts into uniform data format. Remove
//       the original for the migration.
#[derive(Clone)]
struct Interrupt {
    pub interrupt_name: String,
    pub per_cpu_values: Vec<u64>,
    pub average_value: f64,
}

impl Interrupt {
    fn new(interrupt_name: String) -> Self {
        Interrupt {
            interrupt_name,
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
        Ok(interrupt_number) => format!("Interrupt #{}", interrupt_number,),
        Err(_) => interrupt.interrupt_name.clone(),
    }
}

/// Check if the interrupt name is the special interrupt MIS or ERR
fn is_interrupt_name_mis_err(interrupt_name: &String) -> bool {
    interrupt_name.to_uppercase() == "MIS" || interrupt_name.to_uppercase() == "ERR"
}

/// Sort all interrupt metric names:
/// - All Interrupt #* metrics are shown first (ordered by the interrupt numbers)
/// - If the name is MIS or ERR, they are shown at last
/// - Others are sorted by name
fn get_sorted_metric_names(mut interrupt_metric_names: Vec<String>) -> Vec<String> {
    fn get_interrupt_number(s: &str) -> Option<u32> {
        if s.starts_with("Interrupt #") {
            s["Interrupt #".len()..]
                .split_whitespace()
                .next()
                .and_then(|num| num.parse().ok())
        } else {
            None
        }
    }

    interrupt_metric_names.sort_by(|a, b| {
        let a_mis_err = is_interrupt_name_mis_err(a);
        let b_mis_err = is_interrupt_name_mis_err(b);
        match (a_mis_err, b_mis_err) {
            (true, true) => a.cmp(b),
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => match (get_interrupt_number(a), get_interrupt_number(b)) {
                (Some(num_a), Some(num_b)) => num_a.cmp(&num_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.cmp(b),
            },
        }
    });

    interrupt_metric_names
}

// TODO: ------------------------------------------------------------------------------------------
impl GetData for InterruptData {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        process_collected_raw_data(buffer)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["keys".to_string(), "values".to_string()])
    }

    fn get_data(
        &mut self,
        buffer: Vec<ProcessedData>,
        query: String,
        _metrics: &mut DataMetrics,
    ) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::InterruptData(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => get_lines(values[0].clone()),
            "values" => {
                let (_, key) = &param[2];
                get_line_data(values.clone(), key.to_string())
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
        time_series_data.sorted_metric_names =
            get_sorted_metric_names(time_series_data.metrics.keys().cloned().collect());

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    use super::{InterruptData, InterruptDataRaw, InterruptLine, InterruptLineData};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::get_file;
    use crate::utils::DataMetrics;
    use crate::visualizer::{DataVisualizer, GetData};

    #[test]
    fn test_collect_data() {
        let mut id = InterruptDataRaw::new();
        let params = CollectorParams::new();

        id.collect_data(&params).unwrap();
        assert!(!id.data.is_empty());
    }

    #[test]
    fn test_get_data_interrupt_line_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut id_raw = InterruptDataRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        id_raw.collect_data(&params).unwrap();

        buffer.push(Data::InterruptDataRaw(id_raw));
        for buf in buffer {
            processed_buffer.push(InterruptData::new().process_raw_data(buf).unwrap());
        }
        let id = match &processed_buffer[0] {
            ProcessedData::InterruptData(value) => value.clone(),
            _ => unreachable!(),
        };
        for interrupt_line_data in &id.interrupt_data {
            match &interrupt_line_data.interrupt_line {
                InterruptLine::InterruptNr(_) => {
                    assert!(!interrupt_line_data.interrupt_type.is_empty());
                    assert!(!interrupt_line_data.interrupt_device.is_empty());
                    assert!(!interrupt_line_data.per_cpu.is_empty());
                }
                InterruptLine::InterruptStr(value) => {
                    if value.to_uppercase() == "MIS" || value.to_uppercase() == "ERR" {
                        assert_eq!(interrupt_line_data.interrupt_type, *value);
                        assert_eq!(interrupt_line_data.per_cpu.len(), 1);
                    } else {
                        assert!(!interrupt_line_data.interrupt_type.is_empty());
                        assert!(!interrupt_line_data.per_cpu.is_empty());
                    }
                    assert!(interrupt_line_data.interrupt_device.is_empty());
                }
                InterruptLine::None => unreachable!(),
            }
        }
    }

    #[test]
    fn test_get_data_interrupt_lines() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut id_zero = InterruptDataRaw::new();
        let mut id_one = InterruptDataRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        id_zero.collect_data(&params).unwrap();
        id_one.collect_data(&params).unwrap();

        buffer.push(Data::InterruptDataRaw(id_zero));
        buffer.push(Data::InterruptDataRaw(id_one));
        for buf in buffer {
            processed_buffer.push(InterruptData::new().process_raw_data(buf).unwrap());
        }
        let json = InterruptData::new()
            .get_data(
                processed_buffer,
                "run=test&get=keys".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<String> = serde_json::from_str(&json).unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_get_data_interrupt_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut id = InterruptDataRaw::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::<ProcessedData>::new();
        let params = CollectorParams::new();

        id.collect_data(&params).unwrap();

        buffer.push(Data::InterruptDataRaw(id));
        for buf in buffer {
            processed_buffer.push(InterruptData::new().process_raw_data(buf).unwrap());
        }
        let json = InterruptData::new()
            .get_data(
                processed_buffer.clone(),
                "run=test&get=keys".to_string(),
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let values: Vec<String> = serde_json::from_str(&json).unwrap();
        let key_query = format!("run=test&get=values&key={}", values[0]);
        let ld_json = InterruptData::new()
            .get_data(
                processed_buffer,
                key_query,
                &mut DataMetrics::new(String::new()),
            )
            .unwrap();
        let line_data: Vec<InterruptLineData> = serde_json::from_str(&ld_json).unwrap();
        assert!(!line_data.is_empty());
        assert!(!line_data[0].per_cpu.is_empty());
    }

    #[test]
    fn test_process_raw_data() {
        let mut raw_data = Vec::new();
        let file = get_file(
            "tests/test-data/aperf_2023-07-26_18_37_43/".to_string(),
            "interrupts".to_string(),
        )
        .unwrap();
        match bincode::deserialize_from::<_, Data>(file) {
            Ok(v) => raw_data.push(v),
            Err(e) => match *e {
                bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
                e => panic!("{:#?}", e),
            },
        };
        let mut dv = DataVisualizer::new(
            ProcessedData::InterruptData(InterruptData::new()),
            String::new(),
            String::new(),
            String::new(),
            false,
        );
        let processed_data = dv.data.process_raw_data(raw_data[0].clone()).unwrap();
        match processed_data {
            ProcessedData::InterruptData(ref value) => {
                assert_eq!(
                    value.interrupt_data[0].interrupt_line,
                    InterruptLine::InterruptNr(1),
                    "{:#?}",
                    value
                );
                assert_eq!(
                    value.interrupt_data[0].interrupt_type, *"IO-APIC",
                    "Invalid interrupt type"
                );
                assert_eq!(
                    value.interrupt_data[0].interrupt_device, *"i8042",
                    "Invalid interrupt device"
                );
            }
            _ => unreachable!("Invalid data type in interrupts"),
        }
    }
}
