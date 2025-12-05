import { ALL_DATA_TYPES, DataType, TimeSeriesData, Statistics } from "../definitions/types";
import { CPU_DATA_TYPES, PROCESSED_DATA } from "../definitions/data-config";

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

/**
 * Compute the number of CPUs from time series metrics whose series are all CPUs
 */
export function getRunNumCpus(runName: string): number {
  for (const cpuDataType of CPU_DATA_TYPES) {
    const reportData = PROCESSED_DATA[cpuDataType].runs[runName] as TimeSeriesData;
    if (reportData == undefined) continue;
    for (const metricName in reportData.metrics) {
      let numCpus = 0;
      for (const series of reportData.metrics[metricName].series) {
        if (series.series_name.toLowerCase().startsWith("cpu")) {
          numCpus++;
        }
      }
      if (numCpus > 0) return numCpus;
    }
  }
  // no CPU data type was collected, so return 0
  return 0;
}

/**
 * Format a number with suffix K, M, or G
 */
export function formatNumber(n: number) {
  if (n === null || isNaN(n)) return NaN;
  if (n >= 1e9) return (n / 1e9).toFixed(2) + "G";
  if (n >= 1e6) return (n / 1e6).toFixed(2) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(2) + "K";
  return n.toFixed(2);
}

export function shouldShowCpuSeries(seriesName: string, selectedAggregate: boolean, selectedCpus: boolean[]) {
  if (seriesName === "Aggregate") {
    return selectedAggregate;
  } else if (seriesName.startsWith("CPU")) {
    return !!selectedCpus[Number(seriesName.substring(3))];
  } else {
    return true;
  }
}

/**
 * Scales KB data and determines appropriate unit based on the maximum value in the dataset
 */
export function scaleKBData(
  data: number[],
  originalUnit: string,
): {
  scaledData: number[];
  scaledUnit: string;
  scaleFactor: number;
} {
  const { scaledUnit, scaleFactor } = getKBScaling(data, originalUnit);
  return {
    scaledData: scaleFactor === 1 ? data : data.map((v) => v / scaleFactor),
    scaledUnit,
    scaleFactor,
  };
}

/**
 * Scales KB statistics using the same logic as scaleKBData
 */
export function scaleKBStats(
  stats: Statistics,
  originalUnit: string,
  allValues: number[],
): {
  scaledStats: Statistics;
  scaledUnit: string;
  scaleFactor: number;
} {
  const { scaledUnit, scaleFactor } = getKBScaling(allValues, originalUnit);

  if (scaleFactor === 1) {
    return { scaledStats: stats, scaledUnit, scaleFactor };
  }

  const scaledStats: Statistics = {
    avg: stats.avg / scaleFactor,
    std: stats.std / scaleFactor,
    min: stats.min / scaleFactor,
    max: stats.max / scaleFactor,
    p50: stats.p50 / scaleFactor,
    p90: stats.p90 / scaleFactor,
    p99: stats.p99 / scaleFactor,
    p99_9: stats.p99_9 / scaleFactor,
  };

  return { scaledStats, scaledUnit, scaleFactor };
}

/**
 * Determines the appropriate scaling factor and unit for KB data
 */
function getKBScaling(
  data: number[],
  originalUnit: string,
): {
  scaledUnit: string;
  scaleFactor: number;
} {
  if (!originalUnit.includes("KB")) {
    return { scaledUnit: originalUnit, scaleFactor: 1 };
  }

  const filteredData = data.filter((v) => !isNaN(v) && isFinite(v));
  if (filteredData.length === 0) {
    return { scaledUnit: originalUnit, scaleFactor: 1 };
  }

  const maxValue = Math.max(...filteredData);

  if (maxValue >= 1024 * 1024) {
    return {
      scaledUnit: originalUnit.replace("KB", "GB"),
      scaleFactor: 1024 * 1024,
    };
  } else if (maxValue >= 1024) {
    return {
      scaledUnit: originalUnit.replace("KB", "MB"),
      scaleFactor: 1024,
    };
  }

  return { scaledUnit: originalUnit, scaleFactor: 1 };
}
