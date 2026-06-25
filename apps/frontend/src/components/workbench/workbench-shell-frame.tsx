"use client";

import { useLayoutEffect, useState } from "react";
import type { ReactNode } from "react";

type WorkbenchShellFrameProps = {
  assistantOverlay?: ReactNode;
  rail: ReactNode;
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
  sidebar,
  workspace,
}: WorkbenchShellFrameProps) {
  const [windowMode, setWindowMode] = useState<WorkbenchWindowMode>("standard");
  const [fullscreen, setFullscreen] = useState(false);

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
    >
      {assistantOverlay}
      <div className="workbench-shell__rail">{rail}</div>
      <div className="workbench-shell__sidebar">{sidebar}</div>
      <div className="workbench-shell__workspace">{workspace}</div>
    </div>
  );
}
