"use client";

import type { ReactNode } from "react";

import type { SidebarSection } from "@/components/workbench/workbench-types";

type WorkbenchSidebarPanelProps = {
  shortTitle: string;
  roleLabel: string;
  title: string;
  subtitle: string;
  sidebarSection: SidebarSection;
  studySection?: ReactNode;
  modelSection?: ReactNode;
  workflowSection?: ReactNode;
  storeSection?: ReactNode;
  librarySection?: ReactNode;
  systemSection?: ReactNode;
};

export function WorkbenchSidebarPanel({
  shortTitle,
  roleLabel,
  title,
  subtitle,
  sidebarSection,
  studySection,
  modelSection,
  workflowSection,
  storeSection,
  librarySection,
  systemSection,
}: WorkbenchSidebarPanelProps) {
  return (
    <aside
      className="workspace-sidebar panel"
      data-workbench-panel="sidebar"
      data-workbench-sidebar-section={sidebarSection}
      data-workbench-surface="built-in"
    >
      <div className="sidebar-header">
        <div className="sidebar-header__brand">
          <img alt={`${shortTitle} mark`} className="sidebar-header__mark" src="/kyuubiki.png" />
          <p className="eyebrow">{roleLabel}</p>
        </div>
        <h1>{title}</h1>
        <p>{subtitle}</p>
      </div>

      {sidebarSection === "study" ? studySection : null}
      {sidebarSection === "model" ? modelSection : null}
      {sidebarSection === "workflow" ? workflowSection : null}
      {sidebarSection === "store" ? storeSection : null}
      {sidebarSection === "library" ? librarySection : null}
      {sidebarSection === "system" ? systemSection : null}
    </aside>
  );
}
