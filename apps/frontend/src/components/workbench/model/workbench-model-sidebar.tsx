"use client";

import type { ReactNode } from "react";

type ModelPanelTab = "tools" | "tree";

type WorkbenchModelSidebarProps = {
  modelTab: ModelPanelTab;
  onModelTabChange: (tab: ModelPanelTab) => void;
  isTruss3d: boolean;
  toolsTabLabel: string;
  treeTabLabel: string;
  toolsContent?: ReactNode;
  treeContent?: ReactNode;
};

export function WorkbenchModelSidebar({
  modelTab,
  onModelTabChange,
  isTruss3d,
  toolsTabLabel,
  treeTabLabel,
  toolsContent,
  treeContent,
}: WorkbenchModelSidebarProps) {
  return (
    <div className={`sidebar-stack panel-scroll-window${isTruss3d ? " sidebar-stack--space" : ""}`}>
      <div className="panel-tabs">
        <button
          className={`panel-tab${modelTab === "tools" ? " panel-tab--active" : ""}`}
          onClick={() => onModelTabChange("tools")}
          type="button"
        >
          {toolsTabLabel}
        </button>
        <button
          className={`panel-tab${modelTab === "tree" ? " panel-tab--active" : ""}`}
          onClick={() => onModelTabChange("tree")}
          type="button"
        >
          {treeTabLabel}
        </button>
      </div>

      {modelTab === "tools" ? toolsContent : null}
      {modelTab === "tree" ? treeContent : null}
    </div>
  );
}
