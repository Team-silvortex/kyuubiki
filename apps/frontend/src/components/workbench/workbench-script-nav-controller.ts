"use client";

import { applyStudyKindSelection, isWorkbenchStudyKind } from "@/components/workbench/workbench-study-kind-controller";
import type { WorkbenchStudyKind } from "@/lib/workbench/history";
import { applyWorkbenchGovernancePatch } from "@/lib/workbench/governance";

type ScriptNavControllerDeps = {
  action: string;
  payload: Record<string, unknown>;
  studyKind: WorkbenchStudyKind;
  studyKindResetHandlers: Partial<Record<WorkbenchStudyKind, () => void>>;
  setStudyKind: (value: WorkbenchStudyKind) => void;
  handleSidebarSectionChange: (section: "study" | "model" | "workflow" | "library" | "system") => void;
  recordHistory: (label: string) => void;
  changeStudyTypeLabel: string;
  setStudyTab: (value: "summary" | "controls") => void;
  setModelTab: (value: "tools" | "tree") => void;
  setModelToolsPage: (value: "overview" | "study" | "studio" | "materials" | "generate") => void;
  setLibraryTab: (value: "results" | "samples" | "projects" | "models" | "jobs") => void;
  setSystemPanelTab: (value: "config" | "scripts" | "runtime" | "data") => void;
  setAssistantWindowOpen: (value: boolean) => void;
  setSystemDataTab: (value: "jobs" | "results") => void;
  handleLanguageChange: (value: string) => void;
  setTheme: (value: "linen" | "marine" | "graphite") => void;
  currentFrontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  setFrontendRuntimeMode: (value: "orchestrated_gui" | "direct_mesh_gui") => void;
  currentDirectMeshEndpointsText: string;
  setDirectMeshEndpointsText: (value: string) => void;
  setDirectMeshSelectionMode: (value: "healthiest" | "first_reachable") => void;
  refreshHealth: () => Promise<void>;
  refreshJobHistory: () => Promise<void>;
  refreshResults: () => Promise<void>;
  refreshProjects: () => Promise<void>;
  refreshSecurityEvents: () => Promise<void>;
};

export async function handleWorkbenchScriptNavAction({
  action,
  payload,
  studyKind,
  studyKindResetHandlers,
  setStudyKind,
  handleSidebarSectionChange,
  recordHistory,
  changeStudyTypeLabel,
  setStudyTab,
  setModelTab,
  setModelToolsPage,
  setLibraryTab,
  setSystemPanelTab,
  setAssistantWindowOpen,
  setSystemDataTab,
  handleLanguageChange,
  setTheme,
  currentFrontendRuntimeMode,
  setFrontendRuntimeMode,
  currentDirectMeshEndpointsText,
  setDirectMeshEndpointsText,
  setDirectMeshSelectionMode,
  refreshHealth,
  refreshJobHistory,
  refreshResults,
  refreshProjects,
  refreshSecurityEvents,
}: ScriptNavControllerDeps): Promise<Record<string, unknown> | null> {
  switch (action) {
    case "nav/setSidebarSection": {
      const section = payload.section;
      if (section === "study" || section === "model" || section === "workflow" || section === "library" || section === "system") {
        handleSidebarSectionChange(section);
      }
      return { ok: true, action, section };
    }
    case "nav/setStudyKind": {
      const nextStudyKind = payload.studyKind;
      if (isWorkbenchStudyKind(nextStudyKind)) {
        recordHistory(changeStudyTypeLabel);
        applyStudyKindSelection({
          currentStudyKind: studyKind,
          nextStudyKind,
          setStudyKind,
          resetHandlers: studyKindResetHandlers,
        });
      }
      return { ok: true, action, studyKind: nextStudyKind };
    }
    case "nav/setTabs": {
      if (payload.studyTab === "summary" || payload.studyTab === "controls") {
        setStudyTab(payload.studyTab);
      }
      if (payload.modelTab === "tools" || payload.modelTab === "tree") {
        setModelTab(payload.modelTab);
      }
      if (
        payload.modelToolsPage === "overview" ||
        payload.modelToolsPage === "study" ||
        payload.modelToolsPage === "studio" ||
        payload.modelToolsPage === "materials" ||
        payload.modelToolsPage === "generate"
      ) {
        setModelToolsPage(payload.modelToolsPage);
      }
      if (
        payload.libraryTab === "results" ||
        payload.libraryTab === "samples" ||
        payload.libraryTab === "projects" ||
        payload.libraryTab === "models" ||
        payload.libraryTab === "jobs"
      ) {
        setLibraryTab(payload.libraryTab);
      }
      if (
        payload.systemPanelTab === "config" ||
        payload.systemPanelTab === "assistant" ||
        payload.systemPanelTab === "scripts" ||
        payload.systemPanelTab === "runtime" ||
        payload.systemPanelTab === "data"
      ) {
        if (payload.systemPanelTab === "assistant") {
          setAssistantWindowOpen(true);
          setSystemPanelTab("config");
        } else {
          setSystemPanelTab(payload.systemPanelTab);
        }
      }
      if (payload.systemDataTab === "jobs" || payload.systemDataTab === "results") {
        setSystemDataTab(payload.systemDataTab);
      }
      return { ok: true, action };
    }
    case "settings/patch": {
      if (typeof payload.language === "string" && payload.language.trim()) {
        handleLanguageChange(payload.language.trim());
      }
      if (payload.theme === "linen" || payload.theme === "marine" || payload.theme === "graphite") {
        setTheme(payload.theme);
      }
      const governedPatch = applyWorkbenchGovernancePatch({
        currentFrontendRuntimeMode,
        currentDirectMeshEndpointsText,
        nextFrontendRuntimeMode:
          payload.frontendRuntimeMode === "orchestrated_gui" || payload.frontendRuntimeMode === "direct_mesh_gui"
            ? payload.frontendRuntimeMode
            : undefined,
        nextDirectMeshEndpointsText:
          typeof payload.directMeshEndpointsText === "string" ? payload.directMeshEndpointsText : undefined,
      });
      if (typeof payload.directMeshEndpointsText === "string") setDirectMeshEndpointsText(governedPatch.directMeshEndpointsText);
      setFrontendRuntimeMode(governedPatch.frontendRuntimeMode);
      if (payload.directMeshSelectionMode === "healthiest" || payload.directMeshSelectionMode === "first_reachable") {
        setDirectMeshSelectionMode(payload.directMeshSelectionMode);
      }
      return {
        ok: true,
        action,
        frontendRuntimeMode: governedPatch.frontendRuntimeMode,
        directMeshEndpointsText: governedPatch.directMeshEndpointsText,
        governanceViolation: governedPatch.violation,
      };
    }
    case "runtime/refreshAll": {
      await Promise.all([refreshHealth(), refreshJobHistory(), refreshResults(), refreshProjects(), refreshSecurityEvents()]);
      return { ok: true, action };
    }
    default:
      return null;
  }
}
