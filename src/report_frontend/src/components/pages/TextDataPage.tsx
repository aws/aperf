import { DataPageProps, TextData } from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import { Box, Cards, Container } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import React from "react";
import { RunHeader } from "../data/RunSystemInfo";

/**
 * This component renders the page for text data
 */
export default function (props: DataPageProps) {
  const [textAreaHeight, setTextAreaHeight] = React.useState(0);

  // Below code only runs once to enable resizing of the text area
  React.useEffect(() => {
    const updateTextAreaHeight = () => {
      setTextAreaHeight(window.innerHeight * 0.75);
    };

    updateTextAreaHeight();

    window.addEventListener("resize", updateTextAreaHeight);

    return () => window.removeEventListener("resize", updateTextAreaHeight);
  }, []);

  const textWidthPercentage = Math.floor(100 / RUNS.length);

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
          content: () => {
            const reportData = PROCESSED_DATA[props.dataType].runs[runName] as TextData;

            return (
              <div style={{ paddingTop: "10px", paddingRight: "30px" }}>
                <Container>
                  {!reportData && (
                    <Box textAlign="center" color="inherit">
                      <b>No data collected</b>
                      <Box variant="p" color="inherit">
                        This data was not collected in the Aperf run
                      </Box>
                    </Box>
                  )}
                  {reportData && (
                    <div
                      style={{
                        width: "100%",
                        height: textAreaHeight,

                        // Enable scrolling
                        overflowY: "auto",
                        overflowX: "auto",

                        // Text formatting
                        whiteSpace: "pre",
                        fontFamily: "monospace",
                        fontSize: "14px",
                        lineHeight: 1.4,

                        // Better scrollbar experience
                        scrollbarWidth: "thin",
                        WebkitOverflowScrolling: "touch",
                      }}
                    >
                      {reportData.lines.join("\n")}
                    </div>
                  )}
                </Container>
              </div>
            );
          },
          width: textWidthPercentage,
        })),
      }}
    />
  );
}
