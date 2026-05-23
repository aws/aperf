import React from "react";
import { DataPageProps, DataType, ProfilingData } from "../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../definitions/data-config";
import {
  Container,
  SegmentedControl,
  SegmentedControlProps,
  Select,
  SelectProps,
  SpaceBetween,
  Table,
} from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import IframeGraph from "../data/IframeGraph";
import ProfilePanel from "../data/profile-panel/ProfilePanel";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { RunHeader } from "../data/RunSystemInfo";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import { ShowFindingsPanelButton } from "../analytics/FindingsSplitPanel";
import { buildKeyValueTable } from "../data/KeyValueTable";
import { ProfileAnalyticalFindings } from "../analytics/AnalyticalFindings";
import { useReportState } from "../ReportStateProvider";

/**
 * Collect all profiler instance names across all runs for the Select dropdown
 */
function getAllInstanceNames(dataType: DataType): SelectProps.Option[] {
  const sizes = new Map<string, number>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as ProfilingData;
    if (reportData == undefined) continue;
    for (const [name, profiler] of Object.entries(reportData.profilers)) {
      if (!sizes.has(name)) sizes.set(name, 0);
      for (const profile of Object.values(profiler.profiles)) {
        sizes.set(name, sizes.get(name) + (profile.profile_graph?.graph_size || 0));
      }
    }
  }
  return Array.from(sizes.keys())
    .sort((a, b) => sizes.get(b) - sizes.get(a))
    .map((name) => ({ label: name, value: name }));
}

/**
 * Compute the list of profile names (for the SegmentedControl) sorted descending alphabetically
 */
function getProfileNames(dataType: DataType, instanceName: string): SegmentedControlProps.Option[] {
  const profileNames = new Set<string>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType].runs[runName] as ProfilingData;
    const instance = reportData?.profilers?.[instanceName];
    if (instance == undefined) continue;
    for (const profileName in instance.profiles) {
      profileNames.add(profileName);
    }
  }
  return Array.from(profileNames)
    .sort((a, b) => b.localeCompare(a))
    .map((name) => ({
      id: name,
      text: DATA_DESCRIPTIONS[dataType].fieldDescriptions[name]?.readableName || name,
    }));
}

/**
 * This component renders the page for ProfilingData data type, where the graphs are rendered within Iframes
 */
export default function (props: DataPageProps) {
  const { searchKey, setSearchKey } = useReportState();

  // Select dropdown: select Profiler
  const instanceOptions = React.useMemo(() => getAllInstanceNames(props.dataType), [props.dataType]);
  const [selectedInstance, setSelectedInstance] = React.useState<SelectProps.Option | null>(instanceOptions[0] || null);

  const instanceName = selectedInstance?.value || "";

  // SegmentedControl: profile within the selected Profiler
  const profileOptions = React.useMemo(
    () => getProfileNames(props.dataType, instanceName),
    [props.dataType, instanceName],
  );
  const [selectedProfile, setSelectedProfile] = React.useState(profileOptions[0]?.id || "");

  // Page-level mode toggle: "profile" (native profile) or "iframe" (legacy SVG/HTML)
  const [pageMode, setPageMode] = React.useState<"profile" | "iframe">("profile");

  const graphRowPercentage = Math.floor(100 / RUNS.length);

  // Reset profile selection when Profiler changes
  React.useEffect(() => {
    if (selectedProfile) {
      const match = profileOptions.find((opt) => opt.id === selectedProfile);
      setSelectedProfile(match?.id || profileOptions[0]?.id || "");
    } else {
      setSelectedProfile(profileOptions[0]?.id || "");
    }
  }, [instanceName]);

  // Handle searchKey from DataLink navigation
  React.useEffect(() => {
    if (searchKey) {
      const matchingInstance = instanceOptions.find((opt) => opt.value === searchKey);
      if (matchingInstance) {
        setSelectedInstance(matchingInstance);
        setSearchKey("");
      }
    }
  }, [searchKey, instanceOptions]);

  // Check if any run has profile data for the current instance/profile
  const hasProfileData = React.useMemo(() => {
    if (!instanceName || !selectedProfile) return false;
    return RUNS.some((runName) => {
      const runData = PROCESSED_DATA[props.dataType]?.runs[runName] as ProfilingData | undefined;
      const profile = runData?.profilers?.[instanceName]?.profiles?.[selectedProfile];
      return profile?.blocks?.length > 0;
    });
  }, [props.dataType, instanceName, selectedProfile]);

  // Metadata comes from the profiler instance
  const { tableItems, tableColumnDefinitions } = React.useMemo(() => {
    if (!instanceName) return { tableItems: [], tableColumnDefinitions: [] };
    const dataByRun = new Map(
      RUNS.map((runName) => {
        const runData = PROCESSED_DATA[props.dataType]?.runs[runName] as ProfilingData | undefined;
        const metadata = runData?.profilers?.[instanceName]?.metadata;
        return [runName, metadata] as const;
      }),
    );
    return buildKeyValueTable(dataByRun);
  }, [props.dataType, instanceName]);

  return (
    <SpaceBetween size="l">
      <Header
        variant={"h1"}
        info={<ReportHelpPanelLink dataType={props.dataType} fieldKey={"summary"} />}
        actions={
          <SpaceBetween direction="horizontal" size="xs">
            <ShowFindingsPanelButton />
            {profileOptions.length > 1 && (
              <SegmentedControl
                selectedId={selectedProfile}
                onChange={({ detail }) => setSelectedProfile(detail.selectedId)}
                options={profileOptions}
              />
            )}
          </SpaceBetween>
        }
      >
        {DATA_DESCRIPTIONS[props.dataType].readableName}
      </Header>

      <Select
        selectedOption={selectedInstance}
        onChange={({ detail }) => setSelectedInstance(detail.selectedOption)}
        options={instanceOptions}
        placeholder="Select a profiler instance"
        filteringType="auto"
      />

      {/* TODO: remove iframe display logic when profile-panel visualization is finalized */}
      {selectedInstance && selectedProfile && (
        <SpaceBetween size="xs">
          <Container
            header={
              <Header
                variant="h2"
                actions={
                  hasProfileData ? (
                    <SegmentedControl
                      selectedId={pageMode}
                      onChange={({ detail }) => setPageMode(detail.selectedId as "profile" | "iframe")}
                      options={[
                        { id: "profile", text: "APerf Profile" },
                        { id: "iframe", text: "Legacy Visualization" },
                      ]}
                    />
                  ) : undefined
                }
              >
                {hasProfileData ? instanceName : "Profiling Data"}
              </Header>
            }
          >
            {pageMode === "profile" && hasProfileData ? (
              <ProfilePanel dataType={props.dataType} instanceName={instanceName} selectedProfile={selectedProfile} />
            ) : (
              <div style={{ display: "flex" }}>
                {RUNS.map((runName) => (
                  <div
                    key={runName}
                    style={{ width: `${graphRowPercentage}%`, paddingTop: "10px", paddingRight: "30px" }}
                  >
                    <SpaceBetween size="xs">
                      <RunHeader runName={runName} />
                      <IframeGraph
                        dataType={props.dataType}
                        runName={runName}
                        profilerName={instanceName}
                        graphName={selectedProfile}
                      />
                    </SpaceBetween>
                  </div>
                ))}
              </div>
            )}
          </Container>

          {tableItems.length > 0 && (
            <Table
              variant="container"
              header={<Header variant="h2">Metadata</Header>}
              columnDefinitions={tableColumnDefinitions}
              items={tableItems}
              sortingDisabled={false}
              enableKeyboardNavigation={true}
              resizableColumns={true}
              wrapLines={true}
            />
          )}

          <Container header={<Header variant="h2">Analytical Findings</Header>}>
            <div style={{ display: "flex" }}>
              {RUNS.map((runName) => (
                <div
                  key={runName}
                  style={{ width: `${graphRowPercentage}%`, paddingTop: "10px", paddingRight: "30px" }}
                >
                  <ProfileAnalyticalFindings
                    dataType={props.dataType}
                    runName={runName}
                    profileInstance={selectedInstance.value}
                  />
                </div>
              ))}
            </div>
          </Container>
        </SpaceBetween>
      )}
    </SpaceBetween>
  );
}
