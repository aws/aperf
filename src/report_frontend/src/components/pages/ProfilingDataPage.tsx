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
} from "@cloudscape-design/components";
import Header from "@cloudscape-design/components/header";
import ProfilePanel from "../data/profile-panel/ProfilePanel";
import { DATA_DESCRIPTIONS } from "../../definitions/data-descriptions";
import { ReportHelpPanelLink } from "../misc/ReportHelpPanel";
import { ShowFindingsPanelButton } from "../analytics/FindingsSplitPanel";
import { PerKeyAnalyticalFindings } from "../analytics/AnalyticalFindings";
import { useReportState } from "../ReportStateProvider";

/**
 * Collect all profiler names across all runs for the Select dropdown.
 * Sorted alphabetically for stable presentation.
 */
function getAllProfilerNames(dataType: DataType): SelectProps.Option[] {
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
function getProfileNames(dataType: DataType, profilerName: string): SegmentedControlProps.Option[] {
  const profileNames = new Set<string>();
  for (const runName of RUNS) {
    const reportData = PROCESSED_DATA[dataType]?.runs?.[runName] as ProfilingData | undefined;
    const profiler = reportData?.profilers?.[profilerName];
    if (!profiler?.profiles) continue;
    for (const profileName in profiler.profiles) {
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
  const profilerOptions = React.useMemo(() => getAllProfilerNames(props.dataType), [props.dataType]);
  const [selectedProfiler, setSelectedProfiler] = React.useState<SelectProps.Option | null>(profilerOptions[0] || null);

  const profilerName = selectedProfiler?.value || "";

  // SegmentedControl: profile within the selected Profiler
  const profileOptions = React.useMemo(
    () => getProfileNames(props.dataType, profilerName),
    [props.dataType, profilerName],
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
  }, [profilerName]);

  // Handle searchKey from DataLink navigation
  React.useEffect(() => {
    if (searchKey) {
      const matchingProfiler = profilerOptions.find((opt) => opt.value === searchKey);
      if (matchingProfiler) {
        setSelectedProfiler(matchingProfiler);
        setSearchKey("");
      }
    }
  }, [searchKey, profilerOptions]);

  return (
    <SpaceBetween size="l">
      <Header
        variant={"h1"}
        info={<ReportHelpPanelLink dataType={props.dataType} fieldKey={"summary"} />}
        actions={<ShowFindingsPanelButton />}
      >
        {DATA_DESCRIPTIONS[props.dataType].readableName}
      </Header>

      <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
        <div style={{ flex: "1 1 auto", minWidth: 0 }}>
          <Select
            selectedOption={selectedProfiler}
            onChange={({ detail }) => setSelectedProfiler(detail.selectedOption)}
            options={profilerOptions}
            placeholder="Select a profiler"
            filteringType="auto"
          />
        </div>
        {profileOptions.length > 1 && (
          <SegmentedControl
            selectedId={selectedProfile}
            onChange={({ detail }) => setSelectedProfile(detail.selectedId)}
            options={profileOptions}
          />
        )}
      </div>

      {selectedProfiler && selectedProfile && (
        <SpaceBetween size="xs">
          <Container header={<Header variant="h2">{profilerName}</Header>}>
            <ProfilePanel dataType={props.dataType} profilerName={profilerName} selectedProfile={selectedProfile} />
          </Container>
          <Container header={<Header variant="h2">Analytical Findings</Header>}>
            <div style={{ display: "flex" }}>
              {RUNS.map((runName) => (
                <div
                  key={runName}
                  style={{ width: `${graphRowPercentage}%`, paddingTop: "10px", paddingRight: "30px" }}
                >
                  <PerKeyAnalyticalFindings
                    dataType={props.dataType}
                    runName={runName}
                    dataKey={selectedProfiler.value}
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
