"use client";

import type {
  LibraryPanelTab,
  ModelPanelTab,
  SidebarSection,
  StudyPanelTab,
  SystemDataTab,
  SystemPanelTab,
  WorkflowPanelTab,
} from "@/components/workbench/workbench-types";
import type { ModelToolsPage } from "@/components/workbench/model/workbench-model-sidebar";

type WorkbenchUiActionControllerDeps = {
  workflowCatalogLength: number;
  workflowCatalogBusy: boolean;
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  adminFilterProjectId: string;
  adminFilterModelVersionId: string;
  systemDataTab: SystemDataTab;
  truss3dLinkMode: boolean;
  immersiveViewport: boolean;
  setSidebarSection: (value: SidebarSection) => void;
  setStudyTab: (value: StudyPanelTab) => void;
  setModelTab: (value: ModelPanelTab) => void;
  setModelToolsPage: (value: ModelToolsPage) => void;
  setLibraryTab: (value: LibraryPanelTab) => void;
  setWorkflowPanelTab: (value: WorkflowPanelTab) => void;
  setSystemPanelTab: (value: SystemPanelTab) => void;
  setSystemDataTab: (value: SystemDataTab) => void;
  setAdminFilterProjectId: (value: string) => void;
  setAdminFilterModelVersionId: (value: string) => void;
  setSelectedAdminJobId: (value: string) => void;
  setSelectedAdminResultJobId: (value: string) => void;
  setTruss3dViewPreset: (value: "iso" | "front" | "right" | "top") => void;
  setTruss3dProjectionMode: (value: "ortho" | "persp") => void;
  setTruss3dBoxSelectMode: (value: boolean) => void;
  setTruss3dShowGrid: (value: boolean) => void;
  setTruss3dShowLabels: (value: boolean) => void;
  setTruss3dShowNodes: (value: boolean) => void;
  setImmersiveToolDrawerOpen: (value: React.SetStateAction<boolean>) => void;
  setImmersiveHelpDrawerOpen: (value: React.SetStateAction<boolean>) => void;
  setTruss3dFocusRequestVersion: (updater: (current: number) => number) => void;
  setTruss3dResetRequestVersion: (updater: (current: number) => number) => void;
  refreshWorkflowCatalog: () => Promise<void>;
  recordManualDslAction: (action: string, payload: Record<string, unknown>) => void;
  toggleTruss3dLinkMode: () => void;
  toggleImmersiveViewport: () => Promise<void>;
};

export function createWorkbenchUiActionController({
  workflowCatalogLength,
  workflowCatalogBusy,
  selectedProjectId,
  selectedVersionId,
  adminFilterProjectId,
  adminFilterModelVersionId,
  systemDataTab,
  truss3dLinkMode,
  immersiveViewport,
  setSidebarSection,
  setStudyTab,
  setModelTab,
  setModelToolsPage,
  setLibraryTab,
  setWorkflowPanelTab,
  setSystemPanelTab,
  setSystemDataTab,
  setAdminFilterProjectId,
  setAdminFilterModelVersionId,
  setSelectedAdminJobId,
  setSelectedAdminResultJobId,
  setTruss3dViewPreset,
  setTruss3dProjectionMode,
  setTruss3dBoxSelectMode,
  setTruss3dShowGrid,
  setTruss3dShowLabels,
  setTruss3dShowNodes,
  setImmersiveToolDrawerOpen,
  setImmersiveHelpDrawerOpen,
  setTruss3dFocusRequestVersion,
  setTruss3dResetRequestVersion,
  refreshWorkflowCatalog,
  recordManualDslAction,
  toggleTruss3dLinkMode,
  toggleImmersiveViewport,
}: WorkbenchUiActionControllerDeps) {
  const handleSidebarSectionChange = (section: SidebarSection) => {
    const nextSection = section === "study" ? "model" : section;
    setSidebarSection(nextSection);
    if (nextSection === "model") {
      setModelTab("tools");
    }
    if (nextSection === "workflow" && workflowCatalogLength === 0 && !workflowCatalogBusy) {
      void refreshWorkflowCatalog();
    }
    recordManualDslAction("nav/setSidebarSection", { section: nextSection });
  };

  const handleStudyTabChange = (tab: StudyPanelTab) => {
    setStudyTab(tab);
    recordManualDslAction("nav/setTabs", { studyTab: tab });
  };

  const handleModelTabChange = (tab: ModelPanelTab) => {
    setModelTab(tab);
    recordManualDslAction("nav/setTabs", { modelTab: tab });
  };

  const handleModelToolsPageChange = (page: ModelToolsPage, currentModelTab: ModelPanelTab) => {
    setModelToolsPage(page);
    recordManualDslAction("nav/setTabs", { modelTab: currentModelTab, modelToolsPage: page });
  };

  const handleLibraryTabChange = (tab: LibraryPanelTab) => {
    setLibraryTab(tab);
    if (tab === "samples" && workflowCatalogLength === 0 && !workflowCatalogBusy) {
      void refreshWorkflowCatalog();
    }
    recordManualDslAction("nav/setTabs", { libraryTab: tab });
  };

  const handleWorkflowPanelTabChange = (tab: WorkflowPanelTab) => {
    setWorkflowPanelTab(tab);
    if ((tab === "catalog" || tab === "builder") && workflowCatalogLength === 0 && !workflowCatalogBusy) {
      void refreshWorkflowCatalog();
    }
    recordManualDslAction("nav/setTabs", { workflowPanelTab: tab });
  };

  const handleSystemPanelTabChange = (tab: SystemPanelTab) => {
    setSystemPanelTab(tab);
    recordManualDslAction("nav/setTabs", { systemPanelTab: tab });
  };

  const handleSystemDataTabChange = (tab: SystemDataTab) => {
    setSystemDataTab(tab);
    recordManualDslAction("nav/setTabs", { systemPanelTab: "data", systemDataTab: tab });
  };

  const handleAdminFilterProjectChange = (value: string) => {
    setAdminFilterProjectId(value);
    recordManualDslAction("data/setFilters", { activeTab: systemDataTab, projectId: value, modelVersionId: adminFilterModelVersionId });
  };

  const handleAdminFilterModelVersionChange = (value: string) => {
    setAdminFilterModelVersionId(value);
    recordManualDslAction("data/setFilters", { activeTab: systemDataTab, projectId: adminFilterProjectId, modelVersionId: value });
  };

  const handleSelectAdminJob = (jobId: string) => {
    setSelectedAdminJobId(jobId);
    recordManualDslAction("data/selectRecord", { activeTab: "jobs", jobId });
  };

  const handleSelectAdminResult = (jobId: string) => {
    setSelectedAdminResultJobId(jobId);
    recordManualDslAction("data/selectRecord", { activeTab: "results", resultJobId: jobId });
  };

  const handleTruss3dViewPresetChange = (preset: "iso" | "front" | "right" | "top") => {
    setTruss3dViewPreset(preset);
    recordManualDslAction("viewport/set3dView", { preset });
  };

  const handleTruss3dProjectionModeChange = (mode: "ortho" | "persp") => {
    setTruss3dProjectionMode(mode);
    recordManualDslAction("viewport/set3dView", { projection: mode });
  };

  const handleTruss3dBoxSelectModeChange = (next: boolean) => {
    setTruss3dBoxSelectMode(next);
    recordManualDslAction("viewport/setUiState", { boxSelectMode: next });
  };

  const handleTruss3dShowGridChange = (next: boolean) => {
    setTruss3dShowGrid(next);
    recordManualDslAction("viewport/toggleFlags", { grid: next });
  };

  const handleTruss3dShowLabelsChange = (next: boolean) => {
    setTruss3dShowLabels(next);
    recordManualDslAction("viewport/toggleFlags", { labels: next });
  };

  const handleTruss3dShowNodesChange = (next: boolean) => {
    setTruss3dShowNodes(next);
    recordManualDslAction("viewport/toggleFlags", { nodes: next });
  };

  const handleToggleTruss3dLinkMode = () => {
    toggleTruss3dLinkMode();
    recordManualDslAction("viewport/setUiState", { linkMode: !truss3dLinkMode });
  };

  const handleToggleImmersiveViewport = async () => {
    await toggleImmersiveViewport();
    recordManualDslAction("viewport/setUiState", { immersiveViewport: !immersiveViewport });
  };

  const handleToggleImmersiveToolDrawer = () => {
    setImmersiveToolDrawerOpen((current) => {
      const next = !current;
      recordManualDslAction("viewport/setUiState", { toolDrawerOpen: next });
      return next;
    });
  };

  const handleToggleImmersiveHelpDrawer = () => {
    setImmersiveHelpDrawerOpen((current) => {
      const next = !current;
      recordManualDslAction("viewport/setUiState", { helpDrawerOpen: next });
      return next;
    });
  };

  const handleTruss3dFocusViewport = () => {
    setTruss3dFocusRequestVersion((current) => current + 1);
    recordManualDslAction("viewport/focus3d", {});
  };

  const handleTruss3dResetViewport = () => {
    setTruss3dResetRequestVersion((current) => current + 1);
    recordManualDslAction("viewport/reset3d", {});
  };

  const useCurrentProjectAsAdminFilter = () => {
    setAdminFilterProjectId(selectedProjectId ?? "");
    recordManualDslAction("data/setFilters", {
      activeTab: systemDataTab,
      projectId: selectedProjectId ?? "",
      modelVersionId: adminFilterModelVersionId,
    });
  };

  const useCurrentVersionAsAdminFilter = () => {
    setAdminFilterModelVersionId(selectedVersionId ?? "");
    recordManualDslAction("data/setFilters", {
      activeTab: systemDataTab,
      projectId: adminFilterProjectId,
      modelVersionId: selectedVersionId ?? "",
    });
  };

  const clearAdminFilters = () => {
    setAdminFilterProjectId("");
    setAdminFilterModelVersionId("");
    recordManualDslAction("data/setFilters", { activeTab: systemDataTab, projectId: "", modelVersionId: "" });
  };

  return {
    clearAdminFilters,
    handleAdminFilterModelVersionChange,
    handleAdminFilterProjectChange,
    handleLibraryTabChange,
    handleModelTabChange,
    handleModelToolsPageChange,
    handleSelectAdminJob,
    handleSelectAdminResult,
    handleSidebarSectionChange,
    handleStudyTabChange,
    handleSystemDataTabChange,
    handleSystemPanelTabChange,
    handleToggleImmersiveHelpDrawer,
    handleToggleImmersiveToolDrawer,
    handleToggleImmersiveViewport,
    handleToggleTruss3dLinkMode,
    handleTruss3dBoxSelectModeChange,
    handleTruss3dFocusViewport,
    handleTruss3dProjectionModeChange,
    handleTruss3dResetViewport,
    handleTruss3dShowGridChange,
    handleTruss3dShowLabelsChange,
    handleTruss3dShowNodesChange,
    handleTruss3dViewPresetChange,
    handleWorkflowPanelTabChange,
    useCurrentProjectAsAdminFilter,
    useCurrentVersionAsAdminFilter,
  };
}
