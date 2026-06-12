"use client";

import type { ReactNode } from "react";

type WorkbenchShellFrameProps = {
  assistantOverlay?: ReactNode;
  rail: ReactNode;
  sidebar: ReactNode;
  workspace: ReactNode;
};

export function WorkbenchShellFrame({
  assistantOverlay,
  rail,
  sidebar,
  workspace,
}: WorkbenchShellFrameProps) {
  return (
    <div
      className="workbench-shell"
      data-workbench-automation-contract="v1"
      data-workbench-shell="root"
      data-workbench-shell-extensible="false"
    >
      {assistantOverlay}
      <div className="workbench-shell__rail">{rail}</div>
      <div className="workbench-shell__sidebar">{sidebar}</div>
      <div className="workbench-shell__workspace">{workspace}</div>
    </div>
  );
}
