import React, { ReactNode } from "react";
import {
  ALL_DATA_TYPES,
  ALL_FINDING_TYPES,
  DataType,
  FindingType,
  NumCpusPerRun,
  SelectedCpusPerRun,
  Stat,
} from "../definitions/types";
import { NUM_METRICS_PER_PAGE } from "../definitions/constants";
import { RUNS, TIME_SERIES_DATA_TYPES } from "../definitions/data-config";

interface ReportState {
  dataComponent: DataType;
  setDataComponent: (newDataComponent: DataType) => void;
  numMetricGraphsPerPage: number;
  setNumMetricGraphsPerPage: (newMetricsPerPage: number) => void;
  showHelpPanel: boolean;
  setShowHelpPanel: (newShowHelpPanel: boolean) => void;
  helpPanelDataType: DataType;
  setHelpPanelDataType: (newHelpPanelDataType: DataType) => void;
  helpPanelFieldKey: string;
  setHelpPanelFieldKey: (newHelpPanelFieldKey: string) => void;
  showSplitPanel: boolean;
  setShowSplitPanel: (newShowSplitPanel: boolean) => void;
  splitPanelSize: number;
  setSplitPanelSize: (newSplitPanelSize: number) => void;
  updateFilteringText: (text: string) => void;
  setUpdateFilteringText: (newUpdateFilteringText: (text: string) => void) => void;
  combineGraphs: boolean;
  setCombineGraphs: (newCombineGraphs: boolean) => void;
  numCpusPerRun: NumCpusPerRun;
  setNumCpusPerRun: (newNumCpusPerRun: NumCpusPerRun) => void;
  selectedCpusPerRun: SelectedCpusPerRun;
  setSelectedCpusPerRun: (newSelectedCpusPerRun: SelectedCpusPerRun) => void;
  darkMode: boolean;
  setDarkMode: (newDarkMode: boolean) => void;
  searchKey: string;
  setSearchKey: (newSearchKey: string) => void;
  analyticalFindingsDataTypes: { [runName: string]: DataType[] };
  updateAnalyticalFindingsDataTypes: (runName: string, newDataTypes: DataType[]) => void;
  analyticalFindingsTypes: { [runName: string]: FindingType[] };
  updateAnalyticalFindingsTypes: (runName: string, newFindingTypes: FindingType[]) => void;
  statisticalFindingsDataTypes: { [runName: string]: DataType[] };
  updateStatisticalFindingsDataTypes: (runName: string, newDataTypes: DataType[]) => void;
  statisticalFindingsStats: { [runName: string]: Stat[] };
  updateStatisticalFindingsStats: (runName: string, newStats: Stat[]) => void;
  statisticalFindingsTypes: { [runName: string]: FindingType[] };
  updateStatisticalFindingsTypes: (runName: string, newFindingTypes: FindingType[]) => void;
}

const ReportStateContext = React.createContext<ReportState | undefined>(undefined);

/**
 * Provides global states to be used by child components
 */
export default function (props: { children: ReactNode }) {
  const [dataComponent, setDataComponent] = React.useState<DataType>("systeminfo");
  const [numMetricGraphsPerPage, setNumMetricGraphsPerPage] = React.useState(() => {
    const stored = localStorage.getItem("numMetricGraphsPerPage");
    return stored ? parseInt(stored, 10) : NUM_METRICS_PER_PAGE;
  });
  const [showHelpPanel, setShowHelpPanel] = React.useState(false);
  const [helpPanelDataType, setHelpPanelDataType] = React.useState<DataType>("systeminfo");
  const [helpPanelFieldKey, setHelpPanelFieldKey] = React.useState<string>("general");
  const [showSplitPanel, setShowSplitPanel] = React.useState(false);
  const [splitPanelSize, setSplitPanelSize] = React.useState<number>(500);
  // To store a function in React state, we need to set the value with a function that returns the function,
  // to be distinguished from the function argument supported by the React useState and setState function
  const [updateFilteringText, setUpdateFilteringText] = React.useState<(text: string) => void>(() => () => {});
  const [combineGraphs, setCombineGraphs] = React.useState(false);
  const [numCpusPerRun, setNumCpusPerRun] = React.useState<NumCpusPerRun>({});
  const [selectedCpusPerRun, setSelectedCpusPerRun] = React.useState<SelectedCpusPerRun>({});
  const [darkMode, setDarkMode] = React.useState(() => {
    const saved = localStorage.getItem("aperf-dark-mode");
    return saved ? JSON.parse(saved) : false;
  });
  const [searchKey, setSearchKey] = React.useState("");

  const [analyticalFindingsDataTypes, setAnalyticalFindingsDataTypes] = React.useState<{
    [runName: string]: DataType[];
  }>(Object.fromEntries(RUNS.map((runName) => [runName, [...ALL_DATA_TYPES]])));

  const [analyticalFindingsTypes, setAnalyticalFindingsTypes] = React.useState<{
    [runName: string]: FindingType[];
  }>(Object.fromEntries(RUNS.map((runName) => [runName, [...ALL_FINDING_TYPES]])));

  const [statisticalFindingsDataTypes, setStatisticalFindingsDataTypes] = React.useState<{
    [runName: string]: DataType[];
  }>(Object.fromEntries(RUNS.map((runName) => [runName, TIME_SERIES_DATA_TYPES])));

  const [statisticalFindingsStats, setStatisticalFindingsStats] = React.useState<{
    [runName: string]: Stat[];
  }>(Object.fromEntries(RUNS.map((runName) => [runName, ["avg"]])));

  const [statisticalFindingsTypes, setStatisticalFindingsTypes] = React.useState<{
    [runName: string]: FindingType[];
  }>(Object.fromEntries(RUNS.map((runName) => [runName, ["negative"]])));

  const reportState: ReportState = {
    dataComponent,
    setDataComponent,
    numMetricGraphsPerPage,
    setNumMetricGraphsPerPage: (newNumMetricGraphsPerPage: number) => {
      setNumMetricGraphsPerPage(newNumMetricGraphsPerPage);
      localStorage.setItem("numMetricGraphsPerPage", newNumMetricGraphsPerPage.toString());
    },
    showHelpPanel,
    setShowHelpPanel: (newShowHelpPanel) => {
      const curShowHelpPanel = showHelpPanel;
      setShowHelpPanel(newShowHelpPanel);
      if (curShowHelpPanel != newShowHelpPanel) {
        // Trigger a window resize event when the side panel is opened or closed, so that the Plotly graphs
        // can resize accordingly. Add a 50ms delay here to let the changed components render first, so that
        // the actual component size can be correctly read
        setTimeout(() => window.dispatchEvent(new Event("resize")), 50);
      }
    },
    helpPanelDataType,
    setHelpPanelDataType,
    helpPanelFieldKey,
    setHelpPanelFieldKey,
    showSplitPanel,
    setShowSplitPanel,
    splitPanelSize,
    setSplitPanelSize,
    updateFilteringText,
    setUpdateFilteringText,
    combineGraphs,
    setCombineGraphs,
    numCpusPerRun,
    setNumCpusPerRun,
    selectedCpusPerRun,
    setSelectedCpusPerRun,
    darkMode,
    setDarkMode: (newDarkMode: boolean) => {
      setDarkMode(newDarkMode);
      localStorage.setItem("aperf-dark-mode", JSON.stringify(newDarkMode));
    },
    searchKey,
    setSearchKey,
    analyticalFindingsDataTypes,
    updateAnalyticalFindingsDataTypes: (runName: string, newDataTypes: DataType[]) => {
      setAnalyticalFindingsDataTypes((prev) => ({ ...prev, [runName]: newDataTypes }));
    },
    analyticalFindingsTypes,
    updateAnalyticalFindingsTypes: (runName: string, newFindingsFilter: FindingType[]) => {
      setAnalyticalFindingsTypes((prev) => ({ ...prev, [runName]: newFindingsFilter }));
    },
    statisticalFindingsDataTypes,
    updateStatisticalFindingsDataTypes: (runName: string, newDataTypes: DataType[]) => {
      setStatisticalFindingsDataTypes((prev) => ({ ...prev, [runName]: newDataTypes }));
    },
    statisticalFindingsStats,
    updateStatisticalFindingsStats: (runName: string, newStats: Stat[]) => {
      setStatisticalFindingsStats((prev) => ({ ...prev, [runName]: newStats }));
    },
    statisticalFindingsTypes,
    updateStatisticalFindingsTypes: (runName: string, newFindingTypes: FindingType[]) => {
      setStatisticalFindingsTypes((prev) => ({ ...prev, [runName]: newFindingTypes }));
    },
  };

  return <ReportStateContext.Provider value={reportState}>{props.children}</ReportStateContext.Provider>;
}

export function useReportState() {
  const context = React.useContext(ReportStateContext);
  if (context == undefined) {
    throw new Error("useReportState must be used within ReportStateProvider");
  }
  return context;
}
