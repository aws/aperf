import { Box, KeyValuePairs, KeyValuePairsProps, Popover } from "@cloudscape-design/components";
import { PROCESSED_DATA } from "../../definitions/data-config";
import { KeyValueData } from "../../definitions/types";
import React from "react";
import Header from "@cloudscape-design/components/header";

const SYSTEM_INFO_ITEMS_CACHE = new Map<string, KeyValuePairsProps.Item[]>();

export function RunHeader(props: { runName: string }) {
  return (
    <Popover position={"top"} size={"large"} content={<RunSystemInfo runName={props.runName} />}>
      <Header variant={"h3"}>{props.runName}</Header>
    </Popover>
  );
}

export function RunSystemInfo(props: { runName: string }) {
  const curRunSystemInfo = (PROCESSED_DATA["systeminfo"].runs[props.runName] as KeyValueData)?.key_value_groups[""]
    ?.key_values;

  if (curRunSystemInfo == undefined) {
    return (
      <Box textAlign="center" color="inherit">
        <b>No system info collected</b>
        <Box variant="p" color="inherit">
          The system info was not collected in the APerf run
        </Box>
      </Box>
    );
  }

  let keyValueItems: KeyValuePairsProps.Item[];
  if (SYSTEM_INFO_ITEMS_CACHE.has(props.runName)) {
    keyValueItems = SYSTEM_INFO_ITEMS_CACHE.get(props.runName);
  } else {
    keyValueItems = [];
    for (const systemInfoKey in curRunSystemInfo) {
      keyValueItems.push({
        label: systemInfoKey,
        value: curRunSystemInfo[systemInfoKey],
      });
    }
    keyValueItems.sort((a: KeyValuePairsProps.Pair, b: KeyValuePairsProps.Pair) =>
      (a.label as string).localeCompare(b.label as string),
    );
    SYSTEM_INFO_ITEMS_CACHE.set(props.runName, keyValueItems);
  }

  return <KeyValuePairs columns={4} items={keyValueItems} />;
}
