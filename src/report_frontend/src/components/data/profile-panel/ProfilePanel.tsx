import React from "react";
import { Profile } from "../../../definitions/types";
import { useReportState } from "../../ReportStateProvider";
import { useProfilePanelState, ViewMode } from "./ProfilePanelStateProvider";
import { DataType, ProfilingData } from "../../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../../definitions/data-config";
import {
  Box,
  Checkbox,
  Input,
  SegmentedControl,
  SpaceBetween,
  StatusIndicator,
  Table,
} from "@cloudscape-design/components";
import { RunHeader } from "../RunSystemInfo";
import { ProfilePanelStateProvider } from "./ProfilePanelStateProvider";
import { useContainerWidth, useHeatmapLayout, useHeatmapSelection } from "./utils";
import { FlamegraphCanvas } from "./FlamegraphCanvas";
import { HeatmapCanvas, HeatmapInfoBar } from "./Heatmap";
import { TopFunctionsTable } from "./TopFunctionsTable";
import { buildKeyValueTable } from "../KeyValueTable";
import { FrameType, FRAME_TYPE_COLORS, FRAME_TYPE_LABELS, getThemeColors } from "./colors";
import { FlamegraphNode } from "./utils";
import { createProfileViewDerivedValues } from "./profileView";
import DataUnavailableBox from "../DataUnavailableBox";

// Height in pixels reserved for the time axis label above the heatmap grid
const AXIS_HEIGHT = 20;
const FILTER_DEBOUNCE_MS = 1000;

interface ProfilePanelProps {
  readonly dataType: DataType;
  readonly profilerName: string;
  readonly selectedProfile: string;
}

export default function ProfilePanel({ dataType, profilerName, selectedProfile }: ProfilePanelProps) {
  return (
    <ProfilePanelStateProvider>
      <ProfilePanelBody dataType={dataType} profilerName={profilerName} selectedProfile={selectedProfile} />
    </ProfilePanelStateProvider>
  );
}

/**
 * Reads viewMode from the provider and either renders the per-run heatmap+flamegraph
 * grid (for "flamegraph" / "top_functions") or a single full-width metadata table
 * (for "metadata"). Sits inside the provider so the toolbar's segmented control
 * controls both branches.
 */
function ProfilePanelBody({ dataType, profilerName, selectedProfile }: ProfilePanelProps) {
  const { viewMode } = useProfilePanelState();
  const widthPercent = Math.floor(100 / RUNS.length);

  // Find base run profile
  const baseRunData = PROCESSED_DATA[dataType]?.runs[RUNS[0]] as ProfilingData | undefined;
  const baseProfile = baseRunData?.profilers?.[profilerName]?.profiles?.[selectedProfile];
  const validBaseline = baseProfile && baseProfile.blocks?.length > 0 ? baseProfile : undefined;

  return (
    <>
      <div style={{ marginBottom: 12 }}>
        <ProfilePanelToolbar />
      </div>
      {viewMode === "metadata" ? (
        <MetadataTable dataType={dataType} profilerName={profilerName} />
      ) : (
        <div style={{ display: "flex", width: "100%" }}>
          {RUNS.map((runName, runIdx) => {
            const runData = PROCESSED_DATA[dataType]?.runs[runName] as ProfilingData | undefined;
            const profiler = runData?.profilers?.[profilerName];
            const profile = profiler?.profiles?.[selectedProfile];

            return (
              <div
                key={runName}
                style={{
                  flex: `0 0 ${widthPercent}%`,
                  minWidth: 0,
                  overflow: "hidden",
                  paddingTop: "10px",
                  paddingRight: "30px",
                  boxSizing: "border-box",
                }}
              >
                <SpaceBetween size="xs">
                  <RunHeader runName={runName} />
                  {profiler && profile ? (
                    <ProfilePanelView
                      key={`${profilerName}/${selectedProfile}`}
                      analytics={profile}
                      baseline={runIdx > 0 ? validBaseline : undefined}
                      isBaseRun={runIdx === 0}
                      startTimeMs={profiler.start_time_ms}
                      blockWidthMs={profiler.block_width_ms}
                    />
                  ) : (
                    <DataUnavailableBox dataType={dataType} runName={runName} />
                  )}
                </SpaceBetween>
              </div>
            );
          })}
        </div>
      )}
    </>
  );
}

/**
 * Full-width per-profiler metadata table, rendered when viewMode === "metadata".
 * Builds a one-row-per-key cross-run table from each run's `Profiler.metadata`.
 */
function MetadataTable({ dataType, profilerName }: { readonly dataType: DataType; readonly profilerName: string }) {
  const { tableItems, tableColumnDefinitions } = React.useMemo(() => {
    const dataByRun = new Map(
      RUNS.map((runName) => {
        const runData = PROCESSED_DATA[dataType]?.runs?.[runName] as ProfilingData | undefined;
        const metadata = runData?.profilers?.[profilerName]?.metadata;
        return [runName, metadata] as const;
      }),
    );
    return buildKeyValueTable(dataType, dataByRun);
  }, [dataType, profilerName]);

  if (tableItems.length === 0) {
    return <EmptyProfileState message="No metadata available for this profiler." />;
  }
  return (
    <Table
      variant="embedded"
      columnDefinitions={tableColumnDefinitions}
      items={tableItems}
      sortingDisabled={false}
      enableKeyboardNavigation={true}
      resizableColumns={true}
      wrapLines={true}
    />
  );
}

interface ProfilePanelViewProps {
  readonly analytics: Profile;
  readonly baseline?: Profile;
  readonly isBaseRun?: boolean;
  readonly startTimeMs: number;
  readonly blockWidthMs: number;
}

/**
 * Main composition root for the heatmap + flamegraph visualization.
 * Renders: info bar → heatmap canvas → flamegraph/top_functions view → tooltip overlay.
 * Reads shared state (search, filter, diff, viewMode) from ProfilePanelContext.
 */
function ProfilePanelView({ analytics, baseline, isBaseRun, startTimeMs, blockWidthMs }: ProfilePanelViewProps) {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);

  const { searchRe, filterRe, reverse, showDiff, viewMode, baseRunSelection, setBaseRunSelection } =
    useProfilePanelState();

  const containerRef = React.useRef<HTMLDivElement>(null);
  const containerWidth = useContainerWidth(containerRef);

  const alignedStartTimeMs = startTimeMs - (startTimeMs % blockWidthMs);
  const numBlocks = analytics.blocks?.length ?? 0;

  const layout = useHeatmapLayout(numBlocks, blockWidthMs, alignedStartTimeMs, containerWidth);
  const sel = useHeatmapSelection(numBlocks, layout.groupSize, isBaseRun, setBaseRunSelection);

  const [tooltip, setTooltip] = React.useState<{ x: number; y: number; text: string } | null>(null);

  const tooltipRef = React.useRef<HTMLDivElement>(null);
  React.useLayoutEffect(() => {
    const el = tooltipRef.current;
    const panel = containerRef.current;
    if (!el || !panel || !tooltip) return;
    const below = tooltip.y - 10;
    const fitsBelow = below + el.offsetHeight <= panel.clientHeight;
    el.style.top = `${fitsBelow ? below : Math.max(0, tooltip.y - el.offsetHeight - 10)}px`;
  });
  const [zoomedNode, setZoomedNode] = React.useState<FlamegraphNode | null>(null);

  // Derive selections and variables per panel
  const [deriveProfileView] = React.useState(createProfileViewDerivedValues);
  const {
    cellTotals,
    maxCellTotal,
    selSamples,
    flamegraphRoot,
    flatNodes,
    baselineRoot,
    diffing,
    searchMatches,
    framesList,
    baselineByFrame,
  } = deriveProfileView({
    profile: analytics,
    baseline,
    filterRe,
    searchRe,
    reverse,
    showDiff,
    selection: sel.selection,
    baselineSelection: sel.baselineSelection,
    baseRunSelection,
    groupSize: layout.groupSize,
    numDataCells: layout.numDataCells,
  });

  React.useEffect(() => {
    setZoomedNode(null);
  }, [flamegraphRoot]);

  if (numBlocks === 0) {
    return <EmptyProfileState message="No samples recorded for this profile." />;
  }

  return (
    <div
      ref={containerRef}
      style={{
        position: "relative",
        width: "100%",
        minWidth: 0,
        overflow: "hidden",
      }}
    >
      {/* Info bar: selection stats, search match count, zoom controls */}
      <HeatmapInfoBar
        selection={sel.selection}
        blockWidthMs={blockWidthMs}
        selSamples={selSamples}
        searchRe={searchRe}
        flamegraphRoot={flamegraphRoot}
        searchMatches={searchMatches}
        zoomedNode={zoomedNode}
        setZoomedNode={setZoomedNode}
        zoomLevels={layout.zoomLevels}
        zoomId={layout.zoomId}
        setZoomId={layout.setZoomId}
        onReset={sel.resetSelection}
      />

      {/* Time-based heatmap grid: drag to select, ctrl-drag for baseline, double-click to reset */}
      <HeatmapCanvas
        numBlocks={numBlocks}
        numDataCells={layout.numDataCells}
        totalCells={layout.totalCells}
        bufferCellsBefore={layout.bufferCellsBefore}
        cellTotals={cellTotals}
        maxCellTotal={maxCellTotal}
        groupSize={layout.groupSize}
        rowsPerCol={layout.rowsPerCol}
        numColumns={layout.numColumns}
        cellSize={layout.cellSize}
        heatmapGridHeight={layout.heatmapGridHeight}
        blockWidthMs={blockWidthMs}
        alignedStartTimeMs={alignedStartTimeMs}
        alignedStartMs={layout.alignedStartMs}
        zoom={layout.zoom}
        selection={sel.selection}
        baselineSelection={sel.baselineSelection}
        dragStart={sel.dragStart}
        dragEnd={sel.dragEnd}
        dragIsBaseline={sel.dragIsBaseline}
        axisHeight={AXIS_HEIGHT}
        setSelection={sel.setSelection}
        setBaselineSelection={sel.setBaselineSelection}
        setDragStart={sel.setDragStart}
        setDragEnd={sel.setDragEnd}
        setDragIsBaseline={sel.setDragIsBaseline}
        setTooltip={setTooltip}
        onDoubleClick={sel.resetSelection}
      />

      {viewMode === "flamegraph" ? (
        <FlamegraphCanvas
          flatNodes={flatNodes}
          flamegraphRoot={flamegraphRoot}
          containerWidth={containerWidth}
          baselineRoot={baselineRoot}
          diffing={diffing}
          searchRe={searchRe}
          zoomedNode={zoomedNode}
          setZoomedNode={setZoomedNode}
          setTooltip={setTooltip}
        />
      ) : (
        <TopFunctionsTable framesList={framesList} searchRe={searchRe} baselineByFrame={baselineByFrame} />
      )}

      {/* Floating tooltip positioned near the cursor */}
      {tooltip && (
        <div
          ref={tooltipRef}
          style={{
            position: "absolute",
            ...(tooltip.x > containerWidth / 2 ? { right: containerWidth - tooltip.x + 12 } : { left: tooltip.x + 12 }),
            top: tooltip.y - 10,
            background: theme.bgSurface,
            color: theme.textPrimary,
            border: `1px solid ${theme.border}`,
            borderRadius: 4,
            padding: "4px 8px",
            fontSize: 11,
            fontFamily: "monospace",
            whiteSpace: "pre-wrap",
            overflowWrap: "anywhere",
            pointerEvents: "none",
            zIndex: 100,
            maxWidth: 400,
          }}
        >
          {tooltip.text}
        </div>
      )}
    </div>
  );
}

function ProfilePanelToolbar() {
  const {
    searchRegex,
    filterRegex,
    reverse,
    viewMode,
    showDiff,
    setSearchRegex,
    setFilterRegex,
    setReverse,
    setShowDiff,
    setViewMode,
  } = useProfilePanelState();
  const [filterText, setFilterText] = React.useState(filterRegex);
  React.useEffect(() => {
    if (filterText === filterRegex) return;
    const timer = setTimeout(() => setFilterRegex(filterText), FILTER_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [filterText, filterRegex, setFilterRegex]);

  return (
    <SpaceBetween direction="vertical" size="xs">
      <SpaceBetween direction="horizontal" size="s" alignItems="center">
        <Input
          value={searchRegex}
          onChange={({ detail }) => setSearchRegex(detail.value)}
          placeholder="Search (regex)"
          type="search"
        />
        <Input
          value={filterText}
          onChange={({ detail }) => setFilterText(detail.value)}
          onKeyDown={({ detail }) => {
            if (detail.key === "Enter") setFilterRegex(filterText);
          }}
          placeholder="Filter (regex)"
          type="search"
        />
        <Checkbox checked={reverse} onChange={({ detail }) => setReverse(detail.checked)}>
          Reverse
        </Checkbox>
        <Checkbox checked={showDiff} onChange={({ detail }) => setShowDiff(detail.checked)}>
          Diff
        </Checkbox>
        <SegmentedControl
          selectedId={viewMode}
          onChange={({ detail }) => setViewMode(detail.selectedId as ViewMode)}
          options={[
            { id: "flamegraph", text: "Flamegraph" },
            { id: "top_functions", text: "Top Functions" },
            { id: "metadata", text: "Metadata" },
          ]}
        />
      </SpaceBetween>
      {viewMode !== "metadata" && <FrameTypeLegend />}
    </SpaceBetween>
  );
}

function FrameTypeLegend() {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);
  const order: FrameType[] = ["interpreted", "jit", "inlined", "native", "cpp", "vdso", "kernel", "c1"];
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 10, fontSize: 11, color: theme.textSubtle }}>
      {order.map((t) => (
        <span key={t} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <span
            style={{
              display: "inline-block",
              width: 12,
              height: 12,
              background: FRAME_TYPE_COLORS[t],
              border: `1px solid ${theme.borderSubtle}`,
              borderRadius: 2,
            }}
          />
          {FRAME_TYPE_LABELS[t]}
        </span>
      ))}
    </div>
  );
}

/**
 * Placeholder rendered in place of the heatmap+flamegraph when a run has no
 * samples for the selected profile (e.g. allocation profiling on a workload
 * that allocates very little). Keeps the per-run column framing so the user
 * can still see which run is empty alongside runs that have data.
 */
function EmptyProfileState({ message }: { readonly message: string }) {
  return (
    <Box padding="m" textAlign="center" color="text-status-inactive">
      <StatusIndicator type="info">{message}</StatusIndicator>
    </Box>
  );
}
