"use client";

import type { ModelToolsPage } from "@/components/workbench/model/workbench-model-sidebar";
import type { ModelPanelTab, SidebarSection } from "@/components/workbench/workbench-types";

type CopyShape = {
  immersiveStudy: string;
  immersiveModel: string;
  immersiveLibrary: string;
  immersiveTools: string;
  immersiveHelp: string;
  enterImmersive: string;
  exitImmersive: string;
};

type WorkbenchViewportHeadActionsProps = {
  t: CopyShape;
  isTruss3d: boolean;
  immersiveViewport: boolean;
  immersiveToolDrawerOpen: boolean;
  immersiveHelpDrawerOpen: boolean;
  sidebarSection: SidebarSection;
  modelTab: ModelPanelTab;
  modelToolsPage: ModelToolsPage;
  jobStatus?: string | null;
  handleSidebarSectionChange: (section: SidebarSection) => void;
  setModelTab: (tab: ModelPanelTab) => void;
  setModelToolsPage: (page: ModelToolsPage) => void;
  handleToggleImmersiveToolDrawer: () => void;
  handleToggleImmersiveHelpDrawer: () => void;
  handleToggleImmersiveViewport: () => void | Promise<void>;
};

export function WorkbenchViewportHeadActions({
  t,
  isTruss3d,
  immersiveViewport,
  immersiveToolDrawerOpen,
  immersiveHelpDrawerOpen,
  sidebarSection,
  modelTab,
  modelToolsPage,
  jobStatus,
  handleSidebarSectionChange,
  setModelTab,
  setModelToolsPage,
  handleToggleImmersiveToolDrawer,
  handleToggleImmersiveHelpDrawer,
  handleToggleImmersiveViewport,
}: WorkbenchViewportHeadActionsProps) {
  return (
    <>
      {isTruss3d && immersiveViewport ? (
        <div className="immersive-switches">
          <button
            className={`ghost-button ghost-button--compact${sidebarSection === "model" && modelTab === "tools" && modelToolsPage === "study" ? " ghost-button--active" : ""}`}
            onClick={() => {
              handleSidebarSectionChange("model");
              setModelTab("tools");
              setModelToolsPage("study");
            }}
            type="button"
          >
            {t.immersiveStudy}
          </button>
          <button
            className={`ghost-button ghost-button--compact${sidebarSection === "model" && modelTab === "tools" && modelToolsPage !== "study" ? " ghost-button--active" : ""}`}
            onClick={() => {
              handleSidebarSectionChange("model");
              setModelTab("tools");
              if (modelToolsPage === "study") setModelToolsPage("studio");
            }}
            type="button"
          >
            {t.immersiveModel}
          </button>
          <button
            className={`ghost-button ghost-button--compact${sidebarSection === "library" ? " ghost-button--active" : ""}`}
            onClick={() => handleSidebarSectionChange(sidebarSection === "library" ? "model" : "library")}
            type="button"
          >
            {t.immersiveLibrary}
          </button>
          <button
            className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen ? " ghost-button--active" : ""}`}
            onClick={handleToggleImmersiveToolDrawer}
            type="button"
          >
            {t.immersiveTools}
          </button>
          <button
            className={`ghost-button ghost-button--compact${immersiveHelpDrawerOpen ? " ghost-button--active" : ""}`}
            onClick={handleToggleImmersiveHelpDrawer}
            type="button"
          >
            {t.immersiveHelp}
          </button>
        </div>
      ) : null}
      {isTruss3d && !immersiveViewport ? (
        <div className="immersive-switches">
          <button
            className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen ? " ghost-button--active" : ""}`}
            onClick={handleToggleImmersiveToolDrawer}
            type="button"
          >
            {t.immersiveTools}
          </button>
          <button
            className={`ghost-button ghost-button--compact${immersiveHelpDrawerOpen ? " ghost-button--active" : ""}`}
            onClick={handleToggleImmersiveHelpDrawer}
            type="button"
          >
            {t.immersiveHelp}
          </button>
        </div>
      ) : null}
      {isTruss3d ? (
        <button className={`ghost-button ghost-button--compact${immersiveViewport ? " ghost-button--active" : ""}`} onClick={() => void handleToggleImmersiveViewport()} type="button">
          {immersiveViewport ? t.exitImmersive : t.enterImmersive}
        </button>
      ) : null}
      <span>{jobStatus ?? "idle"}</span>
    </>
  );
}
