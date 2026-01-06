import React from "react";
import { AnalyticalFinding, DataType, FindingType, TimeSeriesMetricProps } from "../../definitions/types";
import {
  Icon,
  SpaceBetween,
  Container,
  ColumnLayout,
  Link,
  Box,
  TextFilter,
  Pagination,
  TextFilterProps,
  PaginationProps,
} from "@cloudscape-design/components";
import { PER_DATA_ANALYTICAL_FINDINGS, RUNS } from "../../definitions/data-config";
import { useReportState } from "../ReportStateProvider";
import { DataLink, SamePageDataLink } from "../misc/DataNavigation";
import { getFindingTypeIconName } from "../../utils/utils";
import { ReportHelpPanelIcon, ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import { PER_RUN_ANALYTICAL_FINDINGS, RunAnalyticalFinding } from "../../utils/analytics";
import Header from "@cloudscape-design/components/header";
import {
  ANALYTICAL_FINDINGS_DATA_TYPE_OPTIONS,
  dataTypesToOptions,
  FINDING_TYPE_OPTIONS,
  FindingsFilter,
  findingTypesToOptions,
  isFindingTypeExpected,
} from "./common";
import { SelectProps } from "@cloudscape-design/components/select/interfaces";
import { useCollection } from "@cloudscape-design/collection-hooks";
import { SingleMetricGraphPopover } from "../data/MetricGraph";

function getFindingColor(score: number): string {
  if (score == 0 || isNaN(score)) {
    return "rgba(125, 125, 125, 0.3)";
  } else if (score > 0) {
    return `rgba(0, ${Math.min(100 + score / 2, 255)}, 0, 0.3)`;
  } else {
    return `rgba(${Math.min(100 + Math.abs(score / 2), 255)}, 0, 0, 0.3)`;
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
 * Retrieves the relative analytical findings based on the filters.
 */
function getAnalyticalFindings(runName: string, dataTypes: DataType[], findingTypes: FindingType[]) {
  return PER_RUN_ANALYTICAL_FINDINGS[runName].filter(
    (findingData) =>
      dataTypes.includes(findingData.dataType) && isFindingTypeExpected(findingData.finding.score, findingTypes),
  );
}

/**
 * This component renders a single analytical finding. If showDataLink is true,
 * it contains a link that directs the user to the corresponding data page that this
 * finding belongs to. If showSamePageLink is true, the link only sets the filtering
 * text of the current page.
 */
interface FindingProps {
  readonly dataType?: DataType;
  readonly runName?: string;
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
      }}
    >
      {/*The below div creates the first row that pushes the two children to the left and right end*/}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "5px",
          width: "100%",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flex: "1" }}>
          <FindingIcon score={props.finding.score} />
          <Box variant={"h4"}>{props.finding.rule_name}</Box>
        </div>

        {props.showDataLink && (
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexShrink: 0 }}>
            <ReportHelpPanelIcon dataType={props.dataType} fieldKey={props.dataKey} />
            <SingleMetricGraphPopover dataType={props.dataType} runName={props.runName} metricName={props.dataKey} />
          </div>
        )}
      </div>

      <div style={{ display: "inline" }}>
        {props.showDataLink && (
          <>
            <DataLink dataType={props.dataType || "systeminfo"} dataKey={props.dataKey || ""} />
            {": "}
          </>
        )}
        {props.showSamePageLink && (
          <>
            <SamePageDataLink dataKey={props.dataKey || ""} />
            {": "}
          </>
        )}
        {props.finding.description} <b>{props.finding.message}</b>{" "}
        {props.finding.reference && (
          <Link variant={"info"} external href={props.finding.reference}>
            Learn more
          </Link>
        )}
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
 * Defines how filtering texts (in the search bar) can be used to filter analytical findings
 */
function filterFindings(finding: RunAnalyticalFinding, filteringText: string) {
  const lowerCaseFilteringText = filteringText.toLowerCase();
  return (
    finding.finding.rule_name.toLowerCase().includes(lowerCaseFilteringText) ||
    finding.finding.message.toLowerCase().includes(lowerCaseFilteringText)
  );
}

/**
 * Helper component to render the search bar and pagination at the same row
 */
function FindingsSearchBarAndPagination(props: { filterProps: TextFilterProps; paginationProps: PaginationProps }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        flexWrap: "wrap",
        gap: "16px",
      }}
    >
      <div style={{ flexGrow: 1, flexShrink: 1, flexBasis: "fit-content", maxWidth: "70%" }}>
        <TextFilter {...props.filterProps} filteringPlaceholder={"Search findings"} />
      </div>
      <div style={{ marginLeft: "auto" }}>
        <Pagination {...props.paginationProps} />
      </div>
    </div>
  );
}

/**
 * This component renders the flattened list of all analytical findings within the given run
 * for all the data type, sorted by the score in ascending order. It is to be rendered in the
 * report home page to provide a holistic overview.
 */
export function GlobalAnalyticalFindings(props: { runName: string }) {
  const {
    analyticalFindingsDataTypes,
    analyticalFindingsTypes,
    updateAnalyticalFindingsDataTypes,
    updateAnalyticalFindingsTypes,
  } = useReportState();

  const selectedDataTypeOptions = dataTypesToOptions(analyticalFindingsDataTypes[props.runName]);
  const selectedFindingTypeOptions = findingTypesToOptions(analyticalFindingsTypes[props.runName]);

  const findings = getAnalyticalFindings(
    props.runName,
    analyticalFindingsDataTypes[props.runName],
    analyticalFindingsTypes[props.runName],
  );

  const { items, filterProps, filteredItemsCount, paginationProps } = useCollection(findings, {
    pagination: { pageSize: 5 },
    filtering: {
      filteringFunction: filterFindings,
    },
  });

  return (
    <Container
      fitHeight
      header={
        <Header
          variant={"h3"}
          counter={`${filteredItemsCount}`}
          info={<ReportHelpPanelLink dataType={"systeminfo"} fieldKey={"analyticalFinding"} />}
          actions={
            <SpaceBetween direction={"horizontal"} size={"xxs"}>
              <FindingsFilter
                options={ANALYTICAL_FINDINGS_DATA_TYPE_OPTIONS}
                selectedOptions={selectedDataTypeOptions}
                setSelectedOptions={(options) =>
                  updateAnalyticalFindingsDataTypes(
                    props.runName,
                    options.map((option) => option.value as DataType),
                  )
                }
                type={"data types"}
              />
              <FindingsFilter
                options={FINDING_TYPE_OPTIONS}
                selectedOptions={selectedFindingTypeOptions}
                setSelectedOptions={(options) =>
                  updateAnalyticalFindingsTypes(
                    props.runName,
                    options.map((option) => option.value as FindingType),
                  )
                }
                type={"finding types"}
              />
            </SpaceBetween>
          }
        >
          Analytical Findings
        </Header>
      }
    >
      <SpaceBetween size={"xxxs"}>
        <FindingsSearchBarAndPagination filterProps={filterProps} paginationProps={paginationProps} />
        {items.map((findingData) => (
          <Finding
            dataType={findingData.dataType}
            runName={props.runName}
            dataKey={findingData.dataKey}
            finding={findingData.finding}
            showDataLink
          />
        ))}
      </SpaceBetween>
    </Container>
  );
}

/**
 * This component renders the flattened list of all analytical findings within a run for the specified
 * data type. It is to be rendered within a data's page.
 */
function LocalAnalyticalFindings(props: { runName: string; dataType: DataType }) {
  const [selectedFindingTypeOptions, setSelectedFindingTypeOptions] =
    React.useState<Readonly<SelectProps.Option[]>>(FINDING_TYPE_OPTIONS);
  const selectedFindingTypes = selectedFindingTypeOptions.map((option) => option.value as FindingType);

  const findings = getAnalyticalFindings(props.runName, [props.dataType], selectedFindingTypes);

  const { items, filterProps, filteredItemsCount, paginationProps } = useCollection(findings, {
    pagination: { pageSize: 3 },
    filtering: {
      filteringFunction: filterFindings,
    },
  });

  return (
    <Container
      fitHeight
      header={
        <Header
          variant={"h3"}
          counter={`${filteredItemsCount}`}
          actions={
            <SpaceBetween direction={"horizontal"} size={"xxs"}>
              <FindingsFilter
                options={FINDING_TYPE_OPTIONS}
                selectedOptions={selectedFindingTypeOptions}
                setSelectedOptions={setSelectedFindingTypeOptions}
                type={"finding types"}
              />
            </SpaceBetween>
          }
        >
          {props.runName}
        </Header>
      }
    >
      <SpaceBetween size={"xxxs"}>
        <FindingsSearchBarAndPagination filterProps={filterProps} paginationProps={paginationProps} />
        {items.map((findingData) => (
          <Finding
            dataType={findingData.dataType}
            runName={props.runName}
            dataKey={findingData.dataKey}
            finding={findingData.finding}
            showSamePageLink
          />
        ))}
      </SpaceBetween>
    </Container>
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
