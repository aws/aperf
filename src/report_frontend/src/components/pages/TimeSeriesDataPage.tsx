import React from "react";
import {
  Box,
  Cards,
  CardsProps,
  Pagination,
  SpaceBetween,
  TextFilter,
  Toggle,
} from "@cloudscape-design/components";
import { DataPageProps, DataType } from "../../definitions/types";
import Header from "@cloudscape-design/components/header";
import { RUNS } from "../../definitions/data-config";
import { getDataTypeSortedMetricNames, getDataTypeNonZeroMetricKeys } from "../../utils/utils";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { useCollection } from "@cloudscape-design/collection-hooks";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import DataConfiguration from "../misc/TimeSeriesDataConfiguration";
import { useReportState } from "../ReportStateProvider";
import MetricGraph from "../data/MetricGraph";
import { RunHeader } from "../data/RunSystemInfo";
import CombinedMetricGraph from "../data/CombinedMetricGraph";

const SORTED_METRIC_NAMES_CACHE = new Map<DataType, string[]>();
const NON_ZERO_METRIC_NAMES_CACHE = new Map<DataType, string[]>();

/**
 * This component renders a page for time series data, where it shows aligned metric graphs across
 * all APerf runs
 */
export default function (props: DataPageProps) {
  const { numMetricGraphsPerPage, combineGraphs, setCombineGraphs } = useReportState();

  const [hideAllZeroMetrics, setHideAllZeroMetrics] = React.useState(true);
  // Compute and cache the sorted metric names across all runs, and the result
  // decide the order of which metric graphs are present
  let sortedMetricKeys: string[];
  if (SORTED_METRIC_NAMES_CACHE.has(props.dataType)) {
    sortedMetricKeys = SORTED_METRIC_NAMES_CACHE.get(props.dataType);
  } else {
    sortedMetricKeys = getDataTypeSortedMetricNames(props.dataType);
    SORTED_METRIC_NAMES_CACHE.set(props.dataType, sortedMetricKeys);
  }

  // If the option is enabled, perform an extra step to filter out names of all-zero metrics
  // so that they will not be rendered
  if (hideAllZeroMetrics) {
    if (NON_ZERO_METRIC_NAMES_CACHE.has(props.dataType)) {
      sortedMetricKeys = NON_ZERO_METRIC_NAMES_CACHE.get(props.dataType);
    } else {
      sortedMetricKeys = getDataTypeNonZeroMetricKeys(props.dataType, sortedMetricKeys);
      NON_ZERO_METRIC_NAMES_CACHE.set(props.dataType, sortedMetricKeys);
    }
  }

  const graphWidthPercentage = Math.floor(100 / RUNS.length);
  const cardDefinition: CardsProps.CardDefinition<string> = {
    header: (metricKey: string) => (
      <Header variant={"h2"} info={<ReportHelpPanelLink type={metricKey} />}>
        {metricKey}
      </Header>
    ),
    sections: combineGraphs
      ? [
          {
            id: "combined_graphs",
            content: (metricKey) => (
              <div style={{ paddingTop: "10px", paddingRight: "30px" }}>
                <CombinedMetricGraph dataType={props.dataType} metricName={metricKey} key={props.dataType} />
              </div>
            ),
          },
        ]
      : RUNS.map((runName) => ({
          id: runName,
          header: <RunHeader runName={runName} />,
          content: (metricKey) => (
            <div style={{ paddingTop: "10px", paddingRight: "30px" }}>
              <MetricGraph dataType={props.dataType} runName={runName} metricName={metricKey} key={props.dataType} />
            </div>
          ),
          width: graphWidthPercentage,
        })),
  };

  const { items, filteredItemsCount, collectionProps, filterProps, paginationProps } = useCollection(sortedMetricKeys, {
    filtering: {
      filteringFunction: (item: string, filteringText: string) =>
        item.toLowerCase().startsWith(filteringText.toLowerCase()),
      empty: <Box variant={"p"}>No metrics were collected</Box>,
      noMatch: <Box variant={"p"}>No metrics found</Box>,
    },
    pagination: { pageSize: Math.floor(numMetricGraphsPerPage / RUNS.length) },
    selection: {},
  });

  return (
    <Cards
      {...collectionProps}
      cardsPerRow={[{ cards: 1 }]}
      preferences={<DataConfiguration />}
      pagination={<Pagination {...paginationProps} />}
      stickyHeader
      header={
        <Header
          variant={"awsui-h1-sticky"}
          description={DATA_DESCRIPTIONS[props.dataType].summary}
          actions={
            <SpaceBetween size={"xs"} direction={"horizontal"}>
              <Toggle checked={combineGraphs} onChange={({ detail }) => setCombineGraphs(detail.checked)}>
                {"Combine run graphs"}
              </Toggle>
              <Toggle checked={hideAllZeroMetrics} onChange={({ detail }) => setHideAllZeroMetrics(detail.checked)}>
                {"Hide all-zero metrics"}
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
          filteringPlaceholder={"Find metrics"}
          countText={`${filteredItemsCount} metrics found`}
        />
      }
      variant={"full-page"}
      items={items}
      cardDefinition={cardDefinition}
    />
  );
}
