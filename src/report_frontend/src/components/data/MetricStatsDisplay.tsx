import React, { useRef } from "react";
import { Statistics, TimeSeriesData, TimeSeriesMetricProps } from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { formatNumber } from "../../utils/utils";
import { Button, SpaceBetween, Box, Popover, StatusIndicator } from "@cloudscape-design/components";

export default function MetricStatsDisplay(props: TimeSeriesMetricProps) {
  const boxRef = useRef<HTMLDivElement>(null);
  const baseRunStats = RUNS[0]
    ? (PROCESSED_DATA[props.dataType].runs[RUNS[0]] as TimeSeriesData)?.metrics?.[props.metricName]?.stats
    : undefined;
  const isBaseRun = RUNS[0] && props.runName === RUNS[0];

  const metrics = (PROCESSED_DATA[props.dataType].runs[props.runName] as TimeSeriesData)?.metrics;
  if (!metrics) return null;

  const metric = metrics[props.metricName];
  if (!metric) return null;

  return (
    <SpaceBetween direction="horizontal" size="xs">
      <Box variant="small">
        <div ref={boxRef}>
          {Object.entries(metric.stats).map(([statName, statValue], index) => {
            if (!isBaseRun && baseRunStats && "std" !== statName && statName in baseRunStats) {
              const baseValue = baseRunStats[statName as keyof Statistics];
              let delta: number;
              let deltaStr: string;
              if (baseValue > 0) {
                delta = ((statValue - baseValue) / baseValue) * 100;
                deltaStr = `${delta >= 0 ? "+" : ""}${delta.toFixed(1)}%`;
              } else {
                delta = statValue - baseValue;
                deltaStr = `${delta >= 0 ? "+" : ""}${delta}`;
              }

              const desired = DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.metricName]?.desired;

              let color = "";
              if (desired === "higher" && delta != 0) {
                color = delta > 0 ? "green" : "red";
              } else if (desired === "lower" && delta != 0) {
                color = delta < 0 ? "green" : "red";
              }

              return (
                <span key={index}>
                  <b>{statName}</b>
                  {": "}
                  {formatNumber(statValue)}{" "}
                  {delta != 0 && (
                    <>
                      (<span style={{ color }}>{deltaStr}</span>)
                    </>
                  )}
                  {index < Object.keys(metric.stats).length - 1 && " | "}
                </span>
              );
            }

            return (
              <span key={index}>
                <b>{statName}</b>
                {": "}
                {formatNumber(statValue)}
                {index < Object.keys(metric.stats).length - 1 && " | "}
              </span>
            );
          })}
        </div>
      </Box>
      <Popover
        dismissButton={false}
        position="top"
        size="small"
        triggerType="custom"
        content={<StatusIndicator type="success">Stats copied</StatusIndicator>}
      >
        <Button
          variant="inline-icon"
          iconName="copy"
          onClick={() => navigator.clipboard.writeText(boxRef.current?.textContent || "")}
        />
      </Popover>
    </SpaceBetween>
  );
}
