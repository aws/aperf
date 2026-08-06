use crate::data::common::data_formats::AperfData;
use crate::data::common::time_series_data_processor::{
    time_series_data_processor_with_sum_aggregate, TimeSeriesDataProcessor,
};
use crate::data::{Data, ProcessData, TimeEnum};
use crate::data_processing::ReportParams;
use crate::ProcessMetric;
use anyhow::{bail, Result};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use strum::IntoEnumIterator;
#[cfg(target_os = "linux")]
use {
    crate::{data_file_path, get_data_name_from_type},
    log::error,
    std::fs,
    std::path::PathBuf,
};

/// The legacy APerf stat struct for kept Backward compatibility.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AperfStat {
    pub time: TimeEnum,
    pub name: String,
    pub data: HashMap<String, u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AperfStats {
    pub time: TimeEnum,
    /// The stats are stored in the format of Map<stat, Map<data, value>>.
    /// Each stat will be processed into a metric, and each data will be
    /// processed into a series within the metric.
    pub stats: HashMap<String, HashMap<String, f64>>,
}

impl AperfStats {
    pub fn new() -> Self {
        Self {
            time: TimeEnum::DateTime(Utc::now()),
            stats: HashMap::new(),
        }
    }

    pub fn for_time(time: TimeEnum) -> Self {
        Self {
            time,
            stats: HashMap::new(),
        }
    }
}

/// Encapsulate all logics of collecting and writing APerf stats.
/// The collected stats will be saved in memory in time order and be written to
/// disk when flush() is called.
#[cfg(target_os = "linux")]
pub struct AperfStatsCollector {
    cur_aperf_stats: AperfStats,
    time_series_aperf_stats: Vec<AperfStats>,
    run_data_dir: Option<PathBuf>,
}

#[cfg(target_os = "linux")]
impl AperfStatsCollector {
    pub fn new() -> Self {
        Self {
            cur_aperf_stats: AperfStats::new(),
            time_series_aperf_stats: Vec::new(),
            run_data_dir: None,
        }
    }

    pub fn initialize(&mut self, run_data_dir: PathBuf) {
        self.run_data_dir = Some(run_data_dir);
    }

    /// Check current time and if at next second, save the current stats and
    /// proceed with a new empty stats.
    /// If we get to a point where the stats is big and we want to limit APerf's
    /// memory usage, we can also flush here.
    fn update_time_series(&mut self) {
        let cur_time = TimeEnum::DateTime(Utc::now());
        let cur_time_diff = match cur_time - self.cur_aperf_stats.time {
            TimeEnum::TimeDiff(time_diff) => time_diff,
            _ => return,
        };
        if cur_time_diff >= 1 {
            self.proceed_to_next_stats(cur_time);
        }
    }

    /// Save current stats and proceed to the next new stats.
    pub fn proceed_to_next_stats(&mut self, next_stats_time: TimeEnum) {
        let cur_aperf_stats = std::mem::replace(
            &mut self.cur_aperf_stats,
            AperfStats::for_time(next_stats_time),
        );
        self.time_series_aperf_stats.push(cur_aperf_stats);
    }

    /// Add a stat. If a stat is added multiple times, the values are summed up.
    pub fn add_stat(&mut self, stat_name: String, data_name: String, stat_value: f64) {
        self.update_time_series();

        *self
            .cur_aperf_stats
            .stats
            .entry(stat_name)
            .or_default()
            .entry(data_name)
            .or_default() += stat_value;
    }

    /// Write all saved stats to disk file.
    pub fn flush(&mut self) -> Result<()> {
        if self.run_data_dir.is_none() {
            bail!("Failed to flush APerf stat since the run data directory path is uninitialized.");
        }

        let aperf_stats_file_path = data_file_path(
            get_data_name_from_type::<AperfStats>(),
            self.run_data_dir.as_ref().unwrap(),
        );
        let mut aperf_stats_file = match fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&aperf_stats_file_path)
        {
            Ok(aperf_stats_file) => aperf_stats_file,
            Err(e) => bail!(
                "Failed to create APerf Stats file at {}: {:?}",
                aperf_stats_file_path.display(),
                e
            ),
        };

        for aperf_stats in &self.time_series_aperf_stats {
            bincode::serialize_into(&mut aperf_stats_file, aperf_stats)?;
        }
        if !self.cur_aperf_stats.stats.is_empty() {
            bincode::serialize_into(&mut aperf_stats_file, &self.cur_aperf_stats)?;
        }

        self.time_series_aperf_stats.clear();
        self.cur_aperf_stats = AperfStats::new();

        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for AperfStatsCollector {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            error!("Failed to flush APerf stats on drop: {e}");
        }
    }
}

fn process_legacy_aperf_stats_raw_data(
    raw_aperf_stats_file: &File,
    time_series_data_processor: &mut TimeSeriesDataProcessor,
) {
    let mut values = Vec::new();
    loop {
        match bincode::deserialize_from::<_, AperfStat>(raw_aperf_stats_file) {
            Ok(v) => values.push(v),
            Err(e) => match *e {
                // EOF
                bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                e => panic!("Error when Deserializing APerf Stats data: {}", e),
            },
        };
    }

    for value in values {
        time_series_data_processor.proceed_to_time(value.time);

        for (stat_key, stat_value) in value.data {
            let stat_key_components: Vec<&str> = stat_key.split('-').collect();
            let data_name = stat_key_components[0];
            let mut stat_name = stat_key_components.get(1).unwrap_or(&data_name).to_string();
            // Backward compatibility - the previous stat name was "print"
            if stat_name == "print" {
                stat_name = "write".to_string();
            }

            time_series_data_processor.add_data_point(data_name, &stat_name, stat_value as f64);
        }
    }
}

impl ProcessData for AperfStats {
    fn compatible_filenames(&self) -> Vec<&str> {
        vec!["aperf_run_stats"]
    }

    fn process_raw_data(
        &mut self,
        report_params: &ReportParams,
        _raw_data: Vec<Data>,
    ) -> Result<AperfData> {
        let mut time_series_data_processor =
            time_series_data_processor_with_sum_aggregate!(report_params.collection_start);
        time_series_data_processor.set_aggregate_series_name("total");

        let (raw_aperf_stats_file, _) = match self.get_raw_data_file(&report_params.run_data_dir) {
            Ok(rs) => rs,
            Err(e) => bail!("Failed to open raw APerf Stats file: {:?}", e),
        };

        // Version check - APerf process pids started being collected in the same
        // version as the new APerf stats.
        if report_params.aperf_process_pids.is_empty() {
            process_legacy_aperf_stats_raw_data(
                &raw_aperf_stats_file,
                &mut time_series_data_processor,
            );
            return Ok(AperfData::TimeSeries(
                time_series_data_processor.get_time_series_data_sorted_by_average(),
            ));
        }

        let mut values = Vec::new();
        loop {
            match bincode::deserialize_from::<_, AperfStats>(&raw_aperf_stats_file) {
                Ok(v) => values.push(v),
                Err(e) => match *e {
                    // EOF
                    bincode::ErrorKind::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break
                    }
                    e => panic!("Error when Deserializing APerf Stats data: {}", e),
                },
            };
        }

        let mut collection_started = false;
        for value in values {
            // Ignore the time diff for data before the collection started, as time_diff
            // computation would have been corrupted and these data are not time-series anyway.
            if !collection_started
                && report_params
                    .collection_start
                    .map_or(true, |collection_start_time| {
                        value.time >= collection_start_time
                    })
            {
                collection_started = true;
            }
            if collection_started {
                time_series_data_processor.proceed_to_time(value.time);
            }

            for (stat_name, stat_data) in value.stats {
                for (data_name, stat_value) in stat_data {
                    time_series_data_processor.add_data_point(&stat_name, &data_name, stat_value);
                }
            }
        }

        let mut metric_name_orders: Vec<String> = ProcessMetric::iter()
            .map(|process_metric| process_metric.to_aperf_stat_metric_name())
            .collect();
        metric_name_orders.push("prepare".to_string());
        metric_name_orders.push("finish".to_string());
        metric_name_orders.push("aperf".to_string());
        let time_series_data = time_series_data_processor
            .get_time_series_data_with_metric_name_order(
                metric_name_orders.iter().map(String::as_str).collect(),
            );
        Ok(AperfData::TimeSeries(time_series_data))
    }
}
