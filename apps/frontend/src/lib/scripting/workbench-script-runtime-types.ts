"use client";

export type WorkbenchScriptLanguage = "en" | "zh" | "ja" | "es";

export type WorkbenchScriptSnapshot = {
  studyKind: string;
  sidebarSection: string;
  studyTab: string;
  modelTab: string;
  libraryTab: string;
  systemPanelTab: string;
  systemDataTab: string;
  language: WorkbenchScriptLanguage;
  theme: string;
  frontendRuntimeMode: string;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  selectedVersionId: string | null;
  selectedAdminJobId: string | null;
  selectedAdminResultJobId: string | null;
  adminFilterProjectId: string;
  adminFilterModelVersionId: string;
  loadedModelName: string;
  activeMaterial: string;
  selectedNode: number | null;
  selectedElement: number | null;
  selectedTruss3dNodeIndices: number[];
  memberDraftNodeIndices: number[];
  immersiveViewport: boolean;
  immersiveToolDrawerOpen: boolean;
  immersiveHelpDrawerOpen: boolean;
  truss3dProjectionMode: string;
  truss3dViewPreset: string;
  truss3dBoxSelectMode: boolean;
  truss3dLinkMode: boolean;
  hasResult: boolean;
  jobStatus: string | null;
  projectCount: number;
  jobHistoryCount: number;
  resultCount: number;
  protocolAgentCount: number;
  healthStatus: string | null;
  message: string;
};

export type WorkbenchScriptActionLogEntry = {
  id: string;
  action: string;
  source?: "script" | "assistant" | "manual";
  status: "started" | "completed" | "failed";
  at: string;
  summary: string;
  payload?: Record<string, unknown>;
  result?: Record<string, unknown>;
  note?: string;
};

export type WorkbenchScriptActionDefinition = {
  id: string;
  category: string;
  risk: "normal" | "sensitive" | "destructive";
  requiresConfirmation?: boolean;
  summary: {
    en: string;
    zh: string;
  };
  payloadExample?: Record<string, unknown>;
};

export type WorkbenchScriptMacroStep = {
  action: string;
  payload?: Record<string, unknown>;
};

export type WorkbenchScriptMacroDefinition = {
  id: string;
  category: string;
  risk: "normal" | "sensitive" | "destructive";
  requiresConfirmation?: boolean;
  summary: {
    en: string;
    zh: string;
  };
  payloadExample?: Record<string, unknown>;
  steps: WorkbenchScriptMacroStep[];
};

export type WorkbenchRecordedMacroDraft = {
  id: string;
  steps: WorkbenchScriptMacroStep[];
};

export type WorkbenchMacroPresetRecord = {
  presetId: string;
  projectId: string;
  name: string;
  macro: WorkbenchRecordedMacroDraft;
  updatedAt: string;
};

