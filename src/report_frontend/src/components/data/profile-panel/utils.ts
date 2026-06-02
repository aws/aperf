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

/** Sum all samples across all thread states in a block */
export function blockTotal(block: { [ts: string]: { [nodeId: string]: number } }): number {
  let sum = 0;
  for (const ts in block) {
    for (const nid in block[ts]) {
      sum += block[ts][nid];
    }
  }
  return sum;
}

/** Aggregate self_samples per node_id from blocks in [start,end) */
export function computeNodeSelfSamples(analytics: Profile, blockStart: number, blockEnd: number): Map<number, number> {
  const nodeSamples = new Map<number, number>();
  const blocks = analytics.blocks;
  if (!blocks) return nodeSamples;
  for (let bi = blockStart; bi < blockEnd; bi++) {
    const block = blocks[bi];
    if (!block) continue;
    for (const ts in block) {
      for (const nid in block[ts]) {
        const nodeId = Number(nid);
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

  const byFrame = new Map<string, { self: number; total: number }>();
  for (let nid = 0; nid < tree.length; nid++) {
    const rawName = frames[tree[nid].frame_id]?.name || "[unknown]";
    const key = frameKey(strippedFrameName(rawName), getFrameType(rawName));
    const entry = byFrame.get(key) || { self: 0, total: 0 };
    entry.self += nodeSelf.get(nid) || 0;
    // Skip this node's total if any ancestor has the same frame_id, so a
    // recursive frame counts once at its outermost call site.
    let ancestorHasSameFrame = false;
    let cur = tree[nid].parent;
    while (cur != null) {
      if (tree[cur].frame_id === tree[nid].frame_id) {
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

/** Aggregate by full stack path (semicolon-separated) */
export function aggregateByStack(
  analytics: Profile,
  nodeSelf: Map<number, number>,
  reverse?: boolean,
): Map<string, { self: number; total: number }> {
  const byStackEmpty = new Map<string, { self: number; total: number }>();
  const tree = analytics.context_tree;
  const frames = analytics.frame_map?.frame_id_to_frame;
  if (!tree || !frames) return byStackEmpty;

  const nodeTotal = new Map<number, number>();
  function total(nid: number): number {
    if (nodeTotal.has(nid)) return nodeTotal.get(nid)!;
    let t = nodeSelf.get(nid) || 0;
    for (const cid of Object.values(tree[nid].children)) t += total(cid);
    nodeTotal.set(nid, t);
    return t;
  }
  for (let i = 0; i < tree.length; i++) total(i);

  // TODO: standard and reverse path building can be optimized further
  if (reverse) {
    // Build reversed paths matching buildReverseFlamegraph's merge-by-name behavior.
    // The reversed flamegraph merges callers by name at each level, so we must do the same.
    const byStack = new Map<string, { self: number; total: number }>();
    for (let nid = 0; nid < tree.length; nid++) {
      const w = nodeSelf.get(nid) || 0;
      if (w === 0) continue;
      const path: string[] = [];
      let cur: number | null = nid;
      while (cur !== null && cur !== 0) {
        path.push(frames[tree[cur].frame_id]?.name || "[unknown]");
        cur = tree[cur].parent;
      }
      // Each prefix of the reversed path corresponds to a node in the reversed flamegraph
      for (let i = 0; i < path.length; i++) {
        const key = path.slice(0, i + 1).join(";");
        const entry = byStack.get(key) || { self: 0, total: 0 };
        entry.total += w;
        if (i === 0) entry.self += w;
        byStack.set(key, entry);
      }
    }
    return byStack;
  }

  const nodePath = new Array<string>(tree.length);
  function buildPath(nid: number): string {
    if (nodePath[nid] != null) return nodePath[nid];
    const name = frames[tree[nid].frame_id]?.name || "[unknown]";
    const parent = tree[nid].parent;
    nodePath[nid] = parent != null && parent !== 0 ? buildPath(parent) + ";" + name : name;
    return nodePath[nid];
  }

  const byStack = new Map<string, { self: number; total: number }>();
  for (let nid = 0; nid < tree.length; nid++) {
    const path = buildPath(nid);
    const entry = byStack.get(path) || { self: 0, total: 0 };
    entry.self += nodeSelf.get(nid) || 0;
    entry.total += nodeTotal.get(nid) || 0;
    byStack.set(path, entry);
  }
  return byStack;
}

/** Get the full stack path for a FlamegraphNode by walking up parents */
export function getNodeStackPath(node: FlamegraphNode): string {
  const parts: string[] = [];
  let cur: FlamegraphNode | null = node;
  while (cur && cur.depth > 0) {
    parts.push(cur.name);
    cur = cur.parent;
  }
  return parts.reverse().join(";");
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

/** Build a flamegraph tree. If reverse, build inverted tree (leaf → root callers). */
export function buildFlamegraph(
  analytics: Profile,
  blockStart: number,
  blockEnd: number,
  filterRe: RegExp | null,
  reverse: boolean,
): FlamegraphNode | null {
  const tree = analytics.context_tree;
  const frames = analytics.frame_map?.frame_id_to_frame;
  if (!tree || !frames || tree.length === 0) return null;

  const nodeSelf = computeNodeSelfSamples(analytics, blockStart, blockEnd);

  const stackMatched = new Array<boolean>(tree.length).fill(!filterRe);
  if (filterRe) {
    function walk(nid: number, ancestorMatched: boolean) {
      const name = frames[tree[nid].frame_id]?.name || "";
      const matched = ancestorMatched || filterRe!.test(name);
      stackMatched[nid] = matched;
      for (const cid of Object.values(tree[nid].children)) walk(cid, matched);
    }
    walk(0, false);
  }

  if (reverse) return buildReverseFlamegraph(analytics, nodeSelf, stackMatched);
  return buildForwardFlamegraph(analytics, nodeSelf, stackMatched);
}

function buildForwardFlamegraph(
  analytics: Profile,
  nodeSelf: Map<number, number>,
  stackMatched: boolean[],
): FlamegraphNode | null {
  const tree = analytics.context_tree;
  const frames = analytics.frame_map.frame_id_to_frame;

  const totalSamples = new Map<number, number>();
  function computeTotal(nodeId: number): number {
    if (totalSamples.has(nodeId)) return totalSamples.get(nodeId)!;
    const selfPart = stackMatched[nodeId] ? nodeSelf.get(nodeId) || 0 : 0;
    let total = selfPart;
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
    const self_ = stackMatched[nodeId] ? nodeSelf.get(nodeId) || 0 : 0;
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

function buildReverseFlamegraph(
  analytics: Profile,
  nodeSelf: Map<number, number>,
  stackMatched: boolean[],
): FlamegraphNode | null {
  const tree = analytics.context_tree;
  const frames = analytics.frame_map.frame_id_to_frame;

  const stacks: { path: string[]; weight: number }[] = [];
  for (let nid = 0; nid < tree.length; nid++) {
    const w = nodeSelf.get(nid) || 0;
    if (w === 0 || !stackMatched[nid]) continue;
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
}

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
    return [
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
  }, [blockWidthMs]);

  const initialZoom = React.useMemo(() => {
    for (let i = 0; i < zoomLevels.length; i++) {
      const z = zoomLevels[i];
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
    setSelection(([s, e]) => {
      if (numBlocks === 0) return [0, 0];
      const alignedStart = Math.floor(s / groupSize) * groupSize;
      const alignedEnd = Math.min(numBlocks, Math.ceil(e / groupSize) * groupSize);
      // Reset to full range when the previous selection has gone out of bounds
      // OR realigning has produced an empty range (e.g. switching from an empty
      // profile back to a non-empty one would otherwise leave selection at [0,0]
      // and render an empty flamegraph).
      if (alignedStart >= numBlocks || alignedEnd <= alignedStart) return [0, numBlocks];
      if (alignedStart === s && alignedEnd === e) return [s, e];
      return [alignedStart, alignedEnd];
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
