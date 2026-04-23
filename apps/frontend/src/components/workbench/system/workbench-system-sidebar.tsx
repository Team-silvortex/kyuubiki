"use client";

import type { ReactNode } from "react";

type SystemPanelTab = "config" | "assistant" | "scripts" | "runtime" | "data";

type WorkbenchSystemSidebarProps = {
  systemPanelTab: SystemPanelTab;
  onSystemPanelTabChange: (tab: SystemPanelTab) => void;
  configTabLabel: string;
  assistantTabLabel: string;
  scriptsTabLabel: string;
  runtimeTabLabel: string;
  dataTabLabel: string;
  configContent?: ReactNode;
  assistantContent?: ReactNode;
  scriptsContent?: ReactNode;
  runtimeContent?: ReactNode;
  dataContent?: ReactNode;
};

export function WorkbenchSystemSidebar({
  systemPanelTab,
  onSystemPanelTabChange,
  configTabLabel,
  assistantTabLabel,
  scriptsTabLabel,
  runtimeTabLabel,
  dataTabLabel,
  configContent,
  assistantContent,
  scriptsContent,
  runtimeContent,
  dataContent,
}: WorkbenchSystemSidebarProps) {
  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--editor">
        <button
          className={`panel-tab${systemPanelTab === "config" ? " panel-tab--active" : ""}`}
          onClick={() => onSystemPanelTabChange("config")}
          type="button"
        >
          {configTabLabel}
        </button>
        <button
          className={`panel-tab${systemPanelTab === "assistant" ? " panel-tab--active" : ""}`}
          onClick={() => onSystemPanelTabChange("assistant")}
          type="button"
        >
          {assistantTabLabel}
        </button>
        <button
          className={`panel-tab${systemPanelTab === "scripts" ? " panel-tab--active" : ""}`}
          onClick={() => onSystemPanelTabChange("scripts")}
          type="button"
        >
          {scriptsTabLabel}
        </button>
        <button
          className={`panel-tab${systemPanelTab === "runtime" ? " panel-tab--active" : ""}`}
          onClick={() => onSystemPanelTabChange("runtime")}
          type="button"
        >
          {runtimeTabLabel}
        </button>
        <button
          className={`panel-tab${systemPanelTab === "data" ? " panel-tab--active" : ""}`}
          onClick={() => onSystemPanelTabChange("data")}
          type="button"
        >
          {dataTabLabel}
        </button>
      </div>

      {systemPanelTab === "config" ? configContent : null}
      {systemPanelTab === "assistant" ? assistantContent : null}
      {systemPanelTab === "scripts" ? scriptsContent : null}
      {systemPanelTab === "runtime" ? runtimeContent : null}
      {systemPanelTab === "data" ? dataContent : null}
    </div>
  );
}
