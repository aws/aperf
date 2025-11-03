import React from "react";
import { createRoot } from "react-dom/client";
import Report from "./components/Report";
import ReportStateProvider from "./components/ReportStateProvider";

document.addEventListener("DOMContentLoaded", () => {
  const root = createRoot(document.getElementById("root")!);
  root.render(
    <ReportStateProvider>
      <Report />
    </ReportStateProvider>
  );
});