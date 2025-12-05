import React from "react";
import { DataPageProps, DataType, GraphData } from "../../definitions/types";
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

const NUM_GRAPHS_PER_PAGE = 10;

/**
 * Collect all graph groups across all runs and transform them into the format required
 * by SegmentedControl
 */
function getAllGraphGroups(dataType: DataType): SegmentedControlProps.Option[] {
  const graphGroupNames: string[] = [];
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as GraphData;
    if (reportData == undefined) continue;
    for (const graphGroup of reportData.graph_groups) {
      if (!graphGroupNames.includes(graphGroup.group_name)) {
        graphGroupNames.push(graphGroup.group_name);
      }
    }
  }

  const allGraphGroups: SegmentedControlProps.Option[] = [];
  for (const groupName of graphGroupNames) {
    allGraphGroups.push({
      id: groupName,
      text: DATA_DESCRIPTIONS[dataType].fieldDescriptions[groupName]?.readableName || groupName,
    });
  }

  return allGraphGroups;
}

/**
 * Compute the list of graph names sorted by size
 */
function getGraphNames(dataType: DataType, graphGroupName: string): string[] {
  const graphSizes = new Map<string, number>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as GraphData;
    const graphGroup = reportData?.graph_groups.find((graphGroup) => graphGroup.group_name === graphGroupName);
    if (graphGroup == undefined) continue;
    for (const graphName in graphGroup.graphs) {
      if (!graphSizes.has(graphName)) {
        graphSizes.set(graphName, 0);
      }
      graphSizes.set(graphName, graphSizes.get(graphName) + (graphGroup.graphs[graphName]?.graph_size || 0));
    }
  }
  return Array.from(graphSizes.keys()).sort((a, b) => graphSizes.get(b) - graphSizes.get(a));
}

/**
 * This component renders the page for graph data type, where the graphs are rendered within Iframes
 */
export default function (props: DataPageProps) {
  const allGraphGroups = React.useMemo(() => getAllGraphGroups(props.dataType), [props.dataType]);

  const [graphGroupName, setGraphGroupName] = React.useState(allGraphGroups[0]?.id || "");

  const graphRowPercentage = Math.floor(100 / RUNS.length);
  const cardDefinition: CardsProps.CardDefinition<string> = {
    header: (graphName: string) => <Header variant={"h2"}>{graphName}</Header>,
    sections: RUNS.map((runName) => ({
      id: runName,
      header: <RunHeader runName={runName} />,
      content: (graphName) => (
        <div style={{ paddingTop: "10px", paddingRight: "30px" }}>
          <IframeGraph dataType={props.dataType} runName={runName} graphGroup={graphGroupName} graphName={graphName} />
        </div>
      ),
      width: graphRowPercentage,
    })),
  };

  const sortedGraphNames = getGraphNames(props.dataType, graphGroupName);
  const { items, filteredItemsCount, collectionProps, filterProps, paginationProps } = useCollection(sortedGraphNames, {
    filtering: {
      filteringFunction: (item: string, filteringText: string) =>
        item.toLowerCase().includes(filteringText.toLowerCase()),
      empty: <Box variant={"p"}>No graphs were collected</Box>,
      noMatch: <Box variant={"p"}>No graphs found</Box>,
    },
    pagination: { pageSize: Math.floor(NUM_GRAPHS_PER_PAGE / RUNS.length) },
  });

  return (
    <Cards
      {...collectionProps}
      cardsPerRow={[{ cards: 1 }]}
      pagination={<Pagination {...paginationProps} />}
      stickyHeader={true}
      header={
        <Header
          variant={"awsui-h1-sticky"}
          info={<ReportHelpPanelLink type="summary" />}
          actions={
            allGraphGroups.length > 1 && (
              <SegmentedControl
                selectedId={graphGroupName}
                onChange={({ detail }) => setGraphGroupName(detail.selectedId)}
                options={allGraphGroups}
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
          filteringPlaceholder={"Find graphs"}
          countText={`${filteredItemsCount} graphs found`}
        />
      }
      variant={"full-page"}
      items={items}
      cardDefinition={cardDefinition}
    />
  );
}
