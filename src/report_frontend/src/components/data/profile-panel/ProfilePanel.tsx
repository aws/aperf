import React from "react";
import { Profile } from "../../../definitions/types";
import { useReportState } from "../../ReportStateProvider";
import { useProfilePanelState, ViewMode } from "./ProfilePanelStateProvider";
import { DataType, ProfilingData } from "../../../definitions/types";
import { PROCESSED_DATA, RUNS } from "../../../definitions/data-config";
import { Box, Checkbox, Input, SegmentedControl, SpaceBetween, StatusIndicator } from "@cloudscape-design/components";
import { RunHeader } from "../RunSystemInfo";
import { ProfilePanelStateProvider } from "./ProfilePanelStateProvider";
import { useRegex, useContainerWidth, useHeatmapLayout, useHeatmapSelection } from "./utils";
import { FlamegraphCanvas } from "./FlamegraphCanvas";
import { HeatmapCanvas, HeatmapInfoBar } from "./Heatmap";
import { MethodsTable } from "./MethodsTable";
import { FrameType, FRAME_TYPE_COLORS, FRAME_TYPE_LABELS, getThemeColors, getFrameType } from "./colors";
import {
  blockTotal,
  computeNodeSelfSamples,
  aggregateByFrame,
  aggregateByStack,
  FlamegraphNode,
  buildFlamegraph,
  flattenFlamegraph,
} from "./utils";

// Height in pixels reserved for the time axis label above the heatmap grid
const AXIS_HEIGHT = 20;

interface ProfilePanelProps {
  readonly dataType: DataType;
  readonly instanceName: string;
  readonly selectedProfile: string;
}

export default function ProfilePanel({ dataType, instanceName, selectedProfile }: ProfilePanelProps) {
  const widthPercent = Math.floor(100 / RUNS.length);

  // Find base run profile
  const baseRunData = PROCESSED_DATA[dataType]?.runs[RUNS[0]] as ProfilingData | undefined;
  const baseProfile = baseRunData?.profilers?.[instanceName]?.profiles?.[selectedProfile];
  const validBaseline = baseProfile && baseProfile.blocks?.length > 0 ? baseProfile : undefined;

  return (
    <ProfilePanelStateProvider>
      <div style={{ marginBottom: 12 }}>
        <ProfilePanelToolbar />
      </div>
      <div style={{ display: "flex" }}>
        {RUNS.map((runName, runIdx) => {
          const runData = PROCESSED_DATA[dataType]?.runs[runName] as ProfilingData | undefined;
          const profiler = runData?.profilers?.[instanceName];
          const profile = profiler?.profiles?.[selectedProfile];

          return (
            <div key={runName} style={{ width: `${widthPercent}%`, paddingTop: "10px", paddingRight: "30px" }}>
              <SpaceBetween size="xs">
                <RunHeader runName={runName} />
                {profiler && profile ? (
                  <ProfilePanelView
                    key={selectedProfile}
                    analytics={profile}
                    baseline={runIdx > 0 ? validBaseline : undefined}
                    isBaseRun={runIdx === 0}
                    startTimeMs={profiler.start_time_ms}
                    blockWidthMs={profiler.block_width_ms}
                  />
                ) : (
                  <EmptyProfileState message="No profile data available for this run." />
                )}
              </SpaceBetween>
            </div>
          );
        })}
      </div>
    </ProfilePanelStateProvider>
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

  const { searchRegex, filterRegex, reverse, showDiff, viewMode, baseRunSelection, setBaseRunSelection } =
    useProfilePanelState();

  const searchRe = useRegex(searchRegex);
  const filterRe = useRegex(filterRegex);

  const containerRef = React.useRef<HTMLDivElement>(null);
  const containerWidth = useContainerWidth(containerRef);

  const alignedStartTimeMs = startTimeMs - (startTimeMs % blockWidthMs);
  const numBlocks = analytics.blocks?.length ?? 0;

  const layout = useHeatmapLayout(numBlocks, blockWidthMs, alignedStartTimeMs, containerWidth);
  const sel = useHeatmapSelection(numBlocks, layout.groupSize, isBaseRun, setBaseRunSelection);

  const [tooltip, setTooltip] = React.useState<{ x: number; y: number; text: string } | null>(null);
  const [zoomedNode, setZoomedNode] = React.useState<FlamegraphNode | null>(null);

  const blockTotals = React.useMemo(() => (analytics.blocks ?? []).map(blockTotal), [analytics.blocks]);

  const cellTotals = React.useMemo(() => {
    if (layout.groupSize === 1) return blockTotals;
    const out = new Array<number>(layout.numDataCells).fill(0);
    for (let i = 0; i < numBlocks; i++) {
      out[Math.floor(i / layout.groupSize)] += blockTotals[i];
    }
    return out;
  }, [blockTotals, layout.groupSize, numBlocks, layout.numDataCells]);

  const maxCellTotal = React.useMemo(() => Math.max(1, ...cellTotals), [cellTotals]);

  const flamegraphRoot = React.useMemo(
    () => buildFlamegraph(analytics, sel.selection[0], sel.selection[1], filterRe, reverse),
    [analytics, sel.selection, filterRe, reverse],
  );

  React.useEffect(() => {
    setZoomedNode(null);
  }, [flamegraphRoot]);

  // Baseline diff: compute per-frame sample aggregation for the baseline profile.
  // Priority: intra-profile ctrl-drag selection > cross-profile diff (when showDiff is on).
  const baselineByFrame = React.useMemo(() => {
    if (sel.baselineSelection) {
      const baseSelf = computeNodeSelfSamples(analytics, sel.baselineSelection[0], sel.baselineSelection[1]);
      return aggregateByFrame(analytics, baseSelf);
    }
    if (!showDiff || !baseline || baseline === analytics) return null;
    const start = baseRunSelection ? baseRunSelection[0] : 0;
    const end = baseRunSelection ? baseRunSelection[1] : baseline.blocks.length;
    const baseSelf = computeNodeSelfSamples(baseline, start, end);
    return aggregateByFrame(baseline, baseSelf);
  }, [baseline, analytics, sel.baselineSelection, showDiff, baseRunSelection]);

  // Per-stack aggregation for flamegraph diff coloring (difffolded.pl semantics)
  const baselineByStack = React.useMemo(() => {
    if (sel.baselineSelection) {
      const baseSelf = computeNodeSelfSamples(analytics, sel.baselineSelection[0], sel.baselineSelection[1]);
      return aggregateByStack(analytics, baseSelf, reverse);
    }
    if (!showDiff || !baseline || baseline === analytics) return null;
    const start = baseRunSelection ? baseRunSelection[0] : 0;
    const end = baseRunSelection ? baseRunSelection[1] : baseline.blocks.length;
    const baseSelf = computeNodeSelfSamples(baseline, start, end);
    return aggregateByStack(baseline, baseSelf, reverse);
  }, [baseline, analytics, sel.baselineSelection, showDiff, baseRunSelection, reverse]);

  const currentByStack = React.useMemo(() => {
    if (!baselineByStack) return null;
    const curSelf = computeNodeSelfSamples(analytics, sel.selection[0], sel.selection[1]);
    return aggregateByStack(analytics, curSelf, reverse);
  }, [analytics, sel.selection, baselineByStack, reverse]);

  // Flatten the tree into a list for rendering and aggregation
  const flatNodes = React.useMemo(() => (flamegraphRoot ? flattenFlamegraph(flamegraphRoot) : []), [flamegraphRoot]);

  // Total samples in the current selection (shown in the info bar)
  const selSamples = React.useMemo(() => {
    let sum = 0;
    for (let i = sel.selection[0]; i < sel.selection[1]; i++) sum += blockTotals[i] || 0;
    return sum;
  }, [sel.selection, blockTotals]);

  // Count of frames matching the search regex and their total sample weight
  const searchMatches = React.useMemo(() => {
    if (!searchRe) return { count: 0, totalSamples: 0 };
    let count = 0;
    let totalSamples = 0;
    for (const n of flatNodes) {
      if (n.depth > 0 && searchRe.test(n.name)) {
        count++;
        totalSamples += n.totalSamples;
      }
    }
    return { count, totalSamples };
  }, [flatNodes, searchRe]);

  // Aggregate flat nodes into a per-method summary for the Methods table view.
  // Deduplicates by frame name+type, avoids double-counting recursive calls in "total".
  const methodsList = React.useMemo(() => {
    if (!flamegraphRoot) return [];
    const byKey = new Map<
      string,
      { name: string; type: ReturnType<typeof getFrameType>; self: number; total: number }
    >();
    for (const n of flatNodes) {
      if (n.depth === 0) continue;
      const type = getFrameType(n.name);
      const stripped = type === "native" ? n.name : n.name.slice(0, -4);
      const key = `${stripped}\u0000${type}`;
      const entry = byKey.get(key) || { name: stripped, type, self: 0, total: 0 };
      entry.self += n.selfSamples;
      let ancestorHasSameFrame = false;
      let cur = n.parent;
      while (cur) {
        if (cur.name === n.name) {
          ancestorHasSameFrame = true;
          break;
        }
        cur = cur.parent;
      }
      if (!ancestorHasSameFrame) entry.total += n.totalSamples;
      byKey.set(key, entry);
    }
    const rootTotal = flamegraphRoot.totalSamples || 1;
    return Array.from(byKey.values())
      .map((v) => ({
        name: v.name,
        type: v.type,
        self: v.self,
        total: v.total,
        selfPct: (v.self / rootTotal) * 100,
        totalPct: (v.total / rootTotal) * 100,
      }))
      .sort((a, b) => b.self - a.self);
  }, [flatNodes, flamegraphRoot]);

  if (numBlocks === 0) {
    return <EmptyProfileState message="No samples recorded for this profile." />;
  }

  return (
    <div ref={containerRef} style={{ position: "relative", width: "100%" }}>
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

      {/* Flamegraph (click to zoom) or Methods table, toggled by provider viewMode */}
      {viewMode === "flamegraph" ? (
        <FlamegraphCanvas
          flatNodes={flatNodes}
          flamegraphRoot={flamegraphRoot}
          containerWidth={containerWidth}
          baselineByStack={baselineByStack}
          currentByStack={currentByStack}
          searchRe={searchRe}
          zoomedNode={zoomedNode}
          setZoomedNode={setZoomedNode}
          setTooltip={setTooltip}
          heatmapGridHeight={layout.heatmapGridHeight}
          axisHeight={AXIS_HEIGHT}
        />
      ) : (
        <MethodsTable methodsList={methodsList} searchRe={searchRe} baselineByFrame={baselineByFrame} />
      )}

      {/* Floating tooltip positioned near the cursor */}
      {tooltip && (
        <div
          style={{
            position: "absolute",
            left: Math.min(tooltip.x + 12, containerWidth - 300),
            top: tooltip.y - 10,
            background: theme.bgSurface,
            color: theme.textPrimary,
            border: `1px solid ${theme.border}`,
            borderRadius: 4,
            padding: "4px 8px",
            fontSize: 11,
            fontFamily: "monospace",
            whiteSpace: "pre",
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
          value={filterRegex}
          onChange={({ detail }) => setFilterRegex(detail.value)}
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
          ]}
        />
      </SpaceBetween>
      <FrameTypeLegend />
    </SpaceBetween>
  );
}

function FrameTypeLegend() {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);
  const order: FrameType[] = ["interpreted", "jit", "inlined", "native", "kernel", "c1"];
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
