use anyhow::{bail, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use super::types::*;

const CHUNK_HEADER_SIZE: usize = 68;
const CHUNK_SIGNATURE: u32 = 0x464c5200;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
enum State {
    #[default]
    NewChunk,
    Reading,
    Eof,
    Incomplete,
}

/// JFR binary format reader. Iterates chunk-by-chunk, event-by-event.
/// Based on async-profiler implementation: https://github.com/async-profiler/async-profiler/tree/master
/// src/converter/one/jfr
#[derive(Default)]
pub struct JfrReader {
    data: Vec<u8>,
    pos: usize,

    pub start_nanos: i64,
    pub end_nanos: i64,
    pub chunk_info: ChunkInfo,

    // Constant pools — public for consumers to resolve IDs
    pub types: HashMap<i64, JfrClass>,
    pub types_by_name: HashMap<String, JfrClass>,
    pub threads: HashMap<i64, String>,
    pub java_thread_ids: HashMap<i64, i64>,
    pub classes: HashMap<i64, ClassRef>,
    pub strings: HashMap<i64, String>,
    pub symbols: HashMap<i64, Vec<u8>>,
    pub methods: HashMap<i64, MethodRef>,
    pub stack_traces: HashMap<i64, StackTrace>,
    pub settings: HashMap<String, String>,
    pub enums: HashMap<String, HashMap<i32, String>>,

    // Cached event type IDs for this chunk
    execution_sample: i32,
    native_method_sample: i32,
    wall_clock_sample: i32,
    method_trace: i32,
    alloc_in_new_tlab: i32,
    alloc_outside_tlab: i32,
    alloc_sample: i32,
    live_object: i32,
    monitor_enter: i32,
    thread_park: i32,
    active_setting: i32,
    malloc: i32,
    free: i32,
    cpu_time_sample: i32,
    native_lock: i32,
    has_wall_time_span: bool,

    state: State,
    chunk_end: usize, // absolute position of current chunk end
}

// Public API and Event Readers
impl JfrReader {
    /// Opens a JFR file at `path`, reads it into memory, and parses the first chunk
    /// (header, metadata, and constant pools). Returns a ready-to-iterate reader.
    pub fn open(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Self::from_bytes(data)
    }

    /// Creates a [`JfrReader`] from raw JFR bytes and parses the first chunk.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        let mut reader = JfrReader::default();
        reader.data = data;
        reader.start_nanos = i64::MAX;
        reader.end_nanos = i64::MIN;
        reader.chunk_info.ticks_per_sec = 1;
        reader.execution_sample = -1;
        reader.native_method_sample = -1;
        reader.wall_clock_sample = -1;
        reader.method_trace = -1;
        reader.alloc_in_new_tlab = -1;
        reader.alloc_outside_tlab = -1;
        reader.alloc_sample = -1;
        reader.live_object = -1;
        reader.monitor_enter = -1;
        reader.thread_park = -1;
        reader.active_setting = -1;
        reader.malloc = -1;
        reader.free = -1;
        reader.cpu_time_sample = -1;
        reader.native_lock = -1;

        if !reader.read_chunk()? {
            bail!("Incomplete JFR file");
        }
        Ok(reader)
    }

    /// Returns `true` if there are more chunks to read. When the current chunk
    /// is exhausted (i.e., [`read_event`](Self::read_event) returned `EndOfChunk`),
    /// call this to advance to the next chunk.
    pub fn has_more_chunks(&mut self) -> Result<bool> {
        if self.state == State::NewChunk {
            self.read_chunk()
        } else {
            Ok(self.state == State::Reading)
        }
    }

    /// Returns the next event in the current chunk, or `EndOfChunk` when the chunk
    /// is exhausted. Internal events (e.g., `ActiveSetting`) are consumed
    /// silently and not returned. Call [`has_more_chunks`](Self::has_more_chunks)
    /// after `EndOfChunk` to advance to the next chunk.
    pub fn read_event(&mut self) -> Result<JfrEvent> {
        loop {
            if self.pos + 4 > self.data.len() || self.pos >= self.chunk_end {
                if self.pos < self.data.len() {
                    self.state = State::NewChunk;
                } else {
                    self.state = State::Eof;
                }
                return Ok(JfrEvent::EndOfChunk);
            }

            let event_start = self.pos;
            let size = self.get_varint() as usize;
            let type_id = self.get_varint();

            if size == 0 {
                bail!("Corrupted JFR: invalid event size");
            }

            // Check for chunk boundary
            if type_id == 'L' as i32 && event_start + 4 <= self.data.len() {
                let sig = u32::from_be_bytes([
                    self.data[event_start],
                    self.data[event_start + 1],
                    self.data[event_start + 2],
                    self.data[event_start + 3],
                ]);
                if sig == CHUNK_SIGNATURE {
                    self.pos = event_start;
                    self.state = State::NewChunk;
                    return Ok(JfrEvent::EndOfChunk);
                }
            }

            let event_end = event_start + size;

            let result = self.parse_event(type_id);
            self.pos = event_end; // always advance past event

            if let Some(event) = result {
                return Ok(event);
            }
            // Unknown event type or active setting — skip and continue
        }
    }

    fn parse_event(&mut self, type_id: i32) -> Option<JfrEvent> {
        if type_id == self.execution_sample {
            Some(JfrEvent::ExecutionSample(
                self.read_execution_sample(false, ExecSampleType::Execution),
            ))
        } else if type_id == self.native_method_sample {
            Some(JfrEvent::ExecutionSample(
                self.read_execution_sample(false, ExecSampleType::NativeMethod),
            ))
        } else if type_id == self.wall_clock_sample {
            Some(JfrEvent::ExecutionSample(
                self.read_execution_sample(true, ExecSampleType::WallClock),
            ))
        } else if type_id == self.method_trace {
            Some(JfrEvent::MethodTrace(self.read_method_trace()))
        } else if type_id == self.alloc_in_new_tlab {
            Some(JfrEvent::AllocationSample(
                self.read_allocation_sample(true),
            ))
        } else if type_id == self.alloc_outside_tlab || type_id == self.alloc_sample {
            Some(JfrEvent::AllocationSample(
                self.read_allocation_sample(false),
            ))
        } else if type_id == self.cpu_time_sample {
            Some(JfrEvent::ExecutionSample(self.read_cpu_time_sample()))
        } else if type_id == self.malloc {
            Some(JfrEvent::MallocEvent(self.read_malloc_event(true)))
        } else if type_id == self.free {
            Some(JfrEvent::MallocEvent(self.read_malloc_event(false)))
        } else if type_id == self.live_object {
            Some(JfrEvent::LiveObject(self.read_live_object()))
        } else if type_id == self.monitor_enter {
            Some(JfrEvent::ContendedLock(self.read_contended_lock(false)))
        } else if type_id == self.thread_park {
            Some(JfrEvent::ContendedLock(self.read_contended_lock(true)))
        } else if type_id == self.native_lock {
            Some(JfrEvent::NativeLock(self.read_native_lock_event()))
        } else if type_id == self.active_setting {
            self.read_active_setting();
            None
        } else {
            None
        }
    }

    // Event readers
    fn read_execution_sample(
        &mut self,
        wall: bool,
        sample_type: ExecSampleType,
    ) -> ExecutionSample {
        let time = self.get_varlong();
        let tid = self.get_varint();
        let stack_trace_id = self.get_varint();
        let thread_state = self.get_varint();
        let samples = if wall { self.get_varint() } else { 1 };
        if wall && self.has_wall_time_span {
            self.get_varlong(); // timeSpan ignored
        }
        ExecutionSample {
            time,
            tid,
            stack_trace_id,
            thread_state,
            samples,
            sample_type,
        }
    }

    fn read_method_trace(&mut self) -> MethodTraceEvent {
        let time = self.get_varlong();
        let duration = self.get_varlong();
        let tid = self.get_varint();
        let stack_trace_id = self.get_varint();
        let method = self.get_varint();
        MethodTraceEvent {
            time,
            tid,
            stack_trace_id,
            method,
            duration,
        }
    }

    fn read_allocation_sample(&mut self, tlab: bool) -> AllocationSample {
        let time = self.get_varlong();
        let tid = self.get_varint();
        let stack_trace_id = self.get_varint();
        let class_id = self.get_varint();
        let allocation_size = self.get_varlong();
        let tlab_size = if tlab { self.get_varlong() } else { 0 };
        AllocationSample {
            time,
            tid,
            stack_trace_id,
            class_id,
            allocation_size,
            tlab_size,
        }
    }

    fn read_cpu_time_sample(&mut self) -> ExecutionSample {
        let time = self.get_varlong();
        let stack_trace_id = self.get_varint();
        let tid = self.get_varint();
        let _failed = self.get_byte() != 0;
        let _sampling_period = self.get_varlong();
        let _biased = self.get_byte() != 0;
        ExecutionSample {
            time,
            tid,
            stack_trace_id,
            thread_state: 254,
            samples: 1,
            sample_type: ExecSampleType::CpuTime,
        }
    }

    fn read_malloc_event(&mut self, has_size: bool) -> MallocEvent {
        let time = self.get_varlong();
        let tid = self.get_varint();
        let stack_trace_id = self.get_varint();
        let address = self.get_varlong();
        let size = if has_size { self.get_varlong() } else { 0 };
        MallocEvent {
            time,
            tid,
            stack_trace_id,
            address,
            size,
        }
    }

    fn read_live_object(&mut self) -> LiveObject {
        let time = self.get_varlong();
        let tid = self.get_varint();
        let stack_trace_id = self.get_varint();
        let class_id = self.get_varint();
        let allocation_size = self.get_varlong();
        let allocation_time = self.get_varlong();
        LiveObject {
            time,
            tid,
            stack_trace_id,
            class_id,
            allocation_size,
            allocation_time,
        }
    }

    fn read_contended_lock(&mut self, has_timeout: bool) -> ContendedLock {
        let time = self.get_varlong();
        let duration = self.get_varlong();
        let tid = self.get_varint();
        let stack_trace_id = self.get_varint();
        let class_id = self.get_varint();
        if has_timeout {
            self.get_varlong();
        }
        let _until = self.get_varlong();
        let _address = self.get_varlong();
        ContendedLock {
            time,
            tid,
            stack_trace_id,
            duration,
            class_id,
        }
    }

    fn read_native_lock_event(&mut self) -> NativeLockEvent {
        let time = self.get_varlong();
        let duration = self.get_varlong();
        let tid = self.get_varint();
        let stack_trace_id = self.get_varint();
        let address = self.get_varlong();
        NativeLockEvent {
            time,
            tid,
            stack_trace_id,
            address,
            duration,
        }
    }

    fn read_active_setting(&mut self) {
        // Skip fields until we reach "id", then read name + value strings
        if let Some(cls) = self.types_by_name.get("jdk.ActiveSetting").cloned() {
            for field in &cls.fields {
                self.get_varlong();
                if field.name == "id" {
                    break;
                }
            }
        }
        let name = self.get_string();
        let value = self.get_string();
        if let (Some(n), Some(v)) = (name, value) {
            self.settings.insert(n, v);
        }
    }
}

// Chunk and Constant Pool Parsing
impl JfrReader {
    /// Reads the chunk header, metadata and constant pool. Then sets the read pointer
    /// to the start of the events and sets state to Reading.
    ///
    /// JFR Chunk:
    /// ```text
    /// ┌─── Header ──────────────────────┐
    /// │ signature        u32            │
    /// │ version          u32            │
    /// │ chunkSize        i64            │
    /// │ cpOffset         i64            │
    /// │ metaOffset       i64            │
    /// │ chunkStartNanos  i64            │
    /// │ durationNanos    i64            │
    /// │ chunkStartTicks  i64            │
    /// │ ticksPerSec      i64            │
    /// ├─── Metadata ────────────────────┤
    /// │ size             varint         │
    /// │ type             varint         │
    /// │ startTicks       varlong        │
    /// │ durationTicks    varlong        │
    /// │ metadataId       varlong        │
    /// │ stringCount      varint         │
    /// │ strings          string*        │  ← interned string table used as
    /// │                                 │    keys/values in element attributes
    /// │ elements         element*       │  ← recursive tree of typed nodes
    /// │                                 │    that describe the schema of events
    /// │                                 │    and constant pool entries in this
    /// │                                 │    chunk. We only process class and
    /// │                                 │    field elements. Others:[root, region,
    /// │                                 │    metadata, annotation, setting]
    /// ├─── Constant Pool ───────────────┤
    /// │ size             varint         │
    /// │ type             varint         │
    /// │ startTicks       varlong        │
    /// │ durationTicks    varlong        │
    /// │ delta            varlong        │
    /// │ flush            varint         │
    /// │ poolCount        varint         │
    /// │ pools            pool*          │  ← id-keyed lookup tables for shared
    /// │                                 │    objects: threads, classes, strings,
    /// │                                 │    symbols, methods, stack traces, and
    /// │                                 │    enum values. Events reference these
    /// │                                 │    by id to avoid duplication.
    /// ├─── Events ──────────────────────┤
    /// │ events           event*         │
    /// └─────────────────────────────────┘
    /// ```
    fn read_chunk(&mut self) -> Result<bool> {
        if self.pos + CHUNK_HEADER_SIZE > self.data.len() {
            bail!("Not a valid JFR file");
        }

        let sig = self.get_u32_be_at(self.pos);
        if sig != CHUNK_SIGNATURE {
            bail!("Not a valid JFR file");
        }

        let version = self.get_u32_be_at(self.pos + 4);
        if version < 0x20000 || version > 0x2ffff {
            bail!(
                "Unsupported JFR version: {}.{}",
                version >> 16,
                version & 0xffff
            );
        }

        let chunk_start = self.pos;
        let chunk_size = self.get_i64_be_at(self.pos + 8) as usize;
        if chunk_start + chunk_size > self.data.len() {
            self.state = State::Incomplete;
            return Ok(false);
        }

        let cp_offset = self.get_i64_be_at(self.pos + 16) as usize;
        let meta_offset = self.get_i64_be_at(self.pos + 24) as usize;
        if cp_offset == 0 || meta_offset == 0 {
            self.state = State::Incomplete;
            return Ok(false);
        }

        let chunk_start_nanos = self.get_i64_be_at(self.pos + 32);
        let duration_nanos = self.get_i64_be_at(self.pos + 40);
        let chunk_start_ticks = self.get_i64_be_at(self.pos + 48);
        let ticks_per_sec = self.get_i64_be_at(self.pos + 56);

        self.chunk_info = ChunkInfo {
            start_nanos: chunk_start_nanos,
            end_nanos: chunk_start_nanos + duration_nanos,
            start_ticks: chunk_start_ticks,
            ticks_per_sec,
        };

        self.start_nanos = self.start_nanos.min(chunk_start_nanos);
        self.end_nanos = self.end_nanos.max(chunk_start_nanos + duration_nanos);

        self.chunk_end = chunk_start + chunk_size;

        self.types.clear();
        self.types_by_name.clear();

        self.read_meta(chunk_start + meta_offset)?;
        self.read_constant_pool(chunk_start + cp_offset)?;
        self.cache_event_types();

        self.pos = chunk_start + CHUNK_HEADER_SIZE;
        self.state = State::Reading;
        Ok(true)
    }

    /// Reads chunk specific metadata and elements.
    /// Parses the metadata event at `offset`. Reads the interned string table,
    /// then walks the element tree via [`read_element`](Self::read_element) to
    /// populate [`types`](Self::types) and [`types_by_name`](Self::types_by_name).
    fn read_meta(&mut self, offset: usize) -> Result<()> {
        self.pos = offset;
        let _size = self.get_varint();
        let _type = self.get_varint();
        let _start_ticks = self.get_varlong();
        let _duration_ticks = self.get_varlong();
        let _metadata_id = self.get_varlong();

        let string_count = self.get_varint() as usize;
        let mut meta_strings = Vec::with_capacity(string_count);
        for _ in 0..string_count {
            meta_strings.push(self.get_string().unwrap_or_default());
        }
        self.read_element(&meta_strings);
        Ok(())
    }

    /// Reads a serialized tree of N metadata elements, each comprising attributes
    /// (key/value pairs) and sub-elements.
    /// ```text
    /// Root
    /// ├── Metadata
    /// │   ├── Class 1
    /// │   ├── Class 2
    /// │   │   ...
    /// │   └── Class N
    /// │       ├── Annotation 1 ... N
    /// │       ├── Setting 1 ... N
    /// │       └── Field 1 ... N
    /// │           └── Annotation 1 ... N
    /// └── Region
    /// ```
    fn read_element(&mut self, strings: &[String]) -> Element {
        let name_idx = self.get_varint() as usize;
        let name = strings.get(name_idx).cloned().unwrap_or_default();

        let attr_count = self.get_varint() as usize;
        let mut attributes = HashMap::with_capacity(attr_count);
        for _ in 0..attr_count {
            let k = strings
                .get(self.get_varint() as usize)
                .cloned()
                .unwrap_or_default();
            let v = strings
                .get(self.get_varint() as usize)
                .cloned()
                .unwrap_or_default();
            attributes.insert(k, v);
        }

        let child_count = self.get_varint() as usize;
        let mut children = Vec::with_capacity(child_count);
        for _ in 0..child_count {
            children.push(self.read_element(strings));
        }

        // Create types/fields from metadata
        match name.as_str() {
            "class" => {
                let id: i32 = attributes
                    .get("id")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let type_name = attributes.get("name").cloned().unwrap_or_default();
                let simple_type = attributes
                    .get("simpleType")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let has_super = attributes.contains_key("superType");

                let mut fields = Vec::new();
                for child in &children {
                    if let Element::Field(f) = child {
                        fields.push(f.clone());
                    }
                }

                let cls = JfrClass {
                    id,
                    name: type_name.clone(),
                    simple_type,
                    fields,
                };
                if !has_super {
                    self.types.insert(id as i64, cls.clone());
                }
                self.types_by_name.insert(type_name, cls);
            }
            "field" => {
                let field = JfrField {
                    name: attributes.get("name").cloned().unwrap_or_default(),
                    type_id: attributes
                        .get("class")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                    constant_pool: attributes
                        .get("constantPool")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                };
                return Element::Field(field);
            }
            _ => {}
        }
        Element::Other
    }

    /// Reads the constant pools from a JFR chunk.
    /// Constant pools are stored as a list of pool deltas within the
    /// chunk. Each node contains:
    /// - A header (size, type, start ticks, duration, delta, flush)
    /// - N pools, each identified by a type ID and containing entries
    ///   parsed according to the type's class definition from metadata
    fn read_constant_pool(&mut self, mut cp_offset: usize) -> Result<()> {
        loop {
            self.pos = cp_offset;
            let _size = self.get_varint();
            let _type = self.get_varint();
            let _start_ticks = self.get_varlong();
            let _duration = self.get_varlong();
            let delta = self.get_varlong();
            let _flush = self.get_varint();

            let pool_count = self.get_varint();
            for _ in 0..pool_count {
                let type_id = self.get_varint();
                if let Some(cls) = self.types.get(&(type_id as i64)).cloned() {
                    self.read_constants(&cls);
                } else {
                    // Unknown type — skip its count entries
                    let count = self.get_varint();
                    for _ in 0..count {
                        self.get_varlong(); // skip id + unknown fields
                    }
                }
            }

            if delta == 0 {
                break;
            }
            cp_offset = (cp_offset as i64 + delta) as usize;
        }
        Ok(())
    }

    fn read_constants(&mut self, cls: &JfrClass) {
        match cls.name.as_str() {
            "jdk.types.ChunkHeader" => {
                self.pos += CHUNK_HEADER_SIZE + 3;
            }
            "java.lang.Thread" => self.read_threads(cls.fields.len()),
            "java.lang.Class" => self.read_classes(cls.fields.len()),
            "java.lang.String" => self.read_strings(),
            "jdk.types.Symbol" => self.read_symbols(),
            "jdk.types.Method" => self.read_methods(),
            "jdk.types.StackTrace" => self.read_stack_traces(),
            _ => {
                if cls.simple_type && cls.fields.len() == 1 {
                    self.read_enum_values(&cls.name.clone());
                } else {
                    self.read_other_constants(cls);
                }
            }
        }
    }

    fn read_threads(&mut self, field_count: usize) {
        let count = self.get_varint() as usize;
        self.threads.reserve(count);
        self.java_thread_ids.reserve(count);
        for _ in 0..count {
            let id = self.get_varlong();
            let os_name = self.get_string();
            let _os_thread_id = self.get_varint();
            let java_name = self.get_string();
            let java_thread_id = self.get_varlong();
            // Skip remaining fields
            for _ in 0..field_count.saturating_sub(4) {
                self.get_varlong();
            }
            let name = java_name.or(os_name).unwrap_or_default();
            self.threads.insert(id, name);
            self.java_thread_ids.insert(id, java_thread_id);
        }
    }

    fn read_classes(&mut self, field_count: usize) {
        let count = self.get_varint() as usize;
        self.classes.reserve(count);
        for _ in 0..count {
            let id = self.get_varlong();
            let _loader = self.get_varlong();
            let name = self.get_varlong();
            let _pkg = self.get_varlong();
            let _modifiers = self.get_varint();
            for _ in 0..field_count.saturating_sub(4) {
                self.get_varlong();
            }
            self.classes.insert(id, ClassRef { name });
        }
    }

    fn read_strings(&mut self) {
        let count = self.get_varint() as usize;
        self.strings.reserve(count);
        for _ in 0..count {
            let id = self.get_varlong();
            let s = self.get_string().unwrap_or_default();
            self.strings.insert(id, s);
        }
    }

    fn read_symbols(&mut self) {
        let count = self.get_varint() as usize;
        self.symbols.reserve(count);
        for _ in 0..count {
            let id = self.get_varlong();
            let _encoding = self.get_byte();
            let bytes = self.get_bytes();
            self.symbols.insert(id, bytes);
        }
    }

    fn read_methods(&mut self) {
        let count = self.get_varint() as usize;
        self.methods.reserve(count);
        for _ in 0..count {
            let id = self.get_varlong();
            let cls = self.get_varlong();
            let name = self.get_varlong();
            let sig = self.get_varlong();
            let _modifiers = self.get_varint();
            let _hidden = self.get_varint();
            self.methods.insert(id, MethodRef { cls, name, sig });
        }
    }

    fn read_stack_traces(&mut self) {
        let count = self.get_varint() as usize;
        self.stack_traces.reserve(count);
        for _ in 0..count {
            let id = self.get_varlong();
            let _truncated = self.get_varint();
            let depth = self.get_varint() as usize;
            let mut methods = Vec::with_capacity(depth);
            let mut types = Vec::with_capacity(depth);
            let mut locations = Vec::with_capacity(depth);
            for _ in 0..depth {
                methods.push(self.get_varlong());
                let line = self.get_varint();
                let bci = self.get_varint();
                locations.push((line << 16) | (bci & 0xffff));
                types.push(self.get_byte());
            }
            self.stack_traces.insert(
                id,
                StackTrace {
                    methods,
                    types,
                    locations,
                },
            );
        }
    }

    fn read_enum_values(&mut self, type_name: &str) {
        let count = self.get_varint() as usize;
        let mut map = HashMap::with_capacity(count);
        for _ in 0..count {
            let key = self.get_varlong() as i32;
            let value = self.get_string().unwrap_or_default();
            map.insert(key, value);
        }
        self.enums.insert(type_name.to_string(), map);
    }

    fn read_other_constants(&mut self, cls: &JfrClass) {
        let string_type_id = self.get_type_id("java.lang.String");
        let numeric: Vec<bool> = cls
            .fields
            .iter()
            .map(|f| f.constant_pool || f.type_id != string_type_id)
            .collect();
        let count = self.get_varint() as usize;
        for _ in 0..count {
            self.get_varlong(); // id
            for &is_numeric in &numeric {
                if is_numeric {
                    self.get_varlong();
                } else {
                    self.get_string();
                }
            }
        }
    }

    fn cache_event_types(&mut self) {
        self.execution_sample = self.get_type_id("jdk.ExecutionSample");
        self.native_method_sample = self.get_type_id("jdk.NativeMethodSample");
        self.wall_clock_sample = self.get_type_id("profiler.WallClockSample");
        self.method_trace = self.get_type_id("jdk.MethodTrace");
        self.alloc_in_new_tlab = self.get_type_id("jdk.ObjectAllocationInNewTLAB");
        self.alloc_outside_tlab = self.get_type_id("jdk.ObjectAllocationOutsideTLAB");
        self.alloc_sample = self.get_type_id("jdk.ObjectAllocationSample");
        self.live_object = self.get_type_id("profiler.LiveObject");
        self.monitor_enter = self.get_type_id("jdk.JavaMonitorEnter");
        self.thread_park = self.get_type_id("jdk.ThreadPark");
        self.active_setting = self.get_type_id("jdk.ActiveSetting");
        self.malloc = self.get_type_id("profiler.Malloc");
        self.free = self.get_type_id("profiler.Free");
        self.cpu_time_sample = self.get_type_id("jdk.CPUTimeSample");
        self.native_lock = self.get_type_id("profiler.NativeLock");

        self.has_wall_time_span = self
            .types_by_name
            .get("profiler.WallClockSample")
            .and_then(|c| c.field("timeSpan"))
            .is_some();
    }

    fn get_type_id(&self, name: &str) -> i32 {
        self.types_by_name.get(name).map(|c| c.id).unwrap_or(-1)
    }
}

// Primitive Decoders
// Helper functions to decode varints, strings, etc. at the current pointer.
impl JfrReader {
    pub fn get_varint(&mut self) -> i32 {
        let mut result: i32 = 0;
        let mut shift = 0u32;
        loop {
            let b = self.data[self.pos] as i8;
            self.pos += 1;
            result |= ((b & 0x7f) as i32) << shift;
            if b >= 0 {
                return result;
            }
            shift += 7;
        }
    }

    pub fn get_varlong(&mut self) -> i64 {
        let mut result: i64 = 0;
        let mut shift = 0u32;
        while shift < 56 {
            let b = self.data[self.pos] as i8;
            self.pos += 1;
            result |= ((b & 0x7f) as i64) << shift;
            if b >= 0 {
                return result;
            }
            shift += 7;
        }
        let b = self.data[self.pos];
        self.pos += 1;
        result | ((b as i64) << 56)
    }

    pub fn get_byte(&mut self) -> u8 {
        let b = self.data[self.pos];
        self.pos += 1;
        b
    }

    pub fn get_float(&mut self) -> f32 {
        let bytes: [u8; 4] = self.data[self.pos..self.pos + 4].try_into().unwrap();
        self.pos += 4;
        f32::from_be_bytes(bytes)
    }

    pub fn get_string(&mut self) -> Option<String> {
        let encoding = self.get_byte();
        match encoding {
            0 => None,
            1 => Some(String::new()),
            2 => {
                let id = self.get_varlong();
                self.strings.get(&id).cloned()
            }
            3 => {
                let bytes = self.get_bytes();
                Some(String::from_utf8_lossy(&bytes).into_owned())
            }
            4 => {
                let len = self.get_varint() as usize;
                let mut chars = String::with_capacity(len);
                for _ in 0..len {
                    let c = self.get_varint() as u32;
                    if let Some(ch) = char::from_u32(c) {
                        chars.push(ch);
                    }
                }
                Some(chars)
            }
            5 => {
                let bytes = self.get_bytes();
                Some(bytes.iter().map(|&b| b as char).collect())
            }
            _ => None,
        }
    }

    pub fn get_bytes(&mut self) -> Vec<u8> {
        let len = self.get_varint() as usize;
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        bytes
    }

    fn get_u32_be_at(&self, pos: usize) -> u32 {
        u32::from_be_bytes(self.data[pos..pos + 4].try_into().unwrap())
    }

    fn get_i64_be_at(&self, pos: usize) -> i64 {
        i64::from_be_bytes(self.data[pos..pos + 8].try_into().unwrap())
    }
}

// Resolution Helper Methods
// This helps convert IDs to methods, symbols, etc from the constant pool
// and tracked by their respective maps.
impl JfrReader {
    // Resolve a method ID to (class_name, method_name, signature).
    pub fn resolve_method(&self, method_id: i64) -> Option<(String, String, String)> {
        let method_ref = self.methods.get(&method_id)?;
        let class_ref = self.classes.get(&method_ref.cls)?;

        let class_name = self.resolve_symbol(class_ref.name);
        let method_name = self.resolve_symbol(method_ref.name);
        let sig = self.resolve_symbol(method_ref.sig);

        Some((class_name, method_name, sig))
    }

    // Resolve a symbol ID — tries strings first, then symbols (byte arrays).
    pub fn resolve_symbol(&self, id: i64) -> String {
        if let Some(s) = self.strings.get(&id) {
            return s.clone();
        }
        if let Some(bytes) = self.symbols.get(&id) {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        format!("[unknown:{}]", id)
    }

    // Resolve a thread ID to its name.
    pub fn resolve_thread(&self, tid: i64) -> Option<String> {
        self.threads.get(&tid).cloned()
    }

    // Get the class name for a class constant pool ID.
    pub fn resolve_class(&self, class_id: i64) -> String {
        if let Some(class_ref) = self.classes.get(&class_id) {
            self.resolve_symbol(class_ref.name)
        } else {
            format!("[class:{}]", class_id)
        }
    }
}

// Internal element type for metadata parsing
enum Element {
    Field(JfrField),
    Other,
}
