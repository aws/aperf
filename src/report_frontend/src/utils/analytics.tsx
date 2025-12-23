import { ALL_DATA_TYPES, ALL_STATS, AnalyticalFinding, DataType, Stat, TimeSeriesData } from "../definitions/types";
import { PER_DATA_ANALYTICAL_FINDINGS, PROCESSED_DATA, RUNS } from "../definitions/data-config";
import { DATA_DESCRIPTIONS } from "../definitions/data-descriptions";
import React, { ReactElement, ReactNode } from "react";
import { formatNumber } from "./utils";

/**
 * Extend the information of an analytical finding with data information to be used
 * to render the finding component.
 */
export interface RunAnalyticalFinding {
  readonly dataType: DataType;
  readonly dataKey: string;
  readonly finding: AnalyticalFinding;
}

/**
 * An object storing all analytical findings of a run.
 */
export const PER_RUN_ANALYTICAL_FINDINGS = Object.fromEntries(
  RUNS.map((runName) => [runName, [] as RunAnalyticalFinding[]]),
);

/**
 * Process all analytical findings and group them by runs.
 */
export function processAnalyticalFindings() {
  for (const runName of RUNS) {
    for (const dataType of ALL_DATA_TYPES) {
      const dataFindings = PER_DATA_ANALYTICAL_FINDINGS[dataType];
      if (dataFindings == undefined) continue;
      const dataCurRunFindings = dataFindings.per_run_findings[runName];
      if (dataCurRunFindings) {
        for (const [dataKey, findings] of Object.entries(dataCurRunFindings.findings)) {
          findings.forEach((finding) => PER_RUN_ANALYTICAL_FINDINGS[runName].push({ dataType, dataKey, finding }));
        }
      }
    }
    // The findings with low scores ("bad findings") are put at top
    PER_RUN_ANALYTICAL_FINDINGS[runName].sort((a, b) => a.finding.score - b.finding.score);
  }
}

/**
 * Stores the delta string of all stats of a time-series metric.
 */
type MetricStatsDelta = { [key in Stat]: ReactNode | undefined };

/**
 * Store the stats deltas of all time-series metrics within a run (maps metric name to the stats deltas of the metric).
 */
type RunStatsDelta = { [key in string]: MetricStatsDelta };

/**
 * Stores the stats deltas of all runs for a data type (maps run name to stats deltas within the run).
 */
type DataStatsDelta = { [key in string]: RunStatsDelta };

/**
 * Stores information of a statistical finding, which is to be rendered in the home page.
 */
export interface StatisticalFinding {
  /**
   * The time-series data type the finding belongs to
   */
  readonly dataType: DataType;
  /**
   * The APerf run that the finding belongs to
   */
  readonly runName: string;
  /**
   * The time-series metric name the finding belongs to
   */
  readonly metricName: string;
  /**
   * The type of the stat
   */
  readonly stat: Stat;
  /**
   * Indicates how good or bad the delta is, computed from the
   * desired value set in data-descriptions.ts
   */
  readonly score: number;
  /**
   * Color-coded string to be rendered that reflects the score of the delta
   */
  readonly deltaString: ReactElement;
  /**
   * The value of the stat in current run
   */
  readonly statValue: number;
  /**
   * The value of the stat in the base run
   */
  readonly baseValue: number;
}

/**
 * A cache that stores all metric stats delta strings of a data type across all runs. It is accessed when
 * rendering the stats display.
 */
export const DATA_STATS_DELTA = Object.fromEntries(
  ALL_DATA_TYPES.map((dataType) => [
    dataType,
    Object.fromEntries(RUNS.map((runName) => [runName, {} as RunStatsDelta])) as DataStatsDelta,
  ]),
);

/**
 * An object storing all statistical findings of a run, computed from all metric stats delta and sorted by score.
 */
export const STATISTICAL_FINDINGS = Object.fromEntries(RUNS.map((runName) => [runName, [] as StatisticalFinding[]]));

/**
 * Compute the stats delta of all time-series metrics. The deltas will be cached in DATA_STATS_DELTA, and
 * as the deltas are being computed, statistical findings are being generated and stored in STATISTICAL_FINDINGS.
 */
export function computeAllTimeSeriesStatsDelta() {
  const baseRunName = RUNS[0];

  for (const dataType of ALL_DATA_TYPES) {
    if (PROCESSED_DATA[dataType]?.data_format != "time_series") continue;

    for (let i = 1; i < RUNS.length; i++) {
      const baseTimeSeriesData = PROCESSED_DATA[dataType].runs[baseRunName] as TimeSeriesData;
      const runName = RUNS[i];
      const curTimeSeriesData = PROCESSED_DATA[dataType].runs[runName] as TimeSeriesData;
      if (baseTimeSeriesData == undefined || curTimeSeriesData == undefined) continue;

      const curRunStatsDelta: RunStatsDelta = {};

      for (const metricName of curTimeSeriesData.sorted_metric_names) {
        const baseMetric = baseTimeSeriesData.metrics[metricName];
        const curMetric = curTimeSeriesData.metrics[metricName];
        if (baseMetric == undefined || curMetric == undefined) continue;

        curRunStatsDelta[metricName] = Object.fromEntries(
          ALL_STATS.map((stat) => {
            const baseValue = baseMetric.stats[stat];
            const statValue = curMetric.stats[stat];

            let delta: number;
            let deltaStr: string;
            if (baseValue > 0) {
              delta = (statValue - baseValue) / baseValue;
              deltaStr = `${delta > 0 ? "+" : ""}${(delta * 100).toFixed(1)}%`;
            } else {
              delta = statValue - baseValue;
              deltaStr = `${delta > 0 ? "+" : ""}${formatNumber(delta)}`;
            }

            const desired = DATA_DESCRIPTIONS[dataType].fieldDescriptions[metricName]?.desired;

            let baseScore: number;
            let color: string;
            if (delta == 0 || stat == "std") {
              baseScore = 0;
              color = "";
            } else if (desired === "higher") {
              baseScore = delta > 0 ? 1 : -1;
              color = delta > 0 ? "green" : "red";
            } else if (desired === "lower") {
              baseScore = delta < 0 ? 1 : -1;
              color = delta < 0 ? "green" : "red";
            } else {
              baseScore = 0;
              color = "";
            }

            const score = baseScore * Math.abs(delta);
            // If delta is 0, the two stats are the same, so we do not produce a delta string
            const deltaString = delta != 0 ? <span style={{ color }}>{deltaStr}</span> : undefined;

            if (delta != 0) {
              STATISTICAL_FINDINGS[runName].push({
                dataType,
                runName,
                metricName,
                stat,
                score,
                deltaString,
                baseValue,
                statValue,
              });
            }

            return [stat, deltaString];
          }),
        ) as MetricStatsDelta;
      }

      DATA_STATS_DELTA[dataType][runName] = curRunStatsDelta;
    }
  }
}

/**
 * Retrieves the stat delta string of a time-series metric from the cache.
 */
export function getTimeSeriesStatDeltaString(
  runName: string,
  dataType: DataType,
  metricName: string,
  stat: Stat,
): ReactNode | undefined {
  const metricStatDelta = DATA_STATS_DELTA[dataType][runName][metricName];
  if (!metricStatDelta) return undefined;
  return metricStatDelta[stat];
}
