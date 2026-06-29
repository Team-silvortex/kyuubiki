"use client";

import { useLayoutEffect, useState } from "react";
import type { ReactNode } from "react";
import { createWorkbenchUiStreamingRuntime } from "@/components/workbench/workbench-ui-streaming-runtime";
import {
  buildWorkbenchResolutionStyleVars,
  resolveWorkbenchResolutionAdaptation,
} from "@/components/workbench/workbench-resolution-adaptation";
import type { WorkbenchResolutionAdaptation } from "@/components/workbench/workbench-resolution-adaptation";
import type { SidebarSection } from "@/components/workbench/workbench-types";

type WorkbenchShellFrameProps = {
  assistantOverlay?: ReactNode;
  rail: ReactNode;
  sidebarSection: SidebarSection;
  sidebar: ReactNode;
  workspace: ReactNode;
};

export function WorkbenchShellFrame({
  assistantOverlay,
  rail,
  sidebarSection,
  sidebar,
  workspace,
}: WorkbenchShellFrameProps) {
  const [resolution, setResolution] = useState<WorkbenchResolutionAdaptation>(() =>
    resolveWorkbenchResolutionAdaptation({ width: 1440, height: 900 }),
  );
  const [fullscreen, setFullscreen] = useState(false);
  const uiStreamingRuntime = createWorkbenchUiStreamingRuntime(sidebarSection);
  const resolutionStyleVars = buildWorkbenchResolutionStyleVars(resolution);

  useLayoutEffect(() => {
    if (typeof window === "undefined") return undefined;

    const syncWindowState = () => {
      setResolution(
        resolveWorkbenchResolutionAdaptation({
          width: window.innerWidth,
          height: window.innerHeight,
        }),
      );
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
      data-workbench-window-mode={resolution.windowMode}
      data-workbench-viewport-profile={resolution.profile}
      data-workbench-compact-chrome={resolution.shouldCompactChrome ? "true" : "false"}
      data-workbench-stack-panels={resolution.shouldStackPanels ? "true" : "false"}
      data-workbench-scrollable-shell={resolution.shouldUseScrollableShell ? "true" : "false"}
      data-workbench-fullscreen={fullscreen ? "true" : "false"}
      {...uiStreamingRuntime.shellAttrs}
      style={resolutionStyleVars}
    >
      {assistantOverlay}
      <div className="workbench-shell__rail">{rail}</div>
      <div className="workbench-shell__sidebar">{sidebar}</div>
      <div className="workbench-shell__workspace">{workspace}</div>
    </div>
  );
}
