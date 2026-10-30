import React from "react";
import { useReportState } from "../../ReportStateProvider";
import { FlamegraphNode, frameMatches, matchBaselineStacks, strippedFrameName, useHorizontalZoom } from "./utils";
import {
  FRAME_TYPE_LABELS,
  defaultFlamegraphColor,
  DIFF_EPSILON_PP,
  diffFlamegraphColor,
  diffTextColor,
  getFrameType,
  getThemeColors,
} from "./colors";

const FLAMEGRAPH_ROW_HEIGHT = 18;
const MIN_RENDER_WIDTH_PX = 0.5;

interface Props {
  readonly flatNodes: FlamegraphNode[];
  readonly flamegraphRoot: FlamegraphNode | null;
  readonly containerWidth: number;
  // Baseline flamegraph while diffing; null when its selection has no samples.
  readonly baselineRoot: FlamegraphNode | null;
  readonly diffing: boolean;
  readonly searchRe: RegExp | null;
  readonly zoomedNode: FlamegraphNode | null;
  readonly setZoomedNode: (node: FlamegraphNode | null) => void;
  readonly setTooltip: (t: { x: number; y: number; text: string } | null) => void;
}

export function FlamegraphCanvas({
  flatNodes,
  flamegraphRoot,
  containerWidth,
  baselineRoot,
  diffing,
  searchRe,
  zoomedNode,
  setZoomedNode,
  setTooltip,
}: Props) {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);
  const canvasRef = React.useRef<HTMLCanvasElement>(null);

  const maxDepth = React.useMemo(() => {
    if (flatNodes.length === 0) return 1;
    const zoomX = zoomedNode?.x ?? 0;
    const zoomW = zoomedNode?.w ?? Infinity;
    const zoomDepth = zoomedNode?.depth ?? 0;
    let max = 0;
    for (const n of flatNodes) {
      if (n.depth < zoomDepth) continue;
      if (n.x + n.w <= zoomX || n.x >= zoomX + zoomW) continue;
      if (n.depth > max) max = n.depth;
    }
    return max + 1;
  }, [flatNodes, zoomedNode]);

  const flamegraphHeight = maxDepth * FLAMEGRAPH_ROW_HEIGHT;

  // Horizontal zoom/pan within the zoomed node (or root), reset whenever that changes.
  const baseX = zoomedNode ? zoomedNode.x : 0;
  const baseW = zoomedNode ? zoomedNode.w : (flamegraphRoot?.totalSamples ?? 0);
  const { viewX, viewW, resetView, onMouseDown, consumeDrag } = useHorizontalZoom(canvasRef, baseX, baseW);
  React.useEffect(() => resetView(), [zoomedNode, flamegraphRoot, resetView]);

  // Precompute per-node " (baseline, compare)"
  const nodeDiff = React.useMemo(() => {
    if (!diffing || !flamegraphRoot) return null;
    const counts = matchBaselineStacks(flamegraphRoot, baselineRoot);
    const curTotal = flamegraphRoot.totalSamples || 1;
    const baseTotal = baselineRoot?.totalSamples || 1;
    const fmt = new Intl.NumberFormat();
    const suffixes = new Map<FlamegraphNode, string>();
    const deltas = new Map<FlamegraphNode, number>();
    let maxAbs = 0;
    for (const node of flatNodes) {
      if (node.depth === 0) continue;
      const [base, cur] = counts.get(node) ?? [0, 0];
      suffixes.set(node, ` (${fmt.format(base)}, ${fmt.format(cur)})`);
      const d = cur / curTotal - base / baseTotal;
      // Same noise floor as Top Functions; d is a fraction, the floor is in percentage points.
      if (Math.abs(d) * 100 < DIFF_EPSILON_PP) continue;
      deltas.set(node, d);
      if (Math.abs(d) > maxAbs) maxAbs = Math.abs(d);
    }
    let normalized: Map<FlamegraphNode, number> | null = null;
    if (maxAbs > 0) {
      normalized = new Map();
      for (const [node, d] of deltas) normalized.set(node, d / maxAbs);
    }
    return { suffixes, deltas: normalized };
  }, [flatNodes, flamegraphRoot, baselineRoot, diffing]);
  const nodeDeltaMap = nodeDiff?.deltas ?? null;
  const labelSuffixes = nodeDiff?.suffixes ?? null;

  const nodesByDepth = React.useMemo(() => {
    const map = new Map<number, FlamegraphNode[]>();
    for (const n of flatNodes) {
      const arr = map.get(n.depth);
      if (arr) arr.push(n);
      else map.set(n.depth, [n]);
    }
    return map;
  }, [flatNodes]);

  const scrollRef = React.useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = React.useState(0);
  const [viewportH, setViewportH] = React.useState(0);
  React.useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    let frame = 0;
    const update = () => {
      frame = 0;
      setViewportH(el.clientHeight);
      setScrollTop(el.scrollTop);
    };
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update);
    };
    update();
    el.addEventListener("scroll", onScroll, { passive: true });
    const resize = new ResizeObserver(onScroll);
    resize.observe(el);
    return () => {
      el.removeEventListener("scroll", onScroll);
      resize.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);
  const canvasHeight = Math.min(flamegraphHeight, viewportH);
  const firstRow = Math.floor(scrollTop / FLAMEGRAPH_ROW_HEIGHT);
  const lastRow = Math.ceil((scrollTop + canvasHeight) / FLAMEGRAPH_ROW_HEIGHT);

  // --- Draw flamegraph ---
  React.useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = containerWidth;
    const h = canvasHeight;
    const pixelW = Math.round(w * dpr);
    const pixelH = Math.round(h * dpr);
    if (canvas.width !== pixelW || canvas.height !== pixelH) {
      canvas.width = pixelW;
      canvas.height = pixelH;
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
    }
    const ctx = canvas.getContext("2d")!;
    // Draw in flamegraph coordinates, shifted up by the scroll position.
    ctx.setTransform(dpr, 0, 0, dpr, 0, -scrollTop * dpr);
    ctx.clearRect(0, scrollTop, w, h);

    if (!flamegraphRoot || flamegraphRoot.totalSamples === 0) return;

    const zoomDepth = zoomedNode ? zoomedNode.depth : 0;
    const scale = viewW > 0 ? w / viewW : 0;

    // Collect ancestor chain from zoomed node to root
    const ancestors: FlamegraphNode[] = [];
    if (zoomedNode) {
      let cur = zoomedNode.parent;
      while (cur) {
        ancestors.push(cur);
        cur = cur.parent;
      }
    }

    const drawNode = (node: FlamegraphNode, x: number, nodeW: number, y: number, dimmed: boolean) => {
      if (nodeW < MIN_RENDER_WIDTH_PX) return;

      let fill: string;
      let textColor = theme.textSet;
      let unchanged = false;
      if (node.depth === 0) {
        fill = theme.bgRoot;
      } else if (diffing) {
        const delta = nodeDeltaMap?.get(node) || 0;
        const diff = diffFlamegraphColor(delta);
        unchanged = diff === null;
        fill = diff ?? "#fff";
        if (diff) textColor = diffTextColor(delta);
      } else {
        fill = defaultFlamegraphColor(node.name);
      }
      ctx.fillStyle = fill;
      ctx.fillRect(x, y, nodeW - 0.5, FLAMEGRAPH_ROW_HEIGHT - 1);
      // Outline white frames so they stay distinct from a light background.
      if (unchanged) {
        ctx.strokeStyle = theme.borderSubtle;
        ctx.lineWidth = 0.5;
        ctx.strokeRect(x + 0.5, y + 0.5, Math.max(0, nodeW - 1.5), FLAMEGRAPH_ROW_HEIGHT - 2);
      }

      if (dimmed) {
        ctx.fillStyle = theme.overlayDimmed;
        ctx.fillRect(x, y, nodeW - 0.5, FLAMEGRAPH_ROW_HEIGHT - 1);
      }

      // Search highlight overlay
      if (searchRe && node.depth > 0 && frameMatches(searchRe, node.name)) {
        ctx.fillStyle = "rgba(64,178,255,0.55)";
        ctx.fillRect(x, y, nodeW - 0.5, FLAMEGRAPH_ROW_HEIGHT - 1);
        ctx.strokeStyle = theme.accent;
        ctx.lineWidth = 1;
        ctx.strokeRect(x + 0.5, y + 0.5, Math.max(0, nodeW - 1.5), FLAMEGRAPH_ROW_HEIGHT - 2);
      }

      // Label only the on-screen part, so frames panned partly out of view keep a label.
      const labelX = Math.max(x, 0);
      const labelW = Math.min(x + nodeW, w) - labelX;
      if (labelW > 30) {
        ctx.fillStyle = textColor;
        const maxChars = Math.floor((labelW - 6) / 6.6);
        // append " (baseline, compare)" sample counts
        let suffix = labelSuffixes?.get(node) ?? "";
        if (suffix.length + 2 > maxChars) suffix = "";
        const nameChars = maxChars - suffix.length;
        // Display without the "_[k]"-style type suffix; the type is shown by color and in the tooltip.
        const displayName = strippedFrameName(node.name);
        const name = displayName.length > nameChars ? displayName.slice(0, nameChars - 1) + "…" : displayName;
        const label = name + suffix;
        ctx.fillText(label, labelX + 3, y + 13);
      }
    };

    // Setting the font is costly, so do it once rather than per label.
    ctx.font = "11px monospace";
    ctx.textAlign = "left";

    // Draw ancestors full-width (dimmed)
    for (const anc of ancestors) {
      if (anc.depth < firstRow || anc.depth > lastRow) continue;
      drawNode(anc, 0, w, anc.depth * FLAMEGRAPH_ROW_HEIGHT, true);
    }

    // Draw subtree (zoomed-in region), only the visible rows
    for (let depth = Math.max(firstRow, zoomDepth); depth <= lastRow; depth++) {
      for (const node of nodesByDepth.get(depth) ?? []) {
        if (node.x + node.w <= viewX || node.x >= viewX + viewW) continue;
        const x = (node.x - viewX) * scale;
        const nodeW = node.w * scale;
        drawNode(node, x, nodeW, depth * FLAMEGRAPH_ROW_HEIGHT, false);
      }
    }
  }, [
    nodesByDepth,
    firstRow,
    lastRow,
    flamegraphRoot,
    containerWidth,
    canvasHeight,
    scrollTop,
    theme,
    diffing,
    nodeDeltaMap,
    labelSuffixes,
    searchRe,
    zoomedNode,
    viewX,
    viewW,
  ]);

  // --- Find node at canvas position ---
  const findNodeAt = React.useCallback(
    (mx: number, my: number, canvasWidth: number): FlamegraphNode | null => {
      if (!flamegraphRoot || flamegraphRoot.totalSamples === 0) return null;
      const zoomDepth = zoomedNode ? zoomedNode.depth : 0;
      if (viewW <= 0) return null;
      const depth = Math.floor(my / FLAMEGRAPH_ROW_HEIGHT);
      if (depth < zoomDepth && zoomedNode) {
        let cur = zoomedNode.parent;
        while (cur) {
          if (cur.depth === depth) return cur;
          cur = cur.parent;
        }
        return null;
      }
      const scale = canvasWidth / viewW;
      const nodesAtDepth = nodesByDepth.get(depth);
      if (!nodesAtDepth) return null;
      for (const node of nodesAtDepth) {
        const x = (node.x - viewX) * scale;
        const nw = node.w * scale;
        if (mx >= x && mx < x + nw) return node;
      }
      return null;
    },
    [flamegraphRoot, nodesByDepth, zoomedNode, viewX, viewW],
  );

  const onMouseMove = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!flamegraphRoot || flamegraphRoot.totalSamples === 0) {
        setTooltip(null);
        return;
      }
      const rect = e.currentTarget.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top + (scrollRef.current?.scrollTop ?? 0);
      const rootTotal = flamegraphRoot.totalSamples;
      const node = findNodeAt(mx, my, rect.width);
      if (node) {
        const pct = ((node.totalSamples / rootTotal) * 100).toFixed(2);
        const typeLabel = node.depth > 0 ? `${FRAME_TYPE_LABELS[getFrameType(node.name)]} | ` : "";
        const panel = (e.currentTarget.offsetParent ?? e.currentTarget).getBoundingClientRect();
        setTooltip({
          x: e.clientX - panel.left,
          y: e.clientY - panel.top,
          text: `${strippedFrameName(node.name)}\n${typeLabel}${node.totalSamples} samples (${pct}%) | self: ${node.selfSamples}\n(click to zoom | ctrl+wheel: zoom | shift+wheel or drag: pan)`,
        });
      } else {
        setTooltip(null);
      }
    },
    [flamegraphRoot, findNodeAt, setTooltip],
  );

  const onClick = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (consumeDrag()) return;
      const rect = e.currentTarget.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top + (scrollRef.current?.scrollTop ?? 0);
      const node = findNodeAt(mx, my, rect.width);
      if (!node) return;
      if (zoomedNode && node === zoomedNode) {
        setZoomedNode(null);
      } else if (!zoomedNode && node.depth === 0) {
        setZoomedNode(null);
        resetView();
      } else {
        setZoomedNode(node);
      }
    },
    [findNodeAt, zoomedNode, setZoomedNode, consumeDrag, resetView],
  );

  const onMouseLeave = React.useCallback(() => setTooltip(null), [setTooltip]);

  return (
    <div ref={scrollRef} style={{ marginTop: 8, overflowX: "hidden", overflowY: "auto", maxHeight: "60vh" }}>
      {/* Spacer gives the scroll box the full flamegraph height. It stays unpositioned so the
          panel remains the canvas's offsetParent, which the tooltip position relies on. */}
      <div style={{ height: flamegraphHeight }}>
        <canvas
          ref={canvasRef}
          style={{
            display: "block",
            position: "sticky",
            top: 0,
            width: "100%",
            maxWidth: "100%",
            cursor: "pointer",
          }}
          onMouseDown={onMouseDown}
          onMouseMove={onMouseMove}
          onMouseLeave={onMouseLeave}
          onClick={onClick}
        />
      </div>
    </div>
  );
}

export { FLAMEGRAPH_ROW_HEIGHT };
