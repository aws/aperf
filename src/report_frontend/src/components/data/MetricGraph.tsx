import React from "react";
import { DataType, TimeSeriesData, TimeSeriesMetricProps } from "../../definitions/types";
import { useReportState } from "../ReportStateProvider";
import { CPU_DATA_TYPES, PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import Plot from "react-plotly.js";
import { Box, Button, Popover, SpaceBetween } from "@cloudscape-design/components";
import { getTimeSeriesMetricUnit, shouldShowCpuSeries } from "../../utils/utils";
import MetricGraph from "./MetricGraph";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import MetricStatsDisplay from "./MetricStatsDisplay";

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
} {
  const metrics = (PROCESSED_DATA[dataType].runs[runName] as TimeSeriesData)?.metrics;
  if (metrics === undefined) return { seriesData: [], valueRange: [] };
  const metric = metrics[metricName];
  if (metric === undefined) return { seriesData: [], valueRange: [] };

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

  return { seriesData, valueRange };
}

/**
 * This component renders a single metric graph of a particular time series metric of an APerf run.
 */
export default function (props: TimeSeriesMetricProps) {
  const { selectedCpusPerRun, darkMode } = useReportState();

  const { seriesData, valueRange } = getSeriesData(
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
    <Plot
      data={seriesData}
      layout={{
        xaxis: {
          title: "Seconds",
          gridcolor: darkMode ? "#404040" : "#e0e0e0",
        },
        yaxis: {
          title: getTimeSeriesMetricUnit(props.dataType, props.metricName),
          tickformat: "~s",
          range: valueRange,
          gridcolor: darkMode ? "#404040" : "#e0e0e0",
        },
        autosize: true,
        paper_bgcolor: darkMode ? "#171D25" : "#ffffff",
        plot_bgcolor: darkMode ? "#171D25" : "#ffffff",
        font: { color: darkMode ? "#ffffff" : "#000000" },
        margin: { t: 30, b: 50 },
      }}
      style={{ width: "100%", height: "100%" }}
      useResizeHandler
    />
  );
}

/**
 * This component renders a quick preview of a metric and the same metric in the base run.
 */
export function MetricGraphsPopover(props: TimeSeriesMetricProps) {
  const runsInScope = [props.runName];
  if (RUNS[0] != props.runName) {
    runsInScope.push(RUNS[0]);
  }

  const content = (
    <SpaceBetween size={"s"}>
      {runsInScope.map((runName) => (
        <SpaceBetween size={"xxs"}>
          {runName == props.runName && <b>{runName}</b>}
          {runName != props.runName && (
            <span>
              {runName}
              {" (Base run)"}
            </span>
          )}
          <MetricStatsDisplay dataType={props.dataType} runName={runName} metricName={props.metricName} />
          <MetricGraph dataType={props.dataType} runName={runName} metricName={props.metricName} key={props.dataType} />
        </SpaceBetween>
      ))}
    </SpaceBetween>
  );

  return (
    <Popover
      triggerType={"custom"}
      wrapTriggerText={false}
      position={"left"}
      size={"large"}
      fixedWidth
      header={`[${DATA_DESCRIPTIONS[props.dataType].readableName}] ${props.metricName}`}
      content={content}
    >
      <div title={"Preview the metric graph."} style={{ display: "inline-block" }}>
        <Button iconName={"zoom-in"} variant={"icon"} />
      </div>
    </Popover>
  );
}
