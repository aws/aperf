use crate::data::perf_stat::{NamedCtr, NamedTypeCtr, PerfType};

static INSTRUCTIONS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instructions",
    config: 0xc0,
};
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
static L2: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L2",
    config: 0x1f25,
};
static INSTRUCTION_TLB: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instruction-TLB",
    config: 0x2011,
};
static INSTRUCTION_TLB_TW: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instruction-TLB-TW",
    config: 0x0e11,
};
static DATA_RD_TLB: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-RD-TLB",
    config: 0x2012,
};
static DATA_ST_TLB: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-ST-TLB",
    config: 0x2013,
};
static DATA_RD_TLB_TW: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-RD-TLB-TW",
    config: 0x0e12,
};
static DATA_ST_TLB_TW: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-ST-TLB-TW",
    config: 0x0e13,
};
static STALL_FRONTEND: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Frontend-Stalls",
    config: 0x600019c,
};
static STALL_BACKEND: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Backend-Stalls",
    config: 0x02a4,
};

lazy_static! {
    pub static ref SPR_CTRS: Vec<NamedCtr<'static>> = [
        NamedCtr {
            name: "l2-mpki",
            nrs: vec![L2],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "inst-tlb-mpki",
            nrs: vec![INSTRUCTION_TLB],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "inst-tlb-tw-pki",
            nrs: vec![INSTRUCTION_TLB_TW],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-rd-tlb-mpki",
            nrs: vec![DATA_RD_TLB],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-st-tlb-mpki",
            nrs: vec![DATA_ST_TLB],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-rd-tlb-tw-pki",
            nrs: vec![DATA_RD_TLB_TW],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-st-tlb-tw-pki",
            nrs: vec![DATA_ST_TLB_TW],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
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
