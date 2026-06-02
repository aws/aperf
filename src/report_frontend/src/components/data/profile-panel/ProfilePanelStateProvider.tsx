import React from "react";

export type ViewMode = "flamegraph" | "top_functions" | "metadata";

export interface ProfilePanelState {
  searchRegex: string;
  filterRegex: string;
  reverse: boolean;
  showDiff: boolean;
  viewMode: ViewMode;
  baseRunSelection: [number, number] | null;
  setSearchRegex: (v: string) => void;
  setFilterRegex: (v: string) => void;
  setReverse: (v: boolean) => void;
  setShowDiff: (v: boolean) => void;
  setViewMode: (v: ViewMode) => void;
  setBaseRunSelection: (v: [number, number]) => void;
}

const ProfilePanelContext = React.createContext<ProfilePanelState | undefined>(undefined);

export function ProfilePanelStateProvider({ children }: { children: React.ReactNode }) {
  const [searchRegex, setSearchRegex] = React.useState("");
  const [filterRegex, setFilterRegex] = React.useState("");
  const [reverse, setReverse] = React.useState(false);
  const [showDiff, setShowDiff] = React.useState(false);
  const [viewMode, setViewMode] = React.useState<ViewMode>("flamegraph");
  const [baseRunSelection, setBaseRunSelection] = React.useState<[number, number] | null>(null);
  const value = React.useMemo(
    () => ({
      searchRegex,
      filterRegex,
      reverse,
      showDiff,
      viewMode,
      baseRunSelection,
      setSearchRegex,
      setFilterRegex,
      setReverse,
      setShowDiff,
      setViewMode,
      setBaseRunSelection,
    }),
    [searchRegex, filterRegex, reverse, showDiff, viewMode, baseRunSelection],
  );
  return <ProfilePanelContext.Provider value={value}>{children}</ProfilePanelContext.Provider>;
}

export function useProfilePanelState() {
  const context = React.useContext(ProfilePanelContext);
  if (context == undefined) {
    throw new Error("useProfilePanelState must be used within ProfilePanelStateProvider");
  }
  return context;
}
