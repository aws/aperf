import React from "react";
import { useReportState } from "../ReportStateProvider";
import { DataTypeStatisticalFindings } from "./StatisticalFindings";
import { SegmentedControl, SplitPanel, ToggleButton } from "@cloudscape-design/components";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { SplitPanelType } from "../../definitions/types";
import { FindingsIcon } from "../misc/Icons";
import { DataTypeAnalyticalFindings } from "./AnalyticalFindings";
import { TIME_SERIES_DATA_TYPES } from "../../definitions/data-config";

/**
 * This component defines the split panel of the app layout to show analytical or statistical
 * findings of a data type.
 */
export default function () {
  const { dataComponent } = useReportState();

  const [splitPanelType, setSplitPanelType] = React.useState<SplitPanelType>("analytical");

  const isTimeSeriesData = TIME_SERIES_DATA_TYPES.includes(dataComponent);

  return (
    <SplitPanel
      header={`${DATA_DESCRIPTIONS[dataComponent].readableName} ${!isTimeSeriesData || splitPanelType == "analytical" ? "Analytical Findings" : "Statistical Findings"}`}
      headerActions={
        isTimeSeriesData && (
          <SegmentedControl
            selectedId={splitPanelType}
            onChange={({ detail }) => setSplitPanelType(detail.selectedId as SplitPanelType)}
            options={[
              { text: "Analytical Findings", id: "analytical" },
              { text: "Statistical Findings", id: "statistical" },
            ]}
          />
        )
      }
      closeBehavior={"hide"}
    >
      {/*Non time-series data only show the analytical findings, while time-series data control which findings to show through the state*/}
      {(!isTimeSeriesData || splitPanelType == "analytical") && <DataTypeAnalyticalFindings dataType={dataComponent} />}
      {isTimeSeriesData && splitPanelType == "statistical" && <DataTypeStatisticalFindings dataType={dataComponent} />}
    </SplitPanel>
  );
}

/**
 * This component renders a toggle button that opens or closes the finding split panel.
 */
export function ShowFindingsPanelButton() {
  const { showSplitPanel, setShowSplitPanel } = useReportState();

  return (
    <ToggleButton
      onChange={({ detail }) => setShowSplitPanel(detail.pressed)}
      pressed={showSplitPanel}
      iconSvg={<FindingsIcon />}
    >
      {"Findings"}
    </ToggleButton>
  );
}
