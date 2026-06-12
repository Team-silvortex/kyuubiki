"use client";

import type { ReactNode } from "react";

import type { SidebarSection } from "@/components/workbench/workbench-types";

type RailItem = {
  key: SidebarSection;
  symbol: string;
  label: string;
};

type WorkbenchSidebarMountProps = {
  shortTitle: string;
  roleLabel: string;
  title: string;
  subtitle: string;
  railItems: RailItem[];
  sidebarSection: SidebarSection;
  onSidebarSectionChange: (section: SidebarSection) => void;
  studySection?: ReactNode;
  modelSection?: ReactNode;
  workflowSection?: ReactNode;
  librarySection?: ReactNode;
  systemSection?: ReactNode;
};

export function WorkbenchSidebarMount({
  shortTitle,
  roleLabel,
  title,
  subtitle,
  railItems,
  sidebarSection,
  onSidebarSectionChange,
  studySection,
  modelSection,
  workflowSection,
  librarySection,
  systemSection,
}: WorkbenchSidebarMountProps) {
  return (
    <>
      <aside className="app-rail panel">
        <div className="rail-brand">
          <img alt={`${shortTitle} mark`} className="rail-brand__mark" src="/kyuubiki.png" />
          <strong>{shortTitle}</strong>
          <span>tamamono 1.6.0</span>
        </div>
        <div className="rail-nav">
          {railItems.map((item) => (
            <button
              key={item.key}
              className={`rail-button${sidebarSection === item.key ? " rail-button--active" : ""}`}
              onClick={() => onSidebarSectionChange(item.key)}
              type="button"
            >
              <span>{item.symbol}</span>
              <small>{item.label}</small>
            </button>
          ))}
        </div>
      </aside>

      <aside className="workspace-sidebar panel">
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
        {sidebarSection === "library" ? librarySection : null}
        {sidebarSection === "system" ? systemSection : null}
      </aside>
    </>
  );
}
