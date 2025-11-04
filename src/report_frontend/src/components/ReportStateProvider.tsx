import React, { ReactNode } from "react";
import { DataType, NumCpusPerRun, SelectedCpusPerRun } from "../definitions/types";
import { NUM_METRICS_PER_PAGE } from "../definitions/constants";

interface ReportState {
  dataComponent: DataType;
  setDataComponent: (newDataComponent: DataType) => void;
  numMetricGraphsPerPage: number;
  setNumMetricGraphsPerPage: (newMetricsPerPage: number) => void;
  showHelpPanel: boolean;
  setShowHelpPanel: (newShowHelpPanel: boolean) => void;
  helpPanelType: string;
  setHelpPanelType: (newHelpPanelFieldKey: string) => void;
  combineGraphs: boolean;
  setCombineGraphs: (newCombineGraphs: boolean) => void;
  numCpusPerRun: NumCpusPerRun;
  setNumCpusPerRun: (newNumCpusPerRun: NumCpusPerRun) => void;
  selectedCpusPerRun: SelectedCpusPerRun;
  setSelectedCpusPerRun: (newSelectedCpusPerRun: SelectedCpusPerRun) => void;
}

const ReportStateContext = React.createContext<ReportState | undefined>(undefined);

/**
 * Provides global states to be used by child components
 */
export default function (props: { children: ReactNode }) {
  const [dataComponent, setDataComponent] = React.useState<DataType>("systeminfo");
  const [numMetricGraphsPerPage, setNumMetricGraphsPerPage] = React.useState(NUM_METRICS_PER_PAGE);
  const [showHelpPanel, setShowHelpPanel] = React.useState(true);
  const [helpPanelType, setHelpPanelType] = React.useState<string>("general");
  const [combineGraphs, setCombineGraphs] = React.useState(false);
  const [numCpusPerRun, setNumCpusPerRun] = React.useState<NumCpusPerRun>({});
  const [selectedCpusPerRun, setSelectedCpusPerRun] = React.useState<SelectedCpusPerRun>({});

  const reportState: ReportState = {
    dataComponent,
    setDataComponent,
    numMetricGraphsPerPage,
    setNumMetricGraphsPerPage,
    showHelpPanel,
    setShowHelpPanel,
    helpPanelType,
    setHelpPanelType,
    combineGraphs,
    setCombineGraphs,
    numCpusPerRun,
    setNumCpusPerRun,
    selectedCpusPerRun,
    setSelectedCpusPerRun,
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
