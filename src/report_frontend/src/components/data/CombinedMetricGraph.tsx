import { DataType, TimeSeriesData } from "../../definitions/types";
import { CPU_DATA_TYPES, PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { useReportState } from "../ReportStateProvider";
import { getSeriesTrace, shouldShowCpuSeries, useSeriesValues } from "../../utils/utils";
import { MAX_SERIES_PER_GRAPH } from "../../definitions/constants";
import { Box } from "@cloudscape-design/components";
import React from "react";
import Plot from "react-plotly.js";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { MetricDecodingStatusBox } from "./StatusBoxes";

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

  const visibleSeries: { runName: string; seriesName: string; seriesIndex: number }[] = [];
  for (const runName of RUNS) {
    const curRunMetrics = (PROCESSED_DATA[props.dataType].runs[runName] as TimeSeriesData)?.metrics;
    if (curRunMetrics === undefined) continue;
    const curRunMetric = curRunMetrics[props.metricName];
    if (curRunMetric === undefined) continue;

    curRunMetric.series.forEach((seriesName, seriesIndex) => {
      if (
        !isCpuData ||
        shouldShowCpuSeries(
          seriesName,
          seriesIndex === curRunMetric.default_stats_series_idx,
          selectedCpusPerRun[runName].aggregate,
          selectedCpusPerRun[runName].cpus,
        )
      ) {
        visibleSeries.push({ runName, seriesName, seriesIndex });
      }
    });
  }

  // Only decode the values if the graph can show them
  const canShowSeries = visibleSeries.length > 0 && visibleSeries.length <= MAX_SERIES_PER_GRAPH;
  const values = useSeriesValues(props.dataType, props.metricName, canShowSeries ? RUNS : []);

  if (visibleSeries.length > MAX_SERIES_PER_GRAPH) {
    return (
      <Box textAlign="center" color="inherit">
        <b>Too many series</b>
        <Box variant="p" color="inherit">
          {`The number of series in the graph is larger than ${MAX_SERIES_PER_GRAPH}. Please reduce the number of visible series in each run.`}
        </Box>
      </Box>
    );
  } else if (visibleSeries.length == 0) {
    return (
      <Box textAlign="center" color="inherit">
        <b>No visible series</b>
        <Box variant="p" color="inherit">
          No visible series selected to be included in the combined metric graph.
        </Box>
      </Box>
    );
  }
  if (!(values instanceof Map)) {
    return <MetricDecodingStatusBox error={values} />;
  }

  const seriesData = visibleSeries.map(({ runName, seriesName, seriesIndex }) =>
    getSeriesTrace(
      `${runName}:${seriesName}`,
      (PROCESSED_DATA[props.dataType].runs[runName] as TimeSeriesData).time_diff,
      values.get(runName)[seriesIndex],
      true,
    ),
  );

  return (
    <Plot
      data={seriesData}
      layout={{
        xaxis: {
          title: "Time Diff",
          gridcolor: darkMode ? "#404040" : "#e0e0e0",
        },
        yaxis: {
          title:
            DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.metricName]?.unit ||
            DATA_DESCRIPTIONS[props.dataType].defaultUnit,
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
