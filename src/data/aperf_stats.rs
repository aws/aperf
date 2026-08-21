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

/// The resource usage of one subprocess that APerf launched and waited for, recorded
/// when it is reaped.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SubProcessUsage {
    name: String,
    start_time: TimeEnum,
    end_time: TimeEnum,
    /// Seconds of CPU time spent in user space over the whole run.
    user_time: f64,
    /// Seconds of CPU time spent in kernel space over the whole run.
    kernel_time: f64,
    /// Peak resident set size in bytes over the whole run.
    max_rss_bytes: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AperfStats {
    pub time: TimeEnum,
    /// The stats are stored in the format of Map<stat, Map<data, value>>.
    /// Each stat will be processed into a metric, and each data will be
    /// processed into a series within the metric.
    pub stats: HashMap<String, HashMap<String, f64>>,
    /// The usage of subprocesses that APerf started and reaped, the data
    /// within will be turned into visualizable time-series stats during
    /// raw data processing.
    sub_process_usages: Vec<SubProcessUsage>,
}

impl AperfStats {
    pub fn new() -> Self {
        Self::for_time(TimeEnum::DateTime(Utc::now()))
    }

    pub fn for_time(time: TimeEnum) -> Self {
        Self {
            time,
            stats: HashMap::new(),
            sub_process_usages: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.stats.is_empty() && self.sub_process_usages.is_empty()
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

    /// Record the resource usage of a subprocess that APerf started and reaped, along with
    /// the window it ran for.
    pub fn add_sub_process_usage(
        &mut self,
        process_name: &str,
        start_time: TimeEnum,
        end_time: TimeEnum,
        rusage: libc::rusage,
    ) {
        self.update_time_series();

        self.cur_aperf_stats
            .sub_process_usages
            .push(SubProcessUsage {
                name: process_name.to_string(),
                start_time,
                end_time,
                user_time: rusage.ru_utime.tv_sec as f64
                    + rusage.ru_utime.tv_usec as f64 / 1_000_000.0,
                kernel_time: rusage.ru_stime.tv_sec as f64
                    + rusage.ru_stime.tv_usec as f64 / 1_000_000.0,
                // ru_maxrss is in kilobytes on Linux.
                max_rss_bytes: rusage.ru_maxrss as f64 * 1024.0,
            });
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
        if !self.cur_aperf_stats.is_empty() {
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

/// Compute the share percentage that should be attributed to each second within
/// the time range (start_time_seconds, end_time_seconds), in the format of
/// [(second, shares of total for that second)] - all shares add up to 1.
fn per_second_shares(start_time_seconds: f64, end_time_seconds: f64) -> Vec<(u64, f64)> {
    let duration = end_time_seconds - start_time_seconds;

    if !duration.is_finite() || duration <= 0.0 {
        return vec![(end_time_seconds.ceil().max(0.0) as u64, 1.0)];
    }

    // A data point at second `n` covers the interval ending at it, which is what an accumulative
    // metric sampled once a second reports, so second `n` here means (n-1, n].
    let first_second = start_time_seconds.floor() as i64 + 1;
    let last_second = end_time_seconds.ceil() as i64;
    (first_second..=last_second)
        .map(|second| {
            let overlap =
                end_time_seconds.min(second as f64) - start_time_seconds.max((second - 1) as f64);
            (second.max(0) as u64, overlap / duration)
        })
        .collect()
}

/// Turn every recorded subprocess usage into a time-series data point for each second
/// it ran in.
fn spread_sub_process_usages_into_stats(
    all_aperf_stats: &mut Vec<AperfStats>,
    collection_start: DateTime<Utc>,
) {
    let sub_process_usages: Vec<SubProcessUsage> = all_aperf_stats
        .iter()
        .flat_map(|aperf_stats| aperf_stats.sub_process_usages.clone())
        .collect();
    if sub_process_usages.is_empty() {
        return;
    }

    // Find the index within all_aperf_stats for the stats at each second.
    let mut index_of_second: HashMap<u64, usize> = HashMap::new();
    for (index, aperf_stats) in all_aperf_stats.iter().enumerate() {
        if let TimeEnum::DateTime(_) = aperf_stats.time {
            if let TimeEnum::TimeDiff(second) =
                aperf_stats.time - TimeEnum::DateTime(collection_start)
            {
                index_of_second.insert(second, index);
            }
        }
    }

    for sub_process_usage in sub_process_usages {
        let (TimeEnum::DateTime(start_time), TimeEnum::DateTime(end_time)) =
            (sub_process_usage.start_time, sub_process_usage.end_time)
        else {
            continue;
        };
        let per_second_shares = per_second_shares(
            (start_time - collection_start).as_seconds_f64(),
            (end_time - collection_start).as_seconds_f64(),
        );

        for (second, share) in per_second_shares {
            let index = *index_of_second.entry(second).or_insert_with(|| {
                all_aperf_stats.push(AperfStats::for_time(TimeEnum::DateTime(
                    collection_start + chrono::Duration::seconds(second as i64),
                )));
                all_aperf_stats.len() - 1
            });
            let stats = &mut all_aperf_stats[index].stats;

            for (process_metric, cpu_seconds) in [
                (ProcessMetric::UserSpaceTime, sub_process_usage.user_time),
                (
                    ProcessMetric::KernelSpaceTime,
                    sub_process_usage.kernel_time,
                ),
            ] {
                *stats
                    .entry(process_metric.to_aperf_stat_metric_name())
                    .or_default()
                    .entry(sub_process_usage.name.clone())
                    .or_default() += cpu_seconds * share;
            }

            let peak_rss = stats
                .entry(ProcessMetric::ResidentSetSizeBytes.to_aperf_stat_metric_name())
                .or_default()
                .entry(sub_process_usage.name.clone())
                .or_default();
            *peak_rss = peak_rss.max(sub_process_usage.max_rss_bytes);
        }
    }

    all_aperf_stats.sort_by_key(|aperf_stats| aperf_stats.time);
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

        if let Some(collection_start) = report_params
            .collection_start
            .or_else(|| values.first().map(|aperf_stats| aperf_stats.time))
        {
            // Stats recorded before the collection started, such as the time spent preparing the
            // collectors, have no second of their own. Anchor them at the collection start so they
            // land at second 0.
            for aperf_stats in &mut values {
                if aperf_stats.time < collection_start {
                    aperf_stats.time = collection_start;
                }
            }
            if let TimeEnum::DateTime(collection_start) = collection_start {
                spread_sub_process_usages_into_stats(&mut values, collection_start);
            }
        }

        for value in values {
            time_series_data_processor.proceed_to_time(value.time);

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
