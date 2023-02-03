use crate::data::perf_stat::{NamedCtr, NamedTypeCtr, PerfType};

/// Graviton Events
static INSTRUCTIONS: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Instructions", config: 0x08};
static CYCLES: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Cycles", config: 0x11};
static FRONTEND_STALLS: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Frontend-Stalls", config: 0x23};
static BRANCHES: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Branches", config: 0x10};
static CODE_SPARSITY: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Code-Sparsity", config: 0x11c};
static INSTRUCTION_TLB: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Instruction-TLB", config: 0x2};
static INSTRUCTION_TLB_TW: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Instruction-TLB-TW", config: 0x35};
static L1_INSTRUCTIONS: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "L1-Instructions", config: 0x1};
static BACKEND_STALLS: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Backend-Stalls", config: 0x24};
static L3: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "L3", config: 0x37};
static L2: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "L2", config: 0x17};
static DATA_TLB: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Data-TLB", config: 0x5};
static DATA_TLB_TW: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "Data-TLB-TW", config: 0x34};
static L1_DATA: NamedTypeCtr = NamedTypeCtr {perf_type: PerfType::RAW, name: "L1-Data", config: 0x3};

lazy_static! {
    pub static ref PERF_LIST: Vec<NamedCtr<'static>> = [
        NamedCtr{name: "ipc", nrs: vec![INSTRUCTIONS], drs: vec![CYCLES], scale: 1},
        NamedCtr{name: "stall-frontend-pkc", nrs: vec![FRONTEND_STALLS], drs: vec![CYCLES], scale: 1000},
        NamedCtr{name: "branch-mpki", nrs: vec![BRANCHES], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "code-sparsity", nrs: vec![CODE_SPARSITY], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "inst-tlb-mpki", nrs: vec![INSTRUCTION_TLB], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "inst-tlb-tw-pki", nrs: vec![INSTRUCTION_TLB_TW], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "inst-l1-mpki", nrs: vec![L1_INSTRUCTIONS], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "stall-backend-pkc", nrs: vec![BACKEND_STALLS], drs: vec![CYCLES], scale: 1000},
        NamedCtr{name: "l3-mpki", nrs: vec![L3], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "l2-mpki", nrs: vec![L2], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "data-tlb-mpki", nrs: vec![DATA_TLB], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "data-tlb-tw-pki", nrs: vec![DATA_TLB_TW], drs: vec![INSTRUCTIONS], scale: 1000},
        NamedCtr{name: "data-l1-mpki", nrs: vec![L1_DATA], drs: vec![INSTRUCTIONS], scale: 1000},
    ].to_vec();
}
