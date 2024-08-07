use crate::data::perf_stat::{NamedCtr, NamedTypeCtr, PerfType};

static STALL_BACKEND_1: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Backend-Stalls-1",
    config: 0xf7ae,
};
static STALL_BACKEND_2: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Backend-Stalls-2",
    config: 0x27af,
};
static CYCLES: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Cycles",
    config: 0x0076,
};

lazy_static! {
    pub static ref MILAN_CTRS: Vec<NamedCtr<'static>> = [NamedCtr {
        name: "stall_backend",
        nrs: vec![STALL_BACKEND_1, STALL_BACKEND_2],
        drs: vec![CYCLES],
        scale: 1000
    }]
    .to_vec();
}
