import React from "react";
import { ALL_DATA_TYPES, DataType, TimeSeriesData, FindingType } from "../definitions/types";
import { CPU_DATA_TYPES, PROCESSED_DATA } from "../definitions/data-config";
import { IconProps } from "@cloudscape-design/components/icon/interfaces";
import { DATA_DESCRIPTIONS } from "../definitions/data-descriptions";
import { DECODED_VALUES_CACHE_BYTES } from "../definitions/constants";

export function extractDataTypeFromFragment(fragment: string): DataType {
  if (!fragment || !fragment.startsWith("#")) {
    return "systeminfo";
  }
  const dataType = fragment.substring(1) as DataType;
  if (!ALL_DATA_TYPES.includes(dataType)) {
    return "systeminfo";
  }
  return dataType;
}

/**
 * Get the processing error of a run for the data type, if its processing failed
 */
export function getRunProcessingError(dataType: DataType, runName: string): string | undefined {
  const reportData = PROCESSED_DATA[dataType];
  return reportData.runs[runName] === undefined ? reportData.run_errors?.[runName] : undefined;
}

/**
 * Get the list of sorted metric names that contain at least one non-zero data point
 */
export function getDataTypeNonZeroMetricNames(dataType: DataType, sortedMetricNames: string[]): string[] {
  const reportData = PROCESSED_DATA[dataType];
  if (!reportData || reportData.data_format != "time_series") {
    throw new Error(`getNonZeroMetricKeys invoked for invalid time series data: ${dataType}`);
  }

  return sortedMetricNames.filter((metricKey) => {
    for (const runName in reportData.runs) {
      const curRunMetrics = (reportData.runs[runName] as TimeSeriesData).metrics;
      if (
        metricKey in curRunMetrics &&
        (curRunMetrics[metricKey].stats.min != 0 || curRunMetrics[metricKey].stats.max != 0)
      ) {
        return true;
      }
    }
    return false;
  });
}

const CPU_SERIES_NAME_PATTERN = /^CPU(\d+)$/;

export function getCpuIdFromSeriesName(seriesName: string): number | undefined {
  const cpuId = CPU_SERIES_NAME_PATTERN.exec(seriesName);
  return cpuId === null ? undefined : Number(cpuId[1]);
}

/**
 * Collect the online CPU ids of a run from data types that contain the CPU info.
 */
export function getRunCpuIds(runName: string): number[] {
  const cpuIds = new Set<number>();
  for (const cpuDataType of CPU_DATA_TYPES) {
    const reportData = PROCESSED_DATA[cpuDataType].runs[runName] as TimeSeriesData;
    if (reportData == undefined) continue;
    for (const metricName in reportData.metrics) {
      reportData.metrics[metricName].series
        .map(getCpuIdFromSeriesName)
        .filter((cpuId): cpuId is number => cpuId !== undefined)
        .forEach((cpuId) => cpuIds.add(cpuId));
    }
  }
  return [...cpuIds].sort((cpuId, otherCpuId) => cpuId - otherCpuId);
}

/** Inflate a zlib stream from its base64 chunks, see src/data/common/compression.rs. */
export async function inflate(chunks: readonly string[]): Promise<ArrayBuffer> {
  const compressed = chunks.map((chunk) => {
    const binary = atob(chunk);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
  });
  const stream = new Blob(compressed).stream().pipeThrough(new DecompressionStream("deflate"));
  return new Response(stream).arrayBuffer();
}

// The decoded values of recently viewed metrics, from the least to the most recently used
const decodedValuesCache = new Map<string, Float64Array[]>();
let decodedValuesCacheBytes = 0;

function getDecodedValuesCacheKey(dataType: DataType, runName: string, metricName: string): string {
  return [dataType, runName, metricName].join("\n");
}

/** Cache the decoded values of a metric in a run, and drop the least recently used ones over the size limit. */
function cacheDecodedValues(key: string, values: Float64Array[]) {
  // All series of a metric share one buffer
  const numBytes = (values: Float64Array[]) => values[0]?.buffer.byteLength ?? 0;
  decodedValuesCache.delete(key);
  decodedValuesCache.set(key, values);
  decodedValuesCacheBytes += numBytes(values);
  for (const [oldestKey, oldestValues] of decodedValuesCache) {
    if (decodedValuesCacheBytes <= DECODED_VALUES_CACHE_BYTES || oldestKey === key) break;
    decodedValuesCache.delete(oldestKey);
    decodedValuesCacheBytes -= numBytes(oldestValues);
  }
}

/** Decode the values of every series of a metric in a run: one value per time of the run, NaN for no value. */
export async function decodeSeriesValues(
  dataType: DataType,
  runName: string,
  metricName: string,
): Promise<Float64Array[]> {
  const key = getDecodedValuesCacheKey(dataType, runName, metricName);
  const cachedValues = decodedValuesCache.get(key);
  if (cachedValues !== undefined) {
    // Move the values to the most recently used
    decodedValuesCache.delete(key);
    decodedValuesCache.set(key, cachedValues);
    return cachedValues;
  }
  const runData = PROCESSED_DATA[dataType].runs[runName] as TimeSeriesData;
  const metric = runData.metrics[metricName];
  // The values are little-endian, as are typed arrays in browsers
  const values = new Float64Array(await inflate(metric.values));
  const numTimes = runData.time_diff.length;
  const seriesValues = metric.series.map((_, i) => values.subarray(i * numTimes, (i + 1) * numTimes));
  cacheDecodedValues(key, seriesValues);
  return seriesValues;
}

/** The runs among the given ones that have the metric. */
function getRunsWithMetric(dataType: DataType, metricName: string, runNames: readonly string[]): string[] {
  return runNames.filter(
    (runName) => (PROCESSED_DATA[dataType].runs[runName] as TimeSeriesData)?.metrics[metricName] !== undefined,
  );
}

/** Decode the series values of a metric in every given run that has it. Undefined while decoding, or the error if it failed. */
export function useSeriesValues(
  dataType: DataType,
  metricName: string,
  runNames: readonly string[],
): Map<string, Float64Array[]> | Error | undefined {
  const key = [dataType, metricName, ...runNames].join("\n");
  const [result, setResult] = React.useState<{ key: string; values: Map<string, Float64Array[]> | Error }>();
  React.useEffect(() => {
    let active = true;
    const runsWithMetric = getRunsWithMetric(dataType, metricName, runNames);
    Promise.all(runsWithMetric.map((runName) => decodeSeriesValues(dataType, runName, metricName))).then(
      (values) => {
        if (active) setResult({ key, values: new Map(runsWithMetric.map((runName, i) => [runName, values[i]])) });
      },
      (error) => {
        if (active) setResult({ key, values: error instanceof Error ? error : new Error(String(error)) });
      },
    );
    return () => {
      active = false;
    };
  }, [key]);
  if (result && result.key === key) {
    return result.values;
  }
  // Show the cached values right away, instead of the decoding status until the effect above finishes
  const runsWithMetric = getRunsWithMetric(dataType, metricName, runNames);
  const cachedValues = runsWithMetric.map((runName) =>
    decodedValuesCache.get(getDecodedValuesCacheKey(dataType, runName, metricName)),
  );
  if (cachedValues.every((values): values is Float64Array[] => values !== undefined)) {
    return new Map(runsWithMetric.map((runName, i) => [runName, cachedValues[i]]));
  }
  return undefined;
}

// The line of every decoded series, reused by every render, since plotly.js redraws a graph whenever its arrays are new
const seriesLines = new WeakMap<Float64Array, { x: number[]; y: Float64Array; numValues: number }>();

/** Build the plotly.js line of a series, from its first to its last value and connected across the times without a value. */
export function getSeriesTrace(
  name: string,
  times: number[],
  values: Float64Array,
  visible: Plotly.PlotData["visible"],
): Partial<Plotly.PlotData> {
  let line = seriesLines.get(values);
  if (line === undefined) {
    let first = values.length;
    let last = -1;
    let numValues = 0;
    values.forEach((value, i) => {
      if (!Number.isNaN(value)) {
        first = Math.min(first, i);
        last = i;
        numValues++;
      }
    });
    line = { x: times.slice(first, last + 1), y: values.subarray(first, last + 1), numValues };
    seriesLines.set(values, line);
  }
  return {
    name,
    x: line.x,
    y: line.y,
    type: "scatter",
    // The default mode of plotly.js, but only counting the times with a value
    mode: line.numValues < 20 ? "lines+markers" : "lines",
    connectgaps: true,
    visible,
  };
}

/**
 * Format a number with suffix K, M, or G
 */
export function formatNumber(n: number) {
  if (n === null || isNaN(n)) return NaN;
  if (n >= 1e12) return (n / 1e12).toFixed(2) + "T";
  if (n >= 1e9) return (n / 1e9).toFixed(2) + "G";
  if (n >= 1e6) return (n / 1e6).toFixed(2) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(2) + "K";
  return n.toFixed(2);
}

export function shouldShowCpuSeries(
  seriesName: string,
  isAggregate: boolean,
  selectedAggregate: boolean,
  selectedCpus: Set<number>,
) {
  if (isAggregate) {
    return selectedAggregate;
  }
  const cpuId = getCpuIdFromSeriesName(seriesName);
  return cpuId === undefined ? true : selectedCpus.has(cpuId);
}

/**
 * Finds the unit of a time-series metric.
 */
export function getTimeSeriesMetricUnit(dataType: DataType, metricName: string): string {
  return DATA_DESCRIPTIONS[dataType].fieldDescriptions[metricName]?.unit || DATA_DESCRIPTIONS[dataType].defaultUnit;
}

/**
 * Maps a finding type to its corresponding icon name.
 */
export function getFindingTypeIconName(findingType: FindingType): IconProps.Name {
  switch (findingType) {
    case "negative":
      return "face-sad";
    case "zero":
      return "face-neutral";
    case "positive":
      return "face-happy";
  }
}

/**
 * Maps a finding type to its human-readable name to be rendered.
 */
export function getFindingTypeReadableName(findingType: FindingType): string {
  switch (findingType) {
    case "negative":
      return "Bad";
    case "zero":
      return "Neutral";
    case "positive":
      return "Good";
  }
}
