export const APERF_SERVICE_NAME = "APerf";

/**
 * The default maximum number of metric graphs to render in a page
 */
export const NUM_METRICS_PER_PAGE = 20;

/**
 * The maximum number of series allowed to be included in a metric graph (if
 * the actual number is larger, skip rendering a graph to avoid performance
 * cost)
 */
export const MAX_SERIES_PER_GRAPH = 200;

/**
 * When the number of CPUs are less than or equal to this value, show all
 * series by default. Otherwise, only show the aggregate series by default.
 */
export const MAX_NUM_CPU_SHOW_DEFAULT = 32;
