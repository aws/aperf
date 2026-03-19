use crate::data::common::common_raw_data::{
    parse_common_raw_time_series_data, TimeSeriesDataBuilder,
};
use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_average_aggregate;
#[cfg(target_os = "linux")]
use crate::data::common::utils::collect_file_paths_in_dir;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    crate::PDError,
    chrono::Utc,
    log::{debug, warn},
    std::fs,
    std::io::{Read, Seek, SeekFrom},
    std::path::PathBuf,
};

#[cfg(target_os = "linux")]
pub fn collect_efa_metrics_file_paths(
    efa_metrics_root_path: &str,
) -> HashMap<String, HashMap<String, File>> {
    let mut efa_metrics_file_paths: HashMap<String, HashMap<String, File>> = HashMap::new();

    let efa_metrics_dir = PathBuf::from(efa_metrics_root_path);
    if !efa_metrics_dir.exists() {
        // If the EFA dir does not exist, which is a common case, users should not see
        // any warning messages
        debug!("No EFA metrics found");
        return efa_metrics_file_paths;
    }
    let hardware_entries = match fs::read_dir(&efa_metrics_dir) {
        Ok(hardware_entries) => hardware_entries,
        Err(e) => {
            warn!(
                "Failed to read EFA hardware entries at {}: {}",
                efa_metrics_dir.display(),
                e
            );
            return efa_metrics_file_paths;
        }
    };

    for hardware_entry in hardware_entries {
        let hardware_entry = match hardware_entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Failed to read EFA hardware entry: {}", e);
                continue;
            }
        };
        let hardware_name = hardware_entry.file_name().to_string_lossy().into_owned();
        debug!("Found hardware {}", hardware_name);

        // Collect /sys/class/infiniband/*/hw_counters/*
        let hardware_counters_dir = hardware_entry.path().join("hw_counters");
        match collect_file_paths_in_dir(&hardware_counters_dir) {
            Ok(hardware_counter_file_paths) => {
                let hardware_counter_files: HashMap<String, File> = hardware_counter_file_paths
                    .into_iter()
                    .filter_map(|(counter_name, counter_path)| {
                        File::open(&counter_path)
                            .ok()
                            .map(|counter_file| (counter_name, counter_file))
                    })
                    .collect();
                efa_metrics_file_paths.insert(hardware_name.clone(), hardware_counter_files);
            }
            Err(e) => {
                warn!(
                    "Failed to read hardware counters at {}: {}",
                    hardware_counters_dir.display(),
                    e
                );
            }
        }

        // Collect /sys/class/infiniband/*/ports/1/hw_counters/*
        let ports_dir = hardware_entry.path().join("ports");
        let ports_entries = match fs::read_dir(&ports_dir) {
            Ok(ports_entries) => ports_entries,
            Err(e) => {
                warn!(
                    "Failed to read ports entries for possible EFA hardware at {}: {}",
                    ports_dir.display(),
                    e
                );
                continue;
            }
        };
        for port_entry in ports_entries {
            let port_entry = match port_entry {
                Ok(entry) => entry,
                Err(e) => {
                    warn!(
                        "Failed to read port for EFA hardware {}: {}",
                        hardware_name, e
                    );
                    continue;
                }
            };
            let port = port_entry.file_name().to_string_lossy().into_owned();
            debug!("Found port {} for EFA hardware {}", port, hardware_name);

            let port_counters_dir = port_entry.path().join("hw_counters");
            match collect_file_paths_in_dir(&port_counters_dir) {
                Ok(port_counter_file_paths) => {
                    let port_counter_files: HashMap<String, File> = port_counter_file_paths
                        .into_iter()
                        .filter_map(|(counter_name, counter_path)| {
                            File::open(&counter_path)
                                .ok()
                                .map(|counter_file| (counter_name, counter_file))
                        })
                        .collect();
                    efa_metrics_file_paths
                        .insert(format!("{}/{}", hardware_name, port), port_counter_files);
                }
                Err(e) => {
                    warn!(
                        "Failed to read port counters at {}: {}",
                        port_counters_dir.display(),
                        e
                    );
                    continue;
                }
            }
        }
    }

    efa_metrics_file_paths
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EfaStatRaw {
    // This is a global state to store all counter file descriptors,
    // so it needs to skip serialization
    #[serde(skip)]
    pub efa_metric_file_paths: HashMap<String, HashMap<String, File>>,
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl EfaStatRaw {
    pub fn new() -> Self {
        EfaStatRaw {
            efa_metric_file_paths: HashMap::new(),
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for EfaStatRaw {
    fn prepare_data_collector(&mut self, _params: &CollectorParams) -> Result<()> {
        // Map from EFA drivers (hardware name or hardware name plus port) to all the counter metrics
        // (another map from counter names to the file descriptor)
        let efa_metric_file_paths: HashMap<String, HashMap<String, File>> =
            collect_efa_metrics_file_paths("/sys/class/infiniband");

        if efa_metric_file_paths.is_empty() {
            return Err(PDError::IgnoredDataPreparationError(
                "No EFA metrics available".to_string(),
            )
            .into());
        }

        self.efa_metric_file_paths = efa_metric_file_paths;

        Ok(())
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());

        let mut common_raw_data_builder = TimeSeriesDataBuilder::new();
        for (efa_component, counter_file_paths) in self.efa_metric_file_paths.iter_mut() {
            common_raw_data_builder.add_component_line(efa_component);
            for (counter_name, counter_file) in counter_file_paths {
                let mut counter_value = String::new();
                counter_file.read_to_string(&mut counter_value)?;
                common_raw_data_builder.add_metric_line(counter_name, &counter_value);
                counter_file.seek(SeekFrom::Start(0))?;
            }
        }
        self.data = common_raw_data_builder.get_data();

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EfaStat;

impl EfaStat {
    pub fn new() -> Self {
        EfaStat
    }
}

impl ProcessData for EfaStat {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_average_aggregate!();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::EfaStatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            // There are EFA counters for the number of working requests (*_wrs) and the number
            // of bytes transmitted through such working requests (*_bytes). A useful metrics is
            // the average number of bytes per working request, so create the below map to compute it:
            // Map<working request type, Map<EFA device, (total bytes, # requests)>>
            let mut avg_bytes_per_wr: HashMap<String, HashMap<String, (f64, f64)>> = [
                // List out known working request types to avoid unnecessary computations
                ("send".to_string(), HashMap::new()),
                ("recv".to_string(), HashMap::new()),
                ("rdma_write".to_string(), HashMap::new()),
                ("rdma_read".to_string(), HashMap::new()),
            ]
            .into_iter()
            .collect();

            // Parse and compute ENA data
            let efa_data = parse_common_raw_time_series_data(&raw_value.data);
            for (efa_metric_name, per_device_metric_value) in &efa_data {
                for (efa_device, efa_metric_value) in per_device_metric_value {
                    let series_value = match time_series_data_processor.add_accumulative_data_point(
                        &efa_metric_name,
                        &efa_device,
                        *efa_metric_value,
                    ) {
                        Some(series_value) => series_value,
                        None => continue,
                    };

                    let mut is_bytes_metric = false;
                    let wrs_type = if efa_metric_name.ends_with("wrs") {
                        efa_metric_name.strip_suffix("_wrs").unwrap_or("")
                    } else if efa_metric_name.ends_with("_bytes") {
                        is_bytes_metric = true;
                        efa_metric_name.strip_suffix("_bytes").unwrap_or("")
                    } else {
                        ""
                    };
                    // If the current metric is for a known working request, store the value for
                    // the later computation of average bytes per working request
                    if let Some(per_device_avg_bytes) = avg_bytes_per_wr.get_mut(wrs_type) {
                        let (wrs_bytes, wrs_num) = per_device_avg_bytes
                            .entry(efa_device.clone())
                            .or_insert((0.0, 0.0));
                        if is_bytes_metric {
                            *wrs_bytes = series_value;
                        } else {
                            *wrs_num = series_value;
                        }
                    }
                }
            }
            // Compute average bytes per working request and add to series values
            for (wrs_type, per_device_avg_bytes) in avg_bytes_per_wr {
                for (efa_device, (wrs_bytes, wrs_num)) in &per_device_avg_bytes {
                    let metric_name = format!("avg_bytes_per_{wrs_type}_wr");
                    time_series_data_processor.add_data_point(
                        &metric_name,
                        efa_device,
                        if *wrs_num > 0.0 {
                            *wrs_bytes / *wrs_num
                        } else {
                            0.0
                        },
                    );
                }
            }
        }

        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(vec![
                // Below are the metrics listed in the official doc, so showing them upfront
                // https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/efa-working-monitor.html
                "tx_bytes",
                "rx_bytes",
                "tx_pkts",
                "rx_pkts",
                "rx_drops",
                "send_bytes",
                "send_wrs",
                "avg_bytes_per_send_wr",
                "recv_bytes",
                "recv_wrs",
                "avg_bytes_per_recv_wr",
                "rdma_write_bytes",
                "rdma_write_wrs",
                "avg_bytes_per_rdma_write_wr",
                "rdma_read_bytes",
                "rdma_read_wrs",
                "avg_bytes_per_rdma_read_wr",
                "rdma_write_wr_err",
                "rdma_read_wr_err",
                "rdma_read_resp_bytes",
                "rdma_write_recv_bytes",
                "retrans_bytes",
                "retrans_pkts",
                "retrans_timeout_events",
                "impaired_remote_conn_events",
                "unresponsive_remote_events",
            ]);

        Ok(AperfData::TimeSeries(time_series_data))
    }
}
