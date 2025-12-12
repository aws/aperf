import React from "react";
import { ALL_DATA_TYPES, AnalyticalFinding, DataType, TimeSeriesMetricProps } from "../../definitions/types";
import { Icon, Link, SpaceBetween, Button } from "@cloudscape-design/components";
import { ANALYTICAL_FINDINGS } from "../../definitions/data-config";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { useReportState } from "../ReportStateProvider";

function linkifyText(text: string) {
  const urlRegex = /(https?:\/\/[^\s]+)/g;
  const parts = text.split(urlRegex);
  
  return parts.map((part, index) => {
    if (urlRegex.test(part)) {
      return <a key={index} href={part} target="_blank" rel="noopener noreferrer">Reference</a>;
    }
    return part;
  });
}

function getFindingColor(score: number): string {
  if (score == 0 || isNaN(score)) {
    return "rgba(125, 125, 125, 0.2)";
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
 * This component renders a single analytical finding. If showDataLink is true,
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
{linkifyText(props.finding.description)}
      </div>
    </div>
  );
}

export function MetricFindings(props: TimeSeriesMetricProps) {
  const { findingsFilter } = useReportState();
  const runFilter = findingsFilter[props.runName] || new Set(["negative", "zero", "positive"]);

  const dataFindings = ANALYTICAL_FINDINGS[props.dataType];
  if (dataFindings == undefined) return null;
  const curRunFindings = dataFindings.per_run_findings[props.runName];
  if (curRunFindings == undefined) return null;
  const metricFindings = curRunFindings.findings[props.metricName];
  if (metricFindings == undefined) return null;

  const sortedMetricFindings = [...metricFindings];
  // The findings with low scores ("bad findings") are put at top
  sortedMetricFindings.sort((a, b) => a.score - b.score);

  const filteredFindings = sortedMetricFindings.filter((finding) => {
    const score = finding.score;
    if (score < 0) return runFilter.has("negative");
    if (score === 0 || isNaN(score)) return runFilter.has("zero");
    if (score > 0) return runFilter.has("positive");
    return false;
  });

  return (
    <SpaceBetween size={"xxxs"}>
      {filteredFindings.map((finding) => (
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
  const { findingsFilter, setFindingsFilter } = useReportState();
  const runFilter = findingsFilter[props.runName] || new Set(["negative", "zero", "positive"]);

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

  const filteredFindings = runFindings.filter((findingData) => {
    const score = findingData.finding.score;
    if (score < 0) return runFilter.has("negative");
    if (score === 0 || isNaN(score)) return runFilter.has("zero");
    if (score > 0) return runFilter.has("positive");
    return false;
  });

  const toggleFilter = (filter: string) => {
    const newFilters = new Set(runFilter);
    if (newFilters.has(filter)) {
      newFilters.delete(filter);
    } else {
      newFilters.add(filter);
    }
    setFindingsFilter(props.runName, newFilters);
  };

  return (
    <SpaceBetween size={"s"}>
      <SpaceBetween direction="horizontal" size="xs">
        <Button
          variant={runFilter.has("negative") ? "primary" : "normal"}
          onClick={() => toggleFilter("negative")}
          iconName="face-sad"
        >
          Bad
        </Button>
        <Button
          variant={runFilter.has("zero") ? "primary" : "normal"}
          onClick={() => toggleFilter("zero")}
          iconName="face-neutral"
        >
          Neutral
        </Button>
        <Button
          variant={runFilter.has("positive") ? "primary" : "normal"}
          onClick={() => toggleFilter("positive")}
          iconName="face-happy"
        >
          Good
        </Button>
      </SpaceBetween>
      <SpaceBetween size={"xxxs"}>
        {filteredFindings.map((findingData) => (
          <Finding
            dataType={findingData.dataType}
            dataKey={findingData.dataKey}
            finding={findingData.finding}
            showDataLink
          />
        ))}
      </SpaceBetween>
    </SpaceBetween>
  );
}
