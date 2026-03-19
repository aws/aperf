use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_average_aggregate;
use crate::data::common::utils::get_cpu_series_name;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
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
    // Some interrupts only have one value (instead of per-cpu)
    pub value: Option<u64>,
}

impl Interrupt {
    fn new(interrupt_name: String) -> Self {
        Interrupt {
            interrupt_name,
            interrupt_info: String::new(),
            per_cpu_values: Vec::new(),
            value: None,
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

        // process every CPU's value
        for _i in 0..num_cpus {
            match raw_columns.next() {
                Some(raw_column) => {
                    if let Ok(cpu_value) = raw_column.parse::<u64>() {
                        interrupt.per_cpu_values.push(cpu_value);
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
            interrupt.value = interrupt.per_cpu_values.first().map(|value| *value);
            interrupt.per_cpu_values.clear();
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
        let mut time_series_data_processor = time_series_data_processor_with_average_aggregate!();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::InterruptDataRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            let per_interrupt_data = parse_raw_interrupt_data(&raw_value.data);
            for interrupt in per_interrupt_data {
                let interrupt_metric_name = get_interrupt_metric_name(&interrupt);

                for (cpu, num_interrupts) in interrupt.per_cpu_values.iter().enumerate() {
                    time_series_data_processor.add_accumulative_data_point(
                        &interrupt_metric_name,
                        &get_cpu_series_name(cpu),
                        *num_interrupts as f64,
                    );
                }
                if let Some(value) = interrupt.value {
                    time_series_data_processor.add_accumulative_data_point(
                        &interrupt_metric_name,
                        &interrupt_metric_name,
                        value as f64,
                    );
                }
            }
        }

        // sort by highest avg
        let time_series_data = time_series_data_processor.get_time_series_data_sorted_by_average();
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
