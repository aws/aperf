import { ALL_DATA_TYPES, DataType, TimeSeriesData, FindingType } from "../definitions/types";
import { CPU_DATA_TYPES, PROCESSED_DATA } from "../definitions/data-config";
import { IconProps } from "@cloudscape-design/components/icon/interfaces";
import { DATA_DESCRIPTIONS } from "../definitions/data-descriptions";

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
      const curMetricCpuIds = reportData.metrics[metricName].series
        .map((series) => getCpuIdFromSeriesName(series.series_name))
        .filter((cpuId): cpuId is number => cpuId !== undefined);
      if (curMetricCpuIds.length > 0) {
        curMetricCpuIds.forEach((cpuId) => cpuIds.add(cpuId));
        break;
      }
    }
  }
  return [...cpuIds].sort((cpuId, otherCpuId) => cpuId - otherCpuId);
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

export function shouldShowCpuSeries(seriesName: string, selectedAggregate: boolean, selectedCpus: Set<number>) {
  if (seriesName === "Aggregate") {
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
