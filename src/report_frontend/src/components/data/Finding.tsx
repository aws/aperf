import React from "react";
import { ALL_DATA_TYPES, AnalyticalFinding, DataType, TimeSeriesMetricProps } from "../../definitions/types";
import { Icon, Link, SpaceBetween } from "@cloudscape-design/components";
import { ANALYTICAL_FINDINGS } from "../../definitions/data-config";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { useReportState } from "../ReportStateProvider";

function getFindingColor(score: number): string {
  if (score == 0 || isNaN(score)) {
    return "#F2CD54";
  } else if (score > 0) {
    return `rgba(0, ${Math.min(score * 60, 255)}, 0, 0.2)`;
  } else {
    return `rgba(${Math.min(Math.abs(score) * 60, 255)}, 0, 0, 0.2)`;
  }
}

function FindingIcon(props: { score: number }) {
  if (props.score > 0) {
    return <Icon name={"face-happy"} />;
  } else if (props.score < 0) {
    return <Icon name={"face-sad"} />;
  } else {
    return <Icon name={"face-neutral"} />;
  }
}

/**
 * This component renders a single analytica finding. If showDataLink is true,
 * it contains a link that directs the user to the corresponding data that this
 * finding belongs to.
 */
export interface FindingProps {
  readonly dataType?: DataType;
  readonly dataKey?: string;
  readonly finding: AnalyticalFinding;
  readonly showDataLink?: boolean;
}

export function Finding(props: FindingProps) {
  const { setDataComponent, setSearchKey } = useReportState();

  return (
    <div
      style={{
        padding: "10px",
        margin: "5px",
        borderRadius: "8px",
        border: "1px solid #ddd",
        backgroundColor: getFindingColor(props.finding.score),
        boxShadow: "0 2px 4px rgba(0, 0, 0, 0.1)",
        display: "flex",
      }}
    >
      <div style={{ marginRight: "10px" }}>
        <FindingIcon score={props.finding.score} />
      </div>
      <div style={{ display: "inline" }}>
        <>
          {props.showDataLink && (
            <Link
              href={`#${props.dataType || "systeminfo"}`}
              onFollow={() => {
                setSearchKey(props.dataKey);
                setDataComponent(props.dataType);
              }}
            >
              {`[${DATA_DESCRIPTIONS[props.dataType || "systeminfo"].readableName}] ${props.dataKey}:`}
            </Link>
          )}{" "}
        </>
        {props.finding.description}
      </div>
    </div>
  );
}

export function MetricFindings(props: TimeSeriesMetricProps) {
  const dataFindings = ANALYTICAL_FINDINGS[props.dataType];
  if (dataFindings == undefined) return null;
  const curRunFindings = dataFindings.per_run_findings[props.runName];
  if (curRunFindings == undefined) return null;
  const metricFindings = curRunFindings.findings[props.metricName];
  if (metricFindings == undefined) return null;

  const sortedMetricFindings = [...metricFindings];
  // The findings with low scores ("bad findings") are put at top
  sortedMetricFindings.sort((a, b) => a.score - b.score);

  return (
    <SpaceBetween size={"xxxs"}>
      {sortedMetricFindings.map((finding) => (
        <Finding finding={finding} />
      ))}
    </SpaceBetween>
  );
}

// Internal data structure to store information of a finding to be rendered
interface RunFindingData {
  readonly dataType: DataType;
  readonly dataKey: string;
  readonly finding: AnalyticalFinding;
}

const RUN_FINDINGS_CACHE = new Map<string, RunFindingData[]>();

/**
 * This component renders the flattened list of all analytical findings within the given run,
 * sorted by the score in ascending order.
 */
export function RunFindings(props: { runName: string }) {
  let runFindings = RUN_FINDINGS_CACHE.get(props.runName);
  if (runFindings == undefined) {
    // Flatten findings of all data types within this run
    runFindings = [];
    for (const dataType of ALL_DATA_TYPES) {
      const dataFindings = ANALYTICAL_FINDINGS[dataType];
      if (dataFindings == undefined) continue;
      const dataCurRunFindings = dataFindings.per_run_findings[props.runName];
      if (dataCurRunFindings) {
        for (const [dataKey, findings] of Object.entries(dataCurRunFindings.findings)) {
          findings.forEach((finding) => runFindings.push({ dataType, dataKey, finding }));
        }
      }
    }
    // The findings with low scores ("bad findings") are put at top
    runFindings.sort((a, b) => a.finding.score - b.finding.score);
    RUN_FINDINGS_CACHE.set(props.runName, runFindings);
  }

  return (
    <SpaceBetween size={"xxxs"}>
      {runFindings.map((findingData) => (
        <Finding
          dataType={findingData.dataType}
          dataKey={findingData.dataKey}
          finding={findingData.finding}
          showDataLink
        />
      ))}
    </SpaceBetween>
  );
}
