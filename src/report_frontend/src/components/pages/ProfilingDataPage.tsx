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
import ProfilePanel from "../data/profile-panel/ProfilePanel";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import { ShowFindingsPanelButton } from "../analytics/FindingsSplitPanel";
import { buildKeyValueTable } from "../data/KeyValueTable";
import { ProfileAnalyticalFindings } from "../analytics/AnalyticalFindings";
import { useReportState } from "../ReportStateProvider";

/**
 * Collect all profiler instance names across all runs for the Select dropdown.
 * Sorted alphabetically for stable presentation.
 */
function getAllInstanceNames(dataType: DataType): SelectProps.Option[] {
  const names = new Set<string>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType]?.runs?.[runName] as ProfilingData | undefined;
    if (!reportData?.profilers) continue;
    for (const name of Object.keys(reportData.profilers)) {
      names.add(name);
    }
  }
  return Array.from(names)
    .sort((a, b) => a.localeCompare(b))
    .map((name) => ({ label: name, value: name }));
}

/**
 * Compute the list of profile names (for the SegmentedControl) sorted descending alphabetically
 */
function getProfileNames(dataType: DataType, instanceName: string): SegmentedControlProps.Option[] {
  const profileNames = new Set<string>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType]?.runs?.[runName] as ProfilingData | undefined;
    const instance = reportData?.profilers?.[instanceName];
    if (!instance?.profiles) continue;
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
 * This component renders the page for ProfilingData data types using the native ProfilePanel
 * (heatmap + flamegraph / methods table).
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

      {selectedInstance && selectedProfile && (
        <SpaceBetween size="xs">
          <Container header={<Header variant="h2">{instanceName}</Header>}>
            <ProfilePanel dataType={props.dataType} instanceName={instanceName} selectedProfile={selectedProfile} />
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
