"use client";

import { useEffect, useState, type ReactNode } from "react";

type SystemPanelTab = "config" | "scripts" | "runtime" | "data";
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
  const [surfaceTab, setSurfaceTab] = useState<SystemSurfaceTab>(
    systemPanelTab === "runtime" || systemPanelTab === "data" ? systemPanelTab : "settings",
  );
  const [settingsPage, setSettingsPage] = useState<SettingsPage>("overview");

  useEffect(() => {
    if (systemPanelTab === "runtime" || systemPanelTab === "data") {
      setSurfaceTab(systemPanelTab);
      return;
    }
    setSurfaceTab("settings");
    setSettingsPage(systemPanelTab);
  }, [systemPanelTab]);

  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--editor">
        <button
          className={`panel-tab${surfaceTab === "settings" ? " panel-tab--active" : ""}`}
          onClick={() => {
            setSurfaceTab("settings");
            setSettingsPage("overview");
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
              className={`panel-tab${settingsPage === "overview" ? " panel-tab--active" : ""}`}
              onClick={() => setSettingsPage("overview")}
              type="button"
            >
              {overviewPageLabel}
            </button>
            <button
              className={`panel-tab${settingsPage === "config" ? " panel-tab--active" : ""}`}
              onClick={() => {
                setSettingsPage("config");
                onSystemPanelTabChange("config");
              }}
              type="button"
            >
              {configPageLabel}
            </button>
            <button
              className={`panel-tab${settingsPage === "scripts" ? " panel-tab--active" : ""}`}
              onClick={() => {
                setSettingsPage("scripts");
                onSystemPanelTabChange("scripts");
              }}
              type="button"
            >
              {scriptsPageLabel}
            </button>
          </div>
          {settingsPage === "overview" ? (
            <div className="runtime-overview-grid">
              <section className="sidebar-card sidebar-card--compact runtime-overview-card">
                <div className="card-head">
                  <h2>{configPageLabel}</h2>
                </div>
                <p className="card-copy">{configOverviewHint}</p>
                <div className="button-row">
                  <button
                    onClick={() => {
                      setSettingsPage("config");
                      onSystemPanelTabChange("config");
                    }}
                    type="button"
                  >
                    {configPageLabel}
                  </button>
                </div>
              </section>
              <section className="sidebar-card sidebar-card--compact runtime-overview-card">
                <div className="card-head">
                  <h2>{scriptsPageLabel}</h2>
                </div>
                <p className="card-copy">{scriptsOverviewHint}</p>
                <div className="button-row">
                  <button
                    onClick={() => {
                      setSettingsPage("scripts");
                      onSystemPanelTabChange("scripts");
                    }}
                    type="button"
                  >
                    {scriptsPageLabel}
                  </button>
                </div>
              </section>
            </div>
          ) : null}
          {settingsPage === "config" ? configContent : null}
          {settingsPage === "scripts" ? scriptsContent : null}
        </section>
      ) : null}

      {surfaceTab === "runtime" ? runtimeContent : null}
      {surfaceTab === "data" ? dataContent : null}
    </div>
  );
}
