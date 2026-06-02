import React from "react";
import { useReportState } from "../../ReportStateProvider";
import { FlamegraphNode, getNodeStackPath } from "./utils";
import { defaultFlamegraphColor, diffFlamegraphColor, getThemeColors } from "./colors";

const FLAMEGRAPH_ROW_HEIGHT = 18;
const MIN_RENDER_WIDTH_PX = 0.5;

interface Props {
  readonly flatNodes: FlamegraphNode[];
  readonly flamegraphRoot: FlamegraphNode | null;
  readonly containerWidth: number;
  readonly baselineByStack: Map<string, { self: number; total: number }> | null;
  readonly currentByStack: Map<string, { self: number; total: number }> | null;
  readonly searchRe: RegExp | null;
  readonly zoomedNode: FlamegraphNode | null;
  readonly setZoomedNode: (node: FlamegraphNode | null) => void;
  readonly setTooltip: (t: { x: number; y: number; text: string } | null) => void;
  readonly heatmapGridHeight: number;
  readonly axisHeight: number;
}

export function FlamegraphCanvas({
  flatNodes,
  flamegraphRoot,
  containerWidth,
  baselineByStack,
  currentByStack,
  searchRe,
  zoomedNode,
  setZoomedNode,
  setTooltip,
  heatmapGridHeight,
  axisHeight,
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

  // Precompute normalized deltas per node (difffolded.pl: delta = cur - base, normalize by max)
  const nodeDeltaMap = React.useMemo(() => {
    if (!baselineByStack || !currentByStack) return null;
    const deltas = new Map<FlamegraphNode, number>();
    let maxAbs = 0;
    for (const node of flatNodes) {
      if (node.depth === 0) continue;
      const path = getNodeStackPath(node);
      const cur = currentByStack.get(path)?.total || 0;
      const base = baselineByStack.get(path)?.total || 0;
      const d = cur - base;
      deltas.set(node, d);
      if (Math.abs(d) > maxAbs) maxAbs = Math.abs(d);
    }
    if (maxAbs === 0) return null;
    const normalized = new Map<FlamegraphNode, number>();
    for (const [node, d] of deltas) normalized.set(node, d / maxAbs);
    return normalized;
  }, [flatNodes, baselineByStack, currentByStack]);

  // --- Draw flamegraph ---
  React.useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = containerWidth;
    const h = flamegraphHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;
    const ctx = canvas.getContext("2d")!;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    if (!flamegraphRoot || flamegraphRoot.totalSamples === 0) return;

    const rootTotal = flamegraphRoot.totalSamples;

    const zoomX = zoomedNode ? zoomedNode.x : 0;
    const zoomW = zoomedNode ? zoomedNode.w : rootTotal;
    const zoomDepth = zoomedNode ? zoomedNode.depth : 0;
    const scale = zoomW > 0 ? w / zoomW : 0;

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
      if (node.depth === 0) {
        fill = theme.bgRoot;
      } else if (nodeDeltaMap) {
        // diffFlamegraphColor returns "transparent" for ~zero deltas; canvases
        // can't paint transparent fills usefully, so fall back to the
        // frame-type color so unchanged frames stay visible at their base color.
        const diff = diffFlamegraphColor(nodeDeltaMap.get(node) || 0);
        fill = diff === "transparent" ? defaultFlamegraphColor(node.name) : diff;
      } else {
        fill = defaultFlamegraphColor(node.name);
      }
      ctx.fillStyle = fill;
      ctx.fillRect(x, y, nodeW - 0.5, FLAMEGRAPH_ROW_HEIGHT - 1);

      if (dimmed) {
        ctx.fillStyle = theme.overlayDimmed;
        ctx.fillRect(x, y, nodeW - 0.5, FLAMEGRAPH_ROW_HEIGHT - 1);
      }

      // Search highlight overlay
      if (searchRe && node.depth > 0 && searchRe.test(node.name)) {
        ctx.fillStyle = "rgba(64,178,255,0.55)";
        ctx.fillRect(x, y, nodeW - 0.5, FLAMEGRAPH_ROW_HEIGHT - 1);
        ctx.strokeStyle = theme.accent;
        ctx.lineWidth = 1;
        ctx.strokeRect(x + 0.5, y + 0.5, Math.max(0, nodeW - 1.5), FLAMEGRAPH_ROW_HEIGHT - 2);
      }

      if (nodeW > 30) {
        ctx.fillStyle = theme.textSet;
        ctx.font = "11px monospace";
        ctx.textAlign = "left";
        const maxChars = Math.floor((nodeW - 6) / 6.6);
        const label = node.name.length > maxChars ? node.name.slice(0, maxChars - 1) + "…" : node.name;
        ctx.fillText(label, x + 3, y + 13);
      }
    };

    // Draw ancestors full-width (dimmed)
    for (const anc of ancestors) {
      drawNode(anc, 0, w, anc.depth * FLAMEGRAPH_ROW_HEIGHT, true);
    }

    // Draw subtree (zoomed-in region)
    for (const node of flatNodes) {
      if (node.depth < zoomDepth) continue;
      if (node.x + node.w <= zoomX || node.x >= zoomX + zoomW) continue;
      const x = (node.x - zoomX) * scale;
      const nodeW = node.w * scale;
      const y = node.depth * FLAMEGRAPH_ROW_HEIGHT;
      drawNode(node, x, nodeW, y, false);
    }
  }, [flatNodes, flamegraphRoot, containerWidth, flamegraphHeight, theme, nodeDeltaMap, searchRe, zoomedNode]);

  const nodesByDepth = React.useMemo(() => {
    const map = new Map<number, FlamegraphNode[]>();
    for (const n of flatNodes) {
      const arr = map.get(n.depth);
      if (arr) arr.push(n);
      else map.set(n.depth, [n]);
    }
    return map;
  }, [flatNodes]);

  // --- Find node at canvas position ---
  const findNodeAt = React.useCallback(
    (mx: number, my: number, canvasWidth: number): FlamegraphNode | null => {
      if (!flamegraphRoot || flamegraphRoot.totalSamples === 0) return null;
      const zoomX = zoomedNode ? zoomedNode.x : 0;
      const zoomW = zoomedNode ? zoomedNode.w : flamegraphRoot.totalSamples;
      const zoomDepth = zoomedNode ? zoomedNode.depth : 0;
      if (zoomW <= 0) return null;
      const depth = Math.floor(my / FLAMEGRAPH_ROW_HEIGHT);
      if (depth < zoomDepth && zoomedNode) {
        let cur = zoomedNode.parent;
        while (cur) {
          if (cur.depth === depth) return cur;
          cur = cur.parent;
        }
        return null;
      }
      const scale = canvasWidth / zoomW;
      const nodesAtDepth = nodesByDepth.get(depth);
      if (!nodesAtDepth) return null;
      for (const node of nodesAtDepth) {
        const x = (node.x - zoomX) * scale;
        const nw = node.w * scale;
        if (mx >= x && mx < x + nw) return node;
      }
      return null;
    },
    [flamegraphRoot, nodesByDepth, zoomedNode],
  );

  const onMouseMove = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!flamegraphRoot || flamegraphRoot.totalSamples === 0) {
        setTooltip(null);
        return;
      }
      const rect = e.currentTarget.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const rootTotal = flamegraphRoot.totalSamples;
      const node = findNodeAt(mx, my, rect.width);
      if (node) {
        const pct = ((node.totalSamples / rootTotal) * 100).toFixed(2);
        setTooltip({
          x: mx,
          y: my + heatmapGridHeight + axisHeight + 8,
          text: `${node.name}\n${node.totalSamples} samples (${pct}%) | self: ${node.selfSamples}\n(click to zoom)`,
        });
      } else {
        setTooltip(null);
      }
    },
    [flamegraphRoot, findNodeAt, heatmapGridHeight, axisHeight, setTooltip],
  );

  const onClick = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const rect = e.currentTarget.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const node = findNodeAt(mx, my, rect.width);
      if (!node) return;
      if (zoomedNode && node === zoomedNode) {
        setZoomedNode(null);
      } else if (!zoomedNode && node.depth === 0) {
        setZoomedNode(null);
      } else {
        setZoomedNode(node);
      }
    },
    [findNodeAt, zoomedNode, setZoomedNode],
  );

  const onMouseLeave = React.useCallback(() => setTooltip(null), [setTooltip]);

  return (
    <div style={{ marginTop: 8, overflowX: "hidden", overflowY: "auto", maxHeight: "60vh" }}>
      <canvas
        ref={canvasRef}
        style={{ display: "block", width: "100%", maxWidth: "100%", cursor: "pointer" }}
        onMouseMove={onMouseMove}
        onMouseLeave={onMouseLeave}
        onClick={onClick}
      />
    </div>
  );
}

export { FLAMEGRAPH_ROW_HEIGHT };
