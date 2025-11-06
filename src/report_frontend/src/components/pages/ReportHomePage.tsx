import React from "react";
import { Container, ContentLayout, Grid } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RUNS } from "../../definitions/data-config";
import { RunSystemInfo } from "../data/RunSystemInfo";

/**
 * This component renders the APerf report's home page, where users can view the system info
 * of each APerf run as well all analytical findings
 */
export default function () {
  // From the definition of the Grid component: "One row equals 12 columns. After the columns
  // reach or exceed the threshold of 12, the next element wraps to create a new row".
  // The below setup is to try to create two rows - the first rows are the system info
  // of every run, and the second rows are the analytical findings of the run. There will
  // be more than two rows if the number of runs is larger than 12.
  const gridColSpan = RUNS.length <= 12 ? Math.floor(12 / RUNS.length) : 1;
  const gridDefinition = new Array(RUNS.length * 2).fill({ colspan: gridColSpan });

  const perRunSystemInfo = RUNS.map((runName) => (
    <Container fitHeight header={<Header variant={"h3"}>System Info</Header>}>
      <RunSystemInfo runName={runName} />
    </Container>
  ));
  const perRunAnalyticalFindings = RUNS.map(() => (
    <Container header={<Header variant={"h3"}>Analytical Findings</Header>} />
  ));

  return (
    <ContentLayout
      header={
        <Header variant={"h1"} description={DATA_DESCRIPTIONS["systeminfo"].summary}>
          APerf Report Home
        </Header>
      }
    >
      <Grid gridDefinition={gridDefinition}>{perRunSystemInfo.concat(perRunAnalyticalFindings)}</Grid>
    </ContentLayout>
  );
}
