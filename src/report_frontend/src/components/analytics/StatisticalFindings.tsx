import React from "react";
import { DataType, FindingType, Stat } from "../../definitions/types";
import { RUNS } from "../../definitions/data-config";
import { STATISTICAL_FINDINGS, StatisticalFinding } from "../../utils/analytics";
import { useCollection } from "@cloudscape-design/collection-hooks";
import {
  Box,
  ColumnLayout,
  Container,
  Pagination,
  SpaceBetween,
  Table,
  TableProps,
} from "@cloudscape-design/components";
import { DataLink, SamePageDataLink } from "../misc/DataNavigation";
import { ReportHelpPanelIcon, ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import Header from "@cloudscape-design/components/header";
import { SelectProps } from "@cloudscape-design/components/select/interfaces";
import { formatNumber, getTimeSeriesMetricUnit } from "../../utils/utils";
import { useReportState } from "../ReportStateProvider";
import {
  dataTypesToOptions,
  FINDING_TYPE_OPTIONS,
  FindingsDescription,
  FindingsFilter,
  findingTypesToOptions,
  isFindingTypeExpected,
  STAT_OPTIONS,
  STATISTICAL_FINDINGS_DATA_TYPE_OPTIONS,
  statsToOptions,
} from "./common";
import { MetricGraphsPopover } from "../data/MetricGraph";

/**
 * Retrieves the relative statistical findings based on the filters.
 */
function getStatisticalFindings(
  runName: string,
  dataTypes: DataType[],
  stats: Stat[],
  findingTypes: FindingType[],
): StatisticalFinding[] {
  return STATISTICAL_FINDINGS[runName].filter(
    (finding) =>
      stats.includes(finding.stat) &&
      dataTypes.includes(finding.dataType) &&
      isFindingTypeExpected(finding.score, findingTypes),
  );
}

/**
 * The common column definitions to be used by the global and per-data statistical finding table.
 */
const COMMON_COLUMN_DEFINITIONS = [
  {
    id: "stat",
    header: "Stat",
    cell: (item: StatisticalFinding) => item.stat,
    sortingComparator: (a: StatisticalFinding, b: StatisticalFinding) => {
      if (a.stat == b.stat) return Math.abs(b.score) - Math.abs(a.score);
      return a.stat.localeCompare(b.stat);
    },
    width: 80,
  },
  {
    id: "delta",
    header: "Delta",
    cell: (item: StatisticalFinding) => item.deltaString,
    sortingComparator: (a: StatisticalFinding, b: StatisticalFinding) => Math.abs(b.score) - Math.abs(a.score),
    width: 100,
  },
  {
    id: "stat_value",
    header: "Value",
    cell: (item: StatisticalFinding) => formatNumber(item.statValue),
    width: 80,
  },
  {
    id: "base_value",
    header: "Base",
    cell: (item: StatisticalFinding) => formatNumber(item.baseValue),
    width: 80,
  },
  {
    id: "unit",
    header: "Unit",
    cell: (item: StatisticalFinding) => getTimeSeriesMetricUnit(item.dataType, item.metricName),
    width: 100,
  },
];

/**
 * This component renders simple information to indicate that statistical findings are not
 * available for base run.
 */
function BaseRunNoFindings(props: { title: string }) {
  return (
    <Container header={<Header variant={"h3"}>{props.title}</Header>}>
      <Box variant={"p"}>{`The base run does not have statistical findings.`}</Box>
    </Container>
  );
}

/**
 * This component renders a table that contains all statistical findings within a run for all the data types.
 * The component is to be rendered within the report home page to provide a holistic view over all the statistical
 * findings of all time-series data.
 */
export function GlobalStatisticalFindings(props: { runName: string }) {
  if (props.runName == RUNS[0]) return <BaseRunNoFindings title={"Statistical Findings"} />;

  // The selected options should be retained even if users navigate away from the home page,
  // so they need to be stored in global context
  const {
    statisticalFindingsDataTypes,
    statisticalFindingsStats,
    statisticalFindingsTypes,
    updateStatisticalFindingsDataTypes,
    updateStatisticalFindingsStats,
    updateStatisticalFindingsTypes,
  } = useReportState();

  const selectedDataTypeOptions = dataTypesToOptions(statisticalFindingsDataTypes[props.runName]);
  const selectedStatOptions = statsToOptions(statisticalFindingsStats[props.runName]);
  const selectedFindingTypeOptions = findingTypesToOptions(statisticalFindingsTypes[props.runName]);

  const findings = getStatisticalFindings(
    props.runName,
    statisticalFindingsDataTypes[props.runName],
    statisticalFindingsStats[props.runName],
    statisticalFindingsTypes[props.runName],
  );

  const columnDefinitions: TableProps.ColumnDefinition<StatisticalFinding>[] = [
    {
      id: "actions",
      header: "",
      cell: (item) => (
        <div style={{ display: "inline" }}>
          <ReportHelpPanelIcon dataType={item.dataType} fieldKey={item.metricName} />
          <MetricGraphsPopover dataType={item.dataType} runName={item.runName} metricName={item.metricName} />
        </div>
      ),
      width: 75,
    },
    {
      id: "data",
      header: "Data",
      cell: (item) => <DataLink dataType={item.dataType} dataKey={item.metricName} />,
      isRowHeader: true,
      width: 250,
      sortingField: "dataType",
    },
    ...COMMON_COLUMN_DEFINITIONS,
  ];

  const { items, filteredItemsCount, collectionProps, paginationProps } = useCollection(findings, {
    pagination: { pageSize: 20 },
    sorting: {
      defaultState: {
        sortingColumn: columnDefinitions[3],
      },
    },
    filtering: {
      empty: <Box variant={"p"}>No statistical findings found</Box>,
    },
  });

  return (
    <Table
      {...collectionProps}
      enableKeyboardNavigation
      resizableColumns
      contentDensity={"compact"}
      pagination={<Pagination {...paginationProps} />}
      header={
        <Header
          variant={"h3"}
          info={<ReportHelpPanelLink dataType={"systeminfo"} fieldKey={"statisticalFinding"} />}
          counter={filteredItemsCount.toString()}
          description={<FindingsDescription />}
          actions={
            <SpaceBetween direction={"horizontal"} size={"xxs"}>
              <FindingsFilter
                options={STATISTICAL_FINDINGS_DATA_TYPE_OPTIONS}
                selectedOptions={selectedDataTypeOptions}
                setSelectedOptions={(options) =>
                  updateStatisticalFindingsDataTypes(
                    props.runName,
                    options.map((option) => option.value as DataType),
                  )
                }
                type={"data types"}
              />
              <FindingsFilter
                options={STAT_OPTIONS}
                selectedOptions={selectedStatOptions}
                setSelectedOptions={(options) =>
                  updateStatisticalFindingsStats(
                    props.runName,
                    options.map((option) => option.value as Stat),
                  )
                }
                type={"stats"}
              />
              <FindingsFilter
                options={FINDING_TYPE_OPTIONS}
                selectedOptions={selectedFindingTypeOptions}
                setSelectedOptions={(options) =>
                  updateStatisticalFindingsTypes(
                    props.runName,
                    options.map((option) => option.value as FindingType),
                  )
                }
                type={"finding types"}
              />
            </SpaceBetween>
          }
        >
          {"Statistical Findings"}
        </Header>
      }
      items={items}
      columnDefinitions={columnDefinitions}
    />
  );
}

/**
 * This component renders a table that contains the statistical findings of the specified data type within
 * a run. It is to be rendered within a data page (so it doesn't need to support a data filter)
 */
function LocalStatisticalFindings(props: { runName: string; dataType: DataType }) {
  // The selected filters are not retained when users navigate away from the page, for simplicity
  const [selectedStatOptions, setSelectedStatOptions] = React.useState<Readonly<SelectProps.Option[]>>([
    STAT_OPTIONS[0],
  ]);
  const [selectedFindingTypeOptions, setSelectedFindingTypeOptions] = React.useState<Readonly<SelectProps.Option[]>>([
    FINDING_TYPE_OPTIONS[0],
  ]);

  const findings = getStatisticalFindings(
    props.runName,
    [props.dataType],
    selectedStatOptions.map((option) => option.value as Stat),
    selectedFindingTypeOptions.map((option) => option.value as FindingType),
  );

  const columnDefinitions: TableProps.ColumnDefinition<StatisticalFinding>[] = [
    {
      id: "data",
      header: "Data",
      cell: (item) => <SamePageDataLink dataKey={item.metricName} />,
      isRowHeader: true,
      width: 150,
      sortingField: "dataType",
    },
    ...COMMON_COLUMN_DEFINITIONS,
  ];

  const { items, filteredItemsCount, collectionProps, paginationProps } = useCollection(findings, {
    pagination: { pageSize: 10 },
    sorting: {
      defaultState: {
        sortingColumn: columnDefinitions[2],
      },
    },
    filtering: {
      empty: <Box variant={"p"}>No statistical findings found</Box>,
    },
  });

  return (
    <Table
      {...collectionProps}
      enableKeyboardNavigation
      resizableColumns
      contentDensity={"compact"}
      pagination={<Pagination {...paginationProps} />}
      header={
        <Header
          variant={"h3"}
          counter={filteredItemsCount.toString()}
          actions={
            <SpaceBetween direction={"horizontal"} size={"xxs"}>
              <FindingsFilter
                options={STAT_OPTIONS}
                selectedOptions={selectedStatOptions}
                setSelectedOptions={setSelectedStatOptions}
                type={"stats"}
              />
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
      items={items}
      columnDefinitions={columnDefinitions}
    />
  );
}

/**
 * This component collects the statistical finding tables of all runs for a data type and
 * renders them side by side.
 */
export function DataTypeStatisticalFindings(props: { dataType: DataType }) {
  return (
    <ColumnLayout columns={RUNS.length}>
      {RUNS.map((runName, index) => {
        if (index == 0) return <BaseRunNoFindings title={runName} />;
        return <LocalStatisticalFindings runName={runName} dataType={props.dataType} />;
      })}
    </ColumnLayout>
  );
}
