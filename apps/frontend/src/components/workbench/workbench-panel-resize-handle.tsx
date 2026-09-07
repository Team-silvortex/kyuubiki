"use client";

import { useLayoutEffect, useRef } from "react";
import type { WorkbenchPanelSize } from "./workbench-panel-layout";
import { getWorkbenchPanelLayoutCopy } from "./workbench-panel-layout-copy";

export function WorkbenchPanelResizeHandle({ panel, language }: { panel: WorkbenchPanelSize; language?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    ref.current?.dispatchEvent(new Event("workbench-layout-mounted", { bubbles: true }));
  }, []);
  return <div ref={ref} className="workbench-panel-resize" data-workbench-resize={panel}
    role="separator" tabIndex={0} aria-label={`workbench-resize:${panel}`}
    aria-orientation={panel === "report" ? "horizontal" : "vertical"}
    title={getWorkbenchPanelLayoutCopy(language).resize} />;
}

export function WorkbenchLayoutReset({ language }: { language?: string }) {
  const label = getWorkbenchPanelLayoutCopy(language).reset;
  return <button type="button" className="workbench-layout-reset" data-workbench-layout-reset="true"
    aria-label={label} title={label}>
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden="true">
      <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
      <path d="M5 3v10M11 3v10M5 10h6" />
    </svg>
  </button>;
}
