import React, { useRef } from "react";
import { Stat, TimeSeriesData, TimeSeriesMetricProps } from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { formatNumber } from "../../utils/utils";
import { Button, Popover, StatusIndicator } from "@cloudscape-design/components";
import { getTimeSeriesStatDeltaString } from "../../utils/analytics";

/**
 * This component renders the stats string with color-coded deltas for a time-series metric.
 * The deltas are pre-computed upon page load (to generate statistical findings), so they are
 * retrieved from the cache.
 */
export default function MetricStatsDisplay(props: TimeSeriesMetricProps) {
  const boxRef = useRef<HTMLDivElement>(null);
  const isBaseRun = RUNS[0] && props.runName === RUNS[0];

  const metrics = (PROCESSED_DATA[props.dataType].runs[props.runName] as TimeSeriesData)?.metrics;
  if (!metrics) return null;
  const metric = metrics[props.metricName];
  if (!metric) return null;

  return (
    // The stats strings are a given a fixed height so that the graphs being rendered below it can be
    // placed at the same level (however if the string overflows the graph below it will still be moved
    // further down)
    <small style={{ display: "inline-block", height: "60px" }}>
      <div ref={boxRef} style={{ display: "inline" }}>
        {Object.entries(metric.stats).map(([statName, statValue], index) => {
          if (!isBaseRun) {
            // The delta and its color-coded string is already available due to generations
            // of statistical findings
            const deltaString = getTimeSeriesStatDeltaString(
              props.runName,
              props.dataType,
              props.metricName,
              statName as Stat,
            );

            return (
              <span key={index}>
                <b>{statName}</b>
                {": "}
                {formatNumber(statValue)} {deltaString && <>({deltaString})</>}
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
      </div>{" "}
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
    </small>
  );
}
