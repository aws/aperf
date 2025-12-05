import React from "react";
import {DataType, TimeSeriesData, TimeSeriesMetricProps} from "../../definitions/types";
import { useReportState } from "../ReportStateProvider";
import { CPU_DATA_TYPES, PROCESSED_DATA } from "../../definitions/data-config";
import Plot from "react-plotly.js";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { Box, SpaceBetween } from "@cloudscape-design/components";
import { shouldShowCpuSeries, scaleKBData } from "../../utils/utils";
import MetricStatsDisplay from "./MetricStatsDisplay";
import {MetricFindings} from "./Finding";

/**
 * Transform processed time series data into the format required by plotly.js.
 */
function getSeriesData(
  dataType: DataType,
  runName: string,
  metricName: string,
  selectedAggregate: boolean,
  selectedCpus: boolean[],
): {
  seriesData: Partial<Plotly.PlotData>[];
  valueRange: number[];
  scaledUnit: string;
} {
  const metrics = (PROCESSED_DATA[dataType].runs[runName] as TimeSeriesData)?.metrics;
  if (metrics === undefined) return { seriesData: [], valueRange: [], scaledUnit: "" };
  const metric = metrics[metricName];
  if (metric === undefined) return { seriesData: [], valueRange: [], scaledUnit: "" };

  const originalUnit =
    DATA_DESCRIPTIONS[dataType].fieldDescriptions[metricName]?.unit || DATA_DESCRIPTIONS[dataType].defaultUnit;

  // Collect all values to determine scaling
  const allValues = metric.series.flatMap((series) => series.values);
  const { scaledUnit, scaleFactor } = scaleKBData(allValues, originalUnit);

  const isCpuDataType = CPU_DATA_TYPES.includes(dataType);
  const seriesData = metric.series.map(
    (series) =>
      ({
        name: series.series_name,
        x: series.time_diff,
        y: scaleFactor === 1 ? series.values : series.values.map((v) => v / scaleFactor),
        type: "scatter",
        visible:
          isCpuDataType && !shouldShowCpuSeries(series.series_name, selectedAggregate, selectedCpus)
            ? "legendonly"
            : true,
      }) as Partial<Plotly.PlotData>,
  );

  const scaledValueRange = scaleFactor === 1 ? metric.value_range : metric.value_range.map((v) => v / scaleFactor);

  return { seriesData, valueRange: scaledValueRange, scaledUnit };
}

/**
 * This component renders a single metric graph of a particular time series metric of an APerf run.
 */
export default function (props: TimeSeriesMetricProps) {
  const { selectedCpusPerRun, darkMode } = useReportState();

  const { seriesData, valueRange, scaledUnit } = getSeriesData(
    props.dataType,
    props.runName,
    props.metricName,
    selectedCpusPerRun[props.runName].aggregate,
    selectedCpusPerRun[props.runName].cpus,
  );

  if (seriesData.length == 0) {
    return (
      <Box textAlign="center" color="inherit">
        <b>No data collected</b>
        <Box variant="p" color="inherit">
          This metric was not collected in the APerf run
        </Box>
      </Box>
    );
  }

  return (
    <SpaceBetween size={"xs"}>
      <MetricStatsDisplay dataType={props.dataType} runName={props.runName} metricName={props.metricName} />
      <Plot
        data={seriesData}
        layout={{
          xaxis: {
            title: "Seconds",
            gridcolor: darkMode ? "#404040" : "#e0e0e0",
          },
          yaxis: {
            title: scaledUnit,
            tickformat: ".3s",
            range: valueRange,
            gridcolor: darkMode ? "#404040" : "#e0e0e0",
          },
          autosize: true,
          paper_bgcolor: darkMode ? "#171D25" : "#ffffff",
          plot_bgcolor: darkMode ? "#171D25" : "#ffffff",
          font: { color: darkMode ? "#ffffff" : "#000000" },
        }}
        style={{ width: "100%", height: "100%" }}
        useResizeHandler
      />
      <MetricFindings dataType={props.dataType} runName={props.runName} metricName={props.metricName} />
    </SpaceBetween>
  );
}
