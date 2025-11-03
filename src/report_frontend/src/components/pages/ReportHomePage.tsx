import React from "react";
import { Container, ContentLayout, Grid, Toggle, Icon } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RUNS } from "../../definitions/data-config";
import { RunSystemInfo } from "../data/RunSystemInfo";
import { useReportState } from "../ReportStateProvider";
import { RunFindings } from "../data/Finding";

/**
 * This component renders the APerf report's home page, where users can view the system info
 * of each APerf run as well all analytical findings
 */
export default function () {
  const { darkMode, setDarkMode } = useReportState();

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
  const perRunAnalyticalFindings = RUNS.map((runName) => (
    <Container header={<Header variant={"h3"}>Analytical Findings</Header>}>
      <RunFindings runName={runName} />
    </Container>
  ));

  return (
    <ContentLayout
      header={
        <Header
          variant={"h1"}
          description={DATA_DESCRIPTIONS["systeminfo"].summary}
          actions={
            <Toggle checked={darkMode} onChange={({ detail }) => setDarkMode(detail.checked)}>
              <Icon
                name="settings"
                svg={
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                    <path
                      d="M12.8166 9.79921C12.8417 9.75608 12.7942 9.70771 12.7497 9.73041C11.9008 10.164 10.9392 10.4085 9.92054 10.4085C6.48046 10.4085 3.69172 7.61979 3.69172 4.17971C3.69172 3.16099 3.93628 2.19938 4.36989 1.3504C4.39259 1.30596 4.34423 1.25842 4.3011 1.28351C2.44675 2.36242 1.2002 4.37123 1.2002 6.67119C1.2002 10.1113 3.98893 12.9 7.42901 12.9C9.72893 12.9 11.7377 11.6535 12.8166 9.79921Z"
                      fill="white"
                      stroke="white"
                      strokeWidth="2"
                      className="filled"
                    />
                  </svg>
                }
              />
            </Toggle>
          }
        >
          APerf Report Home
        </Header>
      }
    >
      <Grid gridDefinition={gridDefinition}>{perRunSystemInfo.concat(perRunAnalyticalFindings)}</Grid>
    </ContentLayout>
  );
}
