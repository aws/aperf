import React from "react";
import { AppLayout, Box, Spinner } from "@cloudscape-design/components";
import DataNavigation from "./misc/DataNavigation";
import { useReportState } from "./ReportStateProvider";
import { PROCESSED_DATA, RUNS } from "../definitions/data-config";
import { extractDataTypeFromFragment, getRunNumCpus } from "../utils/utils";
import { ReportHelpPanel } from "./misc/ReportHelpPanel";
import { NumCpusPerRun, SelectedCpusPerRun } from "../definitions/types";
import TimeSeriesDataPage from "./pages/TimeSeriesDataPage";
import KeyValuePairsDataPage from "./pages/KeyValuePairsDataPage";
import GraphDataPage from "./pages/GraphDataPage";
import TextDataPage from "./pages/TextDataPage";
import ReportHomePage from "./pages/ReportHomePage";
import { MAX_NUM_CPU_SHOW_DEFAULT } from "../definitions/constants";

/**
 * This component creates the APerf report top-level layout and controls which specific data tab to render
 */
export default function () {
  const { dataComponent, setDataComponent, setNumCpusPerRun, setSelectedCpusPerRun } = useReportState();
  const [preprocessing, setPreprocessing] = React.useState(true);

  React.useEffect(() => {
    // Allow the usage of URL fragment to control which data type to render
    const dataType = extractDataTypeFromFragment(window.location.hash);
    if (dataType) setDataComponent(dataType);
    else setDataComponent("systeminfo");

    // Read the number of CPUs from processed data to be used by the configuration
    const numCpusPerRun: NumCpusPerRun = {};
    const selectedCpusPerRun: SelectedCpusPerRun = {};
    RUNS.forEach((runName) => {
      const numCpusCurRun = getRunNumCpus(runName);
      numCpusPerRun[runName] = numCpusCurRun;
      // When the number of CPUs is too large, the metric graph only show aggregate by default;
      // otherwise show all CPUs
      selectedCpusPerRun[runName] = {
        aggregate: true,
        cpus: Array(numCpusCurRun).fill(numCpusCurRun <= MAX_NUM_CPU_SHOW_DEFAULT),
      };
    });
    setNumCpusPerRun(numCpusPerRun);
    setSelectedCpusPerRun(selectedCpusPerRun);
    setPreprocessing(false);
  }, []);

  const dataFormat = dataComponent == "systeminfo" ? "home" : PROCESSED_DATA[dataComponent].data_format;

  return (
    <AppLayout
      contentType={"cards"}
      toolsOpen={true}
      tools={<ReportHelpPanel />}
      navigationOpen={true}
      navigation={<DataNavigation />}
      // This is to make the page horizontally scrollable when there are multiple runs. The contents
      // should be at least as wide as the browser windows minus the width of the navigation and help
      // panel (about 650). Each metric graph should at least be 750px wide.
      minContentWidth={Math.max(window.screen.width - 650, 750 * RUNS.length)}
      content={
        <>
          {preprocessing && <Spinner size={"large"} />}
          {!preprocessing && dataFormat == "home" && <ReportHomePage />}
          {!preprocessing && dataFormat == "time_series" && (
            <TimeSeriesDataPage dataType={dataComponent} key={dataComponent} />
          )}
          {!preprocessing && dataFormat == "key_value" && (
            <KeyValuePairsDataPage dataType={dataComponent} key={dataComponent} />
          )}
          {!preprocessing && dataFormat == "graph" && <GraphDataPage dataType={dataComponent} key={dataComponent} />}
          {!preprocessing && dataFormat == "text" && <TextDataPage dataType={dataComponent} key={dataComponent} />}
          {!preprocessing && (dataFormat as string) === "" && (
            <Box textAlign="center" color="inherit">
              <b>Unavailable Data</b>
              <Box variant="p" color="inherit">
                This data was not collected in any APerf runs.
              </Box>
            </Box>
          )}
        </>
      }
    />
  );
}
