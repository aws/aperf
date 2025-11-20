import { DataType, TimeSeriesData } from "../../definitions/types";
import { CPU_DATA_TYPES, PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { useReportState } from "../ReportStateProvider";
import { shouldShowCpuSeries, scaleKBData } from "../../utils/utils";
import { MAX_SERIES_PER_GRAPH } from "../../definitions/constants";
import { Box } from "@cloudscape-design/components";
import React from "react";
import Plot from "react-plotly.js";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";

interface CombinedMetricGraphProps {
  readonly dataType: DataType;
  readonly metricName: string;
}

/**
 * This component renders a single metric graph for a time series metric across all APerf runs.
 * All visible series will be prepended with its run name and included in the graph.
 */
export default function (props: CombinedMetricGraphProps) {
  const { selectedCpusPerRun, darkMode } = useReportState();

  const isCpuData = CPU_DATA_TYPES.includes(props.dataType);
  const originalUnit =
    DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.metricName]?.unit ||
    DATA_DESCRIPTIONS[props.dataType].defaultUnit;

  // Collect all values across all runs to determine scaling
  const allValues: number[] = [];
  for (const runName of RUNS) {
    const curRunMetrics = (PROCESSED_DATA[props.dataType].runs[runName] as TimeSeriesData).metrics;
    if (curRunMetrics === undefined) continue;
    const curRunMetric = curRunMetrics[props.metricName];
    if (curRunMetric === undefined) continue;

    for (const series of curRunMetric.series) {
      if (
        !isCpuData ||
        shouldShowCpuSeries(series.series_name, selectedCpusPerRun[runName].aggregate, selectedCpusPerRun[runName].cpus)
      ) {
        allValues.push(...series.values);
      }
    }
  }

  const { scaledUnit, scaleFactor } = scaleKBData(allValues, originalUnit);

  const seriesData: Partial<Plotly.PlotData>[] = [];
  for (const runName of RUNS) {
    const curRunMetrics = (PROCESSED_DATA[props.dataType].runs[runName] as TimeSeriesData).metrics;
    if (curRunMetrics === undefined) continue;
    const curRunMetric = curRunMetrics[props.metricName];
    if (curRunMetric === undefined) continue;

    for (const series of curRunMetric.series) {
      if (
        !isCpuData ||
        shouldShowCpuSeries(series.series_name, selectedCpusPerRun[runName].aggregate, selectedCpusPerRun[runName].cpus)
      ) {
        seriesData.push({
          name: `${runName}:${series.series_name || ""}`,
          x: series.time_diff,
          y: scaleFactor === 1 ? series.values : series.values.map((v) => v / scaleFactor),
          type: "scatter",
          visible: true,
        });
      }
    }
  }

  if (seriesData.length > MAX_SERIES_PER_GRAPH) {
    return (
      <Box textAlign="center" color="inherit">
        <b>Too many series</b>
        <Box variant="p" color="inherit">
          {`The number of series in the graph is larger than ${MAX_SERIES_PER_GRAPH}. Please reduce the number of visible series in each run.`}
        </Box>
      </Box>
    );
  } else if (seriesData.length == 0) {
    return (
      <Box textAlign="center" color="inherit">
        <b>No visible series</b>
        <Box variant="p" color="inherit">
          No visible series selected to be included in the combined metric graph.
        </Box>
      </Box>
    );
  }

  return (
    <Plot
      data={seriesData}
      layout={{
        xaxis: {
          title: "Time Diff",
          gridcolor: darkMode ? "#404040" : "#e0e0e0",
        },
        yaxis: {
          title: scaledUnit,
          gridcolor: darkMode ? "#404040" : "#e0e0e0",
        },
        autosize: true,
        paper_bgcolor: darkMode ? "#171D25" : "#ffffff",
        plot_bgcolor: darkMode ? "#171D25" : "#ffffff",
        font: { color: darkMode ? "#ffffff" : "#000000" },
      }}
      style={{ width: "100%", height: "100%" }}
    />
  );
}
