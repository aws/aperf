import React from "react";
import { KeyValueData } from "../../definitions/types";
import { RUNS } from "../../definitions/data-config";
import { TableProps } from "@cloudscape-design/components";
import { RunHeader } from "./RunSystemInfo";

export type TableItem = { [key in string]: string };

/**
 * Build table items and column definitions from a map of run name to KeyValueData.
 * Shared by KeyValueDataPage (top-level key-value data) and GraphMetadata (profiler metadata).
 */
export function buildKeyValueTable(dataByRun: Map<string, KeyValueData | undefined>) {
  let isDummySection = true;
  const keyValueTableItems = new Map<string, TableItem>();

  for (const [runName, data] of dataByRun) {
    if (!data?.key_value_groups) continue;
    for (const groupName in data.key_value_groups) {
      if (groupName !== "") isDummySection = false;
      for (const [key, value] of Object.entries(data.key_value_groups[groupName].key_values)) {
        const tableItemsKey = `${groupName} ${key}`.toLowerCase();
        if (!keyValueTableItems.has(tableItemsKey)) {
          const newTableItem: TableItem = { sectionName: groupName, key: key };
          for (const rn of RUNS) {
            newTableItem[rn] = "";
          }
          keyValueTableItems.set(tableItemsKey, newTableItem);
        }
        keyValueTableItems.get(tableItemsKey)![runName] = value;
      }
    }
  }

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
