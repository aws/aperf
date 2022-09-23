extern crate ctor;

use anyhow::Result;
use crate::data::{CollectData, Data, DataType, TimeEnum};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use crate::visualizer::{DataVisualizer, GetData};
use chrono::prelude::*;
use ctor::ctor;
use log::debug;
use procfs::{CpuTime, KernelStats};
use serde::{Deserialize, Serialize};

pub static CPU_UTILIZATION_FILE_NAME: &str = "cpu_utilization";

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CpuData {
    pub time: TimeEnum,
    pub cpu: i64,
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub irq: u64,
    pub softirq: u64,
    pub idle: u64,
    pub iowait: u64,
    pub steal: u64,
}

impl CpuData {
    fn new() -> Self {
        CpuData {
            time: TimeEnum::DateTime(Utc::now()),
            cpu: 0,
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

    fn set_data(&mut self, cpu: i64, cpu_time: &CpuTime) {
        self.cpu = cpu;
        self.user = cpu_time.user;
        self.nice = cpu_time.nice;
        self.system = cpu_time.system;
        self.irq = cpu_time.irq.unwrap_or_default();
        self.softirq = cpu_time.softirq.unwrap_or_default();
        self.idle = cpu_time.idle;
        self.iowait = cpu_time.iowait.unwrap_or_default();
        self.steal = cpu_time.steal.unwrap_or_default();
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

    fn clear_per_cpu_data(&mut self) {
        self.per_cpu.clear();
    }
}

impl CollectData for CpuUtilization {
    fn collect_data(&mut self) -> Result<()> {
        let stat = KernelStats::new().unwrap();
        let time_now = Utc::now();
        self.clear_per_cpu_data();

        /* Get total numbers */
        self.set_total(-1, stat.total);
        self.set_total_time(time_now);

        debug!("Total CPU Utilization: {:#?}", self.total);
        /* Get per_cpu numbers */
        for (i, cpu) in stat.cpu_time.iter().enumerate() {
            let mut current_cpu_data = CpuData::new();

            /* Set this CPU's data */
            current_cpu_data.set_data(i as i64, cpu);
            current_cpu_data.set_time(TimeEnum::DateTime(time_now));

            /* Push to Vec of per_cpu data */
            self.add_per_cpu_data(current_cpu_data);
        }
        debug!("Per CPU Utilization: {:#?}", self.per_cpu);
        Ok(())
    }
}

impl Default for CpuUtilization {
    fn default() -> Self {
        Self::new()
    }
}

impl GetData for CpuUtilization {}

#[ctor]
fn init_cpu_utilization() {
    let cpu_utilization = CpuUtilization::new();
    let file_name = CPU_UTILIZATION_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::CpuUtilization(cpu_utilization.clone()),
        file_name.clone(),
        false
    );
    let dv = DataVisualizer::new(
        Data::CpuUtilization(cpu_utilization),
        file_name.clone(),
        String::new(),
        String::new(),
        file_name.clone(),
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
    use super::CpuUtilization;
    use crate::data::CollectData;

    #[test]
    fn test_collect_data() {
        let mut cpu_utilization = CpuUtilization::new();

        assert!(cpu_utilization.collect_data().unwrap() == ());
        assert!(cpu_utilization.total.cpu == -1);
        assert!(!cpu_utilization.per_cpu.is_empty());
    }
}
