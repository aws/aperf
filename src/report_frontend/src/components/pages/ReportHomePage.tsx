import React from "react";
import { ColumnLayout, Container, ContentLayout, SpaceBetween } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RUNS } from "../../definitions/data-config";
import { RunSystemInfo } from "../data/RunSystemInfo";

/**
 * This component renders the APerf report's home page, where users can view the system info
 * of each APerf run as well all analytical findings
 */
export default function () {
  return (
    <ContentLayout
      header={
        <Header variant={"h1"} description={DATA_DESCRIPTIONS["systeminfo"].summary}>
          APerf Report Home
        </Header>
      }
    >
      <ColumnLayout columns={RUNS.length}>
        {RUNS.map((runName) => (
          <SpaceBetween size={"m"}>
            <Header variant={"h2"}>{runName}</Header>
            <div style={{ height: "25vh" }}>
              <Container fitHeight header={<Header variant={"h3"}>System Info</Header>}>
                <RunSystemInfo runName={runName} />
              </Container>
            </div>
            <div style={{ height: "60vh" }}>
              <Container fitHeight header={<Header variant={"h3"}>Analytical Findings</Header>} />
            </div>
          </SpaceBetween>
        ))}
      </ColumnLayout>
    </ContentLayout>
  );
}
