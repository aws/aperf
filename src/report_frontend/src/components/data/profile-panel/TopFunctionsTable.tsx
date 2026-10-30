import React from "react";
import { Box, Pagination, Table, TableProps } from "@cloudscape-design/components";
import { useCollection } from "@cloudscape-design/collection-hooks";
import { useReportState } from "../../ReportStateProvider";
import {
  FRAME_TYPE_COLORS,
  FRAME_TYPE_LABELS,
  DIFF_EPSILON_PP,
  diffFlamegraphColor,
  diffTextColor,
  getThemeColors,
} from "./colors";
import { frameKey } from "./utils";
import { BaselineFrameStats, FrameEntry } from "./profileView";

interface Props {
  readonly framesList: FrameEntry[];
  readonly searchRe: RegExp | null;
  // Map from `frameKey(name, type)` to the baseline's stats for that frame.
  // Used by the diff-tint coloring and the "(baseline, compare)" counts.
  readonly baselineByFrame: Map<string, BaselineFrameStats> | null;
}

export function TopFunctionsTable({ framesList, searchRe, baselineByFrame }: Props) {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);
  const scope = React.useId();

  // Precompute normalized deltas for diff coloring; deltas under the noise
  // floor count as "no change".
  const frameDeltaMap = React.useMemo(() => {
    if (!baselineByFrame) return null;
    const deltas = new Map<string, number>();
    let maxAbs = 0;
    for (const f of framesList) {
      const key = frameKey(f.name, f.type);
      const basePercent = baselineByFrame.get(key)?.totalPercent || 0;
      const d = f.totalPercent - basePercent;
      if (Math.abs(d) < DIFF_EPSILON_PP) continue;
      deltas.set(key, d);
      if (Math.abs(d) > maxAbs) maxAbs = Math.abs(d);
    }
    if (maxAbs === 0) return null;
    const normalized = new Map<string, number>();
    for (const [key, d] of deltas) normalized.set(key, d / maxAbs);
    return normalized;
  }, [framesList, baselineByFrame]);

  // Numeric row ids used as `data-row` hooks for the row styles below (frame
  // names can contain any character, so they can't go into a CSS selector).
  const rowIds = React.useMemo(() => {
    const ids = new Map<string, number>();
    framesList.forEach((f, i) => ids.set(frameKey(f.name, f.type), i));
    return ids;
  }, [framesList]);

  const Cell = React.useCallback(
    (content: React.ReactNode, align: "left" | "right" = "left") => (
      <div style={{ textAlign: align, fontFamily: "monospace", fontSize: 12 }}>{content}</div>
    ),
    [],
  );

  // " (baseline, compare)" sample counts appended to Self/Total while diffing.
  const diffCounts = React.useCallback(
    (entry: FrameEntry, field: "self" | "total"): string => {
      if (!baselineByFrame) return "";
      const base = baselineByFrame.get(frameKey(entry.name, entry.type))?.[field] || 0;
      return ` (${base.toLocaleString()}, ${entry[field].toLocaleString()})`;
    },
    [baselineByFrame],
  );

  const columnDefinitions: TableProps.ColumnDefinition<FrameEntry>[] = React.useMemo(
    () => [
      {
        id: "name",
        header: "Frame",
        sortingField: "name",
        isRowHeader: true,
        cell: (item: FrameEntry) =>
          Cell(
            <span data-row={rowIds.get(frameKey(item.name, item.type))} style={{ wordBreak: "break-all" }}>
              {item.name}
            </span>,
          ),
      },
      {
        id: "type",
        header: "Type",
        sortingField: "type",
        cell: (item: FrameEntry) =>
          Cell(
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
        cell: (item: FrameEntry) =>
          Cell(`${item.self.toLocaleString()} (${item.selfPercent.toFixed(2)}%)${diffCounts(item, "self")}`, "right"),
      },
      {
        id: "total",
        header: "Total",
        sortingField: "totalPercent",
        cell: (item: FrameEntry) =>
          Cell(
            `${item.total.toLocaleString()} (${item.totalPercent.toFixed(2)}%)${diffCounts(item, "total")}`,
            "right",
          ),
      },
    ],
    [Cell, diffCounts, rowIds, theme.borderSubtle],
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

  // Cloudscape `Table` has no row-level style hook, and painting a wrapper
  // inside each cell leaves gaps at the cell padding and on wrapped rows. So
  // emit one rule per visible row that paints the whole <tr> via `:has()`.
  // A search match keeps its diff tint and gets an accent outline instead.
  const rowCss = React.useMemo(() => {
    const rules: string[] = [];
    for (const item of items) {
      const key = frameKey(item.name, item.type);
      // The Frame column is the row header, so it renders as <th>.
      const tr = `[data-scope="${scope}"] tr:has([data-row="${rowIds.get(key)}"])`;
      const row = `${tr} > td, ${tr} > th`;
      const delta = frameDeltaMap?.get(key) || 0;
      const fill = frameDeltaMap ? diffFlamegraphColor(delta) : null;
      const matched = searchRe !== null && searchRe.test(item.name);
      const decls: string[] = [];
      if (fill) decls.push(`background-color: ${fill} !important`, `color: ${diffTextColor(delta)} !important`);
      else if (matched) decls.push(`background-color: ${theme.searchHighlight} !important`);
      if (fill && matched) decls.push(`box-shadow: inset 0 2px 0 ${theme.accent}, inset 0 -2px 0 ${theme.accent}`);
      if (decls.length > 0) rules.push(`${row} { ${decls.join("; ")}; }`);
    }
    return rules.join("\n");
  }, [items, scope, rowIds, frameDeltaMap, searchRe, theme.searchHighlight, theme.accent]);

  return (
    <div data-scope={scope} style={{ marginTop: 8, maxHeight: "60vh", overflowY: "auto", overflowX: "hidden" }}>
      {rowCss && <style>{rowCss}</style>}
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
