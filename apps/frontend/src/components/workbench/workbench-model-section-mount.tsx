"use client";

import type { ReactNode } from "react";

import { WorkbenchModelSidebar, type ModelToolsPage } from "@/components/workbench/model/workbench-model-sidebar";

type ModelPanelTab = "tools" | "tree";

type WorkbenchModelSectionMountProps = {
  modelTab: ModelPanelTab;
  onModelTabChange: (tab: ModelPanelTab) => void;
  toolsPage: ModelToolsPage;
  onToolsPageChange: (page: ModelToolsPage) => void;
  isTruss3d: boolean;
  toolsTabLabel: string;
  treeTabLabel: string;
  toolsPageOverviewLabel: string;
  toolsPageStudyLabel: string;
  toolsPageStudioLabel: string;
  toolsPageMaterialsLabel: string;
  toolsPageGenerateLabel: string;
  studyOverviewHint: string;
  studioOverviewHint: string;
  materialsOverviewHint: string;
  generateOverviewHint: string;
  browseOverviewHint: string;
  studyContent?: ReactNode;
  studioContent?: ReactNode;
  materialsContent?: ReactNode;
  generateContent?: ReactNode;
  treeContent?: ReactNode;
};

export function WorkbenchModelSectionMount({
  modelTab,
  onModelTabChange,
  toolsPage,
  onToolsPageChange,
  isTruss3d,
  toolsTabLabel,
  treeTabLabel,
  toolsPageOverviewLabel,
  toolsPageStudyLabel,
  toolsPageStudioLabel,
  toolsPageMaterialsLabel,
  toolsPageGenerateLabel,
  studyOverviewHint,
  studioOverviewHint,
  materialsOverviewHint,
  generateOverviewHint,
  browseOverviewHint,
  studyContent,
  studioContent,
  materialsContent,
  generateContent,
  treeContent,
}: WorkbenchModelSectionMountProps) {
  return (
    <WorkbenchModelSidebar
      modelTab={modelTab}
      onModelTabChange={onModelTabChange}
      toolsPage={toolsPage}
      onToolsPageChange={onToolsPageChange}
      isTruss3d={isTruss3d}
      toolsTabLabel={toolsTabLabel}
      treeTabLabel={treeTabLabel}
      toolsPageOverviewLabel={toolsPageOverviewLabel}
      toolsPageStudyLabel={toolsPageStudyLabel}
      toolsPageStudioLabel={toolsPageStudioLabel}
      toolsPageMaterialsLabel={toolsPageMaterialsLabel}
      toolsPageGenerateLabel={toolsPageGenerateLabel}
      studyOverviewHint={studyOverviewHint}
      studioOverviewHint={studioOverviewHint}
      materialsOverviewHint={materialsOverviewHint}
      generateOverviewHint={generateOverviewHint}
      browseOverviewHint={browseOverviewHint}
      studyContent={studyContent}
      studioContent={studioContent}
      materialsContent={materialsContent}
      generateContent={generateContent}
      treeContent={treeContent}
    />
  );
}
