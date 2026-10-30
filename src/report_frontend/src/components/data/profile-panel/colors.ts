// --- Color utilities ---

// --- Theme colors (dark/light pairs) ---

export interface ThemeColors {
  textMuted: string;
  textPrimary: string;
  textSubtle: string;
  textSet: string;
  accent: string;
  border: string;
  borderSubtle: string;
  bgSurface: string;
  bgMuted: string;
  bgBuffer: string;
  bgRoot: string;
  overlayDim: string;
  overlayDimmed: string;
  selectionBorder: string;
  baselineBorder: string;
  dragOverlay: string;
  baselineDragOverlay: string;
  baselineOverlay: string;
  searchHighlight: string;
}

const DARK_THEME: ThemeColors = {
  textMuted: "#aaa",
  textPrimary: "#eee",
  textSubtle: "#ccc",
  textSet: "#111",
  accent: "#58a6ff",
  border: "#555",
  borderSubtle: "#555",
  bgSurface: "#1e1e1e",
  bgMuted: "#2a2a2a",
  bgBuffer: "#2a2a2a",
  bgRoot: "#676767",
  overlayDim: "rgba(0,0,0,0.55)",
  overlayDimmed: "rgba(0,0,0,0.4)",
  selectionBorder: "#58a6ff",
  baselineBorder: "#ff6b6b",
  dragOverlay: "rgba(88,166,255,0.3)",
  baselineDragOverlay: "rgba(205,50,50,0.3)",
  baselineOverlay: "rgba(205,50,50,0.25)",
  searchHighlight: "rgba(64,178,255,0.25)",
};

const LIGHT_THEME: ThemeColors = {
  textMuted: "#666",
  textPrimary: "#111",
  textSubtle: "#333",
  textSet: "#111",
  accent: "#0073bb",
  border: "#ccc",
  borderSubtle: "#999",
  bgSurface: "#fff",
  bgMuted: "#f0f0f0",
  bgBuffer: "#e0e0e0",
  bgRoot: "#ddd",
  overlayDim: "rgba(255,255,255,0.55)",
  overlayDimmed: "rgba(255,255,255,0.4)",
  selectionBorder: "#0073bb",
  baselineBorder: "#cc0000",
  dragOverlay: "rgba(0,115,187,0.25)",
  baselineDragOverlay: "rgba(239,34,34,0.25)",
  baselineOverlay: "rgba(139,34,34,0.2)",
  searchHighlight: "rgba(64,178,255,0.2)",
};

export function getThemeColors(darkMode: boolean): ThemeColors {
  return darkMode ? DARK_THEME : LIGHT_THEME;
}

// Async-profiler heatmap palette: white → orange → red.
// Uses async-profiler's setH(0.4) base (dR=256, dG≈102, dB=0) and its ratio→color formula.
const HEATMAP_BASE_R = 256;
const HEATMAP_BASE_G = 102;
const HEATMAP_BASE_B = 0;

export function heatmapColor(ratio: number): string {
  // ratio is expected in [0, 1]. 0 → show transparent, increasing → orange → red.
  if (ratio <= 0) return "#ffffff";
  const clamped = Math.max(0, Math.min(1, ratio));
  const ratioM = (clamped * 192 + 24) | 0;
  const C = 255 - Math.abs(255 - (ratioM << 1));
  const m = 255 - ratioM - (C >> 1);
  const r = ((HEATMAP_BASE_R * C) >> 8) + m;
  const g = ((HEATMAP_BASE_G * C) >> 8) + m;
  const b = ((HEATMAP_BASE_B * C) >> 8) + m;
  return `rgb(${r},${g},${b})`;
}

/**
 * Map frame name suffix to frame type color. Suffixes follow async-profiler convention:
 *   _[j] JIT, _[i] Inlined, _[k] Kernel, _[0] Interpreted, _[1] C1, no suffix Native.
 */
export type FrameType = "jit" | "inlined" | "kernel" | "interpreted" | "c1" | "vdso" | "cpp" | "native";

export function getFrameType(name: string): FrameType {
  if (name.endsWith("_[j]")) return "jit";
  if (name.endsWith("_[i]")) return "inlined";
  if (name.endsWith("_[k]")) return "kernel";
  if (name.endsWith("_[0]")) return "interpreted";
  if (name.endsWith("_[1]")) return "c1";
  if (name.endsWith("_[v]")) return "vdso";
  if (name.endsWith("_[c]")) return "cpp";
  return "native";
}

export const FRAME_TYPE_COLORS: Record<FrameType, string> = {
  interpreted: "#b2e1b2",
  jit: "#50e150",
  inlined: "#50cccc",
  native: "#e15a5a",
  cpp: "#c8c83c",
  kernel: "#e17d00",
  c1: "#cce880",
  vdso: "#918984",
};

export const FRAME_TYPE_LABELS: Record<FrameType, string> = {
  interpreted: "Interpreted",
  jit: "JIT-Compiled",
  inlined: "Inlined",
  native: "Native",
  cpp: "C++",
  kernel: "Kernel",
  c1: "C1-Compiled",
  vdso: "vDSO",
};

export function defaultFlamegraphColor(name: string): string {
  return FRAME_TYPE_COLORS[getFrameType(name)];
}

/**
 * Diff color following https://github.com/brendangregg/FlameGraph/blob/41fee1f99f9276008b7cd112fca19dc3ea84ac32/difffolded.pl:
 * Takes a pre-normalized delta in [-1, 1] where the value is (samples2 - samples1) / maxAbsDelta.
 * Red = grew, Blue = shrank.
 */
/**
 * Noise floor for diffs, in percentage points of each selection's samples: smaller changes in
 * a frame's share count as unchanged. 0.01 matches the Top Functions display precision.
 */
export const DIFF_EPSILON_PP = 0.01;

export function diffFlamegraphColor(delta: number): string | null {
  if (Math.abs(delta) < 0.001) return null;
  const c = diffShade(delta);
  return delta > 0 ? `rgb(255,${c},${c})` : `rgb(${c},${c},255)`;
}

export function diffTextColor(delta: number): string {
  return delta < 0 && diffShade(delta) < 100 ? "#fff" : "#111";
}

/** The two non-dominant RGB channels of a diff fill: 210 for tiny deltas, 0 at |delta| = 1. */
function diffShade(delta: number): number {
  return Math.round(210 * (1 - Math.sqrt(Math.min(1, Math.abs(delta)))));
}
