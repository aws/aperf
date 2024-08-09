use crate::data::perf_stat::{NamedCtr, NamedTypeCtr, PerfType};

static STALL_BACKEND: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Backend-Stalls",
    config: 0x100001ea0,
};
static CYCLES: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Cycles",
    config: 0x0076,
};

lazy_static! {
    pub static ref GENOA_CTRS: Vec<NamedCtr<'static>> = [
        NamedCtr {
            name: "stall_backend_pkc",
            nrs: vec![STALL_BACKEND],
            drs: vec![CYCLES],
            scale: 167 //~= 1000/6
        },
    ]
    .to_vec();
}
