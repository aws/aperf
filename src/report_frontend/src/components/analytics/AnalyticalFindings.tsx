import React from "react";
import {
  ALL_FINDING_TYPES,
  AnalyticalFinding,
  DataType,
  FindingType,
  TimeSeriesMetricProps,
} from "../../definitions/types";
import { Icon, SpaceBetween, Button, Container, ColumnLayout } from "@cloudscape-design/components";
import { PER_DATA_ANALYTICAL_FINDINGS, RUNS } from "../../definitions/data-config";
import { useReportState } from "../ReportStateProvider";
import { DataLink, SamePageDataLink } from "../misc/DataNavigation";
import { getFindingTypeIconName, getFindingTypeReadableName } from "../../utils/utils";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import { PER_RUN_ANALYTICAL_FINDINGS, RunAnalyticalFinding } from "../../utils/analytics";
import Header from "@cloudscape-design/components/header";

function linkifyText(text: string) {
  const urlRegex = /(https?:\/\/[^\s]+)/g;
  const parts = text.split(urlRegex);

  return parts.map((part, index) => {
    if (urlRegex.test(part)) {
      return (
        <a key={index} href={part} target="_blank" rel="noopener noreferrer">
          Reference
        </a>
      );
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
    return <Icon name={getFindingTypeIconName("positive")} />;
  } else if (props.score < 0) {
    return <Icon name={getFindingTypeIconName("negative")} />;
  } else {
    return <Icon name={getFindingTypeIconName("zero")} />;
  }
}

/**
 * Check if a finding's score is covered by the expected finding types
 */
function isFindingTypeExpected(score: number, findingTypes: Set<FindingType>): boolean {
  if (score < 0) return findingTypes.has("negative");
  if (score === 0 || isNaN(score)) return findingTypes.has("zero");
  if (score > 0) return findingTypes.has("positive");
}

/**
 * This component renders a single analytical finding. If showDataLink is true,
 * it contains a link that directs the user to the corresponding data page that this
 * finding belongs to. If showSamePageLink is true, the link only sets the filtering
 * text of the current page.
 */
interface FindingProps {
  readonly dataType?: DataType;
  readonly dataKey?: string;
  readonly finding: AnalyticalFinding;
  readonly showDataLink?: boolean;
  readonly showSamePageLink?: boolean;
}

function Finding(props: FindingProps) {
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
        {props.showDataLink && (
          <>
            <DataLink dataType={props.dataType || "systeminfo"} dataKey={props.dataKey || ""} /> (
            {<ReportHelpPanelLink dataType={props.dataType} fieldKey={props.dataKey} />}){": "}
          </>
        )}
        {props.showSamePageLink && (
          <>
            <SamePageDataLink dataKey={props.dataKey || ""} />
            {": "}
          </>
        )}
        {linkifyText(props.finding.description)}{" "}
      </div>
    </div>
  );
}

/**
 * This component renders all analytica findings of a specific time-series metric.
 */
export function MetricAnalyticalFindings(props: TimeSeriesMetricProps) {
  const dataFindings = PER_DATA_ANALYTICAL_FINDINGS[props.dataType];
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

/**
 * Helper component to render the container that contains the list of findings as well as type-filtering buttons.
 */
function FindingsContainer(props: {
  findings: RunAnalyticalFinding[];
  findingTypes: Set<FindingType>;
  toggleFindingType: (findingType: FindingType) => void;
  samePageLink?: boolean;
}) {
  return (
    <Container
      fitHeight
      header={
        <Header
          variant={"h3"}
          info={
            props.samePageLink ? null : <ReportHelpPanelLink dataType={"systeminfo"} fieldKey={"analyticalFinding"} />
          }
        >
          Analytical Findings
        </Header>
      }
    >
      <SpaceBetween size={"s"}>
        <SpaceBetween direction="horizontal" size="xs">
          {ALL_FINDING_TYPES.map((findingType) => (
            <Button
              variant={props.findingTypes.has(findingType) ? "primary" : "normal"}
              onClick={() => props.toggleFindingType(findingType)}
              iconName={getFindingTypeIconName(findingType)}
            >
              {getFindingTypeReadableName(findingType)}
            </Button>
          ))}
        </SpaceBetween>
        <SpaceBetween size={"xxxs"}>
          {props.findings.map((findingData) => (
            <Finding
              dataType={findingData.dataType}
              dataKey={findingData.dataKey}
              finding={findingData.finding}
              showDataLink={!props.samePageLink}
              showSamePageLink={props.samePageLink}
            />
          ))}
        </SpaceBetween>
      </SpaceBetween>
    </Container>
  );
}

/**
 * This component renders the flattened list of all analytical findings within the given run
 * for all the data type, sorted by the score in ascending order. It is to be rendered in the
 * report home page to provide a holistic overview.
 */
export function GlobalAnalyticalFindings(props: { runName: string }) {
  const { analyticalFindingsTypes, setAnalyticalFindingsTypes } = useReportState();
  const findingTypes = analyticalFindingsTypes[props.runName] || new Set(ALL_FINDING_TYPES);

  const filteredFindings = PER_RUN_ANALYTICAL_FINDINGS[props.runName].filter((findingData) =>
    isFindingTypeExpected(findingData.finding.score, findingTypes),
  );

  const toggleFindingType = (filter: FindingType) => {
    const newFilters = new Set(findingTypes);
    if (newFilters.has(filter)) {
      newFilters.delete(filter);
    } else {
      newFilters.add(filter);
    }
    setAnalyticalFindingsTypes(props.runName, newFilters);
  };

  return (
    <FindingsContainer findings={filteredFindings} findingTypes={findingTypes} toggleFindingType={toggleFindingType} />
  );
}

/**
 * This component renders the flattened list of all analytical findings within a run for the specified
 * data type. It is to be rendered within a data's page.
 */
function LocalAnalyticalFindings(props: { runName: string; dataType: DataType }) {
  const [findingTypes, setFindingTypes] = React.useState<Set<FindingType>>(new Set(ALL_FINDING_TYPES));

  const filteredFindings = PER_RUN_ANALYTICAL_FINDINGS[props.runName].filter(
    (findingData) =>
      findingData.dataType == props.dataType && isFindingTypeExpected(findingData.finding.score, findingTypes),
  );

  const toggleFindingType = (findingType: FindingType) => {
    const newFindingTypes = new Set(findingTypes);
    if (newFindingTypes.has(findingType)) {
      newFindingTypes.delete(findingType);
    } else {
      newFindingTypes.add(findingType);
    }
    setFindingTypes(newFindingTypes);
  };

  return (
    <FindingsContainer
      findings={filteredFindings}
      findingTypes={findingTypes}
      toggleFindingType={toggleFindingType}
      samePageLink
    />
  );
}

/**
 * This component collects the analytical finding tables of all runs for a data type and
 * renders them side by side.
 */
export function DataTypeAnalyticalFindings(props: { dataType: DataType }) {
  return (
    <ColumnLayout columns={RUNS.length}>
      {RUNS.map((runName) => {
        return <LocalAnalyticalFindings runName={runName} dataType={props.dataType} />;
      })}
    </ColumnLayout>
  );
}
