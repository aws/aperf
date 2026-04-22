import React from "react";
import { DataPageProps, DataType, ProfilingData } from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import {
  Box,
  Cards,
  CardsProps,
  Pagination,
  SegmentedControl,
  SegmentedControlProps,
  TextFilter,
} from "@cloudscape-design/components";
import { useCollection } from "@cloudscape-design/collection-hooks";
import Header from "@cloudscape-design/components/header";
import IframeGraph from "../data/IframeGraph";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RunHeader } from "../data/RunSystemInfo";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";

const NUM_PROFILERS_PER_PAGE = 10;

/**
 * Collect all profile names across all runs and profilers and transform them into the format
 * required by SegmentedControl
 */
function getAllProfileNames(dataType: DataType): SegmentedControlProps.Option[] {
  const profileNames: string[] = [];
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as ProfilingData;
    if (reportData == undefined) continue;
    for (const profiler of Object.values(reportData.profilers)) {
      for (const profileName in profiler.profiles) {
        if (!profileNames.includes(profileName)) {
          profileNames.push(profileName);
        }
      }
    }
  }
  return profileNames
    .sort((a, b) => b.localeCompare(a))
    .map((profileName) => ({
      id: profileName,
      text: DATA_DESCRIPTIONS[dataType].fieldDescriptions[profileName]?.readableName || profileName,
    }));
}

/**
 * Compute the list of profiler instance names that have the given profile, sorted by size
 */
function getProfilerNames(dataType: DataType, profileName: string): string[] {
  const profilerSizes = new Map<string, number>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as ProfilingData;
    if (reportData == undefined) continue;
    for (const [instanceName, profiler] of Object.entries(reportData.profilers)) {
      const profile = profiler.profiles[profileName];
      if (profile == undefined) continue;
      if (!profilerSizes.has(instanceName)) profilerSizes.set(instanceName, 0);
      profilerSizes.set(instanceName, profilerSizes.get(instanceName) + (profile.profile_graph?.graph_size || 0));
    }
  }
  return Array.from(profilerSizes.keys()).sort((a, b) => profilerSizes.get(b) - profilerSizes.get(a));
}

/**
 * This component renders the page for ProfilingData data type, where the graphs are rendered within Iframes
 */
export default function (props: DataPageProps) {
  const allProfileNames = React.useMemo(() => getAllProfileNames(props.dataType), [props.dataType]);

  const [profileName, setProfileName] = React.useState(allProfileNames[0]?.id || "");

  const graphRowPercentage = Math.floor(100 / RUNS.length);
  const cardDefinition: CardsProps.CardDefinition<string> = {
    header: (instanceName: string) => <Header variant={"h2"}>{instanceName}</Header>,
    sections: RUNS.map((runName) => ({
      id: runName,
      header: <RunHeader runName={runName} />,
      content: (instanceName) => (
        <div style={{ paddingTop: "10px", paddingRight: "30px" }}>
          <IframeGraph
            dataType={props.dataType}
            runName={runName}
            profilerName={instanceName}
            graphName={profileName}
          />
        </div>
      ),
      width: graphRowPercentage,
    })),
  };

  const sortedProfilerNames = getProfilerNames(props.dataType, profileName);
  const { items, filteredItemsCount, collectionProps, filterProps, paginationProps } = useCollection(
    sortedProfilerNames,
    {
      filtering: {
        filteringFunction: (item: string, filteringText: string) =>
          item.toLowerCase().includes(filteringText.toLowerCase()),
        empty: <Box variant={"p"}>No profilers were collected</Box>,
        noMatch: <Box variant={"p"}>No profilers found</Box>,
      },
      pagination: { pageSize: Math.floor(NUM_PROFILERS_PER_PAGE / RUNS.length) },
    },
  );

  return (
    <Cards
      {...collectionProps}
      cardsPerRow={[{ cards: 1 }]}
      pagination={<Pagination {...paginationProps} />}
      stickyHeader={true}
      header={
        <Header
          variant={"awsui-h1-sticky"}
          info={<ReportHelpPanelLink dataType={props.dataType} fieldKey={"summary"} />}
          actions={
            allProfileNames.length > 1 && (
              <SegmentedControl
                selectedId={profileName}
                onChange={({ detail }) => setProfileName(detail.selectedId)}
                options={allProfileNames}
              />
            )
          }
        >
          {DATA_DESCRIPTIONS[props.dataType].readableName}
        </Header>
      }
      filter={
        <TextFilter
          {...filterProps}
          filteringPlaceholder={"Find profilers"}
          countText={`${filteredItemsCount} profilers found`}
        />
      }
      variant={"full-page"}
      items={items}
      cardDefinition={cardDefinition}
    />
  );
}
