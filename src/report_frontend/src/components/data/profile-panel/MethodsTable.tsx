import React from "react";
import { useReportState } from "../../ReportStateProvider";
import { FrameType, FRAME_TYPE_COLORS, FRAME_TYPE_LABELS, diffFlamegraphColor, getThemeColors } from "./colors";

interface MethodEntry {
  name: string;
  type: FrameType;
  self: number;
  total: number;
  selfPct: number;
  totalPct: number;
}

type SortKey = "name" | "type" | "self" | "selfPct" | "total" | "totalPct";

interface Props {
  readonly methodsList: MethodEntry[];
  readonly searchRe: RegExp | null;
  readonly baselineByFrame: Map<string, { self: number; total: number }> | null;
}

export function MethodsTable({ methodsList, searchRe, baselineByFrame }: Props) {
  const { darkMode } = useReportState();
  const theme = getThemeColors(darkMode);
  const [sortKey, setSortKey] = React.useState<SortKey>("totalPct");
  const [sortAsc, setSortAsc] = React.useState(false);

  // Precompute normalized deltas for diff coloring (sample count difference, normalized by max)
  const methodDeltaMap = React.useMemo(() => {
    if (!baselineByFrame) return null;
    const deltas = new Map<string, number>();
    let maxAbs = 0;
    for (const m of methodsList) {
      const base = baselineByFrame.get(m.name)?.total || 0;
      const d = m.total - base;
      deltas.set(m.name, d);
      if (Math.abs(d) > maxAbs) maxAbs = Math.abs(d);
    }
    if (maxAbs === 0) return null;
    const normalized = new Map<string, number>();
    for (const [name, d] of deltas) normalized.set(name, d / maxAbs);
    return normalized;
  }, [methodsList, baselineByFrame]);

  const handleSort = (key: SortKey) => {
    if (key === sortKey) {
      setSortAsc(!sortAsc);
    } else {
      setSortKey(key);
      setSortAsc(key === "name" || key === "type");
    }
  };

  const sorted = React.useMemo(() => {
    const dir = sortAsc ? 1 : -1;
    return [...methodsList].sort((a, b) => {
      if (sortKey === "name") return dir * a.name.localeCompare(b.name);
      if (sortKey === "type") return dir * a.type.localeCompare(b.type);
      return dir * (a[sortKey] - b[sortKey]);
    });
  }, [methodsList, sortKey, sortAsc]);

  const headerStyle = (key: SortKey, align: "left" | "right"): React.CSSProperties => ({
    textAlign: align,
    padding: "4px 8px",
    cursor: "pointer",
    userSelect: "none",
  });

  const arrow = (key: SortKey) => (sortKey === key ? (sortAsc ? " ▲" : " ▼") : "");

  return (
    <div style={{ marginTop: 8, overflowY: "auto", maxHeight: "60vh" }}>
      <table
        style={{
          width: "100%",
          borderCollapse: "collapse",
          fontSize: 12,
          fontFamily: "monospace",
          color: baselineByFrame ? theme.textSet : theme.textPrimary,
        }}
      >
        <thead>
          <tr
            style={{
              background: theme.bgMuted,
              position: "sticky",
              top: 0,
              color: theme.textPrimary,
            }}
          >
            <th style={headerStyle("name", "left")} onClick={() => handleSort("name")}>
              Method{arrow("name")}
            </th>
            <th style={headerStyle("type", "left")} onClick={() => handleSort("type")}>
              Type{arrow("type")}
            </th>
            <th style={headerStyle("self", "right")} onClick={() => handleSort("self")}>
              Self{arrow("self")}
            </th>
            <th style={headerStyle("selfPct", "right")} onClick={() => handleSort("selfPct")}>
              Self %{arrow("selfPct")}
            </th>
            <th style={headerStyle("total", "right")} onClick={() => handleSort("total")}>
              Total{arrow("total")}
            </th>
            <th style={headerStyle("totalPct", "right")} onClick={() => handleSort("totalPct")}>
              Total %{arrow("totalPct")}
            </th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((m) => {
            const isMatch = searchRe && searchRe.test(m.name);
            const bgColor = methodDeltaMap ? diffFlamegraphColor(methodDeltaMap.get(m.name) || 0) : "transparent";
            return (
              <tr
                key={`${m.name}\u0000${m.type}`}
                style={{
                  background: isMatch ? theme.searchHighlight : bgColor,
                }}
              >
                <td style={{ padding: "2px 8px", wordBreak: "break-all" }}>{m.name}</td>
                <td style={{ padding: "2px 8px", whiteSpace: "nowrap" }}>
                  <span
                    style={{
                      display: "inline-block",
                      width: 10,
                      height: 10,
                      background: FRAME_TYPE_COLORS[m.type],
                      border: `1px solid ${theme.borderSubtle}`,
                      borderRadius: 2,
                      marginRight: 6,
                      verticalAlign: "middle",
                    }}
                  />
                  {FRAME_TYPE_LABELS[m.type]}
                </td>
                <td style={{ padding: "2px 8px", textAlign: "right" }}>{m.self.toLocaleString()}</td>
                <td style={{ padding: "2px 8px", textAlign: "right" }}>{m.selfPct.toFixed(2)}</td>
                <td style={{ padding: "2px 8px", textAlign: "right" }}>{m.total.toLocaleString()}</td>
                <td style={{ padding: "2px 8px", textAlign: "right" }}>{m.totalPct.toFixed(2)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
