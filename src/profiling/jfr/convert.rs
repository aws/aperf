use crate::data::common::data_formats::{KeyValueData, ProfilerData};
use crate::profiling::jfr::{ExecSampleType, JfrEvent, JfrReader};
use crate::profiling::{FrameType, ThreadState};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

/// This converts a JFR to string output, and should be equivalent to jfr print function.
/// Only used for validating data parsing.
pub fn format_jfr(path: &Path) -> Result<String> {
    let mut out = String::new();
    let mut reader = JfrReader::open(path.to_str().unwrap())?;

    writeln!(out, "start_nanos: {}", reader.start_nanos)?;
    writeln!(out, "end_nanos: {}", reader.end_nanos)?;
    writeln!(
        out,
        "chunk ticks_per_sec: {}",
        reader.chunk_info.ticks_per_sec
    )?;
    writeln!(out, "threads: {}", reader.threads.len())?;
    writeln!(out, "methods: {}", reader.methods.len())?;
    writeln!(out, "stack_traces: {}", reader.stack_traces.len())?;
    writeln!(out, "classes: {}", reader.classes.len())?;
    writeln!(out, "strings: {}", reader.strings.len())?;
    writeln!(out, "enums: {:?}", reader.enums)?;
    writeln!(out, "settings: {:?}", reader.settings)?;
    writeln!(out, "---")?;

    loop {
        match reader.read_event() {
            Ok(JfrEvent::EndOfChunk) => {
                if !reader.has_more_chunks().unwrap_or(false) {
                    break;
                }
            }
            Ok(event) => {
                let time_nanos = reader.chunk_info.event_time_to_nanos(event.time());
                let secs = time_nanos / 1_000_000_000;
                let nanos_rem = (time_nanos % 1_000_000_000).unsigned_abs();
                let h = (secs / 3600) % 24;
                let m = (secs / 60) % 60;
                let s = secs % 60;
                let ms = nanos_rem / 1_000_000;
                let time_str = format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms);

                let tid = event.tid();
                let thread_name = reader.resolve_thread(tid as i64).unwrap_or_default();
                let java_tid = reader.java_thread_ids.get(&(tid as i64)).copied();

                match &event {
                    JfrEvent::ExecutionSample(e) => {
                        let state = reader
                            .enums
                            .get("jdk.types.ThreadState")
                            .and_then(|m| m.get(&e.thread_state))
                            .cloned()
                            .unwrap_or_else(|| format!("STATE_{}", e.thread_state));
                        let type_name = match e.sample_type {
                            ExecSampleType::Execution => "jdk.ExecutionSample",
                            ExecSampleType::NativeMethod => "jdk.NativeMethodSample",
                            ExecSampleType::WallClock => "profiler.WallClockSample",
                            ExecSampleType::CpuTime => "jdk.CPUTimeSample",
                        };
                        writeln!(out, "{} {{", type_name)?;
                        writeln!(out, "  startTime = {}", time_str)?;
                        write_thread_field(&mut out, "sampledThread", &thread_name, tid, java_tid)?;
                        writeln!(out, "  state = \"{}\"", state)?;
                        if e.sample_type == ExecSampleType::WallClock {
                            writeln!(out, "  samples = {}", e.samples)?;
                        }
                    }
                    JfrEvent::AllocationSample(e) => {
                        let cls = jvm_type_to_human(&reader.resolve_class(e.class_id as i64));
                        let type_name = if e.tlab_size > 0 {
                            "jdk.ObjectAllocationInNewTLAB"
                        } else {
                            "jdk.ObjectAllocationOutsideTLAB"
                        };
                        writeln!(out, "{} {{", type_name)?;
                        writeln!(out, "  startTime = {}", time_str)?;
                        writeln!(out, "  objectClass = {} (classLoader = null)", cls)?;
                        writeln!(
                            out,
                            "  allocationSize = {}",
                            format_bytes(e.allocation_size)
                        )?;
                        if e.tlab_size > 0 {
                            writeln!(out, "  tlabSize = {}", format_bytes(e.tlab_size))?;
                        }
                        write_thread_field(&mut out, "eventThread", &thread_name, tid, java_tid)?;
                    }
                    JfrEvent::ContendedLock(e) => {
                        let cls = jvm_type_to_human(&reader.resolve_class(e.class_id as i64));
                        writeln!(out, "jdk.JavaMonitorEnter {{")?;
                        writeln!(out, "  startTime = {}", time_str)?;
                        writeln!(out, "  duration = {} ns", e.duration)?;
                        writeln!(out, "  monitorClass = {}", cls)?;
                        write_thread_field(&mut out, "eventThread", &thread_name, tid, java_tid)?;
                    }
                    JfrEvent::LiveObject(e) => {
                        let cls = jvm_type_to_human(&reader.resolve_class(e.class_id as i64));
                        writeln!(out, "profiler.LiveObject {{")?;
                        writeln!(out, "  startTime = {}", time_str)?;
                        writeln!(out, "  objectClass = {}", cls)?;
                        writeln!(
                            out,
                            "  allocationSize = {}",
                            format_bytes(e.allocation_size)
                        )?;
                        writeln!(out, "  allocationTime = {}", e.allocation_time)?;
                    }
                    JfrEvent::MallocEvent(e) => {
                        let type_name = if e.size > 0 {
                            "profiler.Malloc"
                        } else {
                            "profiler.Free"
                        };
                        writeln!(out, "{} {{", type_name)?;
                        writeln!(out, "  startTime = {}", time_str)?;
                        writeln!(out, "  address = 0x{:x}", e.address)?;
                        if e.size > 0 {
                            writeln!(out, "  size = {}", format_bytes(e.size))?;
                        }
                    }
                    JfrEvent::MethodTrace(e) => {
                        writeln!(out, "jdk.MethodTrace {{")?;
                        writeln!(out, "  startTime = {}", time_str)?;
                        writeln!(out, "  duration = {} ns", e.duration)?;
                        writeln!(out, "  method = {}", e.method)?;
                    }
                    JfrEvent::NativeLock(e) => {
                        writeln!(out, "profiler.NativeLock {{")?;
                        writeln!(out, "  startTime = {}", time_str)?;
                        writeln!(out, "  duration = {} ns", e.duration)?;
                        writeln!(out, "  address = 0x{:x}", e.address)?;
                    }
                    JfrEvent::Custom(_) => {
                        writeln!(out, "CustomEvent {{")?;
                        writeln!(out, "  startTime = {}", time_str)?;
                    }
                    JfrEvent::EndOfChunk => unreachable!(),
                }

                if let Some(trace) = reader.stack_traces.get(&(event.stack_trace_id() as i64)) {
                    writeln!(out, "  stackTrace = [")?;
                    for (i, &method_id) in trace.methods.iter().enumerate() {
                        let loc = trace.locations[i];
                        let line = loc >> 16;
                        if let Some((cls, method, sig)) = reader.resolve_method(method_id) {
                            let sig_str = if sig.is_empty() || sig.starts_with("[unknown") {
                                String::new()
                            } else {
                                format_signature(&sig)
                            };
                            writeln!(
                                out,
                                "    {}.{}({}) line: {}",
                                cls.replace('/', "."),
                                method,
                                sig_str,
                                line
                            )?;
                        } else {
                            writeln!(out, "    [unknown:{}]() line: {}", method_id, line)?;
                        }
                    }
                    writeln!(out, "  ]")?;
                }

                writeln!(out, "}}")?;
                writeln!(out)?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(out)
}

fn write_thread_field(
    out: &mut String,
    field: &str,
    name: &str,
    tid: i32,
    java_tid: Option<i64>,
) -> std::fmt::Result {
    if name.is_empty() {
        return writeln!(out, "  {} = N/A", field);
    }
    if let Some(jtid) = java_tid {
        if jtid > 0 {
            return writeln!(out, "  {} = \"{}\" (javaThreadId = {})", field, name, jtid);
        }
    }
    writeln!(out, "  {} = \"{}\" (osThreadId = {})", field, name, tid)
}

fn jvm_type_to_human(name: &str) -> String {
    let mut s = name;
    let mut dims = 0;
    while s.starts_with('[') {
        dims += 1;
        s = &s[1..];
    }
    let base = match s {
        "B" if dims > 0 => "byte",
        "C" if dims > 0 => "char",
        "D" if dims > 0 => "double",
        "F" if dims > 0 => "float",
        "I" if dims > 0 => "int",
        "J" if dims > 0 => "long",
        "S" if dims > 0 => "short",
        "Z" if dims > 0 => "boolean",
        other => {
            let stripped = other
                .strip_prefix('L')
                .and_then(|s| s.strip_suffix(';'))
                .unwrap_or(other);
            return format!("{}{}", stripped.replace('/', "."), "[]".repeat(dims));
        }
    };
    format!("{}{}", base, "[]".repeat(dims))
}

fn round_half_up_1(v: f64) -> f64 {
    (v * 10.0 + 0.5).floor() / 10.0
}

fn format_bytes(bytes: i64) -> String {
    let b = bytes as f64;
    if b >= 1_048_576.0 {
        format!("{:.1} MB", round_half_up_1(b / 1_048_576.0))
    } else if b >= 1024.0 {
        format!("{:.1} kB", round_half_up_1(b / 1024.0))
    } else {
        format!("{} bytes", bytes)
    }
}

fn format_signature(sig: &str) -> String {
    if !sig.starts_with('(') {
        return String::new();
    }
    let end = sig.find(')').unwrap_or(sig.len());
    let params = &sig[1..end];
    let mut result = Vec::new();
    let mut chars = params.chars().peekable();
    while chars.peek().is_some() {
        result.push(parse_type(&mut chars));
    }
    result.join(", ")
}

fn parse_type(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    match chars.next() {
        Some('B') => "byte".into(),
        Some('C') => "char".into(),
        Some('D') => "double".into(),
        Some('F') => "float".into(),
        Some('I') => "int".into(),
        Some('J') => "long".into(),
        Some('S') => "short".into(),
        Some('Z') => "boolean".into(),
        Some('V') => "void".into(),
        Some('[') => format!("{}[]", parse_type(chars)),
        Some('L') => {
            let mut name = String::new();
            for c in chars.by_ref() {
                if c == ';' {
                    break;
                }
                name.push(if c == '/' { '.' } else { c });
            }
            name.rsplit('.').next().unwrap_or(&name).to_string()
        }
        _ => "?".into(),
    }
}

/// This function uses JfrReader to parse async-profiler generated JFR files into
/// APerf ProfilerData.
pub fn jfr_to_profiler_data(path: &Path, block_width_ms: u64) -> Result<ProfilerData> {
    let mut reader = JfrReader::open(path.to_str().unwrap())?;
    let start_time_ms = reader.start_nanos / 1_000_000;

    let frame_type_suffix: HashMap<u8, &str> = reader
        .enums
        .get("jdk.types.FrameType")
        .map(|m| {
            m.iter()
                .map(|(&k, v)| (k as u8, FrameType::from_jfr_name(v).literal_suffix()))
                .collect()
        })
        .unwrap_or_default();

    let thread_state_map = reader
        .enums
        .get("jdk.types.ThreadState")
        .cloned()
        .unwrap_or_default();

    let mut profiler_data = ProfilerData::new(start_time_ms, block_width_ms);

    loop {
        match reader.read_event() {
            Ok(JfrEvent::EndOfChunk) => {
                if !reader.has_more_chunks().unwrap_or(false) {
                    break;
                }
            }
            Ok(event) => {
                let sample_time_ms = reader.chunk_info.event_time_to_millis(event.time());

                let (profile_type, thread_state, samples) = match &event {
                    JfrEvent::ExecutionSample(e) => {
                        let ptype = match e.sample_type {
                            ExecSampleType::Execution
                            | ExecSampleType::NativeMethod
                            | ExecSampleType::CpuTime => "cpu",
                            ExecSampleType::WallClock => "wall",
                        };
                        let state_name = thread_state_map
                            .get(&e.thread_state)
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        (ptype, ThreadState::from_str(state_name), e.samples as u64)
                    }
                    JfrEvent::AllocationSample(_) => ("alloc", ThreadState::None, 1u64),
                    _ => continue,
                };

                if let Some(trace) = reader.stack_traces.get(&(event.stack_trace_id() as i64)) {
                    let mut frames: Vec<String> = trace
                        .methods
                        .iter()
                        .zip(trace.types.iter())
                        .rev()
                        .map(|(&mid, &ftype)| {
                            let suffix = frame_type_suffix.get(&ftype).copied().unwrap_or("");
                            if let Some((cls, method, _)) = reader.resolve_method(mid) {
                                if ftype == 3 || ftype == 4 || cls.is_empty() {
                                    format!("{}{}", method, suffix)
                                } else if method.is_empty() {
                                    format!("{}{}", cls.replace('/', "."), suffix)
                                } else {
                                    format!("{}.{}{}", cls.replace('/', "."), method, suffix)
                                }
                            } else {
                                format!("[unknown:{}]{}", mid, suffix)
                            }
                        })
                        .collect();

                    if let JfrEvent::AllocationSample(e) = &event {
                        let cls = jvm_type_to_human(&reader.resolve_class(e.class_id as i64));
                        frames.push(format!("{}{}", cls, FrameType::Inlined.literal_suffix()));
                    }

                    if !frames.is_empty() {
                        profiler_data.insert_stack(
                            profile_type,
                            sample_time_ms,
                            thread_state,
                            &frames,
                            samples,
                        );
                    }
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(profiler_data)
}

// Reference: https://github.com/async-profiler/async-profiler/blob/master/src/jfrMetadata.h#L46-L70
fn jfr_event_type_name(id: &str) -> Option<&'static str> {
    match id {
        "101" => Some("Execution Sample"),
        "102" => Some("Alloc in New TLAB"),
        "103" => Some("Alloc Outside TLAB"),
        "104" => Some("Monitor Enter"),
        "105" => Some("Thread Park"),
        "106" => Some("CPU Load"),
        "107" => Some("Active Recording"),
        "108" => Some("Active Setting"),
        "109" => Some("OS Information"),
        "110" => Some("CPU Information"),
        "111" => Some("JVM Information"),
        "112" => Some("Initial System Property"),
        "113" => Some("Native Library"),
        "114" => Some("GC Heap Summary"),
        "115" => Some("Method Trace"),
        "116" => Some("Log"),
        "117" => Some("Window"),
        "118" => Some("Live Object"),
        "119" => Some("Wall Clock Sample"),
        "120" => Some("Malloc"),
        "121" => Some("Free"),
        "122" => Some("User Event"),
        "123" => Some("Process Sample"),
        "124" => Some("Native Lock"),
        _ => None,
    }
}

/// Parse JFR metadata JSON into KeyValueData.
pub fn parse_jfr_metadata(metadata_json: &Value) -> KeyValueData {
    let mut key_value_data = KeyValueData::default();

    let Some(events) = metadata_json
        .get("recording")
        .and_then(|r| r.get("events"))
        .and_then(|e| e.as_array())
    else {
        return key_value_data;
    };

    let json_to_string = |v: &Value| match v {
        Value::String(s) => s.clone(),
        _ => v.to_string(),
    };

    for event in events {
        let Some(event_type) = event.get("type").and_then(|t| t.as_str()) else {
            continue;
        };
        let Some(values) = event.get("values").and_then(|v| v.as_object()) else {
            continue;
        };

        match event_type {
            "jdk.JVMInformation" | "jdk.ActiveRecording" => {
                let group = key_value_data
                    .key_value_groups
                    .entry(event_type.to_string())
                    .or_default();
                for (key, value) in values.iter().filter(|(k, _)| *k != "startTime") {
                    group.key_values.insert(key.clone(), json_to_string(value));
                }
            }
            "jdk.ActiveSetting" => {
                if let (Some(name), Some(id)) = (values.get("name"), values.get("id")) {
                    let event_id = json_to_string(id);
                    let event_name = jfr_event_type_name(&event_id)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Event ID: {}", event_id));
                    let value_str = values
                        .get("value")
                        .map(&json_to_string)
                        .unwrap_or_else(|| "null".to_string());
                    let setting = format!("{} : {}", name.as_str().unwrap_or(""), value_str);

                    let settings_group = key_value_data
                        .key_value_groups
                        .entry("jdk.ActiveSetting".to_string())
                        .or_default();
                    settings_group
                        .key_values
                        .entry(event_name)
                        .and_modify(|s| {
                            s.push('\n');
                            s.push_str(&setting);
                        })
                        .or_insert(setting);
                }
            }
            _ => {}
        }
    }

    key_value_data
}
