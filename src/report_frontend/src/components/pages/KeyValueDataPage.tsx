import { DataPageProps, DataType, KeyValueData } from "../../definitions/types";
import React from "react";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { Box, Pagination, SpaceBetween, Table, TableProps, TextFilter, Toggle } from "@cloudscape-design/components";
import { useCollection } from "@cloudscape-design/collection-hooks";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RunHeader } from "../data/RunSystemInfo";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import { ShowFindingsPanelButton } from "../analytics/FindingsSplitPanel";
import { useReportState } from "../ReportStateProvider";

const NUM_KEY_VALUE_PAIRS_PER_PAGE = 50;

type TableItem = { [key in string]: string };

/**
 * Transform processed key value data into formats required by the Table component
 */
function getTableItemsAndDefinitions(dataType: DataType) {
  // If there is only one KeyValueGroup, and it is an empty string, we consider it as dummy
  // and the table will not show the "section"
  let isDummySection = true;
  // The map from lower-case key-value group name + key to TableItem - we want to make
  // group names and keys with only case differences to be still placed at the same row
  const keyValueTableItems = new Map<string, TableItem>();

  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as KeyValueData;
    if (reportData === undefined) continue;
    for (const groupName in reportData.key_value_groups) {
      if (groupName != "") isDummySection = false;
      for (const [key, value] of Object.entries(reportData.key_value_groups[groupName].key_values)) {
        const tableItemsKey = `${groupName} ${key}`.toLowerCase();
        // If a group name or key is only case-different across runs, they will be placed
        // at the same row in the table which shows the first occurrence of the group name
        // or key
        if (!keyValueTableItems.has(tableItemsKey)) {
          const newTableItem = {
            sectionName: groupName,
            key: key,
          };
          for (const runName of RUNS) {
            newTableItem[runName] = "";
          }
          keyValueTableItems.set(tableItemsKey, newTableItem);
        }
        const tableItem = keyValueTableItems.get(tableItemsKey);
        tableItem[runName] = value;
      }
    }
  }

  // ColumnDefinition defines how the table items will be shown
  const tableColumnDefinitions: TableProps.ColumnDefinition<TableItem>[] = [];
  if (!isDummySection) {
    tableColumnDefinitions.push({
      id: "section_name",
      header: "Section",
      cell: (item) => item.sectionName,
      isRowHeader: true,
      sortingField: "sectionName",
    });
  }
  tableColumnDefinitions.push({
    id: "key",
    header: "Key",
    cell: (item) => <b>{item.key}</b>,
    isRowHeader: isDummySection,
    sortingField: "key",
  });
  for (const runName of RUNS) {
    tableColumnDefinitions.push({
      id: `${runName}-value`,
      header: <RunHeader runName={runName} />,
      cell: (item) => item[runName],
    });
  }

  return { tableItems: Array.from(keyValueTableItems.values()), tableColumnDefinitions };
}

/**
 * Filter for table items values are different across runs
 */
function filterItemsWithDiffs(tableItems: TableItem[]): TableItem[] {
  return tableItems.filter((tableItem) => {
    const uniqueValues = new Set<string>();
    for (const runName of RUNS) {
      uniqueValues.add(tableItem[runName]);
    }
    return uniqueValues.size > 1;
  });
}

/**
 * This component renders a page for key value data in the form of a table. Values of the same key
 * across all runs will be shown at the same line.
 */
export default function (props: DataPageProps) {
  const { searchKey, setUpdateFilteringText } = useReportState();

  const { tableItems, tableColumnDefinitions } = React.useMemo(
    () => getTableItemsAndDefinitions(props.dataType),
    [props.dataType],
  );

  const [showDiffOnly, setShowDiffOnly] = React.useState(true);

  const filteredTableItems = showDiffOnly && RUNS.length > 1 ? filterItemsWithDiffs(tableItems) : tableItems;

  const { items, filteredItemsCount, collectionProps, filterProps, paginationProps } = useCollection(
    filteredTableItems,
    {
      filtering: {
        filteringFunction: (item: TableItem, filteringText: string) => {
          const filteringTextLower = filteringText.toLowerCase();
          return (
            item.sectionName.toLowerCase().includes(filteringTextLower) ||
            item.key.toLowerCase().includes(filteringTextLower)
          );
        },
        empty: <Box variant={"p"}>{showDiffOnly ? "All keys have the same value" : "No items were collected"}</Box>,
        noMatch: <Box variant={"p"}>No items found</Box>,
        defaultFilteringText: searchKey,
      },
      pagination: { pageSize: NUM_KEY_VALUE_PAIRS_PER_PAGE },
      selection: {},
      sorting: {
        defaultState: {
          sortingColumn: tableColumnDefinitions[0],
        },
      },
    },
  );

  React.useEffect(() => {
    // Store the function to update the filtering text in the global state, so that they can be accessed
    // by other components to change the filtering text and locate a particular metric.
    // To be distinguished from the function argument supported by the React set state API, we need to
    // pass in a function that returns the actual function.
    setUpdateFilteringText(() => (text: string) => filterProps.onChange({ detail: { filteringText: text } }));
  }, [props.dataType]);

  return (
    <Table
      {...collectionProps}
      enableKeyboardNavigation={true}
      resizableColumns={true}
      pagination={<Pagination {...paginationProps} />}
      stickyHeader={true}
      header={
        <Header
          variant={"awsui-h1-sticky"}
          info={<ReportHelpPanelLink dataType={props.dataType} fieldKey={"summary"} />}
          actions={
            <SpaceBetween direction={"horizontal"} size={"xs"}>
              <ShowFindingsPanelButton />
              <Toggle checked={showDiffOnly} onChange={({ detail }) => setShowDiffOnly(detail.checked)}>
                {"Only show different values"}
              </Toggle>
            </SpaceBetween>
          }
        >
          {DATA_DESCRIPTIONS[props.dataType].readableName}
        </Header>
      }
      filter={
        <TextFilter
          {...filterProps}
          filteringPlaceholder={"Find items"}
          countText={`${filteredItemsCount} items found`}
        />
      }
      variant={"full-page"}
      items={items}
      columnDefinitions={tableColumnDefinitions}
    />
  );
}
