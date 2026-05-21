"use client";

import type { ReactNode } from "react";

type ModelPanelTab = "tools" | "tree";
export type ModelToolsPage = "study" | "studio" | "materials" | "generate";

type WorkbenchModelSidebarProps = {
  modelTab: ModelPanelTab;
  onModelTabChange: (tab: ModelPanelTab) => void;
  toolsPage: ModelToolsPage;
  onToolsPageChange: (page: ModelToolsPage) => void;
  isTruss3d: boolean;
  toolsTabLabel: string;
  treeTabLabel: string;
  toolsPageStudyLabel: string;
  toolsPageStudioLabel: string;
  toolsPageMaterialsLabel: string;
  toolsPageGenerateLabel: string;
  studyContent?: ReactNode;
  studioContent?: ReactNode;
  materialsContent?: ReactNode;
  generateContent?: ReactNode;
  treeContent?: ReactNode;
};

export function WorkbenchModelSidebar({
  modelTab,
  onModelTabChange,
  toolsPage,
  onToolsPageChange,
  isTruss3d,
  toolsTabLabel,
  treeTabLabel,
  toolsPageStudyLabel,
  toolsPageStudioLabel,
  toolsPageMaterialsLabel,
  toolsPageGenerateLabel,
  studyContent,
  studioContent,
  materialsContent,
  generateContent,
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

      {modelTab === "tools" ? (
        <>
          <div className="panel-tabs panel-tabs--wide">
            {studyContent ? (
              <button
                className={`panel-tab${toolsPage === "study" ? " panel-tab--active" : ""}`}
                onClick={() => onToolsPageChange("study")}
                type="button"
              >
                {toolsPageStudyLabel}
              </button>
            ) : null}
            <button
              className={`panel-tab${toolsPage === "studio" ? " panel-tab--active" : ""}`}
              onClick={() => onToolsPageChange("studio")}
              type="button"
            >
              {toolsPageStudioLabel}
            </button>
            {materialsContent ? (
              <button
                className={`panel-tab${toolsPage === "materials" ? " panel-tab--active" : ""}`}
                onClick={() => onToolsPageChange("materials")}
                type="button"
              >
                {toolsPageMaterialsLabel}
              </button>
            ) : null}
            {generateContent ? (
              <button
                className={`panel-tab${toolsPage === "generate" ? " panel-tab--active" : ""}`}
                onClick={() => onToolsPageChange("generate")}
                type="button"
              >
                {toolsPageGenerateLabel}
              </button>
            ) : null}
          </div>
          {toolsPage === "study" ? studyContent : null}
          {toolsPage === "studio" ? studioContent : null}
          {toolsPage === "materials" ? materialsContent : null}
          {toolsPage === "generate" ? generateContent : null}
        </>
      ) : null}
      {modelTab === "tree" ? treeContent : null}
    </div>
  );
}
