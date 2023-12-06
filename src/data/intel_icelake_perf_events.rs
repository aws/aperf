use crate::data::perf_stat::{NamedCtr, NamedTypeCtr, PerfType};

static CYCLES: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Cycles",
    config: 0x3c,
};
static SLOTS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Slots",
    config: 0x01a4,
};
static STALL_FRONTEND: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Frontend-Stalls",
    config: 0x500019c,
};
static STALL_BACKEND: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Backend-Stalls",
    config: 0x02a4,
};

lazy_static! {
    pub static ref ICX_CTRS: Vec<NamedCtr<'static>> = [
        NamedCtr {
            name: "stall-frontend-pkc",
            nrs: vec![STALL_FRONTEND],
            drs: vec![CYCLES],
            scale: 1000
        },
        NamedCtr {
            name: "stall-backend-pkc",
            nrs: vec![STALL_BACKEND],
            drs: vec![SLOTS],
            scale: 1000
        },
    ]
    .to_vec();
}
