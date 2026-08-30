"use client";

import type { ReactNode } from "react";
import { WorkbenchRouteJourney } from "@/components/workbench/workbench-route-journey";

type SystemPanelTab = "overview" | "config" | "scripts" | "runtime" | "data";
type SystemSurfaceTab = "settings" | "runtime" | "data";
type SettingsPage = "overview" | "config" | "scripts";

type WorkbenchSystemSidebarProps = {
  systemPanelTab: SystemPanelTab;
  onSystemPanelTabChange: (tab: SystemPanelTab) => void;
  settingsTabLabel: string;
  overviewPageLabel: string;
  configPageLabel: string;
  scriptsPageLabel: string;
  runtimeTabLabel: string;
  dataTabLabel: string;
  configOverviewHint: string;
  scriptsOverviewHint: string;
  configContent?: ReactNode;
  scriptsContent?: ReactNode;
  runtimeContent?: ReactNode;
  dataContent?: ReactNode;
};

export function WorkbenchSystemSidebar({
  systemPanelTab,
  onSystemPanelTabChange,
  settingsTabLabel,
  overviewPageLabel,
  configPageLabel,
  scriptsPageLabel,
  runtimeTabLabel,
  dataTabLabel,
  configOverviewHint,
  scriptsOverviewHint,
  configContent,
  scriptsContent,
  runtimeContent,
  dataContent,
}: WorkbenchSystemSidebarProps) {
  const surfaceTab: SystemSurfaceTab =
    systemPanelTab === "runtime" || systemPanelTab === "data" ? systemPanelTab : "settings";
  const settingsPage: SettingsPage =
    systemPanelTab === "overview" || systemPanelTab === "config" || systemPanelTab === "scripts"
      ? systemPanelTab
      : "overview";

  return (
    <div className="sidebar-stack panel-scroll-window" data-workbench-system-sidebar="root">
      <div className="panel-tabs panel-tabs--editor">
        <button
          className={`panel-tab${surfaceTab === "settings" ? " panel-tab--active" : ""}`}
          data-workbench-system-surface-tab="settings"
          onClick={() => onSystemPanelTabChange("overview")}
          type="button"
        >
          {settingsTabLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "runtime" ? " panel-tab--active" : ""}`}
          data-workbench-system-surface-tab="runtime"
          onClick={() => onSystemPanelTabChange("runtime")}
          type="button"
        >
          {runtimeTabLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "data" ? " panel-tab--active" : ""}`}
          data-workbench-system-surface-tab="data"
          onClick={() => onSystemPanelTabChange("data")}
          type="button"
        >
          {dataTabLabel}
        </button>
      </div>

      {surfaceTab === "settings" ? (
        settingsPage === "overview" ? (
          <>
            <div className="panel-tabs panel-tabs--overview">
              <button
                className="panel-tab panel-tab--active"
                data-workbench-system-settings-page="overview"
                onClick={() => onSystemPanelTabChange("overview")}
                type="button"
              >
                {overviewPageLabel}
              </button>
            </div>
            <WorkbenchRouteJourney
              steps={[
                {
                  id: "config",
                  title: configPageLabel,
                  hint: configOverviewHint,
                  automation: { "data-workbench-system-settings-page": "config" },
                  onOpen: () => onSystemPanelTabChange("config"),
                },
                {
                  id: "scripts",
                  title: scriptsPageLabel,
                  hint: scriptsOverviewHint,
                  automation: { "data-workbench-system-settings-page": "scripts" },
                  onOpen: () => onSystemPanelTabChange("scripts"),
                },
              ]}
            />
          </>
        ) : (
          <section className="sidebar-card sidebar-card--compact">
            <div className="panel-tabs panel-tabs--wide">
              <button
                className="panel-tab"
                data-workbench-system-settings-page="overview"
                onClick={() => onSystemPanelTabChange("overview")}
                type="button"
              >
                {overviewPageLabel}
              </button>
              <button
                className={`panel-tab${settingsPage === "config" ? " panel-tab--active" : ""}`}
                data-workbench-system-settings-page="config"
                onClick={() => onSystemPanelTabChange("config")}
                type="button"
              >
                {configPageLabel}
              </button>
              <button
                className={`panel-tab${settingsPage === "scripts" ? " panel-tab--active" : ""}`}
                data-workbench-system-settings-page="scripts"
                onClick={() => onSystemPanelTabChange("scripts")}
                type="button"
              >
                {scriptsPageLabel}
              </button>
            </div>
            {settingsPage === "config" ? configContent : null}
            {settingsPage === "scripts" ? scriptsContent : null}
          </section>
        )
      ) : null}

      {surfaceTab === "runtime" ? runtimeContent : null}
      {surfaceTab === "data" ? dataContent : null}
    </div>
  );
}
