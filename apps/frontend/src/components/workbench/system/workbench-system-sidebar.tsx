"use client";

import { useEffect, useState, type ReactNode } from "react";

type SystemPanelTab = "config" | "scripts" | "runtime" | "data";
type SystemSurfaceTab = "settings" | "runtime" | "data";

type WorkbenchSystemSidebarProps = {
  systemPanelTab: SystemPanelTab;
  onSystemPanelTabChange: (tab: SystemPanelTab) => void;
  settingsTabLabel: string;
  configPageLabel: string;
  scriptsPageLabel: string;
  runtimeTabLabel: string;
  dataTabLabel: string;
  configContent?: ReactNode;
  scriptsContent?: ReactNode;
  runtimeContent?: ReactNode;
  dataContent?: ReactNode;
};

export function WorkbenchSystemSidebar({
  systemPanelTab,
  onSystemPanelTabChange,
  settingsTabLabel,
  configPageLabel,
  scriptsPageLabel,
  runtimeTabLabel,
  dataTabLabel,
  configContent,
  scriptsContent,
  runtimeContent,
  dataContent,
}: WorkbenchSystemSidebarProps) {
  const [surfaceTab, setSurfaceTab] = useState<SystemSurfaceTab>(
    systemPanelTab === "runtime" || systemPanelTab === "data" ? systemPanelTab : "settings",
  );

  useEffect(() => {
    if (systemPanelTab === "runtime" || systemPanelTab === "data") {
      setSurfaceTab(systemPanelTab);
      return;
    }
    setSurfaceTab("settings");
  }, [systemPanelTab]);

  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--editor">
        <button
          className={`panel-tab${surfaceTab === "settings" ? " panel-tab--active" : ""}`}
          onClick={() => {
            setSurfaceTab("settings");
            onSystemPanelTabChange(systemPanelTab === "scripts" ? "scripts" : "config");
          }}
          type="button"
        >
          {settingsTabLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "runtime" ? " panel-tab--active" : ""}`}
          onClick={() => {
            setSurfaceTab("runtime");
            onSystemPanelTabChange("runtime");
          }}
          type="button"
        >
          {runtimeTabLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "data" ? " panel-tab--active" : ""}`}
          onClick={() => {
            setSurfaceTab("data");
            onSystemPanelTabChange("data");
          }}
          type="button"
        >
          {dataTabLabel}
        </button>
      </div>

      {surfaceTab === "settings" ? (
        <section className="sidebar-card sidebar-card--compact">
          <div className="panel-tabs panel-tabs--wide">
            <button
              className={`panel-tab${systemPanelTab === "config" ? " panel-tab--active" : ""}`}
              onClick={() => onSystemPanelTabChange("config")}
              type="button"
            >
              {configPageLabel}
            </button>
            <button
              className={`panel-tab${systemPanelTab === "scripts" ? " panel-tab--active" : ""}`}
              onClick={() => onSystemPanelTabChange("scripts")}
              type="button"
            >
              {scriptsPageLabel}
            </button>
          </div>
          {systemPanelTab === "config" ? configContent : null}
          {systemPanelTab === "scripts" ? scriptsContent : null}
        </section>
      ) : null}

      {surfaceTab === "runtime" ? runtimeContent : null}
      {surfaceTab === "data" ? dataContent : null}
    </div>
  );
}
