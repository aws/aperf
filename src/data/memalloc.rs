use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::time_series_data_processor_with_average_aggregate;
#[cfg(target_os = "linux")]
use crate::data::{CollectData, CollectorParams};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::visualizer::ReportParams;
#[cfg(target_os = "linux")]
use crate::PDError;
use anyhow::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemallocDataRaw {
    pub time: TimeEnum,
    pub buddyinfo_data: String,
    pub pagetypeinfo_data: String,
    pub slabinfo_data: String,
}

#[cfg(target_os = "linux")]
impl Default for MemallocDataRaw {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl MemallocDataRaw {
    pub fn new() -> Self {
        MemallocDataRaw {
            time: TimeEnum::DateTime(Utc::now()),
            buddyinfo_data: String::new(),
            pagetypeinfo_data: String::new(),
            slabinfo_data: String::new(),
        }
    }
}

#[cfg(target_os = "linux")]
impl CollectData for MemallocDataRaw {
    fn prepare_data_collector(&mut self, _params: &CollectorParams) -> Result<()> {
        if std::fs::read_to_string("/proc/buddyinfo").is_err()
            && std::fs::read_to_string("/proc/pagetypeinfo").is_err()
            && std::fs::read_to_string("/proc/slabinfo").is_err()
        {
            return Err(PDError::IgnoredDataPreparationError(
                "None of memalloc system files are readable".to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn collect_data(&mut self, _params: &CollectorParams) -> Result<()> {
        self.time = TimeEnum::DateTime(Utc::now());
        self.buddyinfo_data = std::fs::read_to_string("/proc/buddyinfo").unwrap_or_default();
        self.pagetypeinfo_data = std::fs::read_to_string("/proc/pagetypeinfo").unwrap_or_default();
        self.slabinfo_data = std::fs::read_to_string("/proc/slabinfo").unwrap_or_default();
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemallocData;

impl MemallocData {
    pub fn new() -> Self {
        MemallocData
    }
}

impl MemallocData {
    fn format_metric_name(name: &str) -> String {
        if let Some(order_str) = name.strip_prefix("buddyinfo_order_") {
            if let Ok(order) = order_str.parse::<u32>() {
                let size_kb = 4 * (1 << order);
                return if size_kb >= 1024 {
                    format!("BuddyInfo order_{} ({}MB)", order, size_kb / 1024)
                } else {
                    format!("BuddyInfo order_{} ({}KB)", order, size_kb)
                };
            }
        } else if name.starts_with("pageblocks_") {
            let migrate_type = name.strip_prefix("pageblocks_type_").unwrap_or("");
            return format!("PageBlocks - {}", migrate_type);
        } else if name.starts_with("pagetype_") {
            let parts: Vec<&str> = name.split('_').collect();
            if parts.len() >= 5 && parts[1] == "order" {
                if let Ok(order) = parts[2].parse::<u32>() {
                    let size_kb = 4 * (1 << order);
                    let migrate_type = parts[4..].join("_");
                    return if size_kb >= 1024 {
                        format!(
                            "PageType {} - order_{} ({}MB)",
                            migrate_type,
                            order,
                            size_kb / 1024
                        )
                    } else {
                        format!(
                            "PageType {} - order_{} ({}KB)",
                            migrate_type, order, size_kb
                        )
                    };
                }
            }
        } else if name.starts_with("slabinfo_") {
            if let Some(metric_type) = name.strip_prefix("slabinfo_") {
                let readable_name = metric_type.replace('_', " ");
                return format!("SlabInfo {}", readable_name);
            }
        }
        name.to_string()
    }
}

impl ProcessData for MemallocData {
    fn process_raw_data(
        &mut self,
        _params: ReportParams,
        raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor = time_series_data_processor_with_average_aggregate!();

        for buffer in raw_data {
            let raw_value = match buffer {
                Data::MemallocDataRaw(ref value) => value,
                _ => panic!("Invalid Data type in raw file"),
            };
            time_series_data_processor.proceed_to_time(raw_value.time);

            // Process buddyinfo data
            for line in raw_value.buddyinfo_data.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 5 {
                    continue;
                }

                let node = parts[1].trim_end_matches(',');
                let zone = parts[3];

                for (order, value_str) in parts[4..].iter().enumerate() {
                    if let Ok(value) = value_str.parse::<f64>() {
                        let metric_name = format!("buddyinfo_order_{}", order);
                        let series_name = format!("node_{}_zone_{}", node, zone);
                        time_series_data_processor.add_data_point(
                            &metric_name,
                            &series_name,
                            value,
                        );
                    }
                }
            }

            // Process pagetypeinfo data
            for line in raw_value.pagetypeinfo_data.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();

                // Process per-order free pages
                if parts.len() >= 6 && parts[0] == "Node" && parts[4] == "type" {
                    let node = parts[1].trim_end_matches(',');
                    let zone = parts[3].trim_end_matches(',');
                    let migrate_type = parts[5];

                    for (order, value_str) in parts[6..].iter().enumerate() {
                        if let Ok(value) = value_str.parse::<f64>() {
                            let metric_name =
                                format!("pagetype_order_{}_type_{}", order, migrate_type);
                            let series_name = format!("node_{}_zone_{}", node, zone);
                            time_series_data_processor.add_data_point(
                                &metric_name,
                                &series_name,
                                value,
                            );
                        }
                    }
                }

                // Process aggregated pageblock counts
                if parts.len() >= 8 && parts[0] == "Node" && parts[2] == "zone" {
                    let zone = parts[3].trim_end_matches(',');
                    let migrate_types = [
                        "Unmovable",
                        "Movable",
                        "Reclaimable",
                        "HighAtomic",
                        "CMA",
                        "Isolate",
                    ];

                    for (idx, migrate_type) in migrate_types.iter().enumerate() {
                        if let Some(value_str) = parts.get(4 + idx) {
                            if let Ok(value) = value_str.parse::<f64>() {
                                let metric_name = format!("pageblocks_type_{}", migrate_type);
                                let series_name = format!("zone_{}", zone);
                                time_series_data_processor.add_data_point(
                                    &metric_name,
                                    &series_name,
                                    value,
                                );
                            }
                        }
                    }
                }
            }

            // Process slabinfo data
            let mut skip_header = true;
            for line in raw_value.slabinfo_data.lines() {
                if skip_header {
                    if line.starts_with("# name") {
                        skip_header = false;
                    }
                    continue;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 16 {
                    continue;
                }

                let slab_name = parts[0];
                let metrics = [
                    ("active_objs", parts.get(1)),
                    ("num_objs", parts.get(2)),
                    ("objsize", parts.get(3)),
                    ("objperslab", parts.get(4)),
                    ("pagesperslab", parts.get(5)),
                    ("limit", parts.get(8)),
                    ("batchcount", parts.get(9)),
                    ("sharedfactor", parts.get(10)),
                    ("active_slabs", parts.get(13)),
                    ("num_slabs", parts.get(14)),
                    ("sharedavail", parts.get(15)),
                ];

                for (metric_type, value_opt) in metrics {
                    if let Some(value_str) = value_opt {
                        if let Ok(value) = value_str.parse::<f64>() {
                            time_series_data_processor.add_data_point(
                                &format!("slabinfo_{}", metric_type),
                                slab_name,
                                value,
                            );
                        }
                    }
                }
            }
        }

        let mut time_series_data = time_series_data_processor.get_time_series_data();

        time_series_data.sorted_metric_names.sort_by(|a, b| {
            let a_is_buddy = a.starts_with("buddyinfo_");
            let b_is_buddy = b.starts_with("buddyinfo_");
            let a_is_pageblocks = a.starts_with("pageblocks_");
            let b_is_pageblocks = b.starts_with("pageblocks_");
            let a_is_pagetype = a.starts_with("pagetype_");
            let b_is_pagetype = b.starts_with("pagetype_");
            let a_is_slabinfo = a.starts_with("slabinfo_");
            let b_is_slabinfo = b.starts_with("slabinfo_");

            // Order: BuddyInfo, PageType, PageBlocks, SlabInfo
            if a_is_buddy != b_is_buddy {
                return b_is_buddy.cmp(&a_is_buddy);
            }
            if a_is_pageblocks != b_is_pageblocks {
                return a_is_pageblocks.cmp(&b_is_pageblocks);
            }
            if a_is_slabinfo != b_is_slabinfo {
                return a_is_slabinfo.cmp(&b_is_slabinfo);
            }

            // For PageType, sort by migrate type first, then by order
            if a_is_pagetype && b_is_pagetype {
                let a_parts: Vec<&str> = a.split('_').collect();
                let b_parts: Vec<&str> = b.split('_').collect();

                if a_parts.len() >= 5 && b_parts.len() >= 5 {
                    let a_type = a_parts[4..].join("_");
                    let b_type = b_parts[4..].join("_");

                    if a_type != b_type {
                        return a_type.cmp(&b_type);
                    }

                    let a_order = a_parts[2].parse::<u32>().unwrap_or(0);
                    let b_order = b_parts[2].parse::<u32>().unwrap_or(0);
                    return a_order.cmp(&b_order);
                }
            }

            let a_order = if a_is_buddy {
                a.strip_prefix("buddyinfo_order_")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            } else if !a_is_pageblocks {
                a.split('_')
                    .nth(2)
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            } else {
                u32::MAX
            };

            let b_order = if b_is_buddy {
                b.strip_prefix("buddyinfo_order_")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            } else if !b_is_pageblocks {
                b.split('_')
                    .nth(2)
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            } else {
                u32::MAX
            };

            if a_order != b_order {
                a_order.cmp(&b_order)
            } else {
                a.cmp(b)
            }
        });

        // Update metric names to include size
        let mut renamed_metrics = std::collections::HashMap::new();
        for (old_name, mut metric) in time_series_data.metrics.drain() {
            let new_name = Self::format_metric_name(&old_name);
            metric.metric_name = new_name.clone();
            renamed_metrics.insert(new_name, metric);
        }
        time_series_data.metrics = renamed_metrics;

        // Update sorted names
        time_series_data.sorted_metric_names = time_series_data
            .sorted_metric_names
            .iter()
            .map(|name| Self::format_metric_name(name))
            .collect();

        Ok(AperfData::TimeSeries(time_series_data))
    }
}
