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
                metric_name: "ipc",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "A lower ipc indicates a performance problem. Proceed to attempt to root cause where the lower IPC bottleneck is coming from by collecting frontend and backend stall metrics.",
            ),
            time_series_stat_intra_run_comparison! (
                baseline_metric_name: "stall-frontend-pkc",
                comparison_metric_name: "stall-backend-pkc",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "Stalls in the frontend are higher than in the backend. Front end stalls commonly occur if the CPU cannot fetch the proper instructions, either because it is speculating the wrong destination for a branch, or stalled waiting to get instructions from memory. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md#drill-down-front-end-stalls",
            ),
            time_series_stat_intra_run_comparison! (
                baseline_metric_name: "stall-backend-pkc",
                comparison_metric_name: "stall-frontend-pkc",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                abs: false,
                delta_ratio: 0.1,
                score: Score::Bad,
                message: "Stalls in the backend are higher than in the frontend. Backend stalls are caused when the CPU is unable to make forward progress executing instructions because a computational resource is full. This is commonly due to lacking enough resources to execute enough memory operations in parallel because the data set is large and current memory requests are waiting for responses. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md#drill-down-back-end-stalls",
            ),
            time_series_stat_threshold! (
                metric_name: "branch-mpki",
                stat: Stat::Average,
                comparator: Comparator::GreaterEqual,
                threshold: 10.0,
                score: Score::VeryBad,
                message: "A branch-mpki average value of >10 indicates the branch prediction logic is bottlenecking the processor.",
            ),
            time_series_data_point_threshold!(
                metric_name: "inst-l1-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 20.0,
                score: Score::VeryBad,
                message: "This indicates the working-set code footprint is large and is spilling out of the fastest cache on the processor and is potentially a bottleneck.",
            ),
            time_series_data_point_threshold!(
                metric_name: "inst-tlb-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 0.0,
                score: Score::VeryBad,
                message: "This indicates the CPU has to do extra stalls to translate the virtual addresses of instructions into physical addresses before fetching them and the footprint is too large.",
            ),
            time_series_data_point_threshold!(
                metric_name: "inst-tlb-tw-pki",
                comparator: Comparator::GreaterEqual,
                threshold: 0.0,
                score: Score::VeryBad,
                message: "This indicates the instruction footprint might be too large.",
            ),
            time_series_data_point_threshold!(
                metric_name: "code-sparsity",
                comparator: Comparator::GreaterEqual,
                threshold: 0.5,
                score: Score::VeryBad,
                message: "This indicates the code being executed by the CPU is very sparse. This counter is only available on Graviton 16xlarge or metal instances. https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/optimization_recommendation.md#optimizing-for-large-instruction-footprint",
            ),
            time_series_data_point_threshold!(
                metric_name: "data-l1-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 20.0,
                score: Score::VeryBad,
                message: "This indicates the working set data footprint could be an issue.",
            ),
            time_series_data_point_threshold!(
                metric_name: "data-l2-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 10.0,
                score: Score::VeryBad,
                message: "This indicates the working set data footprint could be an issue.",
            ),
            time_series_data_point_threshold!(
                metric_name: "data-l3-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 10.0,
                score: Score::VeryBad,
                message: "This indicates the working set data footprint is not fitting in L3 and data references are being served by DRAM. The l3-mpki also indicates the DRAM bandwidth requirement of your application, a higher number means more DRAM bandwidth will be consumed, this may be an issue if your instance is co-located with multiple neighbors also consuming a measurable amount of DRAM bandwidth.",
            ),
            time_series_data_point_threshold!(
                metric_name: "data-tlb-mpki",
                comparator: Comparator::GreaterEqual,
                threshold: 0.0,
                score: Score::VeryBad,
                message: "This indicates the CPU has to do extra stalls to translate the virtual address of load and store instructions into physical addresses the DRAM understands before issuing the load/store to the memory system.",
            ),
            time_series_data_point_threshold!(
                metric_name: "data-tlb-tw-pki",
                comparator: Comparator::GreaterEqual,
                threshold: 0.0,
                score: Score::VeryBad,
                message: "This indicates the CPU has to do extra stalls to translate the virtual address of the load/store instruction into physical addresses the DRAM understands before issuing to the memory system. In this case the stalls are because the CPU must walk the OS built page-table, which requires extra memory references before the requested memory reference from the application can be executed.",
            ),
        ]
    }
}
