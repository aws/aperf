import React from "react";
import { Box, Pagination, Table, TableProps } from "@cloudscape-design/components";
import { useCollection } from "@cloudscape-design/collection-hooks";
import { useReportState } from "../../ReportStateProvider";
import { FrameType, FRAME_TYPE_COLORS, FRAME_TYPE_LABELS, diffFlamegraphColor, getThemeColors } from "./colors";
import { frameKey } from "./utils";

/**
 * One row in the Top Functions table.
 *   - `name` is the display name (frame-type suffix already stripped).
 *   - `type` is the frame type derived from the original raw name.
 *   - `(name, type)` together form the row identity — `frameKey(name, type)`
 *     is used both as `trackBy` and as the lookup key into `baselineByFrame`.
 *   - `self` / `total` are the aggregated sample counts for the current
 *     selection.
 *   - `selfPercent` / `totalPercent` are precomputed because the denominator
 *     (root-total samples) is shared across rows.
 */
interface FrameEntry {
  name: string;
  type: FrameType;
  self: number;
  total: number;
  selfPercent: number;
  totalPercent: number;
}

interface Props {
  readonly framesList: FrameEntry[];
  readonly searchRe: RegExp | null;
  // Map from `frameKey(name, type)` to the baseline's `totalPercent` for that
  // frame. Used by the diff-tint coloring
  readonly baselineByFrame: Map<string, number> | null;
}

export function TopFunctionsTable({ framesList, searchRe, baselineByFrame }: Props) {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);

  // Precompute normalized deltas for diff coloring. Deltas below this
  // threshold (in percentage points) are treated as "no change". 0.01 matches
  // the table's display precision (`toFixed(2)`).
  const DIFF_EPSILON = 0.01;
  const frameDeltaMap = React.useMemo(() => {
    if (!baselineByFrame) return null;
    const deltas = new Map<string, number>();
    let maxAbs = 0;
    for (const f of framesList) {
      const key = frameKey(f.name, f.type);
      const basePercent = baselineByFrame.get(key) || 0;
      const d = f.totalPercent - basePercent;
      if (Math.abs(d) < DIFF_EPSILON) continue;
      deltas.set(key, d);
      if (Math.abs(d) > maxAbs) maxAbs = Math.abs(d);
    }
    if (maxAbs === 0) return null;
    const normalized = new Map<string, number>();
    for (const [key, d] of deltas) normalized.set(key, d / maxAbs);
    return normalized;
  }, [framesList, baselineByFrame]);

  // Cloudscape `Table` has no row-level style hook, so per-row search-highlight
  // and diff-tint backgrounds are painted onto a wrapper inside each cell.
  const cellBackground = React.useCallback(
    (entry: FrameEntry): string => {
      if (searchRe && searchRe.test(entry.name)) return theme.searchHighlight;
      if (frameDeltaMap) return diffFlamegraphColor(frameDeltaMap.get(frameKey(entry.name, entry.type)) || 0);
      return "transparent";
    },
    [searchRe, frameDeltaMap, theme.searchHighlight],
  );

  const Cell = React.useCallback(
    (entry: FrameEntry, content: React.ReactNode, align: "left" | "right" = "left") => {
      const background = cellBackground(entry);
      const tinted = background !== "transparent";
      return (
        <div
          style={{
            background,
            // The diff/search fills are always light pastel colors, so force dark
            // text on tinted cells..
            color: tinted ? "#111" : "inherit",
            // Cloudscape's body cell padding is ~19px inline / ~9px block. Set margin
            // and padding to fill a cell
            margin: "-7px -18px",
            padding: "7px 18px",
            textAlign: align,
            fontFamily: "monospace",
            fontSize: 12,
          }}
        >
          {content}
        </div>
      );
    },
    [cellBackground],
  );

  const columnDefinitions: TableProps.ColumnDefinition<FrameEntry>[] = React.useMemo(
    () => [
      {
        id: "name",
        header: "Frame",
        sortingField: "name",
        isRowHeader: true,
        cell: (item) => Cell(item, <span style={{ wordBreak: "break-all" }}>{item.name}</span>),
      },
      {
        id: "type",
        header: "Type",
        sortingField: "type",
        cell: (item) =>
          Cell(
            item,
            <span style={{ whiteSpace: "nowrap" }}>
              <span
                style={{
                  display: "inline-block",
                  width: 10,
                  height: 10,
                  background: FRAME_TYPE_COLORS[item.type],
                  border: `1px solid ${theme.borderSubtle}`,
                  borderRadius: 2,
                  marginRight: 6,
                  verticalAlign: "middle",
                }}
              />
              {FRAME_TYPE_LABELS[item.type]}
            </span>,
          ),
      },
      {
        id: "self",
        header: "Self",
        sortingField: "selfPercent",
        cell: (item) => Cell(item, `${item.self.toLocaleString()} (${item.selfPercent.toFixed(2)}%)`, "right"),
      },
      {
        id: "total",
        header: "Total",
        sortingField: "totalPercent",
        cell: (item) => Cell(item, `${item.total.toLocaleString()} (${item.totalPercent.toFixed(2)}%)`, "right"),
      },
    ],
    [Cell, theme.borderSubtle],
  );

  // Cloudscape Table renders every visible row synchronously without virtualization,
  // and a typical profile yields thousands of frames.
  const { items, collectionProps, paginationProps } = useCollection(framesList, {
    sorting: {
      defaultState: {
        sortingColumn: columnDefinitions.find((c) => c.id === "total")!,
        isDescending: true,
      },
    },
    pagination: { pageSize: 50 },
  });

  return (
    <div style={{ marginTop: 8, maxHeight: "60vh", overflowY: "auto", overflowX: "hidden" }}>
      <Table
        {...collectionProps}
        variant="embedded"
        items={items}
        columnDefinitions={columnDefinitions}
        trackBy={(item: FrameEntry) => frameKey(item.name, item.type)}
        stickyHeader={true}
        wrapLines={true}
        empty={<Box variant="p">No frames.</Box>}
        pagination={<Pagination {...paginationProps} />}
      />
    </div>
  );
}
