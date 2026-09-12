"use client";

import type { ModelToolsPage } from "@/components/workbench/model/workbench-model-sidebar";
import type { ImmersiveToolTab, ModelPanelTab, SidebarSection } from "@/components/workbench/workbench-types";

type CopyShape = {
  immersiveStudy: string;
  immersiveModel: string;
  immersiveLibrary: string;
  immersiveTools: string;
  immersiveHelp: string;
  enterImmersive: string;
  exitImmersive: string;
  ready: string;
  save: string;
};

type WorkbenchViewportHeadActionsProps = {
  t: CopyShape;
  isTruss3d: boolean;
  immersiveViewport: boolean;
  immersiveToolDrawerOpen: boolean;
  immersiveHelpDrawerOpen: boolean;
  immersiveToolTab: ImmersiveToolTab;
  setImmersiveToolTab: (tab: ImmersiveToolTab) => void;
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
  immersiveToolTab,
  setImmersiveToolTab,
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
  const openLibrary = async () => {
    await handleToggleImmersiveViewport();
    handleSidebarSectionChange("library");
  };
  return (
    <>
      {isTruss3d && immersiveViewport ? (
        <div className="immersive-switches">
          <button className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen && immersiveToolTab === "save" ? " ghost-button--active" : ""}`}
            data-workbench-immersive="save" onClick={() => setImmersiveToolTab("save")} type="button">{t.save}</button>
          <button
            className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen && immersiveToolTab === "study" ? " ghost-button--active" : ""}`}
            data-workbench-immersive="study"
            onClick={() => setImmersiveToolTab("study")}
            type="button"
          >
            {t.immersiveStudy}
          </button>
          <button
            className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen && immersiveToolTab !== "study" && immersiveToolTab !== "save" ? " ghost-button--active" : ""}`}
            data-workbench-immersive="model"
            onClick={() => setImmersiveToolTab("batch")}
            type="button"
          >
            {t.immersiveModel}
          </button>
          <button
            className={`ghost-button ghost-button--compact${sidebarSection === "library" ? " ghost-button--active" : ""}`}
            data-workbench-immersive="library" title={`${t.exitImmersive} / ${t.immersiveLibrary}`}
            onClick={() => void openLibrary()}
            type="button"
          >
            {t.immersiveLibrary}
          </button>
          <button
            className={`ghost-button ghost-button--compact${immersiveToolDrawerOpen ? " ghost-button--active" : ""}`}
            data-workbench-immersive="toggle-tools"
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
            className={`ghost-button ghost-button--compact${sidebarSection === "model" && modelTab === "tools" && modelToolsPage === "studio" ? " ghost-button--active" : ""}`}
            onClick={() => {
              handleSidebarSectionChange("model");
              setModelTab("tools");
              setModelToolsPage("studio");
            }}
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
        <button data-workbench-immersive="toggle" className={`ghost-button ghost-button--compact${immersiveViewport ? " ghost-button--active" : ""}`} onClick={() => void handleToggleImmersiveViewport()} type="button">
          {immersiveViewport ? t.exitImmersive : t.enterImmersive}
        </button>
      ) : null}
      <span>{jobStatus ?? t.ready}</span>
    </>
  );
}
