import React from "react";
import { AppLayout, Box, Spinner } from "@cloudscape-design/components";
import { applyMode, Mode } from "@cloudscape-design/global-styles";
import DataNavigation from "./misc/DataNavigation";
import { useReportState } from "./ReportStateProvider";
import { PROCESSED_DATA, RUNS } from "../definitions/data-config";
import { extractDataTypeFromFragment, getRunNumCpus } from "../utils/utils";
import { ReportHelpPanel } from "./misc/ReportHelpPanel";
import { NumCpusPerRun, SelectedCpusPerRun } from "../definitions/types";
import TimeSeriesDataPage from "./pages/TimeSeriesDataPage";
import KeyValueDataPage from "./pages/KeyValueDataPage";
import GraphDataPage from "./pages/GraphDataPage";
import TextDataPage from "./pages/TextDataPage";
import ReportHomePage from "./pages/ReportHomePage";
import { MAX_NUM_CPU_SHOW_DEFAULT } from "../definitions/constants";

/**
 * This component creates the APerf report top-level layout and controls which specific data tab to render
 */
export default function () {
  const {
    dataComponent,
    showHelpPanel,
    setShowHelpPanel,
    setDataComponent,
    setNumCpusPerRun,
    setSelectedCpusPerRun,
    darkMode,
  } = useReportState();
  const [preprocessing, setPreprocessing] = React.useState(true);
  const [showNavigation, setShowNavigation] = React.useState(true);

  // All the pre-processing logics that set up the report states/variables are defined below. The code
  // will only run once when the report is loaded.
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

  // Apply dark mode using Cloudscape global styles
  React.useEffect(() => {
    applyMode(darkMode ? Mode.Dark : Mode.Light);
    // Set index background color to match the theme
    const backgroundColor = darkMode ? '#171D25' : '#ffffff';
    document.body.style.backgroundColor = backgroundColor;
    document.documentElement.style.backgroundColor = backgroundColor;
  }, [darkMode]);

  const dataFormat = dataComponent == "systeminfo" ? "home" : PROCESSED_DATA[dataComponent].data_format;

  return (
    <AppLayout
      contentType={"cards"}
      toolsOpen={showHelpPanel}
      tools={<ReportHelpPanel />}
      onToolsChange={({ detail }) => {
        setShowHelpPanel(detail.open);
      }}
      navigationOpen={showNavigation}
      navigation={<DataNavigation />}
      onNavigationChange={({ detail }) => {
        setShowNavigation(detail.open);
        // Same reasoning as the help panel (see the definition of setShowHelpPanel)
        setTimeout(() => window.dispatchEvent(new Event("resize")), 50);
      }}
      content={
        <>
          {preprocessing && <Spinner size={"large"} />}
          {!preprocessing && dataFormat == "home" && <ReportHomePage />}
          {!preprocessing && dataFormat == "time_series" && (
            <TimeSeriesDataPage dataType={dataComponent} key={dataComponent} />
          )}
          {!preprocessing && dataFormat == "key_value" && (
            <KeyValueDataPage dataType={dataComponent} key={dataComponent} />
          )}
          {!preprocessing && dataFormat == "graph" && <GraphDataPage dataType={dataComponent} key={dataComponent} />}
          {!preprocessing && dataFormat == "text" && <TextDataPage dataType={dataComponent} key={dataComponent} />}
          {!preprocessing && (dataFormat == "unknown" || (dataFormat as string) === "" || dataFormat === undefined) && (
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
