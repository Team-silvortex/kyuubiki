"use client";

import type { ReactNode } from "react";

import { WorkbenchAppRail } from "@/components/workbench/workbench-app-rail";
import { WorkbenchSidebarPanel } from "@/components/workbench/workbench-sidebar-panel";
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
  assistantLabel?: string;
  assistantOpen?: boolean;
  onAssistantToggle?: () => void;
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
  assistantLabel = "Assistant",
  assistantOpen = false,
  onAssistantToggle = () => {},
}: WorkbenchSidebarMountProps) {
  return (
    <>
      <WorkbenchAppRail
        shortTitle={shortTitle}
        railItems={railItems}
        sidebarSection={sidebarSection}
        onSidebarSectionChange={onSidebarSectionChange}
        assistantLabel={assistantLabel}
        assistantOpen={assistantOpen}
        onAssistantToggle={onAssistantToggle}
      />
      <WorkbenchSidebarPanel
        shortTitle={shortTitle}
        roleLabel={roleLabel}
        title={title}
        subtitle={subtitle}
        sidebarSection={sidebarSection}
        studySection={studySection}
        modelSection={modelSection}
        workflowSection={workflowSection}
        librarySection={librarySection}
        systemSection={systemSection}
      />
    </>
  );
}
