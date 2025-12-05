use crate::computations::Statistics;
use crate::data::data_formats::{AperfData, Series, TimeSeriesData, TimeSeriesMetric};
use crate::data::utils::{get_aggregate_cpu_series_name, get_cpu_series_name};
use crate::data::{CollectData, CollectorParams, Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use crate::PDError;
use anyhow::Result;
use chrono::prelude::*;
use log::{error, info, trace, warn};
use perf_event::events::{Raw, Software};
use perf_event::{Builder, Counter, Group, ReadFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Mutex;

#[cfg(target_arch = "aarch64")]
pub mod arm64_perf_list {
    pub static GRV_EVENTS: &[u8] = include_bytes!("../pmu_configs/grv_perf_list.json");
}
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86_perf_list {
    /// Intel+ events.
    pub static INTEL_EVENTS: &[u8] = include_bytes!("../pmu_configs/intel_perf_list.json");
    pub static ICX_CTRS: &[u8] = include_bytes!("../pmu_configs/intel_icelake_ctrs.json");
    pub static SPR_CTRS: &[u8] = include_bytes!("../pmu_configs/intel_sapphire_rapids_ctrs.json");

    /// AMD+ events.
    pub static AMD_EVENTS: &[u8] = include_bytes!("../pmu_configs/amd_perf_list.json");
    pub static GENOA_CTRS: &[u8] = include_bytes!("../pmu_configs/amd_genoa_ctrs.json");
    pub static MILAN_CTRS: &[u8] = include_bytes!("../pmu_configs/amd_milan_ctrs.json");
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NamedCtr {
    pub name: String,
    pub nrs: Vec<NamedTypeCtr>,
    pub drs: Vec<NamedTypeCtr>,
    pub scale: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NamedTypeCtr {
    pub perf_type: PerfType,
    pub name: String,
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
    pub fn new() -> Self {
        PerfStatRaw {
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

pub fn form_events_map(base: &[u8], plat_counters: &[u8]) -> Result<Vec<NamedCtr>> {
    let mut events_map = indexmap::IndexMap::new();
    for event in &to_events(base)? {
        events_map.insert(event.name.clone(), event.clone());
    }

    if plat_counters != [0; 1] {
        for event in to_events(plat_counters)? {
            if let Some(ctr) = events_map.get_mut(&event.name) {
                ctr.nrs = event.nrs;
                ctr.drs = event.drs;
                ctr.scale = event.scale;
            } else {
                events_map.insert(event.name.clone(), event);
            }
        }
    }
    Ok(events_map.into_values().collect())
}

pub fn to_events(data: &[u8]) -> Result<Vec<NamedCtr>> {
    Ok(serde_json::from_slice(data)?)
}

impl CollectData for PerfStatRaw {
    fn prepare_data_collector(&mut self, params: &CollectorParams) -> Result<()> {
        let num_cpus = match unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN as libc::c_int) } {
            -1 => {
                warn!("Could not get the number of cpus in the system with sysconf.");
                return Err(PDError::CollectorPMUCPUError.into());
            }
            ret => ret as usize,
        };
        let mut cpu_groups: Vec<CpuCtrGroup> = Vec::new();

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "aarch64")] {
                let mut perf_list = to_events(arm64_perf_list::GRV_EVENTS)?;
            } else if #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] {
                let cpu_info = crate::data::utils::get_cpu_info()?;
                let platform_specific_counter: &[u8];
                let base: &[u8];

                /* Get Vendor Specific Perf events */
                if cpu_info.vendor == "GenuineIntel" {
                    base = x86_perf_list::INTEL_EVENTS;

                    /* Get Model specific events */
                    platform_specific_counter = match cpu_info.model_name.as_str() {
                        "Intel(R) Xeon(R) Platinum 8375C CPU @ 2.90GHz" => x86_perf_list::ICX_CTRS,
                        "Intel(R) Xeon(R) Platinum 8488C" => x86_perf_list::SPR_CTRS,
                        _ => &[0; 1],
                    };
                } else if cpu_info.vendor == "AuthenticAMD" {
                    warn!("Event multiplexing may result in bad PMU data."); //TODO: mitigate bad PMU data on AMD instances
                    base = x86_perf_list::AMD_EVENTS;

                    /* Get Model specific events */
                    platform_specific_counter = match cpu_info.model_name.get(..13).unwrap_or_default() {
                        "AMD EPYC 9R14" => x86_perf_list::GENOA_CTRS,
                        "AMD EPYC 7R13" => x86_perf_list::MILAN_CTRS,
                        _ => &[0; 1],
                    };
                } else {
                    return Err(PDError::CollectorPerfUnsupportedCPU.into());
                }

                let mut perf_list = form_events_map(base, platform_specific_counter)?;
            } else {
                return Err(PDError::CollectorPerfUnsupportedCPU.into());
            }
        }
        if let Some(custom_file) = &params.pmu_config {
            let f = File::open(custom_file)?;
            let user_provided_list: Result<Vec<NamedCtr>, serde_json::Error> =
                serde_json::from_reader(&f);
            match user_provided_list {
                Ok(ul) => {
                    info!("Using custom PMU configuration provided by user.");
                    perf_list = ul;
                }
                Err(_) => {
                    error!("User provided PMU configuration is invalid. Aperf exiting...");
                    std::process::exit(1);
                }
            }
        }
        /* Write the pmu_config being used to the recorded data */
        let perf_list_pathbuf = PathBuf::from(&params.data_dir).join("pmu_config.json");
        let f = File::create(&perf_list_pathbuf)?;
        serde_json::to_writer_pretty(f, &perf_list)?;
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
                            ErrorKind::NotFound => warn!("PMU counters not available on this instance type. Refer to APerf documentation for supported instances"),
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
                                        warn!("PMU counters not available on this instance type. Refer to APerf documentation for supported instances")
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
                                        warn!("PMU counters not available on this instance type. Refer to APerf documentation for supported instances")
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

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
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
pub struct PerfStat;

impl PerfStat {
    pub fn new() -> Self {
        PerfStat
    }
}

/// Parse the single-line raw PMU stat collected during APerf record into
/// (cpu number, stat name, numerator, denominator, scale)
fn parse_raw_pmu_stat(raw_pmu_stat: &str) -> Result<(usize, String, f64, f64, f64), String> {
    let mut raw_items = raw_pmu_stat.split(";");

    let header = raw_items
        .next()
        .ok_or(format!("Missing header in raw PMU stat: {raw_pmu_stat}"))?;
    let mut header_parts = header.trim().split_whitespace();
    let cpu = header_parts
        .next()
        .ok_or(format!(
            "Missing CPU in raw PMU stat header: {raw_pmu_stat}"
        ))?
        .parse::<usize>()
        .map_err(|_| format!("Invalid CPU number in raw PMU stat header: {raw_pmu_stat}"))?;
    let pmu_stat_name = header_parts.next().ok_or(format!(
        "Missing PMU stat name in raw PMU stat header: {raw_pmu_stat}"
    ))?;

    let numerator_sum = raw_items
        .next()
        .ok_or(format!(
            "Missing numerators in raw PMU stat header: {raw_pmu_stat}"
        ))?
        .split_whitespace()
        .try_fold(0u64, |acc, nr| {
            nr.parse::<u64>()
                .map(|nr_num| acc.checked_add(nr_num).unwrap_or(acc))
                .map_err(|_| format!("Invalid numerator in raw PMU stat header: {raw_pmu_stat}"))
        })?;
    let denominator_sum = raw_items
        .next()
        .ok_or(format!(
            "Missing denominator in raw PMU stat header: {raw_pmu_stat}"
        ))?
        .split_whitespace()
        .try_fold(0u64, |acc, dr| {
            dr.parse::<u64>()
                .map(|nr_num| acc.checked_add(nr_num).unwrap_or(acc))
                .map_err(|_| format!("Invalid denominator in raw PMU stat header: {raw_pmu_stat}"))
        })?;

    let scale = raw_items
        .next()
        .ok_or(format!(
            "Missing scale in raw PMU stat header: {raw_pmu_stat}"
        ))?
        .parse::<u64>()
        .map_err(|_| format!("Invalid scale in raw PMU stat header: {raw_pmu_stat}"))?;

    Ok((
        cpu,
        pmu_stat_name.to_string(),
        numerator_sum as f64,
        denominator_sum as f64,
        scale as f64,
    ))
}

impl ProcessData for PerfStat {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data = TimeSeriesData::default();
        // the aggregate series to be inserted into all PMU stat metrics
        let mut per_pmu_stat_aggregate_series: HashMap<String, Series> = HashMap::new();

        // initial time used to compute time diff for every series data point
        let mut time_zero: Option<TimeEnum> = None;
        // Keep track of the largest series value for each metric to compute its value range
        let mut per_pmu_stat_min_value: HashMap<String, f64> = HashMap::new();
        // Keep track of the least series value for each metric to compute its value range
        let mut per_pmu_stat_max_value: HashMap<String, f64> = HashMap::new();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::PerfStatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };

            let time_diff: u64 = match raw_value.time - *time_zero.get_or_insert(raw_value.time) {
                TimeEnum::TimeDiff(_time_diff) => _time_diff,
                TimeEnum::DateTime(_) => panic!("Unexpected TimeEnum diff"),
            };

            // To count the sum of every PMU stat's numerator and denominator across all CPUs,
            // for the computation of the aggregate PMU stats, which is
            // <numerator sum> / <denominator sum> * scale
            let mut per_pmu_stat_numerator_sums: HashMap<String, f64> = HashMap::new();
            let mut per_pmu_stat_denominator_sums: HashMap<String, f64> = HashMap::new();

            for raw_pmu_stat in raw_value.data.lines() {
                let (cpu, pmu_stat_name, numerator, denominator, scale) =
                    match parse_raw_pmu_stat(raw_pmu_stat) {
                        Ok(parsed_pmu_stat) => parsed_pmu_stat,
                        Err(message) => {
                            error!("{}", message);
                            continue;
                        }
                    };
                if denominator == 0.0 {
                    continue;
                }
                let pmu_stat_value = numerator / denominator * scale;

                // For the computation of aggregate PMU stats
                per_pmu_stat_numerator_sums
                    .entry(pmu_stat_name.clone())
                    .and_modify(|numerator_sum| *numerator_sum += numerator * scale)
                    .or_insert(numerator * scale);
                per_pmu_stat_denominator_sums
                    .entry(pmu_stat_name.clone())
                    .and_modify(|denominator_sum| *denominator_sum += denominator)
                    .or_insert(denominator);
                // Update min and max series values
                per_pmu_stat_min_value
                    .entry(pmu_stat_name.clone())
                    .and_modify(|min_value| *min_value = (*min_value).min(pmu_stat_value))
                    .or_insert(pmu_stat_value);
                per_pmu_stat_max_value
                    .entry(pmu_stat_name.clone())
                    .and_modify(|max_value| *max_value = (*max_value).max(pmu_stat_value))
                    .or_insert(pmu_stat_value);

                let pmu_stat_metric = time_series_data
                    .metrics
                    .entry(pmu_stat_name.clone())
                    .or_insert(TimeSeriesMetric::new(pmu_stat_name.clone()));

                while cpu >= pmu_stat_metric.series.len() {
                    pmu_stat_metric
                        .series
                        .push(Series::new(get_cpu_series_name(cpu)));
                }
                let cpu_series = &mut pmu_stat_metric.series[cpu];
                cpu_series.time_diff.push(time_diff);
                cpu_series.values.push(pmu_stat_value);
            }

            // Insert average values into aggregate series
            for (pmu_stat_name, numerator_sum) in per_pmu_stat_numerator_sums {
                let denominator_sum = match per_pmu_stat_denominator_sums.get(&pmu_stat_name) {
                    Some(denominator_sum) => *denominator_sum,
                    None => continue,
                };
                let aggregate_series = per_pmu_stat_aggregate_series
                    .entry(pmu_stat_name)
                    .or_insert(Series::new(get_aggregate_cpu_series_name()));
                aggregate_series.time_diff.push(time_diff);
                aggregate_series
                    .values
                    .push(numerator_sum / denominator_sum);
            }
        }

        // Compute the stats of every aggregate series and add them to the corresponding metric;
        // also set every metric's value range
        for (pmu_stat_name, pmu_stat_metric) in &mut time_series_data.metrics {
            if let Some(aggregate_series) = per_pmu_stat_aggregate_series.get_mut(pmu_stat_name) {
                let aggregate_stats = Statistics::from_values(&aggregate_series.values);
                pmu_stat_metric.value_range = (
                    per_pmu_stat_min_value
                        .get(pmu_stat_name)
                        .unwrap_or(&aggregate_stats.min)
                        .floor() as u64,
                    per_pmu_stat_max_value
                        .get(pmu_stat_name)
                        .unwrap_or(&aggregate_stats.max)
                        .ceil() as u64,
                );
                pmu_stat_metric.stats = aggregate_stats;
                aggregate_series.is_aggregate = true;
                pmu_stat_metric.series.push(aggregate_series.clone());
            }
        }
        // The metric order is defined by top down debug method https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md#how-to-collect-pmu-counters
        let top_down_order = [
            "ipc",
            "stall-frontend-pkc",
            "stall-backend-pkc",
            "branch-mpki",
            "inst-l1-mpki",
            "inst-tlb-mpki",
            "inst-tlb-tw-pki",
            "inst-tlb-tw-mpki",
            "code-sparsity",
            "data-l1-mpki",
            "l2-mpki",
            "l3-mpki",
            "data-tlb-mpki",
            "data-tlb-tw-pki",
            "data-st-tlb-mpki",
            "data-st-tlb-tw-pki",
            "data-rd-tlb-mpki",
            "data-rd-tlb-tw-pki",
        ];

        let mut pmu_stat_names: Vec<String> = time_series_data.metrics.keys().cloned().collect();
        pmu_stat_names.sort_by_key(|name| {
            top_down_order
                .iter()
                .position(|&order_name| order_name == name)
                .unwrap_or(top_down_order.len())
        });
        time_series_data.sorted_metric_names = pmu_stat_names;

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    use super::PerfStatRaw;
    use crate::data::{CollectData, CollectorParams};
    use std::io::ErrorKind;

    #[test]
    fn test_collect_data() {
        let mut perf_stat = PerfStatRaw::new();
        let params = CollectorParams::new();

        match perf_stat.prepare_data_collector(&params) {
            Err(e) => {
                if let Some(os_error) = e.downcast_ref::<std::io::Error>() {
                    match os_error.kind() {
                        ErrorKind::PermissionDenied => {
                            panic!("Set /proc/sys/kernel/perf_event_paranoid to 0")
                        }
                        ErrorKind::NotFound => println!("PMU counters not available on this instance type. Refer to APerf documentation for supported instances"),
                        _ => panic!("{}", os_error),
                    }
                }
            }
            Ok(_) => {
                perf_stat.collect_data(&params).unwrap();
                assert!(!perf_stat.data.is_empty());
            }
        }
    }
}
