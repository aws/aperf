import React from "react";
import { ALL_FINDING_TYPES, ALL_STATS, DataType, FindingType, Stat } from "../../definitions/types";
import { RUNS, TIME_SERIES_DATA_TYPES } from "../../definitions/data-config";
import { STATISTICAL_FINDINGS, StatisticalFinding } from "../../utils/analytics";
import { useCollection } from "@cloudscape-design/collection-hooks";
import {
  Box,
  ColumnLayout,
  Container,
  Multiselect,
  Pagination,
  SpaceBetween,
  Table,
  TableProps,
} from "@cloudscape-design/components";
import { DataLink, SamePageDataLink } from "../misc/DataNavigation";
import { ReportHelpPanelIcon, ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { SelectProps } from "@cloudscape-design/components/select/interfaces";
import {
  formatNumber,
  getFindingTypeIconName,
  getFindingTypeReadableName,
  getTimeSeriesMetricUnit,
} from "../../utils/utils";
import { useReportState } from "../ReportStateProvider";

/**
 * Retrieves the relative statistical findings based on the filters.
 */
function getStatisticalFindings(
  runName: string,
  stats: Stat[],
  findingTypes: FindingType[],
  dataTypes: DataType[],
): StatisticalFinding[] {
  const isFindingTypeExpected = (score: number, expectedFindingTypes: FindingType[]): boolean => {
    if (score < 0) {
      return expectedFindingTypes.includes("negative");
    } else if (score > 0) {
      return expectedFindingTypes.includes("positive");
    } else {
      return expectedFindingTypes.includes("zero");
    }
  };
  return STATISTICAL_FINDINGS[runName].filter(
    (finding) =>
      stats.includes(finding.stat) &&
      dataTypes.includes(finding.dataType) &&
      isFindingTypeExpected(finding.score, findingTypes),
  );
}

/**
 * Helper functions to convert list of other types into multi-select options
 */
function dataTypesToOptions(dataTypes: ReadonlyArray<DataType>) {
  return dataTypes.map((dataType) => ({
    label: DATA_DESCRIPTIONS[dataType].readableName,
    value: dataType,
  }));
}
function statsToOptions(stats: ReadonlyArray<Stat>) {
  return stats.map((stat) => ({
    label: stat,
    value: stat,
  }));
}
function findingTypesToOptions(findingTypes: ReadonlyArray<FindingType>) {
  return findingTypes.map((findingType) => ({
    label: getFindingTypeReadableName(findingType),
    value: findingType,
    iconName: getFindingTypeIconName(findingType),
  }));
}

/**
 * Options to be used by the multi-select component in the statistical finding table
 */
const DATA_TYPE_OPTIONS = dataTypesToOptions(TIME_SERIES_DATA_TYPES);
const STAT_OPTIONS = statsToOptions(ALL_STATS);
const FINDING_TYPE_OPTIONS = findingTypesToOptions(ALL_FINDING_TYPES);

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
    id: "base_value",
    header: "Base Value",
    cell: (item: StatisticalFinding) => formatNumber(item.baseValue),
    width: 100,
  },
  {
    id: "stat_value",
    header: "Current Value",
    cell: (item: StatisticalFinding) => formatNumber(item.statValue),
    width: 100,
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
    selectedStatOptions.map((option) => option.value as Stat),
    selectedFindingTypeOptions.map((option) => option.value as FindingType),
    selectedDataTypeOptions.map((option) => option.value as DataType),
  );

  const columnDefinitions: TableProps.ColumnDefinition<StatisticalFinding>[] = [
    {
      id: "data",
      header: "Data",
      cell: (item) => (
        <div style={{ display: "inline" }}>
          <ReportHelpPanelIcon dataType={item.dataType} fieldKey={item.metricName} />
          <DataLink dataType={item.dataType} dataKey={item.metricName} />
        </div>
      ),
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
          info={<ReportHelpPanelLink dataType={"systeminfo"} fieldKey={"statisticalFinding"} />}
          counter={filteredItemsCount.toString()}
          actions={
            <SpaceBetween direction={"horizontal"} size={"xxs"}>
              <Multiselect
                enableSelectAll
                hideTokens
                filteringType={"auto"}
                options={DATA_TYPE_OPTIONS}
                selectedOptions={selectedDataTypeOptions}
                onChange={({ detail }) =>
                  updateStatisticalFindingsDataTypes(
                    props.runName,
                    detail.selectedOptions.map((option) => option.value as DataType),
                  )
                }
                placeholder={"Select data types"}
                i18nStrings={{
                  selectAllText: "Select all",
                }}
              />
              <Multiselect
                enableSelectAll
                hideTokens
                filteringType={"auto"}
                options={STAT_OPTIONS}
                selectedOptions={selectedStatOptions}
                onChange={({ detail }) =>
                  updateStatisticalFindingsStats(
                    props.runName,
                    detail.selectedOptions.map((option) => option.value as Stat),
                  )
                }
                placeholder={"Select stat"}
                i18nStrings={{
                  selectAllText: "Select all",
                }}
              />
              <Multiselect
                enableSelectAll
                hideTokens
                filteringType={"auto"}
                options={FINDING_TYPE_OPTIONS}
                selectedOptions={selectedFindingTypeOptions}
                onChange={({ detail }) =>
                  updateStatisticalFindingsTypes(
                    props.runName,
                    detail.selectedOptions.map((option) => option.value as FindingType),
                  )
                }
                placeholder={"Select finding type"}
                i18nStrings={{
                  selectAllText: "Select all",
                }}
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
    selectedStatOptions.map((option) => option.value as Stat),
    selectedFindingTypeOptions.map((option) => option.value as FindingType),
    [props.dataType],
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
              <Multiselect
                enableSelectAll
                hideTokens
                filteringType={"auto"}
                options={STAT_OPTIONS}
                selectedOptions={selectedStatOptions}
                onChange={({ detail }) => setSelectedStatOptions(detail.selectedOptions)}
                placeholder={"Select stat"}
                i18nStrings={{
                  selectAllText: "Select all",
                }}
              />
              <Multiselect
                enableSelectAll
                hideTokens
                filteringType={"auto"}
                options={FINDING_TYPE_OPTIONS}
                selectedOptions={selectedFindingTypeOptions}
                onChange={({ detail }) => setSelectedFindingTypeOptions(detail.selectedOptions)}
                placeholder={"Select finding type"}
                i18nStrings={{
                  selectAllText: "Select all",
                }}
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
