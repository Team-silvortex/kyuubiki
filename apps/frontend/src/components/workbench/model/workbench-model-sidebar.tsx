"use client";

import { useState, type ReactNode } from "react";

type ModelPanelTab = "tools" | "tree";
type ModelToolsPage = "studio" | "materials" | "generate";

type WorkbenchModelSidebarProps = {
  modelTab: ModelPanelTab;
  onModelTabChange: (tab: ModelPanelTab) => void;
  isTruss3d: boolean;
  toolsTabLabel: string;
  treeTabLabel: string;
  toolsPageStudioLabel: string;
  toolsPageMaterialsLabel: string;
  toolsPageGenerateLabel: string;
  studioContent?: ReactNode;
  materialsContent?: ReactNode;
  generateContent?: ReactNode;
  treeContent?: ReactNode;
};

export function WorkbenchModelSidebar({
  modelTab,
  onModelTabChange,
  isTruss3d,
  toolsTabLabel,
  treeTabLabel,
  toolsPageStudioLabel,
  toolsPageMaterialsLabel,
  toolsPageGenerateLabel,
  studioContent,
  materialsContent,
  generateContent,
  treeContent,
}: WorkbenchModelSidebarProps) {
  const [toolsPage, setToolsPage] = useState<ModelToolsPage>("studio");
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

      {modelTab === "tools" ? (
        <>
          <div className="panel-tabs panel-tabs--wide">
            <button
              className={`panel-tab${toolsPage === "studio" ? " panel-tab--active" : ""}`}
              onClick={() => setToolsPage("studio")}
              type="button"
            >
              {toolsPageStudioLabel}
            </button>
            {materialsContent ? (
              <button
                className={`panel-tab${toolsPage === "materials" ? " panel-tab--active" : ""}`}
                onClick={() => setToolsPage("materials")}
                type="button"
              >
                {toolsPageMaterialsLabel}
              </button>
            ) : null}
            {generateContent ? (
              <button
                className={`panel-tab${toolsPage === "generate" ? " panel-tab--active" : ""}`}
                onClick={() => setToolsPage("generate")}
                type="button"
              >
                {toolsPageGenerateLabel}
              </button>
            ) : null}
          </div>
          {toolsPage === "studio" ? studioContent : null}
          {toolsPage === "materials" ? materialsContent : null}
          {toolsPage === "generate" ? generateContent : null}
        </>
      ) : null}
      {modelTab === "tree" ? treeContent : null}
    </div>
  );
}
