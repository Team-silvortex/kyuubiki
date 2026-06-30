"use client";

import type { SidebarSection } from "@/components/workbench/workbench-types";

type RailItem = {
  key: SidebarSection;
  symbol: string;
  label: string;
};

type WorkbenchAppRailProps = {
  shortTitle: string;
  railItems: RailItem[];
  sidebarSection: SidebarSection;
  onSidebarSectionChange: (section: SidebarSection) => void;
  assistantLabel: string;
  assistantOpen: boolean;
  onAssistantToggle: () => void;
};

export function WorkbenchAppRail({
  shortTitle,
  railItems,
  sidebarSection,
  onSidebarSectionChange,
  assistantLabel,
  assistantOpen,
  onAssistantToggle,
}: WorkbenchAppRailProps) {
  return (
    <aside className="app-rail panel">
      <div className="rail-brand">
        <img alt={`${shortTitle} mark`} className="rail-brand__mark" src="/kyuubiki.png" />
        <strong>{shortTitle}</strong>
        <span>tamamono 1.13.0</span>
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
      <div className="rail-utility">
        <button
          aria-expanded={assistantOpen}
          className={`rail-button rail-button--assistant${assistantOpen ? " rail-button--active" : ""}`}
          onClick={onAssistantToggle}
          type="button"
        >
          <span>A</span>
          <small>{assistantLabel}</small>
        </button>
      </div>
    </aside>
  );
}
