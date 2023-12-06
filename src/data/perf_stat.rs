extern crate ctor;

use crate::data::{CollectData, CollectorParams, Data, DataType, ProcessedData, TimeEnum};
use crate::visualizer::{DataVisualizer, GetData, GraphLimitType, GraphMetadata};
use crate::{PERFORMANCE_DATA, VISUALIZATION_DATA};
use anyhow::Result;
use chrono::prelude::*;
use ctor::ctor;
use log::{trace, warn};
use perf_event::events::{Raw, Software};
use perf_event::{Builder, Counter, Group, ReadFormat};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, ErrorKind};
use std::sync::Mutex;

#[cfg(target_arch = "aarch64")]
use crate::data::grv_perf_events;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use {
    crate::data::intel_icelake_perf_events::ICX_CTRS, crate::data::intel_perf_events,
    crate::data::intel_sapphire_rapids_perf_events::SPR_CTRS, crate::data::utils::get_cpu_info,
    crate::PDError, indexmap::IndexMap,
};

pub static PERF_STAT_FILE_NAME: &str = "perf_stat";

#[derive(Copy, Clone, Debug)]
pub enum PerfType {
    RAW = 4,
}

lazy_static! {
    pub static ref CPU_CTR_GROUPS: Mutex<Vec<CpuCtrGroup>> = Mutex::new(Vec::new());
}

#[derive(Debug)]
pub struct Ctr {
    pub perf_type: u64,
    pub name: String,
    pub config: u64,
    pub counter: Counter,
}

impl Ctr {
    fn new(
        perf_type: u64,
        name: String,
        cpu: usize,
        config: u64,
        group: &mut Group,
    ) -> Result<Self> {
        let raw_config = Raw::new(config);
        Ok(Ctr {
            perf_type,
            name,
            config,
            counter: Builder::new(raw_config)
                .one_cpu(cpu)
                .any_pid()
                .include_kernel()
                .build_with_group(group)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NamedCtr<'a> {
    pub name: &'a str,
    pub nrs: Vec<NamedTypeCtr<'a>>,
    pub drs: Vec<NamedTypeCtr<'a>>,
    pub scale: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct NamedTypeCtr<'a> {
    pub perf_type: PerfType,
    pub name: &'a str,
    pub config: u64,
}

pub struct CpuCtrGroup {
    pub cpu: u64,
    pub name: String,
    pub nr_ctrs: Vec<Ctr>,
    pub dr_ctrs: Vec<Ctr>,
    pub scale: u64,
    pub group: Group,
}

impl CpuCtrGroup {
    fn nr_ctr_add(&mut self, ctr: Ctr) {
        self.nr_ctrs.push(ctr);
    }
    fn dr_ctr_add(&mut self, ctr: Ctr) {
        self.dr_ctrs.push(ctr);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfStatRaw {
    pub time: TimeEnum,
    pub data: String,
}

impl PerfStatRaw {
    fn new() -> Self {
        PerfStatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

impl CollectData for PerfStatRaw {
    fn prepare_data_collector(&mut self, _params: CollectorParams) -> Result<()> {
        let num_cpus = num_cpus::get();
        let mut cpu_groups: Vec<CpuCtrGroup> = Vec::new();

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "aarch64")] {
                let perf_list = grv_perf_events::PERF_LIST.to_vec();
            } else if #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] {
                let platform_specific_counter;
                let cpu_info = get_cpu_info()?;
                let mut perf_list;

                /* Get Vendor Specific Perf events */
                if cpu_info.vendor == "GenuineIntel" {
                    perf_list = intel_perf_events::PERF_LIST.to_vec();

                    /* Get Model specific events */
                    platform_specific_counter = match cpu_info.model_name.as_str() {
                        "Intel(R) Xeon(R) Platinum 8375C CPU @ 2.90GHz" => ICX_CTRS.to_vec(),
                        "Intel(R) Xeon(R) Platinum 8488C" => SPR_CTRS.to_vec(),
                        _ => Vec::new(),
                    };
                } else {
                    return Err(PDError::CollectorPerfUnsupportedCPU.into());
                }

                let mut events_map = IndexMap::new();
                for event in &perf_list {
                    events_map.insert(event.name, event.clone());
                }

                for event in platform_specific_counter {
                    if let Some(ctr) = events_map.get_mut(event.name) {
                        ctr.nrs = event.nrs;
                        ctr.drs = event.drs;
                        ctr.scale = event.scale;
                    } else {
                        events_map.insert(event.name, event);
                    }
                }
                perf_list = events_map.into_values().collect();
            } else {
                return Err(PDError::CollectorPerfUnsupportedCPU.into());
            }
        }
        for cpu in 0..num_cpus {
            for named_ctr in &perf_list {
                let perf_group = Builder::new(Software::DUMMY)
                    .read_format(
                        ReadFormat::GROUP
                            | ReadFormat::TOTAL_TIME_ENABLED
                            | ReadFormat::TOTAL_TIME_RUNNING
                            | ReadFormat::ID,
                    )
                    .any_pid()
                    .one_cpu(cpu)
                    .build_group();

                let group = match perf_group {
                    Err(e) => {
                        match e.kind() {
                            ErrorKind::PermissionDenied => {
                                warn!("Set /proc/sys/kernel/perf_event_paranoid to 0")
                            }
                            ErrorKind::NotFound => warn!("Instance does not expose Perf counters"),
                            _ => warn!("Unknown error when trying to use Perf API"),
                        }
                        return Err(e.into());
                    }
                    Ok(g) => g,
                };
                let mut cpu_group = CpuCtrGroup {
                    cpu: cpu as u64,
                    name: named_ctr.name.to_string(),
                    nr_ctrs: Vec::new(),
                    dr_ctrs: Vec::new(),
                    scale: named_ctr.scale,
                    group,
                };
                for nr in &named_ctr.nrs {
                    let nr_ctr = Ctr::new(
                        nr.perf_type as u64,
                        nr.name.to_string(),
                        cpu,
                        nr.config,
                        &mut cpu_group.group,
                    );
                    match nr_ctr {
                        Err(e) => {
                            if let Some(os_error) = e.downcast_ref::<std::io::Error>() {
                                match os_error.kind() {
                                    ErrorKind::NotFound => {
                                        warn!("Instance does not expose Perf counters")
                                    }
                                    _ => match os_error.raw_os_error().unwrap() {
                                        libc::EMFILE => warn!(
                                            "Too many open files. Increase limit with `ulimit -n 65536`"
                                        ),
                                        _ => warn!("Unknown error when trying to use Perf API."),
                                    },
                                }
                                return Err(e);
                            }
                        }
                        Ok(v) => cpu_group.nr_ctr_add(v),
                    }
                }
                for dr in &named_ctr.drs {
                    let dr_ctr = Ctr::new(
                        dr.perf_type as u64,
                        dr.name.to_string(),
                        cpu,
                        dr.config,
                        &mut cpu_group.group,
                    );
                    match dr_ctr {
                        Err(e) => {
                            if let Some(os_error) = e.downcast_ref::<std::io::Error>() {
                                match os_error.kind() {
                                    ErrorKind::NotFound => {
                                        warn!("Instance does not expose Perf counters")
                                    }
                                    _ => match os_error.raw_os_error().unwrap() {
                                        libc::EMFILE => warn!(
                                            "Too many open files. Increase limit with `ulimit -n 65536`"
                                        ),
                                        _ => warn!("Unknown error when trying to use Perf API."),
                                    },
                                }
                                return Err(e);
                            }
                        }
                        Ok(v) => cpu_group.dr_ctr_add(v),
                    }
                }
                cpu_groups.push(cpu_group);
            }
        }
        for cpu_group in &mut *cpu_groups {
            cpu_group.group.reset()?;
            cpu_group.group.enable()?;
        }
        CPU_CTR_GROUPS.lock().unwrap().append(&mut cpu_groups);
        Ok(())
    }

    fn collect_data(&mut self) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();
        let cpu_groups = &mut *CPU_CTR_GROUPS.lock().unwrap();
        for cpu_group in &mut *cpu_groups {
            let count = cpu_group.group.read()?;
            let mut group_data = format!("{} {};", cpu_group.cpu, cpu_group.name.clone());
            for nr in &cpu_group.nr_ctrs {
                let nr_string = format!(" {}", count[&nr.counter]);
                group_data.push_str(&nr_string);
            }
            group_data.push(';');
            for dr in &cpu_group.dr_ctrs {
                let dr_string = format!(" {}", count[&dr.counter]);
                group_data.push_str(&dr_string);
            }
            group_data.push(';');
            let scale_string = format!("{}", cpu_group.scale);
            group_data.push_str(&scale_string);
            group_data.push('\n');
            cpu_group.group.reset()?;
            self.data.push_str(&group_data);
        }
        trace!("{:#?}", self.data);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerfStat {
    pub perf_stats: Vec<PerCPUNamedStats>,
}

impl PerfStat {
    fn new() -> Self {
        PerfStat {
            perf_stats: Vec::new(),
        }
    }

    fn add_named_stat(&mut self, cpu: u64, stat: NamedStat) {
        for per_cpu_named_stat in &mut self.perf_stats {
            if per_cpu_named_stat.cpu == cpu {
                per_cpu_named_stat.named_stats.push(stat);
                return;
            }
        }
        let mut per_cpu_named_stats = PerCPUNamedStats {
            cpu,
            named_stats: Vec::new(),
        };
        per_cpu_named_stats.named_stats.push(stat);
        self.perf_stats.push(per_cpu_named_stats);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PerCPUNamedStats {
    pub cpu: u64,
    pub named_stats: Vec<NamedStat>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NamedStat {
    pub time: TimeEnum,
    pub name: String,
    pub nr_value: u64,
    pub dr_value: u64,
    pub scale: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndStat {
    pub cpu: i64,
    pub value: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndStats {
    pub time: TimeEnum,
    pub cpus: Vec<EndStat>,
}

impl EndStats {
    fn new() -> Self {
        EndStats {
            time: TimeEnum::DateTime(Utc::now()),
            cpus: Vec::new(),
        }
    }
}

pub struct InterStat {
    pub cpu: u64,
    pub named_stat: NamedStat,
}

fn get_named_stat_for_all_cpus(value: PerfStat, key: String) -> Vec<InterStat> {
    let mut named_stats = Vec::new();
    for per_cpu_named_stat in value.perf_stats {
        for named_stat in per_cpu_named_stat.named_stats {
            if named_stat.name == key {
                named_stats.push(InterStat {
                    cpu: per_cpu_named_stat.cpu,
                    named_stat,
                });
                break;
            }
        }
    }
    named_stats
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EndPerfData {
    pub data: Vec<EndStats>,
    pub metadata: GraphMetadata,
}

fn get_values(values: Vec<PerfStat>, key: String) -> Result<String> {
    let time_zero = &values[0].perf_stats[0].named_stats[0].time;
    let mut metadata = GraphMetadata::new();
    let mut end_values = Vec::new();
    let mut aggregate_value: f64;
    for value in &values {
        let mut end_stats = EndStats::new();
        let mut end_cpu_stats = Vec::new();
        let stats = get_named_stat_for_all_cpus(value.clone(), key.clone());
        let mut aggregate_nr = 0;
        let mut aggregate_dr = 0;
        for stat in &stats {
            let this_cpu_end_stat_value =
                stat.named_stat.nr_value as f64 / stat.named_stat.dr_value as f64;
            let this_cpu_end_stat = EndStat {
                cpu: stat.cpu as i64,
                value: this_cpu_end_stat_value * stat.named_stat.scale as f64,
            };
            metadata.update_limits(GraphLimitType::F64(this_cpu_end_stat.value));
            end_cpu_stats.push(this_cpu_end_stat);
            aggregate_nr += stat.named_stat.nr_value;
            aggregate_dr += stat.named_stat.dr_value;
        }
        aggregate_value =
            (aggregate_nr as f64 / aggregate_dr as f64) * stats[0].named_stat.scale as f64;
        metadata.update_limits(GraphLimitType::F64(aggregate_value));
        let aggr_cpu_stat = EndStat {
            cpu: -1,
            value: aggregate_value,
        };
        end_cpu_stats.push(aggr_cpu_stat);

        end_stats.time = stats[0].named_stat.time - *time_zero;
        end_stats.cpus = end_cpu_stats;
        end_values.push(end_stats);
    }
    let perf_data = EndPerfData {
        data: end_values,
        metadata,
    };
    Ok(serde_json::to_string(&perf_data)?)
}

fn get_named_events(value: PerfStat) -> Result<String> {
    let mut evt_names = Vec::new();
    let named_stats = &value.perf_stats[0].named_stats;
    for stat in named_stats {
        evt_names.push(stat.name.clone());
    }
    Ok(serde_json::to_string(&evt_names)?)
}

impl GetData for PerfStat {
    fn process_raw_data(&mut self, buffer: Data) -> Result<ProcessedData> {
        let mut perf_stat = PerfStat::new();
        let raw_value = match buffer {
            Data::PerfStatRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        let reader = BufReader::new(raw_value.data.as_bytes());
        for line in reader.lines() {
            let line = line?;
            let line_str: Vec<&str> = line.split(';').collect();

            // CPU and Stat name
            let mut cpu_and_name: Vec<&str> = line_str[0].split_whitespace().collect();
            let cpu = cpu_and_name[0].parse::<u64>();
            cpu_and_name.remove(0);
            let stat_name = cpu_and_name.join(" ");

            // Numerators
            let nr_split: Vec<&str> = line_str[1].split_whitespace().collect();
            let mut nr_value: u64 = 0;
            for nr in nr_split {
                nr_value += nr.parse::<u64>()?;
            }

            // Denominators
            let dr_split: Vec<&str> = line_str[2].split_whitespace().collect();
            let mut dr_value: u64 = 0;
            for dr in dr_split {
                dr_value += dr.parse::<u64>()?;
            }

            let scale: u64 = line_str[3].parse::<u64>()?;

            let named_stat = NamedStat {
                time: raw_value.time,
                name: stat_name.to_string(),
                nr_value,
                dr_value,
                scale,
            };
            perf_stat.add_named_stat(cpu?, named_stat);
        }
        let processed_data = ProcessedData::PerfStat(perf_stat);
        Ok(processed_data)
    }

    fn get_calls(&mut self) -> Result<Vec<String>> {
        Ok(vec!["keys".to_string(), "values".to_string()])
    }

    fn get_data(&mut self, buffer: Vec<ProcessedData>, query: String) -> Result<String> {
        let mut values = Vec::new();
        for data in buffer {
            match data {
                ProcessedData::PerfStat(ref value) => values.push(value.clone()),
                _ => unreachable!(),
            }
        }
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query).unwrap();
        if param.len() < 2 {
            panic!("Not enough arguments");
        }
        let (_, req_str) = &param[1];

        match req_str.as_str() {
            "keys" => get_named_events(values[0].clone()),
            "values" => {
                let (_, key) = &param[2];
                get_values(values, key.to_string())
            }
            _ => panic!("Unsupported API"),
        }
    }
}

#[ctor]
fn init_perf_stat_raw() {
    let perf_stat_raw = PerfStatRaw::new();
    let file_name = PERF_STAT_FILE_NAME.to_string();
    let dt = DataType::new(
        Data::PerfStatRaw(perf_stat_raw.clone()),
        file_name.clone(),
        false,
    );
    let js_file_name = file_name.clone() + ".js";
    let perf_stat = PerfStat::new();
    let dv = DataVisualizer::new(
        ProcessedData::PerfStat(perf_stat.clone()),
        file_name.clone(),
        js_file_name,
        include_str!(concat!(env!("JS_DIR"), "/perf_stat.js")).to_string(),
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
    use super::{PerfStat, PerfStatRaw};
    use crate::data::{CollectData, CollectorParams, Data, ProcessedData};
    use crate::visualizer::GetData;
    use std::collections::HashMap;
    use std::io::ErrorKind;

    #[test]
    fn test_collect_data() {
        let mut perf_stat = PerfStatRaw::new();
        let params = CollectorParams::new();

        match perf_stat.prepare_data_collector(params) {
            Err(e) => {
                if let Some(os_error) = e.downcast_ref::<std::io::Error>() {
                    match os_error.kind() {
                        ErrorKind::PermissionDenied => {
                            panic!("Set /proc/sys/kernel/perf_event_paranoid to 0")
                        }
                        ErrorKind::NotFound => println!("Instance does not expose Perf counters"),
                        _ => panic!("{}", os_error),
                    }
                }
            }
            Ok(_) => {
                perf_stat.collect_data().unwrap();
                assert!(!perf_stat.data.is_empty());
            }
        }
    }

    #[test]
    fn test_get_named_events() {
        let mut perf_stat = PerfStatRaw::new();
        let mut buffer: Vec<Data> = Vec::<Data>::new();
        let mut processed_buffer: Vec<ProcessedData> = Vec::new();
        let params = CollectorParams::new();

        match perf_stat.prepare_data_collector(params) {
            Err(e) => {
                if let Some(os_error) = e.downcast_ref::<std::io::Error>() {
                    match os_error.kind() {
                        ErrorKind::PermissionDenied => {
                            panic!("Set /proc/sys/kernel/perf_event_paranoid to 0")
                        }
                        ErrorKind::NotFound => println!("Instance does not expose Perf counters"),
                        _ => panic!("{}", os_error),
                    }
                }
            }
            Ok(_) => {
                perf_stat.collect_data().unwrap();
                buffer.push(Data::PerfStatRaw(perf_stat));
                for buf in buffer {
                    processed_buffer.push(PerfStat::new().process_raw_data(buf).unwrap());
                }
                let events = PerfStat::new()
                    .get_data(processed_buffer, "run=test&get=keys".to_string())
                    .unwrap();
                let values: Vec<&str> = serde_json::from_str(&events).unwrap();
                let mut key_map = HashMap::new();
                let event_names = [
                    "ipc",
                    "branch-mpki",
                    "data-l1-mpki",
                    "inst-l1-mpki",
                    "l2-mpki",
                    "l3-mpki",
                    "stall-frontend-pkc",
                    "stall-backend-pkc",
                    "inst-tlb-mpki",
                    "inst-tlb-tw-pki",
                    "data-tlb-mpki",
                    "data-tlb-tw-pki",
                    "code-sparsity",
                ];
                for name in event_names {
                    key_map.insert(name.to_string(), 0);
                }
                for event in values {
                    assert!(key_map.contains_key(&event.to_string()));
                    let value = key_map.get(&event.to_string()).unwrap() + 1;
                    key_map.insert(event.to_string(), value);
                }
                let mut key_values: Vec<u64> = key_map.into_values().collect();
                key_values.dedup();
                assert_eq!(key_values.len(), 1);
            }
        }
    }
}
