/// This module holds the shared logics or structures used by all data types, for data collection
/// or raw-data processing.
pub mod common_raw_data;
pub mod data_formats;
pub mod processed_data_accessor;
pub mod time_series_data_processor;
pub mod utils;

#[cfg(target_os = "linux")]
pub const THP_DIR: &str = "/sys/kernel/mm/transparent_hugepage";
#[cfg(target_os = "linux")]
pub const HUGETLB_DIR: &str = "/sys/kernel/mm/hugepages";
