#![allow(non_camel_case_types)]

use crate::data::common::common_raw_data::parse_common_raw_time_series_data;
use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_average_aggregate;
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use {
    crate::data::{CollectData, CollectorParams},
    chrono::Utc,
    ethtool::Ethtool,
};

#[cfg(target_os = "linux")]
mod ethtool {
    use crate::data::common::common_raw_data::TimeSeriesDataBuilder;
    use crate::PDError;
    use anyhow::Result;
    use std::fs;
    use std::os::raw::{c_int, c_ulong};

    extern "C" {
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
        fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
    }

    const SIOCETHTOOL: c_ulong = 0x8946;

    /// All ethtool commands are defined in
    /// https://github.com/torvalds/linux/blob/master/include/uapi/linux/ethtool.h
    const ETHTOOL_GSSET_INFO: u32 = 0x00000037;
    const ETHTOOL_GSTRINGS: u32 = 0x0000001b;
    const ETHTOOL_GSTATS: u32 = 0x0000001d;

    /// Length of a string, defined in
    /// https://github.com/torvalds/linux/blob/master/include/uapi/linux/ethtool.h
    const ETH_GSTRING_LEN: usize = 32;

    /// All string set IDs are available in enum ethtool_stringset in
    /// https://github.com/torvalds/linux/blob/master/include/uapi/linux/ethtool.h
    const ETH_SS_STATS: u32 = 1;

    /// The structure used by ioctl to get and set network configuration
    /// See https://man7.org/linux/man-pages/man7/netdevice.7.html
    #[repr(C)]
    struct ifreq {
        ifr_name: [u8; 16],
        ifr_payload: *mut u8,
    }

    /// Get the number of stats of a network interface
    fn get_n_stats(sock: c_int, if_name: &str) -> usize {
        /// The IFR payload structure to get string set information.
        /// See struct ethtool_sset_info in https://github.com/torvalds/linux/blob/master/include/uapi/linux/ethtool.h
        ///     ethtool_get_sset_info in https://github.com/torvalds/linux/blob/master/net/ethtool/ioctl.c
        #[repr(C)]
        struct ethtool_sset_info {
            // ETHTOOL_GSSET_INFO
            cmd: u32,
            // Reserved byte (use 0)
            reserved: u32,
            // The bit mask to indicate the string sets to query for
            sset_mask: u64,
            // The returned data, whose size equals to the number of queried string sets.
            // Set it to fixed size of 64, which is the maximum number of string set queries,
            // for memory safety.
            data: [u32; 64],
        }

        let mut sset_info = vec![0u8; size_of::<ethtool_sset_info>()];
        let sset_info_ptr = sset_info.as_mut_ptr() as *mut ethtool_sset_info;
        unsafe {
            (*sset_info_ptr).cmd = ETHTOOL_GSSET_INFO;
            (*sset_info_ptr).reserved = 0;
            (*sset_info_ptr).sset_mask = 1u64 << ETH_SS_STATS;
        }

        let mut ifr = make_ifreq(if_name, sset_info.as_mut_ptr());
        if unsafe { ioctl(sock, SIOCETHTOOL, &mut ifr) } < 0 {
            return 0;
        }

        unsafe { (*sset_info_ptr).data[0] as usize }
    }

    /// Get all stat names of a network interface
    fn get_stat_names(sock: c_int, if_name: &str, n_stats: usize) -> Vec<String> {
        /// The IFR payload structure to get a string set
        /// See struct ethtool_gstrings in https://github.com/torvalds/linux/blob/master/include/uapi/linux/ethtool.h
        #[repr(C)]
        struct ethtool_gstrings {
            // ETHTOOL_GSTRINGS
            cmd: u32,
            // String set ID (same as the bit mask position)
            string_set: u32,
            // The number of strings in the string set
            len: u32,
            // Followed by: n_stats * ETH_GSTRING_LEN bytes of string data to be filled by kernel.
            // The buffer memory needs to be allocated manually when creating the struct.
        }

        let mut gstrings = vec![0u8; size_of::<ethtool_gstrings>() + n_stats * ETH_GSTRING_LEN];
        let gstrings_ptr = gstrings.as_mut_ptr() as *mut ethtool_gstrings;
        unsafe {
            (*gstrings_ptr).cmd = ETHTOOL_GSTRINGS;
            (*gstrings_ptr).string_set = ETH_SS_STATS;
            (*gstrings_ptr).len = n_stats as u32; // Tell kernel how many strings we want
        }

        let mut ifr = make_ifreq(if_name, gstrings.as_mut_ptr());
        if unsafe { ioctl(sock, SIOCETHTOOL, &mut ifr) } < 0 {
            return vec![];
        }

        // Memory layout after the kernel filled the ifr with response:
        // [cmd + string_set + len: 12 bytes][string0: 32 bytes][string1: 32 bytes]...
        let strings_ptr = unsafe { gstrings.as_ptr().add(size_of::<ethtool_gstrings>()) };
        (0..n_stats)
            .map(|i| {
                let cur_str_ptr = unsafe { strings_ptr.add(i * ETH_GSTRING_LEN) };

                let cur_str_bytes: Vec<u8> = (0..ETH_GSTRING_LEN)
                    .map(|j| unsafe { *cur_str_ptr.add(j) })
                    .take_while(|&b| b != 0) // Stop at null terminator
                    .collect();

                String::from_utf8_lossy(&cur_str_bytes).to_string()
            })
            .collect()
    }

    /// Get all stat values of a network interface
    fn get_stat_values(sock: c_int, if_name: &str, n_stats: usize) -> Vec<u64> {
        /// The IFR payload structure to get device-specific statistics
        /// See struct ethtool_stats in https://github.com/torvalds/linux/blob/master/include/uapi/linux/ethtool.h
        #[repr(C)]
        struct ethtool_stats {
            // ETHTOOL_GSTATS
            cmd: u32,
            // The number of statistics whose values to get
            n_stats: u32,
            // Followed by: n_stats * 8 bytes of u64 values to be filled by kernel.
            // The buffer memory needs to be allocated manually when creating the struct.
        }

        let mut stats = vec![0u8; size_of::<ethtool_stats>() + n_stats * size_of::<u64>()];
        let stats_ptr = stats.as_mut_ptr() as *mut ethtool_stats;
        unsafe {
            (*stats_ptr).cmd = ETHTOOL_GSTATS;
            (*stats_ptr).n_stats = n_stats as u32;
        }

        let mut ifr = make_ifreq(if_name, stats.as_mut_ptr());
        if unsafe { ioctl(sock, SIOCETHTOOL, &mut ifr) } < 0 {
            return vec![0; n_stats];
        }

        // Memory layout after the kernel filled the ifr with response:
        // [cmd + n_stats: 8 bytes][value0: 8 bytes][value1: 8 bytes]...
        let values_ptr = unsafe { stats.as_ptr().add(size_of::<ethtool_stats>()) as *const u64 };
        (0..n_stats)
            .map(|i| unsafe { *values_ptr.add(i) })
            .collect()
    }

    /// Helper method to build a ifreq
    fn make_ifreq(if_name: &str, ifr_payload: *mut u8) -> ifreq {
        let mut ifr_name = [0u8; 16];
        for (i, &b) in if_name.as_bytes().iter().enumerate() {
            ifr_name[i] = b;
        }
        ifreq {
            ifr_name,
            ifr_payload,
        }
    }

    #[derive(Debug)]
    pub struct Ethtool {
        socket: c_int,
        interfaces: Vec<String>,
    }

    impl Ethtool {
        pub fn new() -> Result<Self> {
            let mut interfaces: Vec<String> = Vec::new();
            match fs::read_dir("/sys/class/net") {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let ifname = entry.file_name().to_string_lossy().to_string();

                        // Quickly skip common virtual interfaces
                        if ifname == "lo"
                            || ifname.starts_with("docker")
                            || ifname.starts_with("veth")
                            || ifname.starts_with("br-")
                            || ifname.starts_with("virbr")
                        {
                            continue;
                        }

                        // Ensure that it has device symlink to PCI device
                        let device_path = format!("/sys/class/net/{}/device", ifname);
                        if fs::metadata(&device_path).is_ok() {
                            interfaces.push(ifname);
                        }
                    }
                }
                Err(e) => {
                    return Err(
                        PDError::NetworkInterfaceDetectionFailure(format!("{:?}", e)).into(),
                    );
                }
            }

            // Parameters: AF_INET (2), SOCK_DGRAM (2), protocol (0 = default)
            let sock = unsafe { socket(2, 2, 0) };
            if sock < 0 {
                return Err(PDError::EthToolSocketCreationFailure.into());
            }

            Ok(Ethtool {
                socket: sock,
                interfaces,
            })
        }

        pub fn get_stats(&self) -> String {
            let mut common_raw_data_builder = TimeSeriesDataBuilder::new();

            for interface in &self.interfaces {
                common_raw_data_builder.add_component_line(interface);

                let n_stats = get_n_stats(self.socket, interface);
                let stat_names = get_stat_names(self.socket, interface, n_stats);
                let stat_values = get_stat_values(self.socket, interface, n_stats);

                for i in 0..n_stats {
                    common_raw_data_builder
                        .add_metric_line(&stat_names[i], &stat_values[i].to_string());
                }
            }

            common_raw_data_builder.get_data()
        }
    }

    impl Drop for Ethtool {
        fn drop(&mut self) {
            unsafe {
                close(self.socket);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnaStatRaw {
    // This field is to help collect the ethtool stats, so it needs to skip serialization
    #[cfg(target_os = "linux")]
    #[serde(skip)]
    pub ethtool: Option<Ethtool>,
    pub time: TimeEnum,
    pub data: String,
}

#[cfg(target_os = "linux")]
impl EnaStatRaw {
    pub fn new() -> Self {
        EnaStatRaw {
            ethtool: None,
            time: TimeEnum::DateTime(Utc::now()),
            data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for EnaStatRaw {
    fn prepare_data_collector(&mut self, _params: &CollectorParams) -> Result<()> {
        match Ethtool::new() {
            Ok(ethtool) => self.ethtool = Some(ethtool),
            Err(e) => {
                return Err(anyhow::anyhow!("{:?}", e));
            }
        };

        Ok(())
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.data = self.ethtool.as_ref().unwrap().get_stats();

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnaStat;

impl EnaStat {
    pub fn new() -> Self {
        EnaStat
    }
}

impl ProcessData for EnaStat {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_average_aggregate!();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::EnaStatRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            let ena_data = parse_common_raw_time_series_data(&raw_value.data);
            for (ena_metric_name, per_interface_value) in &ena_data {
                for (ena_interface, per_interface_ena_data) in per_interface_value {
                    let metric_name_parts: Vec<&str> = ena_metric_name.split("_").collect();
                    let mut metric_name = ena_metric_name.clone();
                    let mut series_name = ena_interface.clone();
                    // Transform same metrics across different ENA queues into a single metric
                    // with multiple series
                    if metric_name_parts.len() > 2
                        && metric_name_parts[0] == "queue"
                        && metric_name_parts[1].parse::<u64>().is_ok()
                    {
                        metric_name = metric_name_parts[2..].join("_");
                        series_name = format!(
                            "{}_{}_{}",
                            series_name, metric_name_parts[0], metric_name_parts[1]
                        );
                    }

                    time_series_data_processor.add_accumulative_data_point(
                        &metric_name,
                        &series_name,
                        *per_interface_ena_data,
                    );
                }
            }
        }

        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(vec![
                // Below are the metrics mentioned in the official doc so showing them upfront
                // https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/monitoring-network-performance-ena.html
                "bw_in_allowance_exceeded",
                "bw_out_allowance_exceeded",
                "conntrack_allowance_exceeded",
                "conntrack_allowance_available",
                "linklocal_allowance_exceeded",
                "pps_allowance_exceeded",
            ]);

        Ok(AperfData::TimeSeries(time_series_data))
    }
}

#[cfg(test)]
mod ena_tests {
    #[cfg(target_os = "linux")]
    use {
        super::EnaStatRaw,
        crate::data::{CollectData, CollectorParams},
    };

    #[cfg(target_os = "linux")]
    #[test]
    fn test_collect_data() {
        let mut ena = EnaStatRaw::new();
        let params = CollectorParams::new();

        ena.prepare_data_collector(&params).unwrap();
        assert!(ena.ethtool.is_some());
        ena.collect_data(&params).unwrap();
        assert!(!ena.data.is_empty());
    }
}
