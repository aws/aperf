import React from "react";
import { useReportState } from "../../ReportStateProvider";
import { ZoomLevel } from "./utils";
import { heatmapColor, getThemeColors } from "./colors";
import { FlamegraphNode } from "./utils";

/**
 * HeatmapCanvas: renders the time-series heatmap as a canvas element.
 *  - Each cell represents one or more profiling blocks (depending on zoom level).
 *  - Cell color intensity = sample count normalized against the hottest cell.
 *  - Cells outside the active selection are dimmed.
 *  - Drag to select a time range; ctrl/cmd-drag to set a baseline range for diff.
 *  - Hover shows a tooltip with UTC time, offset, and sample count.
 *
 * HeatmapCanvasProps
 *   Grid geometry (from useHeatmapLayout):
 *      numBlocks — total raw profiling blocks
 *      numDataCells — blocks after grouping (cells with real data)
 *      totalCells — data cells + buffer cells (fills the grid evenly)
 *      bufferCellsBefore — empty cells before real data starts
 *      groupSize — how many raw blocks map to one cell
 *      rowsPerCol, numColumns, cellSize — grid dimensions and pixel size per cell
 *      heatmapGridHeight — total pixel height of the grid
 *      axisHeight — pixel height reserved for the time axis label
 *
 *   Data:
 *      cellTotals — sample count per grouped cell (drives color intensity)
 *      maxCellTotal — peak value for normalizing the color scale
 *
 *   Time:
 *      blockWidthMs — duration each raw block represents
 *      alignedStartTimeMs — wall-clock start snapped to block boundaries
 *      alignedStartMs — visible start after zoom applied
 *
 *   Zoom:
 *      zoom — current zoom level object (range, scale factor)
 *
 *   Selection/drag state (from useHeatmapSelection):
 *      selection — current selected block range [start, end)
 *      baselineSelection — ctrl-drag baseline range (for intra-profile diff)
 *      dragStart, dragEnd, dragIsBaseline — in-progress drag tracking
 *
 *   Callbacks:
 *      setSelection, setBaselineSelection, setDragStart, setDragEnd, setDragIsBaseline — state setters for interaction
 *      setTooltip — show/hide hover tooltip
 *      onDoubleClick — resets selection
 */
interface HeatmapCanvasProps {
  readonly numBlocks: number;
  readonly numDataCells: number;
  readonly totalCells: number;
  readonly bufferCellsBefore: number;
  readonly cellTotals: number[];
  readonly maxCellTotal: number;
  readonly groupSize: number;
  readonly rowsPerCol: number;
  readonly numColumns: number;
  readonly cellSize: number;
  readonly heatmapGridHeight: number;
  readonly blockWidthMs: number;
  readonly alignedStartTimeMs: number;
  readonly alignedStartMs: number;
  readonly zoom: ZoomLevel;
  readonly selection: [number, number];
  readonly baselineSelection: [number, number] | null;
  readonly dragStart: number | null;
  readonly dragEnd: number | null;
  readonly dragIsBaseline: boolean;
  readonly axisHeight: number;
  readonly setSelection: React.Dispatch<React.SetStateAction<[number, number]>>;
  readonly setBaselineSelection: (v: [number, number] | null) => void;
  readonly setDragStart: (v: number | null) => void;
  readonly setDragEnd: (v: number | null) => void;
  readonly setDragIsBaseline: (v: boolean) => void;
  readonly setTooltip: (t: { x: number; y: number; text: string } | null) => void;
  readonly onDoubleClick: () => void;
}

export function HeatmapCanvas({
  numBlocks,
  numDataCells,
  totalCells,
  bufferCellsBefore,
  cellTotals,
  maxCellTotal,
  groupSize,
  rowsPerCol,
  numColumns,
  cellSize,
  heatmapGridHeight,
  blockWidthMs,
  alignedStartTimeMs,
  alignedStartMs,
  zoom,
  selection,
  baselineSelection,
  dragStart,
  dragEnd,
  dragIsBaseline,
  axisHeight,
  setSelection,
  setBaselineSelection,
  setDragStart,
  setDragEnd,
  setDragIsBaseline,
  setTooltip,
  onDoubleClick,
}: HeatmapCanvasProps) {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);
  const canvasRef = React.useRef<HTMLCanvasElement>(null);

  // Draw the heatmap grid onto the canvas. Re-runs when any visual input changes.
  React.useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = numColumns * cellSize;
    const h = heatmapGridHeight + axisHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;
    const c = canvas.getContext("2d")!;
    c.scale(dpr, dpr);
    c.clearRect(0, 0, w, h);

    if (numBlocks === 0) return;

    const selStart = Math.min(selection[0], selection[1]);
    const selEnd = Math.max(selection[0], selection[1]);
    const selCellStart = Math.floor(selStart / groupSize) + bufferCellsBefore;
    const selCellEnd = Math.ceil(selEnd / groupSize) + bufferCellsBefore;
    const dragActive = dragStart !== null && dragEnd !== null;
    const dragCellMin = dragActive ? Math.floor(Math.min(dragStart!, dragEnd!) / groupSize) + bufferCellsBefore : -1;
    const dragCellMax = dragActive ? Math.floor(Math.max(dragStart!, dragEnd!) / groupSize) + bufferCellsBefore : -1;
    const bsCellStart = baselineSelection ? Math.floor(baselineSelection[0] / groupSize) + bufferCellsBefore : -1;
    const bsCellEnd = baselineSelection ? Math.ceil(baselineSelection[1] / groupSize) + bufferCellsBefore : -1;

    for (let i = 0; i < totalCells; i++) {
      const col = Math.floor(i / rowsPerCol);
      const row = i % rowsPerCol;
      const x = col * cellSize;
      const y = axisHeight + row * cellSize;

      const isBuffer = i < bufferCellsBefore || i >= bufferCellsBefore + numDataCells;
      if (isBuffer) {
        c.fillStyle = theme.bgBuffer;
        c.fillRect(x, y, cellSize, cellSize);
      } else {
        const dataIdx = i - bufferCellsBefore;
        const ratio = cellTotals[dataIdx] / maxCellTotal;
        c.fillStyle = heatmapColor(ratio);
        c.fillRect(x, y, cellSize, cellSize);
      }

      if (!isBuffer && (i < selCellStart || i >= selCellEnd)) {
        c.fillStyle = theme.overlayDim;
        c.fillRect(x, y, cellSize, cellSize);
      }

      if (dragActive && i >= dragCellMin && i <= dragCellMax) {
        c.fillStyle = dragIsBaseline ? theme.baselineDragOverlay : theme.dragOverlay;
        c.fillRect(x, y, cellSize, cellSize);
      }

      if (baselineSelection && i >= bsCellStart && i < bsCellEnd) {
        c.fillStyle = theme.baselineOverlay;
        c.fillRect(x, y, cellSize, cellSize);
      }
    }

    strokeCellRangeBorder(c, selCellStart, selCellEnd, rowsPerCol, cellSize, theme.selectionBorder, axisHeight);

    if (baselineSelection) {
      strokeCellRangeBorder(c, bsCellStart, bsCellEnd, rowsPerCol, cellSize, theme.baselineBorder, axisHeight);
    }

    c.fillStyle = theme.textSubtle;
    c.font = "11px monospace";
    const startLabel = new Date(alignedStartMs).toISOString().slice(11, 19);
    c.textAlign = "left";
    c.fillText(startLabel, 0, 12);
  }, [
    numBlocks,
    numDataCells,
    totalCells,
    bufferCellsBefore,
    cellTotals,
    maxCellTotal,
    groupSize,
    selection,
    baselineSelection,
    dragStart,
    dragEnd,
    dragIsBaseline,
    rowsPerCol,
    numColumns,
    cellSize,
    heatmapGridHeight,
    alignedStartMs,
    theme,
  ]);

  // Convert a mouse event position to the corresponding block index in the profile data.
  // Returns null if the cursor is over the axis, buffer area, or out of bounds.
  const getBlockAt = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>): number | null => {
      const rect = e.currentTarget.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top - axisHeight;
      if (y < 0 || y >= heatmapGridHeight) return null;
      const col = Math.floor(x / cellSize);
      const row = Math.floor(y / cellSize);
      if (col < 0 || col >= numColumns || row < 0 || row >= rowsPerCol) return null;
      const cellIdx = col * rowsPerCol + row;
      if (cellIdx < 0 || cellIdx >= totalCells) return null;
      if (cellIdx < bufferCellsBefore || cellIdx >= bufferCellsBefore + numDataCells) return null;
      const dataIdx = cellIdx - bufferCellsBefore;
      const blockIdx = dataIdx * groupSize;
      if (blockIdx >= numBlocks) return null;
      return blockIdx;
    },
    [
      cellSize,
      rowsPerCol,
      numColumns,
      totalCells,
      numBlocks,
      groupSize,
      heatmapGridHeight,
      bufferCellsBefore,
      numDataCells,
    ],
  );

  // Start a drag: record the starting block and whether ctrl/cmd is held (baseline mode)
  const onMouseDown = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const idx = getBlockAt(e);
      if (idx === null) return;
      setDragStart(idx);
      setDragEnd(idx);
      setDragIsBaseline(e.ctrlKey || e.metaKey);
    },
    [getBlockAt, setDragStart, setDragEnd, setDragIsBaseline],
  );

  // During drag: update drag end. Otherwise: show tooltip with cell info.
  const onMouseMove = React.useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const idx = getBlockAt(e);
      if (dragStart !== null && idx !== null) {
        setDragEnd(idx);
        return;
      }
      if (idx === null) {
        setTooltip(null);
        return;
      }
      const rect = e.currentTarget.getBoundingClientRect();
      const cellIdx = Math.floor(idx / groupSize);
      const startMs = idx * blockWidthMs;
      const cellDurationLabel = zoom.cellMs >= 1000 ? `${zoom.cellMs / 1000}s` : `${zoom.cellMs}ms`;
      const utcLabel = new Date(alignedStartTimeMs + startMs).toISOString().slice(11, 22) + " UTC";
      setTooltip({
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
        text: `${utcLabel}  t=${(startMs / 1000).toFixed(2)}s (window=${cellDurationLabel})  samples=${cellTotals[cellIdx] || 0}`,
      });
    },
    [dragStart, getBlockAt, groupSize, blockWidthMs, cellTotals, zoom, alignedStartTimeMs, setDragEnd, setTooltip],
  );

  // End drag: commit the dragged range as either the active selection or baseline selection
  const onMouseUp = React.useCallback(() => {
    if (dragStart !== null && dragEnd !== null) {
      const s = Math.min(dragStart, dragEnd);
      const e = Math.max(dragStart, dragEnd);
      const end = Math.min(numBlocks, e + groupSize);
      if (end > s) {
        if (dragIsBaseline) {
          setBaselineSelection([s, end]);
        } else {
          setSelection([s, end]);
        }
      }
    }
    setDragStart(null);
    setDragEnd(null);
    setDragIsBaseline(false);
  }, [
    dragStart,
    dragEnd,
    numBlocks,
    groupSize,
    dragIsBaseline,
    setSelection,
    setBaselineSelection,
    setDragStart,
    setDragEnd,
    setDragIsBaseline,
  ]);

  // Cancel drag and hide tooltip when cursor leaves the canvas
  const onMouseLeave = React.useCallback(() => {
    setTooltip(null);
    if (dragStart !== null) {
      setDragStart(null);
      setDragEnd(null);
    }
  }, [dragStart, setTooltip, setDragStart, setDragEnd]);

  return (
    <div style={{ overflowX: "auto", overflowY: "hidden" }}>
      <canvas
        ref={canvasRef}
        style={{ display: "block", cursor: "crosshair" }}
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseLeave}
        onDoubleClick={onDoubleClick}
      />
    </div>
  );
}

/**
 * HeatmapCanvas helper: Draws a rectangular border around a contiguous range of
 * cells in the heatmap grid. Used for both the active selection (blue) and
 * baseline selection (red) borders.
 */
function strokeCellRangeBorder(
  c: CanvasRenderingContext2D,
  start: number,
  end: number,
  rowsPerCol: number,
  cellSize: number,
  strokeStyle: string,
  axisHeight: number,
) {
  c.strokeStyle = strokeStyle;
  c.lineWidth = 1.5;
  for (let i = start; i < end; i++) {
    const col = Math.floor(i / rowsPerCol);
    const row = i % rowsPerCol;
    const x = col * cellSize + 0.5;
    const y = axisHeight + row * cellSize + 0.5;
    if (i === start || Math.floor((i - 1) / rowsPerCol) !== col) {
      c.beginPath();
      c.moveTo(x, y);
      c.lineTo(x + cellSize, y);
      c.stroke();
    }
    if (i === end - 1 || Math.floor((i + 1) / rowsPerCol) !== col) {
      c.beginPath();
      c.moveTo(x, y + cellSize - 1);
      c.lineTo(x + cellSize, y + cellSize - 1);
      c.stroke();
    }
    if (i < start + rowsPerCol) {
      c.beginPath();
      c.moveTo(x, y);
      c.lineTo(x, y + cellSize);
      c.stroke();
    }
    if (i >= end - rowsPerCol) {
      c.beginPath();
      c.moveTo(x + cellSize - 1, y);
      c.lineTo(x + cellSize - 1, y + cellSize);
      c.stroke();
    }
  }
}

/**
 * HeatmapInfoBar: displays selection range, time duration, sample count,
 * search match stats, zoomed node name, zoom level selector, and reset button.
 */
interface HeatmapInfoBarProps {
  readonly selection: [number, number];
  readonly blockWidthMs: number;
  readonly selSamples: number;
  readonly searchRe: RegExp | null;
  readonly flamegraphRoot: FlamegraphNode | null;
  readonly searchMatches: { count: number; totalSamples: number };
  readonly zoomedNode: FlamegraphNode | null;
  readonly setZoomedNode: (node: FlamegraphNode | null) => void;
  readonly zoomLevels: ZoomLevel[];
  readonly zoomId: string;
  readonly setZoomId: (id: string) => void;
  readonly onReset: () => void;
}

export function HeatmapInfoBar({
  selection,
  blockWidthMs,
  selSamples,
  searchRe,
  flamegraphRoot,
  searchMatches,
  zoomedNode,
  setZoomedNode,
  zoomLevels,
  zoomId,
  setZoomId,
  onReset,
}: HeatmapInfoBarProps) {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);
  const selTimeSec = (((selection[1] - selection[0]) * blockWidthMs) / 1000).toFixed(1);

  return (
    <div
      style={{
        fontSize: 11,
        fontFamily: "monospace",
        color: theme.textMuted,
        marginBottom: 4,
        display: "flex",
        justifyContent: "space-between",
      }}
    >
      <span>
        Selected: blocks {selection[0]}–{selection[1]} ({selTimeSec}s, {selSamples.toLocaleString()} samples)
        {searchRe && flamegraphRoot && (
          <span style={{ marginLeft: 12, color: theme.accent }}>
            Matches: {searchMatches.count} frames,{" "}
            {((searchMatches.totalSamples / (flamegraphRoot.totalSamples || 1)) * 100).toFixed(2)}%
          </span>
        )}
        {zoomedNode && (
          <span style={{ marginLeft: 12, color: theme.accent }}>
            Zoomed: {zoomedNode.name}{" "}
            <span style={{ cursor: "pointer", textDecoration: "underline" }} onClick={() => setZoomedNode(null)}>
              (reset zoom)
            </span>
          </span>
        )}
      </span>
      <span style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
          Zoom:
          <select
            value={zoomId}
            onChange={(e) => setZoomId(e.target.value)}
            style={{
              fontSize: 11,
              fontFamily: "monospace",
              padding: "1px 4px",
              background: "transparent",
              color: "inherit",
              border: `1px solid ${theme.border}`,
              borderRadius: 3,
            }}
          >
            {zoomLevels.map((z) => (
              <option key={z.id} value={z.id}>
                {z.label}
              </option>
            ))}
          </select>
        </span>
        <span style={{ cursor: "pointer", color: theme.accent }} onClick={onReset}>
          Reset
        </span>
      </span>
    </div>
  );
}
