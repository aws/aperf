import React from "react";
import { DataType, Statistics, TimeSeriesData } from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { formatNumber } from "../../utils/utils";
import { Box } from "@cloudscape-design/components";

interface MetricStatsDisplayProps {
  dataType: DataType;
  runName: string;
  metricName: string;
}

export default function MetricStatsDisplay(props: MetricStatsDisplayProps) {
  const baseRunStats = RUNS[0]
    ? (PROCESSED_DATA[props.dataType].runs[RUNS[0]] as TimeSeriesData)?.metrics?.[props.metricName]?.stats
    : undefined;
  const isBaseRun = RUNS[0] && props.runName === RUNS[0];

  const metrics = (PROCESSED_DATA[props.dataType].runs[props.runName] as TimeSeriesData)?.metrics;
  if (!metrics) return null;

  const metric = metrics[props.metricName];
  if (!metric) return null;

  return (
    <Box variant="small">
      {Object.entries(metric.stats).map(([statName, statValue], index) => {
        const baseText = `${statName}: ${formatNumber(statValue)}`;

        if (!isBaseRun && baseRunStats && "std" !== statName && statName in baseRunStats) {
          const baseValue = baseRunStats[statName as keyof Statistics];
          const percentDiff =
            baseValue === 0 ? (statValue === 0 ? 0 : 100) : ((statValue - baseValue) / baseValue) * 100;
          const diffStr = `${percentDiff >= 0 ? "+" : ""}${percentDiff.toFixed(1)}%`;

          const desired = DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.metricName]?.desired;

          let color = "";
          if (desired === "higher" && percentDiff !== 0) {
            color = percentDiff > 0 ? "green" : "red";
          } else if (desired === "lower" && percentDiff !== 0) {
            color = percentDiff < 0 ? "green" : "red";
          }

          return (
            <span key={index}>
              {baseText} (<span style={{ color }}>{diffStr}</span>)
              {index < Object.keys(metric.stats).length - 1 && " | "}
            </span>
          );
        }

        return (
          <span key={index}>
            {baseText}
            {index < Object.keys(metric.stats).length - 1 && " | "}
          </span>
        );
      })}
    </Box>
  );
}
