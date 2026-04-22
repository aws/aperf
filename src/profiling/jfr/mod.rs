//! JFR (Java Flight Recorder) binary format parser.
//!
//! Provides [`JfrReader`] for iterating over chunks and events in JFR files
//! produced by async-profiler, and [`jfr_to_profiler`] for converting
//! them into APerf's [`Profiler`](crate::data::common::data_formats::Profiler) format.

mod convert;
mod reader;
mod types;

pub use convert::{format_jfr, jfr_to_profiler, parse_jfr_metadata};

pub use reader::JfrReader;
pub use types::*;
