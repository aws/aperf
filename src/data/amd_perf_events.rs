use crate::data::perf_stat::{NamedCtr, NamedTypeCtr, PerfType};

// amd events
static INSTRUCTIONS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instructions",
    config: 0x00c0,
};
static CYCLES: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Cycles",
    config: 0x0076,
};
static BRANCHES: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Branches",
    config: 0x00c3,
};
static L1_DATA: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L1-Data",
    config: 0xff44,
};
static L1_INSTRUCTIONS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L1-Instructions",
    config: 0x1060,
};
static L2: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L2",
    config: 0x0964,
};
static L3: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L3",
    config: 0x0843,
};
static STALL_FRONTEND_PKC: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Frontend-Stalls",
    config: 0x00a9,
};
static INSTRUCTION_TLB: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instruction-TLB",
    config: 0x0084,
};
static INSTRUCTION_TLB_TW: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instruction-TLB-TW",
    config: 0x0f85,
};
static DATA_TLB: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-TLB",
    config: 0xff45,
};
static DATA_TLB_TW: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-TLB-TW",
    config: 0xf045,
};

lazy_static! {
    pub static ref PERF_LIST: Vec<NamedCtr<'static>> = [
        NamedCtr {
            name: "ipc",
            nrs: vec![INSTRUCTIONS],
            drs: vec![CYCLES],
            scale: 1
        },
        NamedCtr {
            name: "branch-mpki",
            nrs: vec![BRANCHES],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-l1-mpki",
            nrs: vec![L1_DATA],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "inst-l1-mpki",
            nrs: vec![L1_INSTRUCTIONS],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "l2-mpki",
            nrs: vec![L2],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "l3-mpki",
            nrs: vec![L3],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "stall_frontend_pkc",
            nrs: vec![STALL_FRONTEND_PKC],
            drs: vec![CYCLES],
            scale: 1000
        },
        NamedCtr {
            name: "inst-tlb-mpki",
            nrs: vec![INSTRUCTION_TLB],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "inst-tlb-tw-mpki",
            nrs: vec![INSTRUCTION_TLB_TW],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-tlb-mpki",
            nrs: vec![DATA_TLB],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-tlb-tw-pki",
            nrs: vec![DATA_TLB_TW],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
    ]
    .to_vec();
}
