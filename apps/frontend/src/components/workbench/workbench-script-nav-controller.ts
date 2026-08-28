"use client";

import { applyStudyKindSelection, isWorkbenchStudyKind } from "@/components/workbench/workbench-study-kind-controller";
import type { SidebarSection, WorkflowPanelTab } from "@/components/workbench/workbench-types";
import type { WorkbenchStudyKind } from "@/lib/workbench/history";
import { applyWorkbenchGovernancePatch } from "@/lib/workbench/governance";

const SIDEBAR_SECTIONS = ["study", "model", "workflow", "store", "library", "system"] as const;
const STUDY_TABS = ["summary", "controls"] as const;
const MODEL_TABS = ["tools", "tree"] as const;
const MODEL_TOOLS_PAGES = ["overview", "study", "studio", "materials", "generate"] as const;
const WORKFLOW_PANEL_TABS = ["overview", "catalog", "builder", "runs"] as const;
const LIBRARY_TABS = ["results", "samples", "projects", "models", "jobs"] as const;
const SYSTEM_PANEL_TABS = ["overview", "config", "assistant", "scripts", "runtime", "data"] as const;
const SYSTEM_DATA_TABS = ["jobs", "results"] as const;

function requiredChoice<const T extends readonly string[]>(
  payload: Record<string, unknown>,
  key: string,
  choices: T,
): T[number] {
  const value = optionalChoice(payload, key, choices);
  if (value === undefined) throw new Error(`${key} is required.`);
  return value;
}

function optionalChoice<const T extends readonly string[]>(
  payload: Record<string, unknown>,
  key: string,
  choices: T,
): T[number] | undefined {
  if (!(key in payload)) return undefined;
  const value = payload[key];
  if (typeof value !== "string" || !(choices as readonly string[]).includes(value)) {
    throw new Error(`Invalid ${key}: ${String(value)}`);
  }
  return value as T[number];
}

type ScriptNavControllerDeps = {
  action: string;
  payload: Record<string, unknown>;
  studyKind: WorkbenchStudyKind;
  studyKindResetHandlers: Partial<Record<WorkbenchStudyKind, () => void>>;
  setStudyKind: (value: WorkbenchStudyKind) => void;
  handleSidebarSectionChange: (section: SidebarSection) => void;
  recordHistory: (label: string) => void;
  changeStudyTypeLabel: string;
  setStudyTab: (value: "summary" | "controls") => void;
  setModelTab: (value: "tools" | "tree") => void;
  setModelToolsPage: (value: "overview" | "study" | "studio" | "materials" | "generate") => void;
  setLibraryTab: (value: "results" | "samples" | "projects" | "models" | "jobs") => void;
  setWorkflowPanelTab: (value: WorkflowPanelTab) => void;
  setSystemPanelTab: (value: "overview" | "config" | "scripts" | "runtime" | "data") => void;
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
  setWorkflowPanelTab,
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
      const section = requiredChoice(payload, "section", SIDEBAR_SECTIONS);
      handleSidebarSectionChange(section);
      return { ok: true, action, section };
    }
    case "nav/setStudyKind": {
      const nextStudyKind = payload.studyKind;
      if (!isWorkbenchStudyKind(nextStudyKind)) {
        throw new Error(`Invalid studyKind: ${String(nextStudyKind)}`);
      }
      recordHistory(changeStudyTypeLabel);
      applyStudyKindSelection({
        currentStudyKind: studyKind,
        nextStudyKind,
        setStudyKind,
        resetHandlers: studyKindResetHandlers,
      });
      return { ok: true, action, studyKind: nextStudyKind };
    }
    case "nav/setTabs": {
      const studyTab = optionalChoice(payload, "studyTab", STUDY_TABS);
      const modelTab = optionalChoice(payload, "modelTab", MODEL_TABS);
      const modelToolsPage = optionalChoice(payload, "modelToolsPage", MODEL_TOOLS_PAGES);
      const workflowPanelTab = optionalChoice(payload, "workflowPanelTab", WORKFLOW_PANEL_TABS);
      const libraryTab = optionalChoice(payload, "libraryTab", LIBRARY_TABS);
      const systemPanelTab = optionalChoice(payload, "systemPanelTab", SYSTEM_PANEL_TABS);
      const systemDataTab = optionalChoice(payload, "systemDataTab", SYSTEM_DATA_TABS);
      const tabs = { studyTab, modelTab, modelToolsPage, workflowPanelTab, libraryTab, systemPanelTab, systemDataTab };
      if (Object.values(tabs).every((value) => value === undefined)) {
        throw new Error("nav/setTabs requires at least one supported tab value.");
      }

      if (studyTab) setStudyTab(studyTab);
      if (modelTab) setModelTab(modelTab);
      if (modelToolsPage) setModelToolsPage(modelToolsPage);
      if (workflowPanelTab) setWorkflowPanelTab(workflowPanelTab);
      if (libraryTab) setLibraryTab(libraryTab);
      if (systemPanelTab) {
        if (systemPanelTab === "assistant") {
          setAssistantWindowOpen(true);
          setSystemPanelTab("config");
        } else {
          setSystemPanelTab(systemPanelTab);
        }
      }
      if (systemDataTab) setSystemDataTab(systemDataTab);
      return { ok: true, action, tabs };
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
