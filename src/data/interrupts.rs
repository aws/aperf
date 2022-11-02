extern crate ctor;

use anyhow::Result;
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};

pub static INTERRUPTS_FILE_NAME: &str = "interrupts";

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
    fn new() -> Self {
        InterruptData {
            interrupt_data: Vec::<InterruptLineData>::new(),
        }
    }

    fn set_interrupt_data(&mut self, data: Vec<InterruptLineData>) {
        self.interrupt_data = data;
    }
}

fn get_intr_line(data: &str) -> Result<InterruptLine> {
    let len = data.len();
    let intr_line = &data[..(len - 1)];
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

impl CollectData for InterruptData {
    fn collect_data(&mut self) -> Result<()> {
        let file = File::open("/proc/interrupts")?;
        let mut reader = BufReader::new(file);

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
            interrupt_line_data.set_time(TimeEnum::DateTime(Utc::now()));
            let line = line?;

            let mut split = line.split_whitespace();

            /* Get type of interrupt line */
            let intr_line = get_intr_line(split.next().unwrap())?;
            interrupt_line_data.set_interrupt_line(intr_line.clone());

            match intr_line {
                InterruptLine::InterruptStr(ref value) => {
                    /* Interrupts of type MIS/ERR are not per cpu */
                    if value.to_uppercase() == "MIS" || value.to_uppercase() == "ERR" {
                        let interrupt_cpu_data = get_interrupt_cpu_data(split.next().unwrap(), 0)?;
                        interrupt_line_data.push_to_per_cpu(interrupt_cpu_data);
                        interrupt_line_data.set_device(value.to_string());
                    } else {
                        /* Other named INTRs are per-cpu */
                        for cpu in 0..cpu_count {
                            let interrupt_cpu_data = get_interrupt_cpu_data(split.next().unwrap(),
                                                        *cpus_nr.get(cpu as usize).unwrap())?;
                            interrupt_line_data.push_to_per_cpu(interrupt_cpu_data);
                        }
                        /*
                         * Their names can contain spaces. Use this until as_str is
                         * merged and available in stable Rust.
                         * https://github.com/rust-lang/rust/issues/77998
                         *
                         * as_str - Get the remaining data as is
                         */
                        let mut device_name = Vec::<&str>::new();
                        loop {
                            let s = split.next();
                            match s {
                                Some(value) => device_name.push(value),
                                None => break,
                            }
                        }
                        interrupt_line_data.set_device(device_name.join(" ").to_string());
                    }
                }
                InterruptLine::InterruptNr(_) => {
                    /* Numbered interrupt lines are per-cpu */
                    for cpu in 0..cpu_count {
                        let interrupt_cpu_data = get_interrupt_cpu_data(split.next().unwrap(),
                                                    *cpus_nr.get(cpu as usize).unwrap())?;
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
        debug!("{:#?}", interrupt_line_datas);
        self.set_interrupt_data(interrupt_line_datas);
        Ok(())
    }
}

fn get_lines(value: InterruptData) -> Result<String> {
    let mut lines = Vec::new();
    for line_data in value.interrupt_data {
        let line_name;
        match line_data.interrupt_line {
            InterruptLine::InterruptStr(v) => line_name = v,
            InterruptLine::InterruptNr(v) => line_name = v.to_string(),
            _ => panic!("Line Nr cannot be None"),
        }
        lines.push(line_name);
    }
    Ok(serde_json::to_string(&lines)?)
}

fn get_key_data(values: Vec<InterruptData>, key: String) -> Vec<InterruptLineData> {
    let mut key_values = Vec::new();
    for value in values {
        for line_data in value.interrupt_data {
            let line_name;
            match line_data.interrupt_line.clone() {
                InterruptLine::InterruptStr(v) => line_name = v,
                InterruptLine::InterruptNr(v) => line_name = v.to_string(),
                _ => panic!("Can't be None"),
            }
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
    let time_zero = key_values[0].time;
    for data in key_values {
        let mut end_value = data.clone();
        end_value.set_time(data.time - time_zero);
        end_values.push(end_value);
    }
    Ok(serde_json::to_string(&end_values)?)
}

impl GetData for InterruptData {
    fn get_data(&mut self, buffer: Vec<Data>, query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                Data::InterruptData(ref value) => values.push(value.clone()),
                _ => panic!("Invalid Data type in file"),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "lines" => {
                return get_lines(values[0].clone());
            }
            "values" => {
                let (_, key) = &param[2];
                return get_line_data(values.clone(), key.to_string());
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_interrupts() {
    let interrupt_data = InterruptData::new();
    let file_name = INTERRUPTS_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::InterruptData(interrupt_data.clone()),
        file_name.clone(),
        false
    );
    let js_file_name = file_name.clone() + &".js".to_string();
    let dv = DataVisualizer::new(
        Data::InterruptData(interrupt_data),
        file_name.clone(),
        js_file_name,
        include_str!("../bin/html_files/js/interrupts.js").to_string(),
        file_name.clone()
    );
    PERFORMANCE_DATA
        .lock()
        .unwrap()
        .add_datatype(file_name.clone(), dt);

    VISUALIZATION_DATA
        .lock()
        .unwrap()
        .add_visualizer(file_name.clone(), dv);
}

#[cfg(test)]
mod tests {
    use super::{InterruptData, InterruptLineData};
    use crate::data::{CollectData, Data};
    use crate::visualizer::GetData;

    #[test]
    fn test_collect_data() {
        let mut id = InterruptData::new();

        assert!(id.collect_data().unwrap() == ());
        assert!(id.interrupt_data.len() > 0);
    }

    #[test]
    fn test_get_data_interrupt_lines() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut id_zero = InterruptData::new();
        let mut id_one = InterruptData::new();

        id_zero.collect_data().unwrap();
        id_one.collect_data().unwrap();

        buffer.push(Data::InterruptData(id_zero));
        buffer.push(Data::InterruptData(id_one));
        let json = InterruptData::new().get_data(buffer, "run=test&get=lines".to_string()).unwrap();
        let values: Vec<String> = serde_json::from_str(&json).unwrap();
        assert!(values.len() > 0);
    }

    #[test]
    fn test_get_data_interrupt_values() {
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut id = InterruptData::new();

        id.collect_data().unwrap();

        buffer.push(Data::InterruptData(id));
        let json = InterruptData::new().get_data(buffer.clone(), "run=test&get=lines".to_string()).unwrap();
        let values: Vec<String> = serde_json::from_str(&json).unwrap();
        let key_query = format!("run=test&get=values&key={}", values[0]);
        let ld_json = InterruptData::new().get_data(buffer, key_query).unwrap();
        let line_data: Vec<InterruptLineData> = serde_json::from_str(&ld_json).unwrap();
        assert!(line_data.len() > 0);
        assert!(line_data[0].per_cpu.len() > 0);
    }
}
