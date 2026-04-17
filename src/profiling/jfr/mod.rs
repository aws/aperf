//! JFR (Java Flight Recorder) binary format parser.
//!
//! Provides [`JfrReader`] for iterating over chunks and events in JFR files
//! produced by async-profiler, and [`jfr_to_profiler_data`] for converting
//! them into APerf's [`ProfilerData`](crate::data::common::data_formats::ProfilerData) format.

mod convert;
mod reader;
mod types;

pub use convert::{format_jfr, jfr_to_profiler_data, parse_jfr_metadata};

pub use reader::JfrReader;
pub use types::*;
