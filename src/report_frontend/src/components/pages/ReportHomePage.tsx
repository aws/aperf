import React from "react";
import { Container, ContentLayout, Grid, Toggle } from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RUNS } from "../../definitions/data-config";
import { RunSystemInfo } from "../data/RunSystemInfo";
import { useReportState } from "../ReportStateProvider";
import { GlobalAnalyticalFindings } from "../analytics/AnalyticalFindings";
import { GlobalStatisticalFindings } from "../analytics/StatisticalFindings";
import { DarkModeIcon } from "../misc/Icons";
import {ReportHelpPanelLink} from "../misc/ReportHelpPanel";

/**
 * This component renders the APerf report's home page, where users can view the system info
 * of each APerf run as well all analytical findings
 */
export default function () {
  const { darkMode, setDarkMode } = useReportState();

  // From the definition of the Grid component: "One row equals 12 columns. After the columns
  // reach or exceed the threshold of 12, the next element wraps to create a new row".
  // The below setup is to try to create four rows - run titles, system info, analytical
  // findings, and statistical findings
  const gridColSpan = RUNS.length <= 12 ? Math.floor(12 / RUNS.length) : 1;
  const gridDefinition = new Array(RUNS.length * 4).fill({ colspan: gridColSpan });

  const perRunHeader = RUNS.map((runName) => (
    <Header
      variant={"h1"}
      description={
        RUNS.length <= 1
          ? undefined
          : runName == RUNS[0]
            ? "This is the base run. All other runs included in the report are analyzed and compared with this run."
            : `This is a comparison run. The data within are analyzed and compared with base run ${RUNS[0]}.`
      }
    >
      {runName}
    </Header>
  ));
  const perRunSystemInfo = RUNS.map((runName) => (
    <Container fitHeight header={<Header variant={"h3"}>System Info</Header>}>
      <RunSystemInfo runName={runName} />
    </Container>
  ));
  const perRunAnalyticalFindings = RUNS.map((runName) => <GlobalAnalyticalFindings runName={runName} />);
  const perRunStatisticalFindings = RUNS.map((runName) => <GlobalStatisticalFindings runName={runName} />);

  return (
    <ContentLayout
      header={
        <Header
          variant={"h1"}
          info={<ReportHelpPanelLink dataType={"systeminfo"} fieldKey={"summary"} />}
          actions={
            <Toggle checked={darkMode} onChange={({ detail }) => setDarkMode(detail.checked)}>
              <DarkModeIcon />
            </Toggle>
          }
        >
          APerf Report Home
        </Header>
      }
    >
      <Grid gridDefinition={gridDefinition}>
        {perRunHeader.concat(perRunSystemInfo, perRunAnalyticalFindings, perRunStatisticalFindings)}
      </Grid>
    </ContentLayout>
  );
}
