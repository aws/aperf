import { DataPageProps, TextData } from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { Cards, Textarea } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import React from "react";
import { RunHeader } from "../data/RunSystemInfo";

/**
 * This component renders the page for text data
 */
export default function (props: DataPageProps) {
  const textWidthPercentage = Math.floor(100 / RUNS.length);
  const textContentsPerRun = new Map<string, string>();
  let maxRows = 1;
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[props.dataType].runs[runName] as TextData;
    if (reportData) {
      textContentsPerRun.set(runName, reportData.lines.join("\n"));
      maxRows = Math.max(maxRows, reportData.lines.length);
    } else {
      textContentsPerRun.set(runName, "The data was not collected in the Aperf run.");
    }
  }

  return (
    <Cards
      cardsPerRow={[{ cards: 1 }]}
      stickyHeader
      header={
        <Header variant={"awsui-h1-sticky"} description={DATA_DESCRIPTIONS[props.dataType].summary}>
          {DATA_DESCRIPTIONS[props.dataType].readableName}
        </Header>
      }
      variant={"full-page"}
      items={[{ name: "content" }]}
      cardDefinition={{
        sections: RUNS.map((runName) => ({
          id: runName,
          header: <RunHeader runName={runName} />,
          content: () => (
            // For some reason the Textarea is still scrollable if just setting rows to maxRows,
            // so increase the number by 5 percent
            <div style={{ paddingTop: "10px", paddingRight: "30px" }}>
              <Textarea readOnly rows={maxRows * 1.05} value={textContentsPerRun.get(runName)} />
            </div>
          ),
          width: textWidthPercentage,
        })),
      }}
    />
  );
}
