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
import { ShowFindingsPanelButton } from "../analytics/FindingsSplitPanel";

const NON_ZERO_METRIC_NAMES_CACHE = new Map<DataType, string[]>();

/**
 * This component renders a page for time series data, where it shows aligned metric graphs across
 * all APerf runs
 */
export default function (props: DataPageProps) {
  const { numMetricGraphsPerPage, combineGraphs, searchKey, setCombineGraphs, setUpdateFilteringText } =
    useReportState();

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
      <Header variant={"h2"} info={<ReportHelpPanelLink dataType={props.dataType} fieldKey={metricKey} />}>
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

  React.useEffect(() => {
    // Store the function to update the filtering text in the global state, so that they can be accessed
    // by other components to change the filtering text and locate a particular metric.
    // To be distinguished from the function argument supported by the React set state API, we need to
    // pass in a function that returns the actual function.
    setUpdateFilteringText(() => (text: string) => filterProps.onChange({ detail: { filteringText: text } }));
  }, [props.dataType]);

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
          info={<ReportHelpPanelLink dataType={props.dataType} fieldKey={"summary"} />}
          actions={
            <SpaceBetween size={"xs"} direction={"horizontal"}>
              <ShowFindingsPanelButton />
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
