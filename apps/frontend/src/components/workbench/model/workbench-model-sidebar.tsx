"use client";

import type { ReactNode } from "react";

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
              className={`panel-tab${toolsPage === "overview" ? " panel-tab--active" : ""}`}
              onClick={() => onToolsPageChange("overview")}
              type="button"
            >
              {toolsPageOverviewLabel}
            </button>
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
          {toolsPage === "overview" ? (
            <div className="runtime-overview-grid">
              {studyContent ? (
                <section className="sidebar-card sidebar-card--compact runtime-overview-card">
                  <div className="card-head">
                    <h2>{toolsPageStudyLabel}</h2>
                  </div>
                  <p className="card-copy">{studyOverviewHint}</p>
                  <div className="button-row">
                    <button onClick={() => onToolsPageChange("study")} type="button">
                      {toolsPageStudyLabel}
                    </button>
                  </div>
                </section>
              ) : null}
              <section className="sidebar-card sidebar-card--compact runtime-overview-card">
                <div className="card-head">
                  <h2>{toolsPageStudioLabel}</h2>
                </div>
                <p className="card-copy">{studioOverviewHint}</p>
                <div className="button-row">
                  <button onClick={() => onToolsPageChange("studio")} type="button">
                    {toolsPageStudioLabel}
                  </button>
                </div>
              </section>
              {materialsContent ? (
                <section className="sidebar-card sidebar-card--compact runtime-overview-card">
                  <div className="card-head">
                    <h2>{toolsPageMaterialsLabel}</h2>
                  </div>
                  <p className="card-copy">{materialsOverviewHint}</p>
                  <div className="button-row">
                    <button onClick={() => onToolsPageChange("materials")} type="button">
                      {toolsPageMaterialsLabel}
                    </button>
                  </div>
                </section>
              ) : null}
              {generateContent ? (
                <section className="sidebar-card sidebar-card--compact runtime-overview-card">
                  <div className="card-head">
                    <h2>{toolsPageGenerateLabel}</h2>
                  </div>
                  <p className="card-copy">{generateOverviewHint}</p>
                  <div className="button-row">
                    <button onClick={() => onToolsPageChange("generate")} type="button">
                      {toolsPageGenerateLabel}
                    </button>
                  </div>
                </section>
              ) : null}
              {treeContent ? (
                <section className="sidebar-card sidebar-card--compact runtime-overview-card">
                  <div className="card-head">
                    <h2>{treeTabLabel}</h2>
                  </div>
                  <p className="card-copy">{browseOverviewHint}</p>
                  <div className="button-row">
                    <button onClick={() => onModelTabChange("tree")} type="button">
                      {treeTabLabel}
                    </button>
                  </div>
                </section>
              ) : null}
            </div>
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
