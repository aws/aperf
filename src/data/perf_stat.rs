use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::{
    time_series_data_processor_with_custom_aggregate, TimeSeriesDataProcessor,
};
use crate::data::common::utils::{get_aggregate_series_name, get_cpu_series_name};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::data_processing::ReportParams;
use crate::UNGROUPED_PMU_MODE;
use anyhow::{Context, Result};
use exmex::{Express, FlatEx};
use include_dir::{include_dir, Dir};
use indexmap::IndexMap;
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Write};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use {
    crate::data::common::utils::{get_online_cpu_ids, raise_fd_limit},
    crate::data::CollectData,
    crate::data_collection::InitParams,
    crate::CPU_INFO,
    anyhow::{anyhow, bail},
    chrono::prelude::*,
    log::info,
    std::{iter, thread, time::Duration},
};

static DEFAULT_PMU_CONFIG_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/pmu_configs");

/// Help build a PMU counter or group with unified error handling logic.
#[cfg(target_os = "linux")]
macro_rules! build_pmu_counter {
    ($build:expr, $on_unsupported:expr) => {
        match $build {
            Ok(counter) => counter,
            Err(e) => match e.raw_os_error() {
                Some(libc::EACCES) | Some(libc::EPERM) => {
                    error!("kernel.perf_event_paranoid is not <=0. Run `sudo sysctl -w kernel.perf_event_paranoid=-1`");
                    return Err(e.into());
                }
                Some(libc::ENOENT) | Some(libc::ENODEV) | Some(libc::EOPNOTSUPP) => $on_unsupported,
                _ => bail!("Failed to create PMU counter: {:?}", e),
            },
        }
    };
}

/// Get the path to PMU config saved in the run dir.
fn get_saved_pmu_config_path(run_dir: &Path) -> PathBuf {
    PathBuf::from(run_dir).join("pmu_config.json")
}

/// Warn user if the number of counters to be collected per CPU is more than the
/// available registers, which means multiplexing is likely required ("likely" because
/// some counters do not consume a register, and the probing result is dynamic).
fn warn_multiplexing(required_counters_per_cpu: usize, counter_limit: usize) {
    if required_counters_per_cpu > counter_limit {
        // TODO: before we come up with a solution so that the default config
        // does not always trigger the warning, keep the message at debug level.
        debug!("At most {required_counters_per_cpu} counters will be collected per CPU, while {counter_limit} PMU registers are currently available. Multiplexing is likely required, in which case the accuracy of the PMU metrics is reduced and more CPU resources are consumed during collection.");
    }
}

/// Custom event type used to build the counter. It essentially duplicates perf-event2's
/// DynamicBuilder and Event::Dynamic, which, unfortunately, fails to parse a config
/// with multiple non-contiguous bit ranges (the format used on AMD). The implementations
/// below fix the issue.
#[cfg(target_os = "linux")]
struct PmuConfigEvent {
    pmu_type: u32,
    /// A format file could have config/config1/config2 mapping to
    /// 3 different 64-bit fields.
    config: [u64; 3],
}

#[cfg(target_os = "linux")]
impl perf_event::events::Event for PmuConfigEvent {
    fn update_attrs(self, attr: &mut perf_event_open_sys::bindings::perf_event_attr) {
        attr.type_ = self.pmu_type;
        attr.config = self.config[0];
        attr.__bindgen_anon_3.config1 = self.config[1];
        attr.__bindgen_anon_4.config2 = self.config[2];
    }
}

#[cfg(target_os = "linux")]
impl PmuConfigEvent {
    /// Parse "pmu/field=val,field=val,.../" and build the event by reading the
    /// PMU type and building the config bit map from each field's config.
    fn from_event_string(event_string: &str) -> Result<Self> {
        let (pmu_name, fields_str) = event_string
            .split_once('/')
            .ok_or_else(|| anyhow!("Missing '/' in the PMU event string {event_string}"))?;
        // Drop trailing '/'
        let fields_str = fields_str.strip_suffix('/').unwrap_or(fields_str);
        // Parse the fields string into Vec<(field_key, field_value)>.
        let mut fields = Vec::new();
        for field_str in fields_str.split(',') {
            let field_str = field_str.trim();
            if field_str.is_empty() {
                continue;
            }
            let (field_key, field_val_str) = field_str
                .split_once('=')
                .ok_or_else(|| anyhow!("Invalid PMU event string: {event_string}"))?;
            // values are hex (0x..) or decimal
            let field_val = field_val_str.trim();
            let field_val = if let Some(hex) = field_val.strip_prefix("0x") {
                u64::from_str_radix(hex, 16)?
            } else {
                field_val.parse::<u64>()?
            };
            fields.push((field_key.trim().to_string(), field_val));
        }

        let pmu_device_path = PathBuf::from("/sys/bus/event_source/devices").join(pmu_name);
        let pmu_type = fs::read_to_string(pmu_device_path.join("type"))?
            .trim()
            .parse::<u32>()?;

        // For each field=value, the layout of the 3 64-bit fields (config, config1, and config2)
        // comes from the kernel's sysfs format file /sys/bus/event_source/devices/<pmu>/format/<field>,
        // e.g. confi1g:0-7, config2:1-8, or AMD's non-contiguous config:0-7,32-35.
        let mut config = [0u64; 3];
        for (field, value) in fields {
            let mut shift = 0;

            let format_str = fs::read_to_string(pmu_device_path.join("format").join(&field))?;
            let (config_idx, ranges_str) =
                if let Some(ranges_str) = format_str.strip_prefix("config:") {
                    (0, ranges_str)
                } else if let Some(ranges_str) = format_str.strip_prefix("config1:") {
                    (1, ranges_str)
                } else if let Some(ranges_str) = format_str.strip_prefix("config2:") {
                    (2, ranges_str)
                } else {
                    bail!("Unexpected PMU format {format_str} for field {field}");
                };

            for range_str in ranges_str.trim().split(',') {
                let (low, high) = match range_str.split_once('-') {
                    Some((low_str, high_str)) => {
                        (low_str.parse::<u32>()?, high_str.parse::<u32>()?)
                    }
                    None => (range_str.parse::<u32>()?, range_str.parse::<u32>()?),
                };
                let width = high - low + 1;
                let mask = (1u64 << width) - 1;
                config[config_idx] |= ((value >> shift) & mask) << low;
                shift += width;
            }
        }

        Ok(Self { pmu_type, config })
    }
}

/// Maps the format of the PMU config, for simpler deserialization/parsing of the config file.
#[derive(Deserialize, Serialize)]
struct PmuConfig {
    pub events: IndexMap<String, String>,
    pub metrics: IndexMap<String, String>,
}

impl PmuConfig {
    /// Parse the PMU config JSON located at the path.
    pub fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    /// Save to a PMU config JSON file at the path.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let file = fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &self)
            .with_context(|| format!("Failed to save PMU config to {}", path.display()))?;
        Ok(())
    }

    /// Parse the byte content of a PMU config JSON.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes).context("Failed to parse PMU config")?)
    }

    /// Parse one of the default PMU config JSONs.
    #[cfg(target_os = "linux")]
    pub fn from_default(name: &str) -> Result<Self> {
        let bytes = DEFAULT_PMU_CONFIG_DIR
            .get_file(name)
            .with_context(|| format!("Failed to read default PMU config file content for {name}"))?
            .contents();
        Self::from_bytes(&bytes)
    }

    /// Extend the current config by another default PMU config JSON.
    #[cfg(target_os = "linux")]
    pub fn extend_from_default(&mut self, name: &str) -> Result<()> {
        let extent_pmu_config = Self::from_default(name)?;
        self.events.extend(extent_pmu_config.events);
        self.metrics.extend(extent_pmu_config.metrics);
        Ok(())
    }

    /// Grouped mode: for each defined metric, create a counter group that contains all the
    /// event counters used by the metric definition. Grouped counters are scheduled together
    /// on CPU, so the metric computation is guaranteed to be correct. However, the same event
    /// across different groups creates one counter per group. Therfore, this mode puts more
    /// loads on the collection multiplexing, and each event is collected for less time.
    #[cfg(target_os = "linux")]
    pub fn create_metric_counter_groups(&self) -> Result<Vec<PmuCollector>> {
        let online_cpu_ids = get_online_cpu_ids()?;
        let metric_expressions = self.get_metric_expressions()?;

        let num_counters_per_cpu = metric_expressions
            .values()
            .map(|metric_expression| metric_expression.var_names().len())
            .sum::<usize>();

        match Self::probe_pmu_counter_limit(online_cpu_ids.last().copied().unwrap()) {
            Ok(pmu_counter_limit) => {
                warn_multiplexing(num_counters_per_cpu, pmu_counter_limit);
            }
            Err(e) => {
                warn!("Failed to probe PMU counter limit: {:?}", e);
            }
        };

        // Each group (one for every metric) also takes one fd.
        // Add some buffers to the expected fd requirement.
        let num_required_fds =
            50 + online_cpu_ids.len() * (num_counters_per_cpu + metric_expressions.len());
        debug!(
            "Require {num_required_fds} fds for the collection of {} PMU metrics over {} CPUs.",
            metric_expressions.len(),
            online_cpu_ids.len()
        );
        raise_fd_limit(num_required_fds as u64)?;

        let mut metric_counter_groups = Vec::new();

        // For each defined metric on each online CPU, create a group that contains all the PMU event counters
        // used by the metric.
        'outer: for (metric_name, metric_expression) in metric_expressions {
            // The event names are in alphabetical order, and their values need to be passed to the
            // expression in the exactly same order for evaluation. Therefore, the order needs to
            // be maintained between collection and report generation.
            let event_names = metric_expression.var_names();
            let mut event_strings: Vec<&str> = Vec::new();
            for event_name in event_names {
                match self.events.get(event_name) {
                    Some(event_string) => event_strings.push(event_string),
                    None => {
                        error!(
                            "Skipping metric {metric_name} due to unrecognized event name {event_name}"
                        );
                        continue 'outer;
                    }
                };
            }

            for &cpu_id in &online_cpu_ids {
                let pmu_metric_counter_group =
                    match Self::create_counter_group(&metric_name, &event_strings, cpu_id)? {
                        Some(pmu_metric_counter_group) => pmu_metric_counter_group,
                        None => continue 'outer,
                    };
                metric_counter_groups.push(PmuCollector::Grouped(pmu_metric_counter_group));
            }
        }

        Ok(metric_counter_groups)
    }

    /// Ungrouped mode: create a counter for each unique event, without any groups. All counters
    /// time-share the PMU registers for collection. This mode puts less load on multiplexing,
    /// and each counter is collected for a longer time, but the counters used to compute a
    /// metric value are not guaranteed to be collected at the same time, unless all counters
    /// can fit in available PMU registers (typically 4-8 depeding on the CPU type).
    #[cfg(target_os = "linux")]
    pub fn create_event_counters(&self) -> Result<Vec<PmuCollector>> {
        let online_cpu_ids = get_online_cpu_ids()?;

        let num_counters_per_cpu = self.events.len();
        match Self::probe_pmu_counter_limit(online_cpu_ids.last().copied().unwrap()) {
            Ok(pmu_counter_limit) => {
                warn_multiplexing(num_counters_per_cpu, pmu_counter_limit);
            }
            Err(e) => {
                warn!("Failed to probe PMU counter limit: {:?}", e);
            }
        };

        // Add some buffers to the expected fd requirement.
        let num_required_fds = 50 + online_cpu_ids.len() * num_counters_per_cpu;
        debug!(
            "Require {num_required_fds} fds for the collection of {} PMU eventd over {} CPUs.",
            self.events.len(),
            online_cpu_ids.len()
        );
        raise_fd_limit(num_required_fds as u64)?;

        let mut event_counters = Vec::new();

        'outer: for (event_name, event_string) in &self.events {
            for &cpu_id in &online_cpu_ids {
                let event = PmuConfigEvent::from_event_string(event_string).with_context(|| {
                    format!("Failed to create event {event_name} from definition {event_string}")
                })?;
                let counter = build_pmu_counter!(
                    perf_event::Builder::new(event)
                        .read_format(
                            perf_event::ReadFormat::TOTAL_TIME_ENABLED
                                | perf_event::ReadFormat::TOTAL_TIME_RUNNING,
                        )
                        .one_cpu(cpu_id)
                        .any_pid()
                        .include_kernel()
                        .build(),
                    {
                        warn!("Skipping PMU event {event_name} as it is not supported.");
                        continue 'outer;
                    }
                );

                event_counters.push(PmuCollector::Ungrouped(PmuEventCounter {
                    cpu_id,
                    event_name: event_name.clone(),
                    counter,
                }));
            }
        }

        Ok(event_counters)
    }

    const PMU_COUNTER_LIMIT_UPPER_BOUND: usize = 16;
    const PMU_COUNTER_LIMIT_LOWER_BOUND: usize = 1;
    /// Probe the number of available PMU registers, i.e. the limit of counter collection
    /// without multiplexing, through binary saerch.
    #[cfg(target_os = "linux")]
    pub fn probe_pmu_counter_limit(cpu_id: usize) -> Result<usize> {
        let cpu_info = match &*CPU_INFO {
            Ok(cpu_info) => cpu_info,
            Err(e) => bail!(
                "Failed to obtain CPU info to select the probing PMU event: {:?}",
                e
            ),
        };

        let probe_event = if cpu_info.is_graviton() {
            "armv8_pmuv3_0/event=0x3/"
        } else if cpu_info.is_intel() {
            "cpu/event=0x51,umask=0x1/"
        } else if cpu_info.is_amd() {
            "cpu/event=0xc2,umask=0x0/"
        } else {
            bail!(
                "Unrecognized CPU type for probe event selection: {:?}",
                cpu_info
            );
        };

        let (mut high, mut low) = (
            Self::PMU_COUNTER_LIMIT_UPPER_BOUND,
            Self::PMU_COUNTER_LIMIT_LOWER_BOUND,
        );
        while low < high {
            let mid = (low + high + 1) / 2;

            let event_strings: Vec<&str> = iter::repeat_n(probe_event, mid).collect();

            let mut probe_group =
                match Self::create_counter_group("pmu_counter_limit_probe", &event_strings, cpu_id)
                {
                    Ok(Some(probe_group)) => probe_group,
                    Ok(None) => bail!("unsupported probe event"),
                    // On X86 a group that is larger than the number of available registers
                    // will fail to be created with EINVAL.
                    Err(_) => {
                        high = mid - 1;
                        continue;
                    }
                };

            probe_group.enable()?;
            thread::sleep(Duration::from_millis(10));
            let probed_data = probe_group.collect()?;

            // The group being fully scheduled means no multiplexings
            if probed_data.time_running >= probed_data.time_enabled {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        let counter_limit = low;

        Ok(counter_limit)
    }

    /// Parse each PMU metric expression into exmes object, to retrieve the identifiers
    /// or to evaluate the expression with identifier values.
    pub fn get_metric_expressions(&self) -> Result<HashMap<String, exmex::FlatEx<f64>>> {
        let mut metric_expressions: HashMap<String, exmex::FlatEx<f64>> = HashMap::new();
        for (metric_name, metric_expression_str) in &self.metrics {
            metric_expressions.insert(
                metric_name.clone(),
                exmex::parse::<f64>(metric_expression_str).with_context(|| {
                    format!("Failed to parse metric expression for {metric_name} = {metric_expression_str}")
                })?,
            );
        }
        Ok(metric_expressions)
    }

    /// Helper function to create a group containing counters built from
    /// each event string.
    #[cfg(target_os = "linux")]
    fn create_counter_group(
        metric_name: &str,
        event_strings: &Vec<&str>,
        cpu_id: usize,
    ) -> Result<Option<PmuMetricCounterGroup>> {
        let mut group = build_pmu_counter!(
            perf_event::Builder::new(perf_event::events::Software::DUMMY)
                .read_format(
                    perf_event::ReadFormat::GROUP
                        | perf_event::ReadFormat::TOTAL_TIME_ENABLED
                        | perf_event::ReadFormat::TOTAL_TIME_RUNNING
                        | perf_event::ReadFormat::ID,
                )
                .one_cpu(cpu_id)
                .any_pid()
                .build_group(),
            unreachable!("Group leader is a software event and cannot be unsupported")
        );
        let mut counters = Vec::new();

        for event_string in event_strings {
            let event = match PmuConfigEvent::from_event_string(event_string) {
                Ok(event) => event,
                Err(e) => {
                    warn!(
                        "Failed to create event {event_string} in metric {metric_name}: {:?}",
                        e
                    );
                    return Ok(None);
                }
            };

            let counter = build_pmu_counter!(
                perf_event::Builder::new(event)
                    .one_cpu(cpu_id)
                    .any_pid()
                    .include_kernel()
                    .build_with_group(&mut group),
                {
                    warn!("PMU event {event_string} in metric {metric_name} is not supported.");
                    return Ok(None);
                }
            );
            counters.push(counter);
        }

        Ok(Some(PmuMetricCounterGroup {
            cpu_id: cpu_id,
            metric_name: metric_name.to_string(),
            counters,
            group,
        }))
    }
}

/// Provides unified interface for grouped and ungrouped collectors.
#[cfg(target_os = "linux")]
pub enum PmuCollector {
    Grouped(PmuMetricCounterGroup),
    Ungrouped(PmuEventCounter),
}

#[cfg(target_os = "linux")]
impl PmuCollector {
    pub fn enable(&mut self) -> Result<()> {
        match self {
            PmuCollector::Grouped(pmu_metric_counter_group) => pmu_metric_counter_group.enable(),
            PmuCollector::Ungrouped(pmu_event_counter) => pmu_event_counter.enable(),
        }
    }

    pub fn collect(&mut self) -> Result<PmuCollectedData> {
        match self {
            PmuCollector::Grouped(pmu_metric_counter_group) => pmu_metric_counter_group.collect(),
            PmuCollector::Ungrouped(pmu_event_counter) => pmu_event_counter.collect(),
        }
    }
}

/// Grouped mode: contains all PMU event counters used by a metric, which are placed
/// in the same group so that they will be scheduled by the CPU atomically. The struct
/// is used for the collection of the grouped event counter values.
#[cfg(target_os = "linux")]
pub struct PmuMetricCounterGroup {
    pub cpu_id: usize,
    pub metric_name: String,
    pub counters: Vec<perf_event::Counter>,
    pub group: perf_event::Group,
}

#[cfg(target_os = "linux")]
impl PmuMetricCounterGroup {
    /// Enable the group for collection.
    pub fn enable(&mut self) -> Result<()> {
        self.group.reset().with_context(|| {
            format!(
                "Failed to reset PMU counter group for metric {} on CPU {}",
                self.metric_name, self.cpu_id
            )
        })?;
        self.group.enable().with_context(|| {
            format!(
                "Failed to enable PMU counter group for metric {} on CPU {}",
                self.metric_name, self.cpu_id
            )
        })?;
        Ok(())
    }

    /// Collect the value of all counters within this group.
    pub fn collect(&mut self) -> Result<PmuCollectedData> {
        let group_data = self.group.read().with_context(|| {
            format!(
                "Failed to read PMU counter group for metric {} on CPU {}",
                self.metric_name, self.cpu_id
            )
        })?;

        let mut counter_values: Vec<u64> = Vec::with_capacity(self.counters.len());
        for counter in &self.counters {
            counter_values.push(group_data[counter]);
        }
        let time_enabled = group_data
            .time_enabled()
            .with_context(|| {
                format!(
                    "Failed to read time_enabled for metric {} on CPU {}",
                    self.metric_name, self.cpu_id
                )
            })?
            .as_secs_f64();
        let time_running = group_data
            .time_running()
            .with_context(|| {
                format!(
                    "Failed to read time_running for metric {} on CPU {}",
                    self.metric_name, self.cpu_id
                )
            })?
            .as_secs_f64();

        self.group.reset()?;

        Ok(PmuCollectedData {
            cpu_id: self.cpu_id,
            identifier: self.metric_name.clone(),
            counter_values,
            time_enabled,
            time_running,
        })
    }
}

/// Ungrouped mode: contains the PMU counter of a defined event. The struct is used
/// for the collection of the event counter value.
#[cfg(target_os = "linux")]
pub struct PmuEventCounter {
    pub cpu_id: usize,
    pub event_name: String,
    pub counter: perf_event::Counter,
}

#[cfg(target_os = "linux")]
impl PmuEventCounter {
    /// Enable the counter for collection.
    pub fn enable(&mut self) -> Result<()> {
        self.counter.reset().with_context(|| {
            format!(
                "Failed to reset PMU counter for event {} on CPU {}",
                self.event_name, self.cpu_id
            )
        })?;
        self.counter.enable().with_context(|| {
            format!(
                "Failed to enable PMU counter for event {} on CPU {}",
                self.event_name, self.cpu_id
            )
        })?;
        Ok(())
    }

    /// Collect the value of the event counter.
    pub fn collect(&mut self) -> Result<PmuCollectedData> {
        let counter_data = self.counter.read_full().with_context(|| {
            format!(
                "Failed to read PMU counter for event {} on CPU {}",
                self.event_name, self.cpu_id
            )
        })?;

        let counter_value = counter_data.count();
        let time_enabled = counter_data
            .time_enabled()
            .with_context(|| {
                format!(
                    "Failed to read time_enabled for event {} on CPU {}",
                    self.event_name, self.cpu_id
                )
            })?
            .as_secs_f64();
        let time_running = counter_data
            .time_running()
            .with_context(|| {
                format!(
                    "Failed to read time_running for event {} on CPU {}",
                    self.event_name, self.cpu_id
                )
            })?
            .as_secs_f64();

        self.counter.reset()?;

        Ok(PmuCollectedData {
            cpu_id: self.cpu_id,
            identifier: self.event_name.clone(),
            counter_values: vec![counter_value],
            time_enabled,
            time_running,
        })
    }
}

/// Contains the collected values of either a PmuMetricCounterGroup or PmuEventCounterData.
/// In group mode, the identifier is the metric name, and counter_values contains the
/// collected value of all counters within the group; in ungrouped mode, the identifier is
/// the event name, and counter_values contains the collected value of the one event counter.
pub struct PmuCollectedData {
    pub cpu_id: usize,
    pub identifier: String,
    pub counter_values: Vec<u64>,
    pub time_enabled: f64,
    pub time_running: f64,
}

impl PmuCollectedData {
    /// Parse (deserialize) the data from the string format as defined below.
    pub fn from_string(data_string: &str) -> Option<Self> {
        let parts: Vec<&str> = data_string.split(';').collect();
        // All fields, including at least one counter value, need to be present.
        if parts.len() < 5 {
            return None;
        }
        let cpu_id = parts[0].parse::<usize>().ok()?;
        let identifier = parts[1].to_string();
        let mut counter_values: Vec<u64> = Vec::new();
        for i in 2..(parts.len() - 2) {
            counter_values.push(parts[i].parse::<u64>().ok()?);
        }
        let time_enabled = parts[parts.len() - 2].parse::<f64>().ok()?;
        let time_running = parts[parts.len() - 1].parse::<f64>().ok()?;

        Some(Self {
            cpu_id,
            identifier,
            counter_values,
            time_enabled,
            time_running,
        })
    }

    /// Serialize the data to string in the format of:
    /// cpu_id;identifier;<counter_value_1>;<counter_value_2>;...;time_enabled;time_running
    pub fn to_string(self) -> String {
        let mut data_string = String::new();
        write!(&mut data_string, "{};{};", self.cpu_id, self.identifier).unwrap();
        self.counter_values
            .into_iter()
            .for_each(|value| write!(&mut data_string, "{value};").unwrap());
        write!(
            &mut data_string,
            "{};{}",
            self.time_enabled, self.time_running
        )
        .unwrap();

        data_string
    }
}

#[derive(Serialize, Deserialize)]
pub struct PerfStatRaw {
    #[cfg(target_os = "linux")]
    #[serde(skip)]
    pub pmu_collectors: Vec<PmuCollector>,
    pub time: TimeEnum,
    pub data: String,
}

/// Skip Debug  for pmu_metrics since they are not implemented for Counter and Group
impl Debug for PerfStatRaw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerfStatRaw")
            .field("time", &self.time)
            .field("data", &self.data)
            .finish()
    }
}

#[cfg(target_os = "linux")]
impl PerfStatRaw {
    pub fn new() -> Self {
        PerfStatRaw {
            pmu_collectors: Vec::new(),
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for PerfStatRaw {
    fn prepare_data_collector(&mut self, init_params: &InitParams) -> Result<()> {
        // Read and parse the PMU config. If user did not provide a PMU config file, use the default one
        // based on the CPU type.
        let pmu_config = if let Some(custom_pmu_config_path) = &init_params.pmu_config {
            match PmuConfig::from_file(custom_pmu_config_path) {
                Ok(custom_pmu_config) => {
                    info!(
                        "Using custom PMU configuration {}",
                        custom_pmu_config_path.display()
                    );
                    custom_pmu_config
                }
                Err(e) => {
                    error!(
                        "Custom PMU configuration {} is invalid: {:?}",
                        custom_pmu_config_path.display(),
                        e
                    );
                    std::process::exit(1);
                }
            }
        } else {
            let cpu_info = match &*CPU_INFO {
                Ok(cpu_info) => cpu_info,
                Err(e) => bail!(
                    "Failed to obtain CPU info for PMU config selection: {:?}",
                    e
                ),
            };

            if cpu_info.is_graviton() {
                let mut pmu_config = PmuConfig::from_default("grv_pmu_config.json")?;

                if cpu_info.is_graviton_5() {
                    pmu_config.extend_from_default("grv_5_pmu_config.json")?;
                }

                pmu_config
            } else if cpu_info.is_intel() {
                let mut pmu_config = PmuConfig::from_default("intel_pmu_config.json")?;

                if cpu_info.is_intel_icelake() {
                    pmu_config.extend_from_default("intel_icelake_pmu_config.json")?;
                } else if cpu_info.is_intel_sapphire_rapids() {
                    pmu_config.extend_from_default("intel_sapphire_rapids_pmu_config.json")?;
                }

                pmu_config
            } else if cpu_info.is_amd() {
                let mut pmu_config = PmuConfig::from_default("amd_pmu_config.json")?;

                if cpu_info.is_amd_genoa() {
                    pmu_config.extend_from_default("amd_genoa_pmu_config.json")?;
                } else if cpu_info.is_amd_milan() {
                    pmu_config.extend_from_default("amd_milan_pmu_config.json")?;
                }

                pmu_config
            } else {
                bail!(
                    "Unrecognized CPU type for PMU config selection: {:?}",
                    cpu_info
                );
            }
        };

        // Write the selected PMU config to the run archive.
        pmu_config.save_to_file(&get_saved_pmu_config_path(&init_params.run_data_dir))?;

        // Depending on the counter mode, either creates ungrouped event counters or per-metric
        // counter groups for collection.
        let mut pmu_collectors = if init_params.pmu_counter_mode == UNGROUPED_PMU_MODE {
            pmu_config
                .create_event_counters()
                .context("Failed to create PMU event counters")?
        } else {
            pmu_config
                .create_metric_counter_groups()
                .context("Failed to create PMU metric counter groups")?
        };

        for pmu_collector in &mut pmu_collectors {
            pmu_collector.enable()?;
        }

        self.pmu_collectors = pmu_collectors;

        Ok(())
    }

    fn collect_data(&mut self, _init_params: &InitParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = String::new();

        for pmu_metric_counter_group in &mut self.pmu_collectors {
            let group_data = pmu_metric_counter_group.collect()?;
            write!(&mut self.data, "{}\n", group_data.to_string())?;
        }

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

/// For backward compatibility, parse the single-line legacy raw PMU stat collected during APerf record into
/// (cpu number, stat name, numerator, denominator, scale)
fn parse_legacy_raw_pmu_stat(raw_pmu_stat: &str) -> Result<(usize, String, f64, f64, f64), String> {
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

/// For backward compatibility, process the raw PMU stat in the legacy (nr/dr-based) format.
fn process_legacy_raw_pmu_stat_data(
    mut time_series_data_processor: TimeSeriesDataProcessor,
    raw_data: &Vec<Data>,
) -> Result<AperfData> {
    for buffer in raw_data {
        let raw_value = match buffer {
            Data::PerfStatRaw(ref value) => value,
            _ => panic!("Invalid Data type in raw file"),
        };
        time_series_data_processor.proceed_to_time(raw_value.time);

        // To count the sum of every PMU stat's numerator and denominator across all CPUs,
        // for the computation of the aggregate PMU stats, which is
        // <numerator sum> / <denominator sum> * scale
        let mut per_pmu_stat_numerator_sums: HashMap<String, f64> = HashMap::new();
        let mut per_pmu_stat_denominator_sums: HashMap<String, f64> = HashMap::new();

        for raw_pmu_stat in raw_value.data.lines() {
            let (cpu, pmu_stat_name, numerator, denominator, scale) =
                match parse_legacy_raw_pmu_stat(raw_pmu_stat) {
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
            time_series_data_processor.add_data_point(
                &pmu_stat_name,
                &get_cpu_series_name(cpu),
                pmu_stat_value,
            );

            // For the computation of aggregate PMU stats
            per_pmu_stat_numerator_sums
                .entry(pmu_stat_name.clone())
                .and_modify(|numerator_sum| *numerator_sum += numerator * scale)
                .or_insert(numerator * scale);
            per_pmu_stat_denominator_sums
                .entry(pmu_stat_name.clone())
                .and_modify(|denominator_sum| *denominator_sum += denominator)
                .or_insert(denominator);
        }

        // Insert average values into aggregate series
        for (pmu_stat_name, numerator_sum) in per_pmu_stat_numerator_sums {
            let denominator_sum = match per_pmu_stat_denominator_sums.get(&pmu_stat_name) {
                Some(denominator_sum) => *denominator_sum,
                None => continue,
            };
            time_series_data_processor.add_aggregate_data_point(
                &pmu_stat_name,
                &get_aggregate_series_name(),
                numerator_sum / denominator_sum,
            );
        }
    }

    // The metric order is defined by top down debug method https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md#how-to-collect-pmu-counters
    let top_down_order = vec![
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
    let time_series_data =
        time_series_data_processor.get_time_series_data_with_metric_name_order(top_down_order);

    Ok(AperfData::TimeSeries(time_series_data))
}

const COUNTER_SCHEDULE_RATE_METRIC_NAME: &str = "mux_counter_schedule_rate";

/// Helper function to compute the per-CPU average counter schedule rate
/// to evaluate the level of multiplexing.
fn update_counter_schedule_rate_metric(
    time_series_data_processor: &mut TimeSeriesDataProcessor,
    per_cpu_counter_schedule_rates: HashMap<usize, Vec<f64>>,
) {
    if per_cpu_counter_schedule_rates.is_empty() {
        return;
    }

    let mut counter_schedule_rate_sum: f64 = 0.0;
    let mut num_counters: usize = 0;
    for (cpu, counter_schedule_rates) in per_cpu_counter_schedule_rates {
        let mut cur_cpu_counter_schedule_rate_sum: f64 = 0.0;
        let cur_cpu_num_counters = counter_schedule_rates.len();
        num_counters += cur_cpu_num_counters;
        for counter_schedule_rate in counter_schedule_rates {
            cur_cpu_counter_schedule_rate_sum += counter_schedule_rate;
            counter_schedule_rate_sum += counter_schedule_rate;
        }

        time_series_data_processor.add_data_point(
            COUNTER_SCHEDULE_RATE_METRIC_NAME,
            &get_cpu_series_name(cpu),
            cur_cpu_counter_schedule_rate_sum / cur_cpu_num_counters as f64 * 100.0,
        );
    }
    // Use the average of all CPU as the aggregate value of the metric.
    time_series_data_processor.add_aggregate_data_point(
        COUNTER_SCHEDULE_RATE_METRIC_NAME,
        &get_aggregate_series_name(),
        counter_schedule_rate_sum / num_counters as f64 * 100.0,
    );
}

/// Process a snapshot of raw PMU counters that were collected using groups.
fn process_single_raw_pmu_metric_counter_group_data(
    time_series_data_processor: &mut TimeSeriesDataProcessor,
    raw_data: &str,
    pmu_metric_expressions: &HashMap<String, FlatEx<f64>>,
    zero_time_running_metrics: &mut HashSet<String>,
) {
    // For every metric, store the sum of every counter used by that metric across
    // all CPUs, to compute the aggregate metric value. The order of the counter
    // sums is in the same alphabetical order.
    let mut per_metric_counter_value_sums: HashMap<String, Vec<f64>> = HashMap::new();
    // For every CPU, store the list of counter schedule rates across all groups,
    // to compute the average at the end to be used as the value for the counter
    // schedule rate metric.
    let mut per_cpu_counter_schedule_rates: HashMap<usize, Vec<f64>> = HashMap::new();

    for pmu_data_string in raw_data.lines() {
        let metric_group_data = match PmuCollectedData::from_string(pmu_data_string) {
            Some(data) => data,
            None => continue,
        };
        let metric_name = metric_group_data.identifier;
        let metric_expression = match pmu_metric_expressions.get(&metric_name) {
            Some(expression) => expression,
            None => continue,
        };
        let cpu_series_name = get_cpu_series_name(metric_group_data.cpu_id);

        // All collected data (including time_enabled and time_running) are accumulative.
        let time_enabled = match time_series_data_processor.get_delta_and_set_previous_value(
            &format!("{metric_name}_time_enabled"),
            &cpu_series_name,
            metric_group_data.time_enabled,
        ) {
            Some(time_enabled) => time_enabled,
            None => continue,
        };
        let time_running = match time_series_data_processor.get_delta_and_set_previous_value(
            &format!("{metric_name}_time_running"),
            &cpu_series_name,
            metric_group_data.time_running,
        ) {
            Some(time_running) => time_running,
            None => continue,
        };

        if time_enabled == 0.0 {
            continue;
        }
        per_cpu_counter_schedule_rates
            .entry(metric_group_data.cpu_id)
            .or_insert_with(|| Vec::new())
            .push(time_running / time_enabled);

        if time_running == 0.0 {
            zero_time_running_metrics.insert(metric_name.clone());
            continue;
        }

        // Use time_enabled and time_running to scale the value, as when more events need to be
        // collected than the hardware limitation, the PMU will multiplex (time-share) all the
        // counters and they will be scheduled to run for different times.
        let scaled_counter_values: Vec<f64> = metric_group_data
            .counter_values
            .iter()
            .map(|&value| (value as f64) * time_enabled / time_running)
            .collect();

        // Use the counter value and metric expression to compute the metric value.
        match metric_expression.eval(&scaled_counter_values) {
            Ok(metric_value) if metric_value.is_finite() => {
                time_series_data_processor.add_data_point(
                    &metric_name,
                    &get_cpu_series_name(metric_group_data.cpu_id),
                    metric_value,
                );
            }
            Err(e) => {
                debug!(
                    "Failed to evaluate PMU metric {metric_name} on CPU {}: {:?}",
                    metric_group_data.cpu_id, e
                );
                continue;
            }
            _ => continue,
        }

        // Sum up the counter values within the metric across all CPUs.
        let counter_value_sums = per_metric_counter_value_sums
            .entry(metric_name)
            .or_insert_with(|| Vec::new());
        for (index, counter_value) in scaled_counter_values.iter().enumerate() {
            if index >= counter_value_sums.len() {
                counter_value_sums.push(*counter_value);
            } else {
                counter_value_sums[index] += *counter_value;
            }
        }
    }

    // Compute and add aggregate series data points.
    for (metric_name, counter_value_sums) in per_metric_counter_value_sums {
        let metric_expression = match pmu_metric_expressions.get(&metric_name) {
            Some(expression) => expression,
            None => continue,
        };
        match metric_expression.eval_vec(counter_value_sums) {
            Ok(metric_value) if metric_value.is_finite() => {
                time_series_data_processor.add_aggregate_data_point(
                    &metric_name,
                    &get_aggregate_series_name(),
                    metric_value,
                );
            }
            Err(e) => debug!(
                "Failed to evaluate the aggregate series of PMU metric {metric_name}: {:?}",
                e
            ),
            _ => continue,
        }
    }

    update_counter_schedule_rate_metric(time_series_data_processor, per_cpu_counter_schedule_rates);
}

/// Process a snapshot of raw PMU counters that were collected without using groups.
fn process_single_raw_pmu_event_counter_data(
    time_series_data_processor: &mut TimeSeriesDataProcessor,
    raw_data: &str,
    pmu_metric_expressions: &HashMap<String, FlatEx<f64>>,
) {
    // Store the delta value of an event counter across all CPUs.
    let mut per_cpu_event_counter_values: HashMap<usize, HashMap<String, f64>> = HashMap::new();
    // Store the sum of every counter value across all CPUs, used to compute the aggregate
    // series value for the metric.
    let mut event_counter_value_sums: HashMap<String, f64> = HashMap::new();
    // For every CPU, store the list of counter schedule rates across all events,
    // to compute the average at the end to be used as the value for the counter
    // schedule rate metric.
    let mut per_cpu_counter_schedule_rates: HashMap<usize, Vec<f64>> = HashMap::new();

    for pmu_data_string in raw_data.lines() {
        let event_counter_data = match PmuCollectedData::from_string(pmu_data_string) {
            Some(data) => data,
            None => continue,
        };
        let event_name = event_counter_data.identifier;
        let cpu_id = event_counter_data.cpu_id;
        let cpu_series_name = get_cpu_series_name(cpu_id);

        // All collected data (including time_enabled and time_running) are accumulative.
        let counter_value = event_counter_data.counter_values[0] as f64;
        let time_enabled = match time_series_data_processor.get_delta_and_set_previous_value(
            &format!("{event_name}_time_enabled"),
            &cpu_series_name,
            event_counter_data.time_enabled,
        ) {
            Some(time_enabled) => time_enabled,
            None => continue,
        };
        let time_running = match time_series_data_processor.get_delta_and_set_previous_value(
            &format!("{event_name}_time_running"),
            &cpu_series_name,
            event_counter_data.time_running,
        ) {
            Some(time_running) => time_running,
            None => continue,
        };

        if time_enabled == 0.0 {
            continue;
        }
        per_cpu_counter_schedule_rates
            .entry(cpu_id)
            .or_insert_with(|| Vec::new())
            .push(time_running / time_enabled);

        if time_running == 0.0 {
            continue;
        }

        // Use time_enabled and time_running to scale the value, as when more events need to be
        // collected than the hardware limitation, the PMU will multiplex (time-share) all the
        // counters  and they will be scheduled to run for different times.
        let scaled_counter_value = counter_value * time_enabled / time_running;
        per_cpu_event_counter_values
            .entry(cpu_id)
            .or_insert_with(|| HashMap::new())
            .insert(event_name.clone(), scaled_counter_value);
        *(event_counter_value_sums.entry(event_name).or_insert(0.0)) += scaled_counter_value;
    }

    for (metric_name, metric_expression) in pmu_metric_expressions {
        // The list of identifiers (event names) in the metric expression.
        let event_names = metric_expression.var_names().to_vec();

        for (&cpu_id, event_counter_values) in &per_cpu_event_counter_values {
            let expression_values: Vec<f64> = event_names
                .iter()
                .map(|event_name| event_counter_values.get(event_name).copied())
                .flatten()
                .collect();

            match metric_expression.eval_vec(expression_values) {
                Ok(metric_value) if metric_value.is_finite() => {
                    time_series_data_processor.add_data_point(
                        metric_name,
                        &get_cpu_series_name(cpu_id),
                        metric_value,
                    );
                }
                Err(e) => {
                    debug!(
                        "Failed to evaluate PMU metric {metric_name} on CPU {}: {:?}",
                        cpu_id, e
                    );
                }
                _ => continue,
            }
        }

        // Compute aggregate series value using computed sums
        let aggregate_expression_values: Vec<f64> = event_names
            .iter()
            .map(|event_name| event_counter_value_sums.get(event_name).copied())
            .flatten()
            .collect();
        match metric_expression.eval_vec(aggregate_expression_values) {
            Ok(metric_value) if metric_value.is_finite() => {
                time_series_data_processor.add_aggregate_data_point(
                    metric_name,
                    &get_aggregate_series_name(),
                    metric_value,
                );
            }
            Err(e) => {
                debug!(
                    "Failed to evaluate the aggregate series of PMU metric {metric_name}: {:?}",
                    e
                );
            }
            _ => continue,
        }
    }

    update_counter_schedule_rate_metric(time_series_data_processor, per_cpu_counter_schedule_rates);
}

impl ProcessData for PerfStat {
    fn process_raw_data(
        &mut self,
        report_params: &ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor =
            time_series_data_processor_with_custom_aggregate!(report_params.collection_start);

        if report_params.pmu_counter_mode.is_empty() {
            return process_legacy_raw_pmu_stat_data(time_series_data_processor, &raw_data);
        }

        // Since the counters accumulates from 0, we should use what has already
        // been collected at the first collection.
        time_series_data_processor.use_first_accumulative_value();

        let pmu_config =
            PmuConfig::from_file(&get_saved_pmu_config_path(&report_params.run_data_dir))
                .with_context(|| {
                    format!(
                        "Failed to open saved PMU config for run {}",
                        report_params.run_name
                    )
                })?;
        let pmu_metric_expressions = pmu_config.get_metric_expressions()?;

        if report_params.pmu_counter_mode == UNGROUPED_PMU_MODE {
            for buffer in raw_data {
                let raw_value = match buffer {
                    Data::PerfStatRaw(ref value) => value,
                    _ => panic!("Invalid Data type in raw file"),
                };
                time_series_data_processor.proceed_to_time(raw_value.time);
                process_single_raw_pmu_event_counter_data(
                    &mut time_series_data_processor,
                    &raw_value.data,
                    &pmu_metric_expressions,
                );
            }
        } else {
            let mut zero_time_running_metrics = HashSet::new();
            for buffer in raw_data {
                let raw_value = match buffer {
                    Data::PerfStatRaw(ref value) => value,
                    _ => panic!("Invalid Data type in raw file"),
                };
                time_series_data_processor.proceed_to_time(raw_value.time);
                process_single_raw_pmu_metric_counter_group_data(
                    &mut time_series_data_processor,
                    &raw_value.data,
                    &pmu_metric_expressions,
                    &mut zero_time_running_metrics,
                );
            }
            for metric in zero_time_running_metrics {
                warn!("PMU metric {metric} might contain too many events to be scheduled for collection. Please reduce the number of events in it or use --ungroup-pmu-events.");
            }
        }

        let mut all_metric_names: Vec<&str> =
            pmu_config.metrics.keys().map(String::as_str).collect();
        all_metric_names.insert(0, COUNTER_SCHEDULE_RATE_METRIC_NAME);
        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(all_metric_names);

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use {
        super::{PerfStatRaw, PmuConfig},
        crate::data::common::utils::get_online_cpu_ids,
        crate::data::CollectData,
        crate::data_collection::InitParams,
        std::io::ErrorKind,
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_probe_pmu_counter_limit() {
        let cpu_id = *get_online_cpu_ids()
            .expect("failed to read online CPUs")
            .last()
            .expect("expected at least one online CPU");
        if let Ok(limit) = PmuConfig::probe_pmu_counter_limit(cpu_id) {
            // The probed PMU register count should land in a sane range
            assert!(
                limit >= 1 && limit <= 8,
                "probed PMU counter limit {limit} is outside the expected range 1..=8"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut perf_stat = PerfStatRaw::new();
        let params = InitParams::default();

        match perf_stat.prepare_data_collector(&params) {
            Err(e) => {
                if let Some(os_error) = e.downcast_ref::<std::io::Error>() {
                    match os_error.kind() {
                        ErrorKind::PermissionDenied => {
                            panic!("Set /proc/sys/kernel/perf_event_paranoid to -1")
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
