"use client";

import type { ReactNode } from "react";
import { WorkbenchRouteJourney } from "@/components/workbench/workbench-route-journey";

type ModelPanelTab = "tools" | "tree";
export type ModelToolsPage = "overview" | "study" | "studio" | "materials" | "generate";

type WorkbenchModelSidebarProps = {
  modelTab: ModelPanelTab;
  onModelTabChange: (tab: ModelPanelTab) => void;
  toolsPage: ModelToolsPage;
  onToolsPageChange: (page: ModelToolsPage) => void;
  isTruss3d: boolean;
  toolsTabLabel: string;
  treeTabLabel: string;
  toolsPageOverviewLabel: string;
  toolsPageStudyLabel: string;
  toolsPageStudioLabel: string;
  toolsPageMaterialsLabel: string;
  toolsPageGenerateLabel: string;
  studyOverviewHint: string;
  studioOverviewHint: string;
  materialsOverviewHint: string;
  generateOverviewHint: string;
  browseOverviewHint: string;
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
  toolsPageOverviewLabel,
  toolsPageStudyLabel,
  toolsPageStudioLabel,
  toolsPageMaterialsLabel,
  toolsPageGenerateLabel,
  studyOverviewHint,
  studioOverviewHint,
  materialsOverviewHint,
  generateOverviewHint,
  browseOverviewHint,
  studyContent,
  studioContent,
  materialsContent,
  generateContent,
  treeContent,
}: WorkbenchModelSidebarProps) {
  return (
    <div
      className={`sidebar-stack panel-scroll-window${isTruss3d ? " sidebar-stack--space" : ""}`}
      data-workbench-model="panel"
    >
      <div className="panel-tabs">
        <button
          className={`panel-tab${modelTab === "tools" ? " panel-tab--active" : ""}`}
          data-workbench-model-tab="tools"
          onClick={() => onModelTabChange("tools")}
          type="button"
        >
          {toolsTabLabel}
        </button>
        <button
          className={`panel-tab${modelTab === "tree" ? " panel-tab--active" : ""}`}
          data-workbench-model-tab="tree"
          onClick={() => onModelTabChange("tree")}
          type="button"
        >
          {treeTabLabel}
        </button>
      </div>

      {modelTab === "tools" ? (
        <>
          {toolsPage === "overview" ? (
            <div className="panel-tabs panel-tabs--overview">
              <button
                className="panel-tab panel-tab--active"
                data-workbench-model-tools-page="overview"
                onClick={() => onToolsPageChange("overview")}
                type="button"
              >
                {toolsPageOverviewLabel}
              </button>
            </div>
          ) : (
            <div className="panel-tabs panel-tabs--wide">
              <button
                className="panel-tab"
                data-workbench-model-tools-page="overview"
                onClick={() => onToolsPageChange("overview")}
                type="button"
              >
                {toolsPageOverviewLabel}
              </button>
              {studyContent ? (
                <button
                  className={`panel-tab${toolsPage === "study" ? " panel-tab--active" : ""}`}
                  data-workbench-model-tools-page="study"
                  onClick={() => onToolsPageChange("study")}
                  type="button"
                >
                  {toolsPageStudyLabel}
                </button>
              ) : null}
              <button
                className={`panel-tab${toolsPage === "studio" ? " panel-tab--active" : ""}`}
                data-workbench-model-tools-page="studio"
                onClick={() => onToolsPageChange("studio")}
                type="button"
              >
                {toolsPageStudioLabel}
              </button>
              {materialsContent ? (
                <button
                  className={`panel-tab${toolsPage === "materials" ? " panel-tab--active" : ""}`}
                  data-workbench-model-tools-page="materials"
                  onClick={() => onToolsPageChange("materials")}
                  type="button"
                >
                  {toolsPageMaterialsLabel}
                </button>
              ) : null}
              {generateContent ? (
                <button
                  className={`panel-tab${toolsPage === "generate" ? " panel-tab--active" : ""}`}
                  data-workbench-model-tools-page="generate"
                  onClick={() => onToolsPageChange("generate")}
                  type="button"
                >
                  {toolsPageGenerateLabel}
                </button>
              ) : null}
            </div>
          )}
          {toolsPage === "overview" ? (
            <WorkbenchRouteJourney
              steps={[
                ...(studyContent ? [{
                  id: "study",
                  title: toolsPageStudyLabel,
                  hint: studyOverviewHint,
                  automation: { "data-workbench-model-tools-page": "study" },
                  onOpen: () => onToolsPageChange("study"),
                }] : []),
                {
                  id: "studio",
                  title: toolsPageStudioLabel,
                  hint: studioOverviewHint,
                  automation: { "data-workbench-model-tools-page": "studio" },
                  onOpen: () => onToolsPageChange("studio"),
                },
                ...(materialsContent ? [{
                  id: "materials",
                  title: toolsPageMaterialsLabel,
                  hint: materialsOverviewHint,
                  automation: { "data-workbench-model-tools-page": "materials" },
                  onOpen: () => onToolsPageChange("materials"),
                }] : []),
                ...(generateContent ? [{
                  id: "generate",
                  title: toolsPageGenerateLabel,
                  hint: generateOverviewHint,
                  automation: { "data-workbench-model-tools-page": "generate" },
                  onOpen: () => onToolsPageChange("generate"),
                }] : []),
                ...(treeContent ? [{
                  id: "tree",
                  title: treeTabLabel,
                  hint: browseOverviewHint,
                  automation: { "data-workbench-model-tab": "tree" },
                  onOpen: () => onModelTabChange("tree"),
                }] : []),
              ]}
            />
          ) : null}
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
