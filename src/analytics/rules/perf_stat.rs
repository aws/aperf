use crate::analytics::rule_templates::{
    time_series_data_point_threshold_rule::time_series_data_point_threshold,
    time_series_stat_intra_run_comparison_rule::time_series_stat_intra_run_comparison,
    time_series_stat_run_comparison_rule::time_series_stat_run_comparison,
    time_series_stat_threshold_rule::time_series_stat_threshold,
};
use crate::analytics::{
    AnalyticalRule, Score, TimeSeriesDataPointThresholdRule, TimeSeriesStatIntraRunComparisonRule,
    TimeSeriesStatRunComparisonRule, TimeSeriesStatThresholdRule,
};
use crate::computations::{Comparator, Stat};
use crate::data::perf_stat::PerfStat;
use crate::data::AnalyzeData;

impl AnalyzeData for PerfStat {
    fn get_analytical_rules(&self) -> Vec<AnalyticalRule> {
        vec![
            time_series_stat_run_comparison! (
                name: "Reduced Instructions per Cycle",
                metric: "ipc",
                stat: Stat::Average,
                comparator: Comparator::LessEqual,
                abs: false,
                delta_ratio: -0.1,
                score: Score::Poor,
                message: "Less instructions are being executed by the CPUs. Check the frontend and backend stall metrics to locate the bottleneck.",
            ),
            time_series_stat_intra_run_comparison! (
                name: "Frontend-Dominated CPU Stalls",
                baseline_metric: "stall-backend-pkc",
                comparison_metric: "stall-frontend-pkc",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Concerning,
                message: "Higher CPU frontend stalls indicate bottlenecks on instruction fetches instead of executions, usually due to speculating the wrong destination for a branch, or getting stalled when fetching instructions from memory. Check the related misprediction metrics for instructions." ,
            ),
            time_series_stat_intra_run_comparison! (
                name: "Backend-Dominated CPU Stalls",
                baseline_metric: "stall-frontend-pkc",
                comparison_metric: "stall-backend-pkc",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Concerning,
                message: "Higher CPU backend stalls indicate bottlenecks on instruction executions instead of fetches, usually due to data cache misses and lacking resources to execute enough memory operations in parallel. Check the related misprediction metrics for data." ,
            ),
            time_series_stat_threshold! (
                name: "High Branch Mispredictions",
                metric: "branch-mpki",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                threshold: 10.0,
                score: Score::Concerning,
                message: "A large number of branch mis-predictions puts bottlenecks on the processor.",
            ),
            time_series_data_point_threshold!(
                name: "High L1 Instruction Cache Misses",
                metric: "inst-l1-mpki",
                comparator: Comparator::Greater,
                threshold: 20.0,
                score: Score::Concerning,
                message: "This indicates the working-set code footprint is large and spilling out of the fastest cache on the processor. This can cause CPU frontend stalls.",
            ),
            time_series_data_point_threshold!(
                name: "Instruction TLB Misses",
                metric: "inst-tlb-mpki",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Concerning,
                message: "This indicates the CPU has to do extra stalls to translate virtual addresses of instructions into physical addresses before fetching them. The instruction footprint might be too large and cause CPU frontend stalls.",
            ),
            time_series_data_point_threshold!(
                name: "Instruction TLB Table Walks",
                metric: "inst-tlb-tw-pki",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Concerning,
                message: "Upon instruction TLB misses, multiple traverses in the page table were required to find the correct physical address for an instruction. The instruction footprint might be too large and cause CPU frontend stalls.",
            ),
            time_series_data_point_threshold!(
                name: "High Code Sparsity",
                metric: "code-sparsity",
                comparator: Comparator::Greater,
                threshold: 0.5,
                score: Score::Bad,
                message: "The code being executed by the CPU is not compact enough and could increase branch mispredictions. Check the related compiler flags.",
            ),
            time_series_data_point_threshold!(
                name: "High L1 Data Cache Misses",
                metric: "data-l1-mpki",
                comparator: Comparator::Greater,
                threshold: 20.0,
                score: Score::Concerning,
                message: "This indicates the working set data footprint could be an issue and can cause CPU backend stalls.",
            ),
            time_series_data_point_threshold!(
                name: "High L2 Cache Misses",
                metric: "l2-mpki",
                comparator: Comparator::Greater,
                threshold: 10.0,
                score: Score::Concerning,
                message: "This indicates the working set data footprint could be an issue and can cause CPU backend stalls.",
            ),
            time_series_data_point_threshold!(
                name: "High L3 Cache Misses",
                metric: "l3-mpki",
                comparator: Comparator::Greater,
                threshold: 10.0,
                score: Score::Bad,
                message: "This indicates the working set data footprint is not fitting in L3, leading to CPU backend stalls and a larger DRAM bandwidth being consumed. There can be a larger issue if your instance is co-located with multiple neighbors that consume a measurable amount of DRAM bandwidth.",
            ),
            time_series_data_point_threshold!(
                name: "Data TLB Misses",
                metric: "data-tlb-mpki",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Concerning,
                message: "This indicates the CPU has to do extra stalls to translate virtual addresses in the load and store instructions into physical addresses the DRAM understands, before issuing them to the memory system. It causes CPU backend stalls.",
            ),
            time_series_data_point_threshold!(
                name: "Data TLB Table Walks",
                metric: "data-tlb-tw-pki",
                comparator: Comparator::Greater,
                threshold: 0.0,
                score: Score::Concerning,
                message: "Upon data TLB misses, the CPUs need to traverse the OS-build page table to find the correct physical address for the virtual address in the load/store instructions, which requires extra memory references before executing them and causes CPU backend stalls.",
            ),
            time_series_data_point_threshold! (
                name: "Old-Style Atomic Instructions",
                metric: "strex-spec-pki",
                comparator: Comparator::GreaterEqual,
                threshold: 10.0,
                score: Score::Poor,
                message: "The STREX instructions are being used extensively. It is an old-style atomic instruction less efficient than the newer LSE instructions.",
            ),
        ]
    }
}
