import { Button, HelpPanel, Link, StatusIndicator } from "@cloudscape-design/components";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import React from "react";
import { useReportState } from "../ReportStateProvider";
import { DataType } from "../../definitions/types";

/**
 * This component renders the help panel on the right side to show metric descriptions and helpful messages.
 */
export function ReportHelpPanel() {
  const { helpPanelDataType, helpPanelFieldKey } = useReportState();

  const metricInfo = DATA_DESCRIPTIONS[helpPanelDataType].fieldDescriptions[helpPanelFieldKey];

  let metricReadableName: string;
  let metricDescription: string;
  if (helpPanelFieldKey == "general") {
    metricReadableName = "Data Information";
    metricDescription = "Learn more about a specific metric or configuration here.";
  } else if (helpPanelFieldKey == "summary") {
    metricReadableName = DATA_DESCRIPTIONS[helpPanelDataType].readableName;
    metricDescription = DATA_DESCRIPTIONS[helpPanelDataType].summary;
  } else {
    metricReadableName = metricInfo?.readableName || helpPanelFieldKey;
    metricDescription = metricInfo?.description || "No extra information available for this metric.";
  }

  let desiredValueMessage: string | undefined = undefined;
  switch (metricInfo?.desired) {
    case "higher":
      desiredValueMessage = "Higher values of this metric usually indicate better performance.";
      break;
    case "lower":
      desiredValueMessage = "Lower values of this metric usually indicate better performance.";
      break;
    case "moderate":
      desiredValueMessage = "Moderate values of this metric usually indicate better performance.";
      break;
    case "depends":
      desiredValueMessage = "The desired values of this metric depends on the type of workload.";
      break;
    case "fixed":
      desiredValueMessage = "The values of this metric should be fixed.";
      break;
  }

  return (
    <HelpPanel
      header={<h2>{metricReadableName}</h2>}
      footer={
        <div>
          <h4>Need help with the metrics?</h4>
          <Link external href={"https://github.com/aws/aperf/issues"}>
            Raise a GitHub issue
          </Link>
        </div>
      }
    >
      <p>{metricDescription}</p>
      {desiredValueMessage && <StatusIndicator type={"warning"}>{desiredValueMessage}</StatusIndicator>}
    </HelpPanel>
  );
}

interface ReportHelpPanelLinkProps {
  readonly dataType: DataType;
  readonly fieldKey: string;
}

/**
 * This component renders an "info" link that configures and shows the help panel.
 */
export function ReportHelpPanelLink(props: ReportHelpPanelLinkProps) {
  const { setHelpPanelDataType, setHelpPanelFieldKey, setShowHelpPanel } = useReportState();
  return (
    <Link
      variant={"info"}
      onFollow={() => {
        setHelpPanelDataType(props.dataType);
        setHelpPanelFieldKey(props.fieldKey);
        setShowHelpPanel(true);
      }}
    >
      info
    </Link>
  );
}

/**
 * This component renders an icon that configures and shows the help panel.
 */
export function ReportHelpPanelIcon(props: ReportHelpPanelLinkProps) {
  const { setHelpPanelDataType, setHelpPanelFieldKey, setShowHelpPanel } = useReportState();
  return (
    <Button
      iconName={"status-info"}
      variant={"icon"}
      onClick={() => {
        setHelpPanelDataType(props.dataType);
        setHelpPanelFieldKey(props.fieldKey);
        setShowHelpPanel(true);
      }}
    >
      info
    </Button>
  );
}
