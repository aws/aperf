// Chunk Constant Pool Types
// Each chunk has a constant pool with references to strings, classes, and other metadata.
// This is used to store frequently referenced objects—such as strings, class names, and
// stack traces to optimize space and reduce overhead.
#[derive(Debug, Clone)]
pub struct ClassRef {
    pub name: i64, // index into strings
}

#[derive(Debug, Clone)]
pub struct MethodRef {
    pub cls: i64,  // index into classes
    pub name: i64, // index into strings/symbols
    pub sig: i64,  // index into strings/symbols
}

#[derive(Debug, Clone)]
pub struct StackTrace {
    pub methods: Vec<i64>,
    pub types: Vec<u8>,
    pub locations: Vec<i32>, // line << 16 | (bci & 0xffff)
}

// Chunk Metadata Types
// Each chunk has metadata to describe its position in the file, start/end time, id, etc.
// Additionally, we record two types of metadata elements fields and classes in these structs.
#[derive(Debug, Clone)]
pub struct JfrField {
    pub name: String,
    pub type_id: i32,
    pub constant_pool: bool,
}

#[derive(Debug, Clone)]
pub struct JfrClass {
    pub id: i32,
    pub name: String,
    pub simple_type: bool,
    pub fields: Vec<JfrField>,
}

impl JfrClass {
    pub fn field(&self, name: &str) -> Option<&JfrField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

// Chunk Timing Info
// This struct contains the timing information for the currently processed chunk,
// which is used to convert between different time units (nanoseconds, milliseconds, etc.)
// and to determine the start and end times of the chunk.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChunkInfo {
    pub start_nanos: i64,
    pub end_nanos: i64,
    pub start_ticks: i64,
    pub ticks_per_sec: i64,
}

impl ChunkInfo {
    pub fn event_time_to_nanos(&self, ticks: i64) -> i64 {
        let nanos_per_tick = 1_000_000_000.0 / self.ticks_per_sec as f64;
        self.start_nanos + ((ticks - self.start_ticks) as f64 * nanos_per_tick) as i64
    }

    pub fn event_time_to_millis(&self, ticks: i64) -> i64 {
        let ms_from_start = (ticks - self.start_ticks) * 1_000 / self.ticks_per_sec;
        self.start_nanos / 1_000_000 + ms_from_start
    }
}

// Sampling Events
// These are the JFR events collected by async-profiler. Each event contains different
// fields describing its information. Shared fields are time, tid, and stack_trace_id.
macro_rules! jfr_event {
    ( $( $variant:ident($type:ty) ),* $(,)? ; $( $unit:ident ),* $(,)? ) => {
        #[derive(Debug, Clone)]
        pub enum JfrEvent {
            $( $variant($type), )*
            $( $unit, )*
        }

        impl JfrEvent {
            pub fn time(&self) -> i64 {
                match self {
                    $( Self::$variant(e) => e.time, )*
                    $( Self::$unit => 0, )*
                }
            }
            pub fn tid(&self) -> i32 {
                match self {
                    $( Self::$variant(e) => e.tid, )*
                    $( Self::$unit => 0, )*
                }
            }
            pub fn stack_trace_id(&self) -> i32 {
                match self {
                    $( Self::$variant(e) => e.stack_trace_id, )*
                    $( Self::$unit => 0, )*
                }
            }
        }
    };
}

jfr_event!(
    // jdk.ExecutionSample / jdk.NativeMethodSample / profiler.WallClockSample / jdk.CPUTimeSample
    // CPU and wall-clock sampling of Java and native threads
    ExecutionSample(ExecutionSample),

    // jdk.ObjectAllocationInNewTLAB / jdk.ObjectAllocationOutsideTLAB
    // Heap allocation events with TLAB info
    AllocationSample(AllocationSample),

    // jdk.JavaMonitorEnter
    // Java monitor contention (synchronized block wait time)
    ContendedLock(ContendedLock),

    // profiler.LiveObject
    // Heap objects still alive at profiling time (for leak detection)
    LiveObject(LiveObject),

    // profiler.Malloc / profiler.Free
    // Native memory allocation and deallocation via malloc/free
    MallocEvent(MallocEvent),

    // jdk.MethodTrace
    // Method-level tracing with entry/exit duration
    MethodTrace(MethodTraceEvent),

    // profiler.NativeLock
    // Native (pthread) mutex contention
    NativeLock(NativeLockEvent),

    // User-defined custom events with raw payload
    Custom(CustomEvent),
    ;
    // Sentinel marking the end of a JFR chunk (not a real event)
    EndOfChunk,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecSampleType {
    Execution,
    NativeMethod,
    WallClock,
    CpuTime,
}

#[derive(Debug, Clone)]
pub struct ExecutionSample {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub thread_state: i32,
    pub samples: i32,
    pub sample_type: ExecSampleType,
}

#[derive(Debug, Clone)]
pub struct AllocationSample {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub class_id: i32,
    pub allocation_size: i64,
    pub tlab_size: i64,
}

#[derive(Debug, Clone)]
pub struct ContendedLock {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub duration: i64,
    pub class_id: i32,
}

#[derive(Debug, Clone)]
pub struct LiveObject {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub class_id: i32,
    pub allocation_size: i64,
    pub allocation_time: i64,
}

#[derive(Debug, Clone)]
pub struct MallocEvent {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub address: i64,
    pub size: i64,
}

#[derive(Debug, Clone)]
pub struct MethodTraceEvent {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub method: i32,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct NativeLockEvent {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub address: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct CustomEvent {
    pub time: i64,
    pub tid: i32,
    pub stack_trace_id: i32,
    pub raw_data: Vec<u8>,
}
