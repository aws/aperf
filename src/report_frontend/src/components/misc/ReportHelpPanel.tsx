import { Button, HelpPanel, Link, List, StatusIndicator } from "@cloudscape-design/components";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import React from "react";
import { useReportState } from "../ReportStateProvider";
import { DataType } from "../../definitions/types";
import ReactMarkdown from "react-markdown";

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
          <HelpfulLinks dataType={helpPanelDataType} fieldKey={helpPanelFieldKey} />
          <h4>Need help with the metrics?</h4>
          <Link external href={"https://github.com/aws/aperf/issues"}>
            Raise a GitHub issue
          </Link>
        </div>
      }
    >
      <p>{metricDescription}</p>
      {desiredValueMessage && <StatusIndicator type={"warning"}>{desiredValueMessage}</StatusIndicator>}
      <OptimizationGuides dataType={helpPanelDataType} fieldKey={helpPanelFieldKey} />
    </HelpPanel>
  );
}

interface ReportHelpPanelProps {
  readonly dataType: DataType;
  readonly fieldKey: string;
}

/**
 * A helper component to render all optimization guides in markdown
 */
function OptimizationGuides(props: ReportHelpPanelProps) {
  const optimizationGuide = DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.fieldKey]?.optimization;

  const NewTabLinkRenderer = (props) => (
    <a href={props.href} target="_blank" rel="noreferrer">
      {props.children}
    </a>
  );

  if (optimizationGuide && optimizationGuide.length > 0) {
    return <ReactMarkdown components={{ a: NewTabLinkRenderer }}>{optimizationGuide.join("\n\n")}</ReactMarkdown>;
  }

  return null;
}

/**
 * A helper component to render all helpful links as the footer in a help panel
 */
function HelpfulLinks(props: ReportHelpPanelProps) {
  const defaultHelpfulLinks = DATA_DESCRIPTIONS[props.dataType].defaultHelpfulLinks ?? [];
  const dataHelpfulLinks = DATA_DESCRIPTIONS[props.dataType].fieldDescriptions[props.fieldKey]?.helpfulLinks ?? [];
  const helpfulLinks = [...dataHelpfulLinks, ...defaultHelpfulLinks];

  if (helpfulLinks.length == 0) return null;

  return (
    <>
      <h4>Helpful Links</h4>
      <List
        items={helpfulLinks.map((link, index) => ({
          id: index.toString(),
          url: link,
        }))}
        renderItem={(item) => ({
          id: item.id,
          content: (
            <Link external target={"_blank"} href={item.url}>
              {item.url}
            </Link>
          ),
        })}
      />
    </>
  );
}

/**
 * This component renders an "info" link that configures and shows the help panel.
 */
export function ReportHelpPanelLink(props: ReportHelpPanelProps) {
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
export function ReportHelpPanelIcon(props: ReportHelpPanelProps) {
  const { setHelpPanelDataType, setHelpPanelFieldKey, setShowHelpPanel } = useReportState();
  return (
    <div title={"View optimization guides and more information."} style={{ display: "inline-block" }}>
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
    </div>
  );
}
