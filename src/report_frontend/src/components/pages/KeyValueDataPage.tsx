import { DataPageProps, DataType, KeyValueData } from "../../definitions/types";
import React from "react";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { Box, Pagination, Table, TableProps, TextFilter, Toggle } from "@cloudscape-design/components";
import { useCollection } from "@cloudscape-design/collection-hooks";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RunHeader } from "../data/RunSystemInfo";

const NUM_KEY_VALUE_PAIRS_PER_PAGE = 50;

type TableItem = { [key in string]: string };

/**
 * Transform processed key value data into formats required by the Table component
 */
function getTableItemsAndDefinitions(dataType: DataType) {
  // Collect all unique keys of a key value group across all runs
  const allKeysPerGroup = new Map<string, Set<string>>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as KeyValueData;
    if (reportData === undefined) continue;
    for (const groupName in reportData.key_value_groups) {
      if (!allKeysPerGroup.has(groupName)) {
        allKeysPerGroup.set(groupName, new Set<string>());
      }
      for (const key in reportData.key_value_groups[groupName].key_values) {
        allKeysPerGroup.get(groupName).add(key);
      }
    }
  }

  // TableItem includes the actual data to be shown in the table
  const tableItems: TableItem[] = [];
  for (const groupName of allKeysPerGroup.keys()) {
    for (const key of allKeysPerGroup.get(groupName)) {
      const tableItem: TableItem = {};
      tableItem.sectionName = groupName;
      tableItem.key = key;
      for (const runName of RUNS) {
        const reportData = PROCESSED_DATA[dataType].runs[runName] as KeyValueData;
        tableItem[runName] = reportData?.key_value_groups[groupName]?.key_values[key] || "";
      }
      tableItems.push(tableItem);
    }
  }

  // ColumnDefinition defines how the table items will be shown
  const tableColumnDefinitions: TableProps.ColumnDefinition<TableItem>[] = [];
  const isDummySection = allKeysPerGroup.size == 1 && allKeysPerGroup.has("");
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

  return { tableItems, tableColumnDefinitions };
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
          description={DATA_DESCRIPTIONS[props.dataType].summary}
          actions={
            <Toggle checked={showDiffOnly} onChange={({ detail }) => setShowDiffOnly(detail.checked)}>
              {"Only show different values"}
            </Toggle>
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
