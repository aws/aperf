import { ALL_DATA_TYPES, ALL_FINDING_TYPES, ALL_STATS, DataType, FindingType, Stat } from "../../definitions/types";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { getFindingTypeIconName, getFindingTypeReadableName } from "../../utils/utils";
import { OptionDefinition } from "@cloudscape-design/components/internal/components/option/interfaces";
import { SelectProps } from "@cloudscape-design/components/select/interfaces";
import { Icon, Multiselect } from "@cloudscape-design/components";
import React from "react";
import { TIME_SERIES_DATA_TYPES } from "../../definitions/data-config";

/**
 * Options to be used by the findings filters
 */
export const ANALYTICAL_FINDINGS_DATA_TYPE_OPTIONS = dataTypesToOptions(ALL_DATA_TYPES);
export const STATISTICAL_FINDINGS_DATA_TYPE_OPTIONS = dataTypesToOptions(TIME_SERIES_DATA_TYPES);
export const STAT_OPTIONS = statsToOptions(ALL_STATS);
export const FINDING_TYPE_OPTIONS = findingTypesToOptions(ALL_FINDING_TYPES);

/**
 * Helper functions to convert list of data types into multi-select options
 */
export function dataTypesToOptions(dataTypes: ReadonlyArray<DataType>) {
  return dataTypes.map((dataType) => ({
    label: dataType == "systeminfo" ? "System Info" : DATA_DESCRIPTIONS[dataType].readableName,
    value: dataType,
  }));
}

/**
 * Helper functions to convert list of stats into multi-select options
 */
export function statsToOptions(stats: ReadonlyArray<Stat>) {
  return stats.map((stat) => ({
    label: stat,
    value: stat,
  }));
}

/**
 * Helper functions to convert list of finding types into multi-select options
 */
export function findingTypesToOptions(findingTypes: ReadonlyArray<FindingType>) {
  return findingTypes.map((findingType) => ({
    label: getFindingTypeReadableName(findingType),
    value: findingType,
    iconName: getFindingTypeIconName(findingType),
  }));
}

/**
 * Checks if the finding's score belongs to one of the expected finding types.
 */
export function isFindingTypeExpected(score: number, expectedFindingTypes: FindingType[]): boolean {
  if (score < 0) {
    return expectedFindingTypes.includes("negative");
  } else if (score > 0) {
    return expectedFindingTypes.includes("positive");
  } else {
    return expectedFindingTypes.includes("zero");
  }
}

/**
 * A multi-select that helps filter the findings to be shown
 */
export function FindingsFilter(props: {
  options: SelectProps.Options;
  selectedOptions: ReadonlyArray<OptionDefinition>;
  setSelectedOptions: (options: ReadonlyArray<OptionDefinition>) => void;
  type: string;
}) {
  return (
    <Multiselect
      enableSelectAll
      hideTokens
      filteringType={"auto"}
      options={props.options}
      selectedOptions={props.selectedOptions}
      onChange={({ detail }) => props.setSelectedOptions(detail.selectedOptions)}
      placeholder={`Select ${props.type}`}
      i18nStrings={{
        selectAllText: "Select all",
      }}
    />
  );
}

/**
 * General guidance on how to use the findings.
 */
export function FindingsDescription() {
  return (
    <div style={{ display: "inline" }}>
      {"Click "}
      <Icon variant={"disabled"} name={"status-info"} />
      {" for optimization guides and more information. "}
      {"Click "}
      <Icon variant={"disabled"} name={"zoom-in"} />
      {" to preview the metric graph."}
    </div>
  );
}
