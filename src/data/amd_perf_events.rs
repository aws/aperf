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
static BRANCH_MISPRED: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Branch-Mispredictions",
    config: 0x00c3,
};
static L1_DATA_FILL: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L1-Data-Fills",
    config: 0xff44,
};
static L1_INSTRUCTION_MISS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L1-Instruction-Misses",
    config: 0x1060,
};
static L2_DEMAND_MISS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "L2-Demand-Misses",
    config: 0x0964,
};
static L1_ANY_FILLS_DRAM: NamedTypeCtr = NamedTypeCtr {
    // Approximately L3 Misses
    perf_type: PerfType::RAW,
    name: "L1-Any-Fills-DRAM",
    config: 0x0844,
};
static STALL_FRONTEND: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Frontend-Stalls",
    config: 0x00a9,
};
static INSTRUCTION_TLB_MISS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instruction-TLB-Misses",
    config: 0x0084,
};
static INSTRUCTION_TLB_TW_MISS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Instruction-TLB-TW-Misses",
    config: 0x0f85,
};
static DATA_TLB_MISS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-TLB-Misses",
    config: 0xff45,
};
static DATA_TLB_TW_MISS: NamedTypeCtr = NamedTypeCtr {
    perf_type: PerfType::RAW,
    name: "Data-TLB-TW-Misses",
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
            nrs: vec![BRANCH_MISPRED],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-l1-mpki",
            nrs: vec![L1_DATA_FILL],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "inst-l1-mpki",
            nrs: vec![L1_INSTRUCTION_MISS],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "l2-mpki",
            nrs: vec![L2_DEMAND_MISS],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "l3-mpki",
            nrs: vec![L1_ANY_FILLS_DRAM],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "stall_frontend_pkc",
            nrs: vec![STALL_FRONTEND],
            drs: vec![CYCLES],
            scale: 1000
        },
        NamedCtr {
            name: "inst-tlb-mpki",
            nrs: vec![INSTRUCTION_TLB_MISS],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "inst-tlb-tw-mpki",
            nrs: vec![INSTRUCTION_TLB_TW_MISS],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-tlb-mpki",
            nrs: vec![DATA_TLB_MISS],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
        NamedCtr {
            name: "data-tlb-tw-pki",
            nrs: vec![DATA_TLB_TW_MISS],
            drs: vec![INSTRUCTIONS],
            scale: 1000
        },
    ]
    .to_vec();
}
