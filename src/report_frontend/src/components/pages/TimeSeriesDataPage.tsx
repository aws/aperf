import React from "react";
import { Box, Cards, CardsProps, Pagination, SpaceBetween, TextFilter, Toggle } from "@cloudscape-design/components";
import { DataPageProps, DataType, TimeSeriesData } from "../../definitions/types";
import Header from "@cloudscape-design/components/header";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { getDataTypeNonZeroMetricNames } from "../../utils/utils";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { useCollection } from "@cloudscape-design/collection-hooks";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import DataConfiguration from "../misc/TimeSeriesDataConfiguration";
import { useReportState } from "../ReportStateProvider";
import MetricGraph from "../data/MetricGraph";
import { RunHeader } from "../data/RunSystemInfo";
import CombinedMetricGraph from "../data/CombinedMetricGraph";

const NON_ZERO_METRIC_NAMES_CACHE = new Map<DataType, string[]>();

/**
 * This component renders a page for time series data, where it shows aligned metric graphs across
 * all APerf runs
 */
export default function (props: DataPageProps) {
  const { numMetricGraphsPerPage, combineGraphs, searchKey, setCombineGraphs } = useReportState();

  const [hideAllZeroMetrics, setHideAllZeroMetrics] = React.useState(true);

  let sortedMetricNames = [];
  // Extract the sorted metric names from any run data. The data should already be post-processed in
  // Rust and the sorted_metric_names field should be consolidated and consistent across all runs.
  for (const runName of RUNS) {
    const time_series_data = PROCESSED_DATA[props.dataType].runs[runName] as TimeSeriesData;
    if (time_series_data) {
      sortedMetricNames = time_series_data.sorted_metric_names;
      break;
    }
  }

  // If the option is enabled, perform an extra step to filter out names of all-zero metrics
  // so that they will not be rendered
  if (hideAllZeroMetrics) {
    if (NON_ZERO_METRIC_NAMES_CACHE.has(props.dataType)) {
      sortedMetricNames = NON_ZERO_METRIC_NAMES_CACHE.get(props.dataType);
    } else {
      sortedMetricNames = getDataTypeNonZeroMetricNames(props.dataType, sortedMetricNames);
      NON_ZERO_METRIC_NAMES_CACHE.set(props.dataType, sortedMetricNames);
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
              <div style={{ paddingRight: "30px", overflowX: "hidden" }}>
                <CombinedMetricGraph dataType={props.dataType} metricName={metricKey} key={props.dataType} />
              </div>
            ),
          },
        ]
      : RUNS.map((runName) => ({
          id: runName,
          header: <RunHeader runName={runName} />,
          content: (metricKey) => (
            <div style={{ paddingRight: "30px", overflowX: "hidden" }}>
              <MetricGraph dataType={props.dataType} runName={runName} metricName={metricKey} key={props.dataType} />
            </div>
          ),
          width: graphWidthPercentage,
        })),
  };

  const { items, filteredItemsCount, collectionProps, filterProps, paginationProps } = useCollection(
    sortedMetricNames,
    {
      filtering: {
        filteringFunction: (item: string, filteringText: string) =>
          item.toLowerCase().includes(filteringText.toLowerCase()),
        empty: <Box variant={"p"}>No metrics were collected</Box>,
        noMatch: <Box variant={"p"}>No metrics found</Box>,
        defaultFilteringText: searchKey,
      },
      pagination: { pageSize: Math.floor(numMetricGraphsPerPage / RUNS.length) },
      selection: {},
    },
  );

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
          info={<ReportHelpPanelLink type="summary" />}
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
