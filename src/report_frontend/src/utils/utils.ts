import { ALL_DATA_TYPES, DataType, TimeSeriesData } from "../definitions/types";
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
 * Use topological sort to decide the aggregate order of metric names to show based on
 * the order within each run
 */
export function getDataTypeSortedMetricNames(dataType: DataType): string[] {
  const reportData = PROCESSED_DATA[dataType];
  if (!reportData || reportData.data_format != "time_series") {
    throw new Error(`getDataTypeSortedMetricNames invoked for invalid time series data: ${dataType}`);
  }

  const dependencyGraph = new Map<string, Set<string>>();
  const inDegreeMap = new Map<string, number>();
  for (const runName in reportData.runs) {
    const curRunSortedMetricNames = (reportData.runs[runName] as TimeSeriesData).sorted_metric_names;
    for (let i = 0; i < curRunSortedMetricNames.length; i++) {
      const curMetricName = curRunSortedMetricNames[i];
      inDegreeMap.set(curMetricName, 0);
      if (!dependencyGraph.has(curMetricName)) {
        dependencyGraph.set(curMetricName, new Set<string>());
      }
      if (i > 0) {
        dependencyGraph.get(curRunSortedMetricNames[i - 1]).add(curMetricName);
      }
    }
  }
  dependencyGraph.forEach((dependents) => {
    dependents.forEach((dependent) => {
      inDegreeMap.set(dependent, inDegreeMap.get(dependent) + 1);
    });
  });

  const result: string[] = [];
  const queue: string[] = [];
  for (const key of inDegreeMap.keys()) {
    if (inDegreeMap.get(key) == 0) {
      queue.push(key);
    }
  }

  while (queue.length > 0) {
    const curKey = queue.shift();
    result.push(curKey);
    for (const dependent of dependencyGraph.get(curKey)) {
      inDegreeMap.set(dependent, inDegreeMap.get(dependent) - 1);
      if (inDegreeMap.get(dependent) == 0) {
        queue.push(dependent);
      }
    }
  }

  // If there are conflicting order, append all unique metric names together
  if (result.length != dependencyGraph.size) {
    console.error(`The sorted metric names of data ${dataType} have conflicting orders`);
    const all_metric_names: string[] = [];
    for (const runName in reportData.runs) {
      const curRunSortedMetricNames = (reportData.runs[runName] as TimeSeriesData).sorted_metric_names;
      for (const metricName of curRunSortedMetricNames) {
        if (!all_metric_names.includes(metricName)) {
          all_metric_names.push(metricName);
        }
      }
    }
    return all_metric_names;
  }

  return result;
}

export function getDataTypeNonZeroMetricKeys(dataType: DataType, sortedMetricKeys: string[]): string[] {
  const reportData = PROCESSED_DATA[dataType];
  if (!reportData || reportData.data_format != "time_series") {
    throw new Error(`getNonZeroMetricKeys invoked for invalid time series data: ${dataType}`);
  }

  return sortedMetricKeys.filter((metricKey) => {
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
