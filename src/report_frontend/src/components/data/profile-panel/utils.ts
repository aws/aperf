import React from "react";
import { Profile } from "../../../definitions/types";
import { getFrameType } from "./colors";

// --- Block/sample aggregation ---

/**
 * Composite key used by the Top Functions table and aggregateByFrame
 */
export function frameKey(name: string, type: ReturnType<typeof getFrameType>): string {
  return `${name} ${type}`;
}

/** Strip the four-character frame-type suffix (`_[j]` / `_[k]` / etc.) for display. */
export function strippedFrameName(rawName: string): string {
  return getFrameType(rawName) === "native" ? rawName : rawName.slice(0, -4);
}

/**
 * Whether a Search or Filter regex matches a frame. It is tested against the name shown on
 * screen, without the type suffix, so "main$" finds ComplexApp.main_[j].
 */
export function frameMatches(re: RegExp, rawName: string): boolean {
  return re.test(strippedFrameName(rawName));
}

/** The Top Functions row name, where all unresolved frames of a frame type share one row. */
export function topFunctionName(rawName: string): string {
  const name = strippedFrameName(rawName);
  return name.startsWith("[unknown]") ? "[unknown]" : name;
}

/**
 * Sum all samples across all thread states in a block. With `matched` (see filteredStacks),
 * count only the samples of call-tree nodes that pass the Filter.
 */
export function blockTotal(
  block: { [ts: string]: { [nodeId: string]: number } },
  matched: boolean[] | null = null,
): number {
  let sum = 0;
  for (const ts in block) {
    for (const nid in block[ts]) {
      if (matched && !matched[Number(nid)]) continue;
      sum += block[ts][nid];
    }
  }
  return sum;
}

/**
 * Which call-tree nodes' samples pass the Filter: a node passes when it or one of its
 * callers matches, so a filtered stack keeps everything it calls. Null without a filter.
 */
export function filteredStacks(analytics: Profile, filterRe: RegExp | null): boolean[] | null {
  const tree = analytics.context_tree;
  const frames = analytics.frame_map?.frame_id_to_frame;
  if (!filterRe || !tree || !frames || tree.length === 0) return null;
  const matched = new Array<boolean>(tree.length).fill(false);
  function walk(nid: number, ancestorMatched: boolean) {
    const name = frames[tree[nid].frame_id]?.name || "";
    const m = ancestorMatched || frameMatches(filterRe!, name);
    matched[nid] = m;
    for (const cid of Object.values(tree[nid].children)) walk(cid, m);
  }
  walk(0, false);
  return matched;
}

/** Aggregate self_samples per node_id from blocks in [start,end) */
export function computeNodeSelfSamples(
  analytics: Profile,
  blockStart: number,
  blockEnd: number,
  matched: boolean[] | null = null,
): Map<number, number> {
  const nodeSamples = new Map<number, number>();
  const blocks = analytics.blocks;
  if (!blocks) return nodeSamples;
  for (let bi = blockStart; bi < blockEnd; bi++) {
    const block = blocks[bi];
    if (!block) continue;
    for (const ts in block) {
      for (const nid in block[ts]) {
        const nodeId = Number(nid);
        if (matched && !matched[nodeId]) continue;
        nodeSamples.set(nodeId, (nodeSamples.get(nodeId) || 0) + block[ts][nid]);
      }
    }
  }
  return nodeSamples;
}

/**
 * Compute per-frame aggregated samples for the Top Functions table.
 */
export function aggregateByFrame(
  analytics: Profile,
  nodeSelf: Map<number, number>,
): Map<string, { self: number; total: number }> {
  const byFrameEmpty = new Map<string, { self: number; total: number }>();
  const tree = analytics.context_tree;
  const frames = analytics.frame_map?.frame_id_to_frame;
  if (!tree || !frames) return byFrameEmpty;

  const nodeTotal = new Map<number, number>();
  function total(nid: number): number {
    if (nodeTotal.has(nid)) return nodeTotal.get(nid)!;
    let t = nodeSelf.get(nid) || 0;
    for (const cid of Object.values(tree[nid].children)) t += total(cid);
    nodeTotal.set(nid, t);
    return t;
  }
  for (let i = 0; i < tree.length; i++) total(i);

  // Row key of each frame_id; different frame_ids can share a row.
  const frameKeys = frames.map((frame) => {
    const rawName = frame?.name || "[unknown]";
    return frameKey(topFunctionName(rawName), getFrameType(rawName));
  });
  const keyOf = (frameId: number) => frameKeys[frameId] ?? frameKey("[unknown]", "native");

  const byFrame = new Map<string, { self: number; total: number }>();
  for (let nid = 0; nid < tree.length; nid++) {
    const key = keyOf(tree[nid].frame_id);
    const entry = byFrame.get(key) || { self: 0, total: 0 };
    entry.self += nodeSelf.get(nid) || 0;
    // Skip this node's total if any ancestor is in the same row, so a
    // recursive frame counts once at its outermost call site.
    let ancestorHasSameFrame = false;
    let cur = tree[nid].parent;
    while (cur != null) {
      if (keyOf(tree[cur].frame_id) === key) {
        ancestorHasSameFrame = true;
        break;
      }
      cur = tree[cur].parent;
    }
    if (!ancestorHasSameFrame) entry.total += nodeTotal.get(nid) || 0;
    byFrame.set(key, entry);
  }
  return byFrame;
}

export interface FlamegraphNode {
  frameId: number;
  name: string;
  selfSamples: number;
  totalSamples: number;
  children: FlamegraphNode[];
  parent: FlamegraphNode | null;
  depth: number;
  x: number;
  w: number;
}

/**
 * Build a flamegraph tree from per-node self samples (see computeNodeSelfSamples), which
 * already have the Filter applied.
 * If reverse, build inverted tree (leaf → root callers).
 */
export function buildFlamegraph(
  analytics: Profile,
  nodeSelf: Map<number, number>,
  reverse: boolean,
): FlamegraphNode | null {
  const tree = analytics.context_tree;
  const frames = analytics.frame_map?.frame_id_to_frame;
  if (!tree || !frames || tree.length === 0) return null;

  if (reverse) return buildReverseFlamegraph(analytics, nodeSelf);
  return buildForwardFlamegraph(analytics, nodeSelf);
}

function buildForwardFlamegraph(analytics: Profile, nodeSelf: Map<number, number>): FlamegraphNode | null {
  const tree = analytics.context_tree;
  const frames = analytics.frame_map.frame_id_to_frame;

  const totalSamples = new Map<number, number>();
  function computeTotal(nodeId: number): number {
    if (totalSamples.has(nodeId)) return totalSamples.get(nodeId)!;
    let total = nodeSelf.get(nodeId) || 0;
    for (const cid of Object.values(tree[nodeId].children)) total += computeTotal(cid);
    totalSamples.set(nodeId, total);
    return total;
  }
  computeTotal(0);

  const rootTotal = totalSamples.get(0) || 0;
  if (rootTotal === 0) return null;

  function buildNode(nodeId: number, depth: number, x: number, parent: FlamegraphNode | null): FlamegraphNode {
    const node = tree[nodeId];
    const total = totalSamples.get(nodeId) || 0;
    const self_ = nodeSelf.get(nodeId) || 0;
    const name = frames[node.frame_id]?.name || "[unknown]";

    const result: FlamegraphNode = {
      frameId: node.frame_id,
      name,
      selfSamples: self_,
      totalSamples: total,
      children: [],
      parent,
      depth,
      x,
      w: total,
    };

    let childX = x;
    const childEntries = Object.entries(node.children)
      .map(([, cid]) => ({ id: cid, total: totalSamples.get(cid) || 0 }))
      .filter((c) => c.total > 0)
      .sort((a, b) => b.total - a.total);
    for (const child of childEntries) {
      result.children.push(buildNode(child.id, depth + 1, childX, result));
      childX += child.total;
    }
    return result;
  }
  return buildNode(0, 0, 0, null);
}

function buildReverseFlamegraph(analytics: Profile, nodeSelf: Map<number, number>): FlamegraphNode | null {
  const tree = analytics.context_tree;
  const frames = analytics.frame_map.frame_id_to_frame;

  const stacks: { path: string[]; weight: number }[] = [];
  for (let nid = 0; nid < tree.length; nid++) {
    const w = nodeSelf.get(nid) || 0;
    if (w === 0) continue;
    const path: string[] = [];
    let cur: number | null = nid;
    while (cur !== null && cur !== 0) {
      path.push(frames[tree[cur].frame_id]?.name || "[unknown]");
      cur = tree[cur].parent;
    }
    stacks.push({ path, weight: w });
  }
  if (stacks.length === 0) return null;

  interface MutNode {
    name: string;
    self: number;
    total: number;
    children: Map<string, MutNode>;
  }
  const root: MutNode = { name: "[root]", self: 0, total: 0, children: new Map() };
  for (const { path, weight } of stacks) {
    root.total += weight;
    let cur = root;
    for (const frame of path) {
      let child = cur.children.get(frame);
      if (!child) {
        child = { name: frame, self: 0, total: 0, children: new Map() };
        cur.children.set(frame, child);
      }
      child.total += weight;
      cur = child;
    }
    cur.self += weight;
  }

  let nextFrameId = 0;
  function convert(n: MutNode, depth: number, x: number, parent: FlamegraphNode | null): FlamegraphNode {
    const result: FlamegraphNode = {
      frameId: nextFrameId++,
      name: n.name,
      selfSamples: n.self,
      totalSamples: n.total,
      children: [],
      parent,
      depth,
      x,
      w: n.total,
    };
    let childX = x;
    const sorted = Array.from(n.children.values()).sort((a, b) => b.total - a.total);
    for (const child of sorted) {
      result.children.push(convert(child, depth + 1, childX, result));
      childX += child.total;
    }
    return result;
  }
  return convert(root, 0, 0, null);
}

/**
 * Match each node of a flamegraph to the same stack (frame names from the root) in a
 * baseline flamegraph, built the same way but possibly from another profile, and return
 * its [baseline, current] total samples. Sibling frames with the same name are counted
 * together on both sides, so a stack's counts don't depend on how its call-tree nodes split.
 */
export function matchBaselineStacks(
  current: FlamegraphNode,
  baseline: FlamegraphNode | null,
): Map<FlamegraphNode, [number, number]> {
  const result = new Map<FlamegraphNode, [number, number]>();
  const sumTotals = (nodes: FlamegraphNode[]) => nodes.reduce((sum, n) => sum + n.totalSamples, 0);
  const childrenByName = (nodes: FlamegraphNode[]) => {
    const byName = new Map<string, FlamegraphNode[]>();
    for (const n of nodes) {
      for (const child of n.children) {
        const group = byName.get(child.name);
        if (group) group.push(child);
        else byName.set(child.name, [child]);
      }
    }
    return byName;
  };

  // Walk both trees together, one stack at a time.
  function visit(curGroup: FlamegraphNode[], baseGroup: FlamegraphNode[]) {
    const counts: [number, number] = [sumTotals(baseGroup), sumTotals(curGroup)];
    for (const n of curGroup) result.set(n, counts);
    const baseChildren = childrenByName(baseGroup);
    for (const [name, children] of childrenByName(curGroup)) visit(children, baseChildren.get(name) ?? []);
  }
  visit([current], baseline ? [baseline] : []);
  return result;
}

/** Flatten flamegraph tree into a list for rendering */
export function flattenFlamegraph(root: FlamegraphNode): FlamegraphNode[] {
  const result: FlamegraphNode[] = [];
  const stack = [root];
  while (stack.length > 0) {
    const node = stack.pop()!;
    result.push(node);
    for (let i = node.children.length - 1; i >= 0; i--) stack.push(node.children[i]);
  }
  return result;
}

export function searchMatchStats(root: FlamegraphNode, re: RegExp): { count: number; totalSamples: number } {
  let count = 0;
  let totalSamples = 0;
  // [node, whether an ancestor matched]; the root itself is never a match.
  const stack: [FlamegraphNode, boolean][] = root.children.map((child) => [child, false]);
  while (stack.length > 0) {
    const [node, ancestorMatched] = stack.pop()!;
    const matched = frameMatches(re, node.name);
    if (matched) {
      count++;
      if (!ancestorMatched) totalSamples += node.totalSamples;
    }
    for (const child of node.children) stack.push([child, ancestorMatched || matched]);
  }
  return { count, totalSamples };
}

// --- State logic hooks ---

/** Compile a regex string into a RegExp, returning null on empty or invalid input. */
export function useRegex(source: string): RegExp | null {
  return React.useMemo<RegExp | null>(() => {
    if (!source) return null;
    try {
      return new RegExp(source);
    } catch {
      return null;
    }
  }, [source]);
}

/** Track the content width of a container element via ResizeObserver. */
export function useContainerWidth(ref: React.RefObject<HTMLDivElement | null>, initial = 800): number {
  const [width, setWidth] = React.useState(initial);

  React.useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new ResizeObserver((entries) => {
      for (const entry of entries) setWidth(entry.contentRect.width);
    });
    obs.observe(el);
    return () => obs.disconnect();
  }, [ref]);

  return width;
}

export interface ZoomLevel {
  id: string;
  label: string;
  groupSize: number;
  rows: number;
  cellMs: number;
  cellPx: number;
  /** True when the recording is too long to draw at this zoom (see MAX_HEATMAP_CELLS). */
  disabled: boolean;
}

/**
 * Most heatmap cells drawn at one zoom level, about 16 minutes at 20 ms per cell.
 * Finer zoom levels are disabled for longer recordings.
 */
export const MAX_HEATMAP_CELLS = 50_000;

export interface HeatmapLayout {
  zoomLevels: ZoomLevel[];
  zoomId: string;
  setZoomId: (id: string) => void;
  zoom: ZoomLevel;
  groupSize: number;
  rowsPerCol: number;
  cellSize: number;
  heatmapGridHeight: number;
  bufferCellsBefore: number;
  totalCells: number;
  numColumns: number;
  numDataCells: number;
  alignedStartMs: number;
}

/**
 * Compute heatmap zoom levels, UTC-aligned buffer geometry, and cell layout.
 */
export function useHeatmapLayout(
  numBlocks: number,
  blockWidthMs: number,
  alignedStartTimeMs: number,
  containerWidth: number,
): HeatmapLayout {
  const zoomLevels = React.useMemo<ZoomLevel[]>(() => {
    const blocksPerSec = Math.max(1, Math.round(1000 / blockWidthMs));
    const levels = [
      {
        id: "0",
        label: `1 sec : ${blockWidthMs} ms`,
        groupSize: 1,
        rows: blocksPerSec,
        cellMs: blockWidthMs,
        cellPx: 6,
      },
      { id: "1", label: "1 min : 1 sec", groupSize: blocksPerSec, rows: 60, cellMs: 1000, cellPx: 5 },
      { id: "2", label: "5 min : 5 sec", groupSize: blocksPerSec * 5, rows: 60, cellMs: 5000, cellPx: 3 },
    ];
    // The coarsest level always stays available.
    return levels.map((z, i) => ({
      ...z,
      disabled: i < levels.length - 1 && numBlocks / z.groupSize > MAX_HEATMAP_CELLS,
    }));
  }, [blockWidthMs, numBlocks]);

  const initialZoom = React.useMemo(() => {
    for (let i = 0; i < zoomLevels.length; i++) {
      const z = zoomLevels[i];
      if (z.disabled) continue;
      const cols = Math.ceil(numBlocks / z.groupSize / z.rows);
      if (cols * z.cellPx <= containerWidth || i === zoomLevels.length - 1) return z.id;
    }
    return zoomLevels[0].id;
  }, [numBlocks, zoomLevels, containerWidth]);

  const [zoomId, setZoomId] = React.useState(initialZoom);
  React.useEffect(() => {
    setZoomId(initialZoom);
  }, [initialZoom]);

  const zoom = zoomLevels.find((z) => z.id === zoomId) || zoomLevels[0];
  const groupSize = zoom.groupSize;
  const rowsPerCol = zoom.rows;
  const cellSize = zoom.cellPx;
  const heatmapGridHeight = rowsPerCol * cellSize;
  const numDataCells = Math.ceil(numBlocks / groupSize);

  const { bufferCellsBefore, totalCells, numColumns, alignedStartMs } = React.useMemo(() => {
    const colMs = zoom.cellMs * rowsPerCol;
    const colAlignedStart = Math.floor(alignedStartTimeMs / colMs) * colMs;
    const extraCols = alignedStartTimeMs === colAlignedStart ? 5 : 4;
    const alignedStart = colAlignedStart - extraCols * colMs;
    const bufferMs = alignedStartTimeMs - alignedStart;
    const bufferCells = Math.floor(bufferMs / zoom.cellMs);
    const dataEndMs = alignedStartTimeMs + numBlocks * blockWidthMs;
    const colAlignedEnd = Math.ceil(dataEndMs / colMs) * colMs;
    const extraColsEnd = dataEndMs === colAlignedEnd ? 5 : 4;
    const alignedEnd = colAlignedEnd + extraColsEnd * colMs;
    const totalMs = alignedEnd - alignedStart;
    const total = Math.round(totalMs / zoom.cellMs);
    return {
      bufferCellsBefore: bufferCells,
      totalCells: total,
      numColumns: Math.max(1, Math.round(total / rowsPerCol)),
      alignedStartMs: alignedStart,
    };
  }, [numBlocks, rowsPerCol, zoom, alignedStartTimeMs, blockWidthMs]);

  return {
    zoomLevels,
    zoomId,
    setZoomId,
    zoom,
    groupSize,
    rowsPerCol,
    cellSize,
    heatmapGridHeight,
    bufferCellsBefore,
    totalCells,
    numColumns,
    numDataCells,
    alignedStartMs,
  };
}

export interface HeatmapSelectionState {
  selection: [number, number];
  setSelection: React.Dispatch<React.SetStateAction<[number, number]>>;
  baselineSelection: [number, number] | null;
  setBaselineSelection: (v: [number, number] | null) => void;
  dragStart: number | null;
  dragEnd: number | null;
  dragIsBaseline: boolean;
  setDragStart: (v: number | null) => void;
  setDragEnd: (v: number | null) => void;
  setDragIsBaseline: (v: boolean) => void;
  resetSelection: () => void;
}

/**
 * Manages selection, baseline selection, and drag state for the heatmap.
 * Publishes selection to shared context when this is the base run.
 * Snaps selection to cell boundaries on groupSize change.
 */
export function useHeatmapSelection(
  numBlocks: number,
  groupSize: number,
  isBaseRun: boolean | undefined,
  setBaseRunSelection?: (v: [number, number]) => void,
): HeatmapSelectionState {
  const [selection, setSelection] = React.useState<[number, number]>([0, numBlocks]);
  const [baselineSelection, setBaselineSelection] = React.useState<[number, number] | null>(null);
  const [dragStart, setDragStart] = React.useState<number | null>(null);
  const [dragEnd, setDragEnd] = React.useState<number | null>(null);
  const [dragIsBaseline, setDragIsBaseline] = React.useState(false);

  React.useEffect(() => {
    if (isBaseRun && setBaseRunSelection) setBaseRunSelection(selection);
  }, [isBaseRun, selection, setBaseRunSelection]);

  React.useEffect(() => {
    // Keep the same tuple when nothing changes: everything derived from the selection
    // (and the flamegraph zoom) is rebuilt when its identity changes.
    const keep = (prev: [number, number], s: number, e: number): [number, number] =>
      prev[0] === s && prev[1] === e ? prev : [s, e];
    setSelection((prev) => {
      const [s, e] = prev;
      if (numBlocks === 0) return keep(prev, 0, 0);
      const alignedStart = Math.floor(s / groupSize) * groupSize;
      const alignedEnd = Math.min(numBlocks, Math.ceil(e / groupSize) * groupSize);
      // Reset to full range when the previous selection has gone out of bounds
      // OR realigning has produced an empty range (e.g. switching from an empty
      // profile back to a non-empty one would otherwise leave selection at [0,0]
      // and render an empty flamegraph).
      if (alignedStart >= numBlocks || alignedEnd <= alignedStart) return keep(prev, 0, numBlocks);
      return keep(prev, alignedStart, alignedEnd);
    });
  }, [groupSize, numBlocks]);

  const resetSelection = React.useCallback(() => {
    setSelection([0, numBlocks]);
    setBaselineSelection(null);
  }, [numBlocks]);

  return {
    selection,
    setSelection,
    baselineSelection,
    setBaselineSelection,
    dragStart,
    dragEnd,
    dragIsBaseline,
    setDragStart,
    setDragEnd,
    setDragIsBaseline,
    resetSelection,
  };
}

/** Horizontal view window over the flamegraph, in sample units. */
interface FlamegraphView {
  x: number;
  w: number;
}

/**
 * Horizontal zoom/pan within [baseX, baseX + baseW) (the zoomed node, or the root).
 * Ctrl/cmd+wheel zooms around the cursor; shift+wheel, horizontal trackpad scroll,
 * or dragging pans. A null view shows the whole base range.
 */
export function useHorizontalZoom(canvasRef: React.RefObject<HTMLCanvasElement | null>, baseX: number, baseW: number) {
  const [view, setView] = React.useState<FlamegraphView | null>(null);

  // Clamp a window to the base range; null when it covers all of it. Zooming
  // stops once a single sample fills the width.
  const clampView = React.useCallback(
    (x: number, w: number): FlamegraphView | null => {
      const minW = Math.min(baseW, 1);
      const clampedW = Math.min(Math.max(w, minW), baseW);
      if (clampedW >= baseW) return null;
      return { x: Math.min(Math.max(x, baseX), baseX + baseW - clampedW), w: clampedW };
    },
    [baseX, baseW],
  );

  // Wheel and drag events can fire faster than the display refreshes, and each
  // view change redraws every node, so apply them at most once per frame.
  const pendingUpdates = React.useRef<((v: FlamegraphView | null) => FlamegraphView | null)[]>([]);
  const frame = React.useRef(0);
  const queueView = React.useCallback((update: (v: FlamegraphView | null) => FlamegraphView | null) => {
    pendingUpdates.current.push(update);
    if (frame.current) return;
    frame.current = requestAnimationFrame(() => {
      frame.current = 0;
      const updates = pendingUpdates.current;
      pendingUpdates.current = [];
      setView((prev) => updates.reduce((v, f) => f(v), prev));
    });
  }, []);
  React.useEffect(() => () => cancelAnimationFrame(frame.current), []);

  // React's onWheel is passive, so preventDefault (to stop the page or the
  // vertical scroll box from also scrolling) needs a native listener.
  React.useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || baseW <= 0) return;
    const onWheel = (e: WheelEvent) => {
      const rect = canvas.getBoundingClientRect();
      // deltaMode is pixels (0), lines (1), or pages (2).
      let unit = 1;
      if (e.deltaMode === 1) unit = 16;
      else if (e.deltaMode === 2) unit = rect.height;

      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const frac = (e.clientX - rect.left) / rect.width;
        queueView((prev) => {
          const x = prev ? prev.x : baseX;
          const w = prev ? prev.w : baseW;
          const newW = w * Math.exp(e.deltaY * unit * 0.002);
          return clampView(x + frac * w - frac * newW, newW);
        });
        return;
      }

      let dx = 0;
      if (e.shiftKey) dx = e.deltaY || e.deltaX;
      else if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) dx = e.deltaX;
      if (dx === 0) return;
      e.preventDefault();
      queueView((prev) => (prev ? clampView(prev.x + ((dx * unit) / rect.width) * prev.w, prev.w) : null));
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => canvas.removeEventListener("wheel", onWheel);
  }, [canvasRef, baseX, baseW, clampView, queueView]);

  // Drag-to-pan while zoomed in. A drag past a few pixels should not also
  // count as a click, so the click handler checks consumeDrag() first.
  const drag = React.useRef<{ startX: number; viewX: number; moved: boolean } | null>(null);
  const onMouseDown = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      // A pan released off the canvas never reaches onClick to consume its drag.
      drag.current = null;
      if (e.button !== 0 || !view) return;
      const width = e.currentTarget.getBoundingClientRect().width;
      const w = view.w;
      drag.current = { startX: e.clientX, viewX: view.x, moved: false };
      const onMove = (ev: MouseEvent) => {
        const d = drag.current;
        if (!d) return;
        const dx = ev.clientX - d.startX;
        if (Math.abs(dx) > 3) d.moved = true;
        if (d.moved) queueView(() => clampView(d.viewX - (dx / width) * w, w));
      };
      const onUp = () => {
        window.removeEventListener("mousemove", onMove);
        window.removeEventListener("mouseup", onUp);
      };
      window.addEventListener("mousemove", onMove);
      window.addEventListener("mouseup", onUp);
    },
    [view, clampView, queueView],
  );
  const consumeDrag = React.useCallback((): boolean => {
    const moved = drag.current?.moved ?? false;
    drag.current = null;
    return moved;
  }, []);

  const resetView = React.useCallback(() => setView(null), []);

  return {
    viewX: view ? view.x : baseX,
    viewW: view ? view.w : baseW,
    resetView,
    onMouseDown,
    consumeDrag,
  };
}
