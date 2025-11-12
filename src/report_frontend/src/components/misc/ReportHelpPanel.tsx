import { HelpPanel, Link, StatusIndicator } from "@cloudscape-design/components";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import React from "react";
import { useReportState } from "../ReportStateProvider";

/**
 * This component renders the help panel on the right side to show metric descriptions and helpful messages.
 */
export function ReportHelpPanel() {
  const { dataComponent, helpPanelType } = useReportState();

  const metricInfo = DATA_DESCRIPTIONS[dataComponent].fieldDescriptions[helpPanelType];

  let metricReadableName: string;
  let metricDescription: string;
  if (helpPanelType == "general") {
    metricReadableName = "Data Information";
    metricDescription = "Learn more about a specific metric or configuration here.";
  } else if (helpPanelType == "summary") {
    metricReadableName = DATA_DESCRIPTIONS[dataComponent].readableName;
    metricDescription = DATA_DESCRIPTIONS[dataComponent].summary;
  } else {
    metricReadableName = metricInfo?.readableName || helpPanelType;
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
  readonly type: string;
}

/**
 * This component renders an "info" link to be shown by a metric name, which controls which help
 * panel to show.
 */
export function ReportHelpPanelLink(props: ReportHelpPanelLinkProps) {
  const { setHelpPanelType, setShowHelpPanel } = useReportState();
  return (
    <Link
      variant={"info"}
      onFollow={() => {
        setHelpPanelType(props.type);
        setShowHelpPanel(true);
      }}
    >
      info
    </Link>
  );
}
