import React from "react";
import { DataType, TimeSeriesData } from "../../definitions/types";
import { useReportState } from "../ReportStateProvider";
import { CPU_DATA_TYPES, PROCESSED_DATA } from "../../definitions/data-config";
import Plot from "react-plotly.js";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { Box, SpaceBetween } from "@cloudscape-design/components";
import { formatNumber, shouldShowCpuSeries } from "../../utils/utils";

/**
 * Transform processed time series data into the format required by plotly.js and generate the
 * stats string.
 */
function getSeriesData(
  dataType: DataType,
  runName: string,
  metricName: string,
  selectedAggregate: boolean,
  selectedCpus: boolean[],
): { seriesData: Partial<Plotly.PlotData>[]; statsString: string; valueRange: number[] } {
  const metrics = (PROCESSED_DATA[dataType].runs[runName] as TimeSeriesData)?.metrics;
  if (metrics === undefined) return { seriesData: [], statsString: "", valueRange: [] };
  const metric = metrics[metricName];
  if (metric === undefined) return { seriesData: [], statsString: "", valueRange: [] };

  const statsString = Object.entries(metric.stats)
    .map(([statName, statValue]) => `${statName}: ${formatNumber(statValue)}`)
    .join(" | ");

  const isCpuDataType = CPU_DATA_TYPES.includes(dataType);
  const seriesData = metric.series.map(
    (series) =>
      ({
        name: series.series_name,
        x: series.time_diff,
        y: series.values,
        type: "scatter",
        visible:
          isCpuDataType && !shouldShowCpuSeries(series.series_name, selectedAggregate, selectedCpus)
            ? "legendonly"
            : true,
      }) as Partial<Plotly.PlotData>,
  );

  const valueRange = metric.value_range;

  return { seriesData, statsString, valueRange };
}

export interface MetricGraphProps {
  readonly dataType: DataType;
  readonly runName: string;
  readonly metricName: string;
}

/**
 * This component renders a single metric graph of a particular time series metric of an APerf run.
 */
export default function (props: MetricGraphProps) {
  const { selectedCpusPerRun } = useReportState();

  const { seriesData, statsString, valueRange } = getSeriesData(
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
          This metric was not collected in the Aperf run
        </Box>
      </Box>
    );
  }

  return (
    <SpaceBetween size={"xs"}>
      <Box variant={"small"}>{statsString}</Box>
      <Plot
        data={seriesData}
        layout={{
          xaxis: { title: "Seconds" },
          yaxis: {
            title:
              DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.metricName]?.unit ||
              DATA_DESCRIPTIONS[props.dataType].defaultUnit,
            tickformat: ".3s",
            range: valueRange,
          },
          autosize: true,
        }}
        style={{ width: "100%", height: "100%" }}
        useResizeHandler
      />
    </SpaceBetween>
  );
}
