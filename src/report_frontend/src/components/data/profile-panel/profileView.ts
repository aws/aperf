import { Profile } from "../../../definitions/types";
import { RUNS } from "../../../definitions/data-config";
import { FrameType, getFrameType } from "./colors";
import {
  aggregateByFrame,
  blockTotal,
  buildFlamegraph,
  computeNodeSelfSamples,
  filteredStacks,
  flattenFlamegraph,
  FlamegraphNode,
  frameKey,
  searchMatchStats,
  topFunctionName,
} from "./utils";

export interface ProfileViewInputs {
  readonly profile: Profile;
  /** The base run's profile, diffed against while `showDiff` is on. */
  readonly baseline: Profile | undefined;
  readonly filterRe: RegExp | null;
  readonly searchRe: RegExp | null;
  readonly reverse: boolean;
  readonly showDiff: boolean;
  readonly selection: [number, number];
  /** A ctrl-drag selection in this run to diff against; takes precedence over `baseline`. */
  readonly baselineSelection: [number, number] | null;
  /** The base run's selection, which limits `baseline`. */
  readonly baseRunSelection: [number, number] | null;
  readonly groupSize: number;
  readonly numDataCells: number;
}

/** Everything the heatmap, info bar, flamegraph, and Top Functions table render. */
export interface ProfileView {
  /** Filtered samples per heatmap cell (`groupSize` blocks each), from the first block. */
  readonly cellTotals: number[];
  /** The largest cell total, at least 1; the top of the heatmap color scale. */
  readonly maxCellTotal: number;
  /** Filtered samples in the selected blocks, shown in the info bar. */
  readonly selSamples: number;
  /** The selection's flamegraph (reversed with `reverse`); null when it has no samples. */
  readonly flamegraphRoot: FlamegraphNode | null;
  /** Every node of `flamegraphRoot`, parents before children, for drawing and hit-testing. */
  readonly flatNodes: FlamegraphNode[];
  /** The diff baseline's flamegraph; null when not diffing or the baseline has no samples. */
  readonly baselineRoot: FlamegraphNode | null;
  /** Whether there is a baseline: a ctrl-drag selection, or Diff on in a later run. */
  readonly diffing: boolean;
  /** Flamegraph frames matching Search, and the samples under them (nested matches count once). */
  readonly searchMatches: { count: number; totalSamples: number };
  /** Top Functions rows for the selection, sorted by self samples. */
  readonly framesList: FrameEntry[];
  /** The baseline's Top Functions stats, keyed by `frameKey`; null when not diffing. */
  readonly baselineByFrame: Map<string, BaselineFrameStats> | null;
}

/** The samples a diff compares against: a profile and its per-node self samples. */
interface BaselineSamples {
  target: Profile;
  nodeSelf: Map<number, number>;
}

/** One row in the Top Functions table. */
export interface FrameEntry {
  name: string;
  type: FrameType;
  self: number;
  total: number;
  selfPercent: number;
  totalPercent: number;
}

/** Baseline sample counts for one frame, plus its share used by the diff tint. */
export interface BaselineFrameStats {
  self: number;
  total: number;
  totalPercent: number;
}

/**
 * Create the deriveProfileView function for one panel. So data affected by search/filter and
 * other settings are consistent across consumers.
 */
export function createProfileViewDerivedValues(): (inputs: ProfileViewInputs) => ProfileView {
  const filterMatched = memoLast(sharedFilteredStacks);
  const blockTotals = memoLast((blocks: Profile["blocks"], matched: boolean[] | null) =>
    (blocks ?? []).map((block) => blockTotal(block, matched)),
  );
  const cells = memoLast(heatmapCells);
  const selSamples = memoLast(sumRange);
  const nodeSelf = memoLast((profile: Profile, matched: boolean[] | null, [start, end]: [number, number]) =>
    sharedNodeSelf(profile, start, end, matched),
  );
  const flamegraph = memoLast(sharedFlamegraph);
  const flatNodes = memoLast((root: FlamegraphNode | null) => (root ? flattenFlamegraph(root) : []));
  const topFunctions = memoLast(topFunctionRows);
  const baselineSamples = memoLast(selectBaselineSamples);
  const baselineFrames = memoLast(baselineFrameStats);
  // Built the same way as the current flamegraph so their stacks can be matched by
  // frame name (see matchBaselineStacks).
  const baselineFlamegraph = memoLast((base: BaselineSamples | null, reverse: boolean) =>
    base ? sharedFlamegraph(base.target, base.nodeSelf, reverse) : null,
  );
  const search = memoLast((root: FlamegraphNode | null, searchRe: RegExp | null) =>
    searchRe && root ? searchMatchStats(root, searchRe) : { count: 0, totalSamples: 0 },
  );

  return function deriveProfileView(inputs: ProfileViewInputs): ProfileView {
    const { profile, filterRe, selection, reverse } = inputs;
    const matched = filterMatched(profile, filterRe);
    const totals = blockTotals(profile.blocks, matched);
    const curSelf = nodeSelf(profile, matched, selection);
    const root = flamegraph(profile, curSelf, reverse);
    const base = baselineSamples(
      profile,
      matched,
      inputs.baseline,
      filterRe,
      inputs.showDiff,
      inputs.baselineSelection,
      inputs.baseRunSelection,
    );
    return {
      ...cells(totals, inputs.groupSize, inputs.numDataCells),
      selSamples: selSamples(totals, selection),
      flamegraphRoot: root,
      flatNodes: flatNodes(root),
      baselineRoot: baselineFlamegraph(base, reverse),
      diffing: base !== null,
      searchMatches: search(root, inputs.searchRe),
      framesList: topFunctions(profile, curSelf),
      baselineByFrame: baselineFrames(base),
    };
  };
}

/** Wrap `fn` to return its last result while it is called again with the same arguments. */
function memoLast<A extends unknown[], R>(fn: (...args: A) => R): (...args: A) => R {
  let last: { args: A; result: R } | null = null;
  return (...args: A) => {
    if (last && args.every((arg, i) => Object.is(arg, last!.args[i]))) return last.result;
    last = { args, result: fn(...args) };
    return last.result;
  };
}

const sharedCaches: Map<Profile, unknown>[] = [];

/** Drop all shared results, when the panel unmounts. */
export function clearSharedProfileResults(): void {
  for (const cache of sharedCaches) cache.clear();
}

function memoPerProfile<A extends unknown[], R>(
  fn: (profile: Profile, ...args: A) => R,
  size = 2,
): (profile: Profile, ...args: A) => R {
  const cache = new Map<Profile, { args: A; result: R }[]>();
  sharedCaches.push(cache);
  return (profile: Profile, ...args: A) => {
    const entries = cache.get(profile) ?? [];
    const hit = entries.find((entry) => args.every((arg, i) => Object.is(arg, entry.args[i])));
    const result = hit ? hit.result : fn(profile, ...args);
    // Re-insert so the Map's order is least to most recently used.
    cache.delete(profile);
    cache.set(profile, hit ? entries : [{ args, result }, ...entries].slice(0, size));
    while (cache.size > Math.max(1, RUNS.length)) cache.delete(cache.keys().next().value!);
    return result;
  };
}

const sharedFilteredStacks = memoPerProfile(filteredStacks);
const sharedNodeSelf = memoPerProfile(computeNodeSelfSamples);
const sharedFlamegraph = memoPerProfile(buildFlamegraph);
const sharedAggregateByFrame = memoPerProfile(aggregateByFrame);

function heatmapCells(blockTotals: number[], groupSize: number, numDataCells: number) {
  let cellTotals = blockTotals;
  if (groupSize !== 1) {
    cellTotals = new Array<number>(numDataCells).fill(0);
    for (let i = 0; i < blockTotals.length; i++) {
      cellTotals[Math.floor(i / groupSize)] += blockTotals[i];
    }
  }
  return { cellTotals, maxCellTotal: cellTotals.reduce((max, v) => (v > max ? v : max), 1) };
}

function sumRange(blockTotals: number[], [start, end]: [number, number]): number {
  let sum = 0;
  for (let i = start; i < end; i++) sum += blockTotals[i] || 0;
  return sum;
}

function selectBaselineSamples(
  profile: Profile,
  matched: boolean[] | null,
  baseline: Profile | undefined,
  filterRe: RegExp | null,
  showDiff: boolean,
  baselineSelection: [number, number] | null,
  baseRunSelection: [number, number] | null,
): BaselineSamples | null {
  if (baselineSelection) {
    return {
      target: profile,
      nodeSelf: sharedNodeSelf(profile, baselineSelection[0], baselineSelection[1], matched),
    };
  }
  if (!showDiff || !baseline || baseline === profile) return null;
  const start = baseRunSelection ? baseRunSelection[0] : 0;
  const end = baseRunSelection ? baseRunSelection[1] : baseline.blocks.length;
  return {
    target: baseline,
    nodeSelf: sharedNodeSelf(baseline, start, end, sharedFilteredStacks(baseline, filterRe)),
  };
}

// The table's diff tint uses `totalPercent` — comparing raw counts would be
// misleading when the baseline and current selections cover different
// durations. Raw `self` / `total` are only displayed alongside the values.
function baselineFrameStats(base: BaselineSamples | null): Map<string, BaselineFrameStats> | null {
  if (!base) return null;
  const { target, nodeSelf } = base;

  let rootTotal = 0;
  for (const v of nodeSelf.values()) rootTotal += v;
  if (rootTotal === 0) rootTotal = 1;

  const byFrame = sharedAggregateByFrame(target, nodeSelf);
  const result = new Map<string, BaselineFrameStats>();
  for (const [key, stats] of byFrame) {
    result.set(key, { ...stats, totalPercent: (stats.total / rootTotal) * 100 });
  }
  return result;
}

// Each row carries the display name + type (cached once here, derived from
// the raw frame name in `frame_map`); `frameKey(name, type)` is the row id.
function topFunctionRows(profile: Profile, nodeSelf: Map<number, number>): FrameEntry[] {
  const frames = profile.frame_map?.frame_id_to_frame;
  if (!frames) return [];

  const byFrame = sharedAggregateByFrame(profile, nodeSelf);

  let rootTotal = 0;
  for (const v of nodeSelf.values()) rootTotal += v;
  if (rootTotal === 0) rootTotal = 1;

  const rows: FrameEntry[] = [];
  const seen = new Set<string>();
  for (const frame of frames) {
    const name = topFunctionName(frame.name);
    const type = getFrameType(frame.name);
    const key = frameKey(name, type);
    if (seen.has(key)) continue;
    seen.add(key);
    const stats = byFrame.get(key);
    if (!stats || (stats.self === 0 && stats.total === 0)) continue;
    rows.push({
      name,
      type,
      self: stats.self,
      total: stats.total,
      selfPercent: (stats.self / rootTotal) * 100,
      totalPercent: (stats.total / rootTotal) * 100,
    });
  }
  return rows.sort((a, b) => b.self - a.self);
}
