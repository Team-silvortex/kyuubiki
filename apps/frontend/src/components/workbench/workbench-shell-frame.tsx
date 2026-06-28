"use client";

import { useLayoutEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  WORKBENCH_UI_STREAMING_CONTRACT_VERSION,
  resolveWorkbenchUiStreamingState,
} from "@/components/workbench/workbench-ui-streaming";
import type { SidebarSection } from "@/components/workbench/workbench-types";

type WorkbenchShellFrameProps = {
  assistantOverlay?: ReactNode;
  rail: ReactNode;
  sidebarSection: SidebarSection;
  sidebar: ReactNode;
  workspace: ReactNode;
};

type WorkbenchWindowMode = "standard" | "compact" | "narrow" | "ultranarrow";

function resolveWorkbenchWindowMode(width: number): WorkbenchWindowMode {
  if (width <= 860) return "ultranarrow";
  if (width <= 1180) return "narrow";
  if (width <= 1440) return "compact";
  return "standard";
}

export function WorkbenchShellFrame({
  assistantOverlay,
  rail,
  sidebarSection,
  sidebar,
  workspace,
}: WorkbenchShellFrameProps) {
  const [windowMode, setWindowMode] = useState<WorkbenchWindowMode>("standard");
  const [fullscreen, setFullscreen] = useState(false);
  const streamingState = resolveWorkbenchUiStreamingState(sidebarSection);

  useLayoutEffect(() => {
    if (typeof window === "undefined") return undefined;

    const syncWindowState = () => {
      setWindowMode(resolveWorkbenchWindowMode(window.innerWidth));
      setFullscreen(Boolean(document.fullscreenElement));
    };

    syncWindowState();
    window.addEventListener("resize", syncWindowState);
    document.addEventListener("fullscreenchange", syncWindowState);
    return () => {
      window.removeEventListener("resize", syncWindowState);
      document.removeEventListener("fullscreenchange", syncWindowState);
    };
  }, []);

  return (
    <div
      className="workbench-shell"
      data-workbench-automation-contract="v1"
      data-workbench-shell="root"
      data-workbench-shell-extensible="false"
      data-workbench-window-mode={windowMode}
      data-workbench-fullscreen={fullscreen ? "true" : "false"}
      data-workbench-ui-streaming-contract={WORKBENCH_UI_STREAMING_CONTRACT_VERSION}
      data-workbench-active-ui-chunks={streamingState.activeChunks.join(" ")}
      data-workbench-prefetch-ui-chunks={streamingState.prefetchChunks.join(" ")}
      data-workbench-evictable-ui-chunks={streamingState.evictableChunks.join(" ")}
      data-workbench-ui-chunk-budget={streamingState.budgetStatus}
    >
      {assistantOverlay}
      <div className="workbench-shell__rail">{rail}</div>
      <div className="workbench-shell__sidebar">{sidebar}</div>
      <div className="workbench-shell__workspace">{workspace}</div>
    </div>
  );
}
