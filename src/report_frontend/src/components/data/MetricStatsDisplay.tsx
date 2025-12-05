import React, { useRef } from "react";
import {DataType, Statistics, TimeSeriesData, TimeSeriesMetricProps} from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { formatNumber, scaleKBStats } from "../../utils/utils";
import { Button, SpaceBetween, Box } from "@cloudscape-design/components";

export default function MetricStatsDisplay(props: TimeSeriesMetricProps) {
  const boxRef = useRef<HTMLDivElement>(null);

  const metrics = (PROCESSED_DATA[props.dataType].runs[props.runName] as TimeSeriesData)?.metrics;
  if (!metrics) return null;

  const metric = metrics[props.metricName];
  if (!metric) return null;

  const originalUnit =
    DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.metricName]?.unit ||
    DATA_DESCRIPTIONS[props.dataType].defaultUnit;

  // Collect all values to determine scaling
  const allValues = metric.series.flatMap((series) => series.values);
  const { scaledStats } = scaleKBStats(metric.stats, originalUnit, allValues);

  const baseRunMetrics = RUNS[0]
    ? (PROCESSED_DATA[props.dataType].runs[RUNS[0]] as TimeSeriesData)?.metrics
    : undefined;
  const baseRunStats = baseRunMetrics?.[props.metricName]?.stats;
  const scaledBaseRunStats =
    baseRunStats && baseRunMetrics?.[props.metricName]
      ? scaleKBStats(
          baseRunStats,
          originalUnit,
          baseRunMetrics[props.metricName].series.flatMap((series) => series.values),
        ).scaledStats
      : undefined;

  const isBaseRun = RUNS[0] && props.runName === RUNS[0];

  return (
    <SpaceBetween direction="horizontal" size="xs">
      <Box variant="small">
        <div ref={boxRef}>
          {Object.entries(scaledStats).map(([statName, statValue], index) => {
            const baseText = `${statName}: ${formatNumber(statValue)}`;

            if (!isBaseRun && scaledBaseRunStats && "std" !== statName && statName in scaledBaseRunStats) {
              const baseValue = scaledBaseRunStats[statName as keyof Statistics];
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
                  {index < Object.keys(scaledStats).length - 1 && " | "}
                </span>
              );
            }

            return (
              <span key={index}>
                {baseText}
                {index < Object.keys(scaledStats).length - 1 && " | "}
              </span>
            );
          })}
        </div>
      </Box>
      <Button
        variant="inline-icon"
        iconName="copy"
        onClick={() => navigator.clipboard.writeText(boxRef.current?.textContent || "")}
      />
    </SpaceBetween>
  );
}
