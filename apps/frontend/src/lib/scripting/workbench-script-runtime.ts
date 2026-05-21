"use client";

export type WorkbenchScriptLanguage = "en" | "zh" | "ja";

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

const MACRO_TEMPLATE_EXACT_RE = /^\{\{\s*(payload|state)\.([a-zA-Z0-9_]+)\s*\}\}$/;
const MACRO_TEMPLATE_INLINE_RE = /\{\{\s*(payload|state)\.([a-zA-Z0-9_]+)\s*\}\}/g;
const WORKBENCH_MACRO_PRESETS_KEY = "kyuubiki-workbench-macro-presets";

export const WORKBENCH_SCRIPT_ACTIONS: WorkbenchScriptActionDefinition[] = [
  {
    id: "nav/setSidebarSection",
    category: "navigation",
    risk: "normal",
    summary: {
      en: "Switch the left rail section.",
      zh: "切换左侧主分区。",
    },
    payloadExample: { section: "system" },
  },
  {
    id: "nav/setStudyKind",
    category: "navigation",
    risk: "normal",
    summary: {
      en: "Switch the active study kind.",
      zh: "切换当前研究类型。",
    },
    payloadExample: { studyKind: "truss_3d" },
  },
  {
    id: "nav/setTabs",
    category: "navigation",
    risk: "normal",
    summary: {
      en: "Switch study, model, library, system, or data subtabs in one action.",
      zh: "一次动作切换 study、model、library、system 或 data 子标签。",
    },
    payloadExample: { libraryTab: "projects", systemPanelTab: "data", systemDataTab: "results" },
  },
  {
    id: "settings/patch",
    category: "settings",
    risk: "normal",
    summary: {
      en: "Patch language, theme, runtime mode, or direct mesh settings.",
      zh: "批量更新语言、主题、运行模式或直连 Mesh 设置。",
    },
    payloadExample: { theme: "graphite", frontendRuntimeMode: "direct_mesh_gui" },
  },
  {
    id: "runtime/refreshAll",
    category: "runtime",
    risk: "normal",
    summary: {
      en: "Refresh health, jobs, results, and projects.",
      zh: "刷新健康状态、任务、结果和项目。",
    },
  },
  {
    id: "project/create",
    category: "project",
    risk: "normal",
    summary: {
      en: "Create and select a project.",
      zh: "创建并选中一个项目。",
    },
    payloadExample: { name: "Automation Demo", description: "Created from WASM Python" },
  },
  {
    id: "project/select",
    category: "project",
    risk: "normal",
    summary: {
      en: "Select a project by id.",
      zh: "按 id 选中项目。",
    },
    payloadExample: { projectId: "proj_123" },
  },
  {
    id: "project/updateSelected",
    category: "project",
    risk: "normal",
    summary: {
      en: "Rename or edit the selected project.",
      zh: "更新当前选中项目。",
    },
    payloadExample: { name: "Renamed Project", description: "Updated from script" },
  },
  {
    id: "project/deleteSelected",
    category: "project",
    risk: "destructive",
    requiresConfirmation: true,
    summary: {
      en: "Delete the selected project after operator confirmation.",
      zh: "删除当前选中项目，执行前需要操作员确认。",
    },
  },
  {
    id: "project/exportJson",
    category: "project",
    risk: "sensitive",
    requiresConfirmation: true,
    summary: {
      en: "Export the selected project as JSON after confirmation.",
      zh: "将当前项目导出为 JSON，执行前需要确认。",
    },
  },
  {
    id: "project/exportZip",
    category: "project",
    risk: "sensitive",
    requiresConfirmation: true,
    summary: {
      en: "Export the selected project as a .kyuubiki bundle after confirmation.",
      zh: "将当前项目导出为 .kyuubiki 包，执行前需要确认。",
    },
  },
  {
    id: "model/generateTruss",
    category: "model",
    risk: "normal",
    summary: {
      en: "Generate the current parametric 2D truss model.",
      zh: "生成当前参数化二维桁架模型。",
    },
  },
  {
    id: "model/generatePanel",
    category: "model",
    risk: "normal",
    summary: {
      en: "Generate the current rectangular panel mesh.",
      zh: "生成当前矩形面板网格。",
    },
  },
  {
    id: "model/save",
    category: "model",
    risk: "normal",
    summary: {
      en: "Save into the selected model or create the first saved version.",
      zh: "保存到当前模型，或创建首个保存版本。",
    },
  },
  {
    id: "model/saveAs",
    category: "model",
    risk: "normal",
    summary: {
      en: "Save the current workspace as a new saved model.",
      zh: "将当前工作区另存为新模型。",
    },
  },
  {
    id: "model/deleteSelected",
    category: "model",
    risk: "destructive",
    requiresConfirmation: true,
    summary: {
      en: "Delete the currently selected saved model after confirmation.",
      zh: "删除当前选中的已保存模型，执行前需要确认。",
    },
  },
  {
    id: "model/renameSelectedVersion",
    category: "model",
    risk: "normal",
    summary: {
      en: "Rename the selected saved version to the current workspace name.",
      zh: "把当前选中版本重命名为当前工作区名称。",
    },
  },
  {
    id: "model/deleteSelectedVersion",
    category: "model",
    risk: "destructive",
    requiresConfirmation: true,
    summary: {
      en: "Delete the selected saved version after confirmation.",
      zh: "删除当前选中的保存版本，执行前需要确认。",
    },
  },
  {
    id: "model/setWorkspaceMeta",
    category: "model",
    risk: "normal",
    summary: {
      en: "Set the workspace model name or active material.",
      zh: "更新当前工作区模型名或活动材料。",
    },
    payloadExample: { loadedModelName: "scripted-space-frame", activeMaterial: "210" },
  },
  {
    id: "state/setParametric",
    category: "state",
    risk: "normal",
    summary: {
      en: "Patch the truss parametric generator inputs.",
      zh: "更新桁架参数化生成器输入。",
    },
    payloadExample: { bays: 8, span: 24, height: 4, loadY: -1800 },
  },
  {
    id: "state/setPanelParametric",
    category: "state",
    risk: "normal",
    summary: {
      en: "Patch the rectangular panel generator inputs.",
      zh: "更新矩形面板生成器输入。",
    },
    payloadExample: { width: 6, height: 3, divisionsX: 8, divisionsY: 4 },
  },
  {
    id: "state/replaceTruss2dModel",
    category: "state",
    risk: "normal",
    summary: {
      en: "Replace the active 2D truss model payload.",
      zh: "替换当前二维桁架模型。",
    },
  },
  {
    id: "state/replaceTruss3dModel",
    category: "state",
    risk: "normal",
    summary: {
      en: "Replace the active 3D truss model payload.",
      zh: "替换当前三维桁架模型。",
    },
  },
  {
    id: "state/replacePlaneModel",
    category: "state",
    risk: "normal",
    summary: {
      en: "Replace the active 2D plane model payload.",
      zh: "替换当前二维平面单元模型。",
    },
  },
  {
    id: "state/projectHeatToThermo",
    category: "state",
    risk: "normal",
    summary: {
      en: "Map the current pure-thermal result into the matching thermo-mechanical study.",
      zh: "把当前纯热结果映射到对应的力-热研究。",
    },
  },
  {
    id: "selection/set",
    category: "selection",
    risk: "normal",
    summary: {
      en: "Select a node or element by index.",
      zh: "按索引选中节点或单元。",
    },
    payloadExample: { nodeIndex: 0, elementIndex: null },
  },
  {
    id: "selection/set3d",
    category: "selection",
    risk: "normal",
    summary: {
      en: "Patch the active 3D node selection, draft link nodes, or link mode state.",
      zh: "更新当前三维节点选择、连线草稿节点或连线模式状态。",
    },
    payloadExample: { nodeIndices: [0, 1], anchorNodeIndex: 0, memberDraftNodeIndices: [0, 1], linkMode: true },
  },
  {
    id: "job/run",
    category: "job",
    risk: "normal",
    summary: {
      en: "Submit the current study.",
      zh: "提交当前研究求解。",
    },
  },
  {
    id: "job/cancel",
    category: "job",
    risk: "sensitive",
    requiresConfirmation: true,
    summary: {
      en: "Cancel the active running job after confirmation.",
      zh: "取消当前运行中的任务，执行前需要确认。",
    },
  },
  {
    id: "history/undo",
    category: "history",
    risk: "normal",
    summary: {
      en: "Undo the last reversible workspace operation.",
      zh: "撤销上一条可回滚操作。",
    },
  },
  {
    id: "history/redo",
    category: "history",
    risk: "normal",
    summary: {
      en: "Redo the last reverted workspace operation.",
      zh: "重做上一条被撤销的操作。",
    },
  },
  {
    id: "viewport/toggleImmersive",
    category: "viewport",
    risk: "normal",
    summary: {
      en: "Enter or exit the immersive 3D viewport.",
      zh: "进入或退出沉浸式三维视图。",
    },
  },
  {
    id: "viewport/set3dView",
    category: "viewport",
    risk: "normal",
    summary: {
      en: "Switch the 3D camera preset or projection mode.",
      zh: "切换三维视角预设或投影模式。",
    },
    payloadExample: { preset: "top", projection: "persp" },
  },
  {
    id: "viewport/focus3d",
    category: "viewport",
    risk: "normal",
    summary: {
      en: "Request the 3D viewport to frame the current selection.",
      zh: "请求三维视口聚焦当前选择。",
    },
  },
  {
    id: "viewport/reset3d",
    category: "viewport",
    risk: "normal",
    summary: {
      en: "Request the 3D viewport to reset its camera.",
      zh: "请求三维视口重置相机。",
    },
  },
  {
    id: "viewport/toggleFlags",
    category: "viewport",
    risk: "normal",
    summary: {
      en: "Toggle 3D grid, labels, or node visibility.",
      zh: "切换三维网格、标签或节点显示。",
    },
    payloadExample: { grid: true, labels: false, nodes: true },
  },
  {
    id: "viewport/setUiState",
    category: "viewport",
    risk: "normal",
    summary: {
      en: "Control immersive viewport drawers, box select, or 3D link mode UI state.",
      zh: "控制沉浸式视图抽屉、框选或三维连线模式等 UI 状态。",
    },
    payloadExample: { immersiveViewport: true, toolDrawerOpen: true, helpDrawerOpen: false, boxSelectMode: false, linkMode: true },
  },
  {
    id: "data/setFilters",
    category: "data",
    risk: "normal",
    summary: {
      en: "Set the jobs/results filter fields and optionally switch the active data tab.",
      zh: "设置 jobs/results 的筛选字段，并可切换当前数据标签。",
    },
    payloadExample: { activeTab: "results", projectId: "proj_123", modelVersionId: "ver_456" },
  },
  {
    id: "data/selectRecord",
    category: "data",
    risk: "normal",
    summary: {
      en: "Select a job or result record in System > Data.",
      zh: "在 System > Data 中选中一条任务或结果记录。",
    },
    payloadExample: { activeTab: "jobs", jobId: "job_123" },
  },
  {
    id: "data/openLinkedContext",
    category: "data",
    risk: "normal",
    summary: {
      en: "Open or apply the linked project/version context from a selected job or result record.",
      zh: "从选中的任务或结果记录打开或应用关联的项目/版本上下文。",
    },
    payloadExample: { target: "result", mode: "apply" },
  },
  {
    id: "data/exportDatabase",
    category: "data",
    risk: "sensitive",
    requiresConfirmation: true,
    summary: {
      en: "Download the durable database export after confirmation.",
      zh: "导出持久化数据库快照，执行前需要确认。",
    },
  },
];

export const WORKBENCH_SCRIPT_MACROS: WorkbenchScriptMacroDefinition[] = [
  {
    id: "macro/projectHeatToThermo",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Project the current thermal result into the matching thermo-mechanical study.",
      zh: "将当前热结果映射到对应的力-热研究。",
    },
    steps: [{ action: "state/projectHeatToThermo" }],
  },
  {
    id: "macro/openDataResults",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Open System > Data > Results and keep the current data filters.",
      zh: "打开 System > Data > Results，并保留当前数据筛选。",
    },
    steps: [
      { action: "nav/setSidebarSection", payload: { section: "system" } },
      { action: "nav/setTabs", payload: { systemPanelTab: "data", systemDataTab: "results" } },
    ],
  },
  {
    id: "macro/openProjectLibrary",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Open Library > Projects for project-oriented browsing.",
      zh: "打开 Library > Projects，便于从项目视角浏览。",
    },
    steps: [
      { action: "nav/setSidebarSection", payload: { section: "library" } },
      { action: "nav/setTabs", payload: { libraryTab: "projects" } },
    ],
  },
  {
    id: "macro/prepare3dEditing",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Switch into the 3D modeling surface with model tools visible.",
      zh: "切到三维建模工作面，并展开建模工具。",
    },
    steps: [
      { action: "nav/setStudyKind", payload: { studyKind: "truss_3d" } },
      { action: "nav/setSidebarSection", payload: { section: "model" } },
      { action: "nav/setTabs", payload: { modelTab: "tools" } },
      { action: "viewport/setUiState", payload: { toolDrawerOpen: true, helpDrawerOpen: false } },
    ],
  },
  {
    id: "macro/focusCurrent3dSelection",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Bring the 3D editor into focus and frame the current selection.",
      zh: "把三维编辑区切到前台，并聚焦当前选择。",
    },
    steps: [
      { action: "nav/setSidebarSection", payload: { section: "model" } },
      { action: "nav/setTabs", payload: { modelTab: "tools" } },
      { action: "viewport/focus3d" },
    ],
  },
  {
    id: "macro/reviewSelectedDataRecord",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Apply the currently selected job/result record back into the workbench context.",
      zh: "把当前选中的任务/结果记录重新应用回工作台上下文。",
    },
    payloadExample: { target: "result" },
    steps: [
      { action: "nav/setSidebarSection", payload: { section: "system" } },
      { action: "nav/setTabs", payload: { systemPanelTab: "data" } },
      { action: "data/openLinkedContext", payload: { target: "{{payload.target}}", mode: "{{payload.mode}}" } },
    ],
  },
  {
    id: "macro/filterProjectResults",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Open System > Data > Results and apply project/version filters from macro input.",
      zh: "打开 System > Data > Results，并套用宏输入里的项目/版本筛选。",
    },
    payloadExample: { projectId: "proj_123", modelVersionId: "ver_456" },
    steps: [
      { action: "nav/setSidebarSection", payload: { section: "system" } },
      { action: "nav/setTabs", payload: { systemPanelTab: "data", systemDataTab: "results" } },
      {
        action: "data/setFilters",
        payload: {
          activeTab: "results",
          projectId: "{{payload.projectId}}",
          modelVersionId: "{{payload.modelVersionId}}",
        },
      },
    ],
  },
  {
    id: "macro/openResultVersionReview",
    category: "macro",
    risk: "normal",
    summary: {
      en: "Select a result record and open its linked saved version for review.",
      zh: "选中一条结果记录，并打开它关联的保存版本进行复查。",
    },
    payloadExample: { resultJobId: "job_123" },
    steps: [
      { action: "nav/setSidebarSection", payload: { section: "system" } },
      { action: "nav/setTabs", payload: { systemPanelTab: "data", systemDataTab: "results" } },
      { action: "data/selectRecord", payload: { activeTab: "results", resultJobId: "{{payload.resultJobId}}" } },
      { action: "data/openLinkedContext", payload: { target: "result", mode: "version", resultJobId: "{{payload.resultJobId}}" } },
    ],
  },
];

export function getWorkbenchScriptActionDefinition(actionId: string) {
  return WORKBENCH_SCRIPT_ACTIONS.find((entry) => entry.id === actionId) ?? null;
}

export function isWorkbenchScriptActionHighRisk(actionId: string) {
  return Boolean(getWorkbenchScriptActionDefinition(actionId)?.requiresConfirmation);
}

export function getWorkbenchScriptMacroDefinition(macroId: string) {
  return WORKBENCH_SCRIPT_MACROS.find((entry) => entry.id === macroId) ?? null;
}

export function buildWorkbenchRecordedMacroDraft(
  actionLog: WorkbenchScriptActionLogEntry[],
  options: {
    id?: string;
    maxSteps?: number;
  } = {},
): WorkbenchRecordedMacroDraft | null {
  const steps = actionLog
    .filter(
      (entry) =>
        entry.action !== "macro/run" &&
        ((entry.source === "manual" && entry.status === "completed") || entry.status === "started"),
    )
    .slice(0, options.maxSteps ?? 12)
    .reverse()
    .flatMap((entry) => {
      const payload = entry.payload;
      return [{ action: entry.action, ...(payload ? { payload } : {}) }];
    });

  if (steps.length === 0) {
    return null;
  }

  return {
    id: options.id ?? "macro/draft-from-log",
    steps,
  };
}

export function isWorkbenchMacroStep(value: unknown): value is WorkbenchScriptMacroStep {
  if (!value || typeof value !== "object") return false;
  const candidate = value as { action?: unknown; payload?: unknown };
  if (typeof candidate.action !== "string" || candidate.action.trim().length === 0) return false;
  if (candidate.payload === undefined) return true;
  return Boolean(candidate.payload && typeof candidate.payload === "object" && !Array.isArray(candidate.payload));
}

export function parseWorkbenchRecordedMacroDraft(value: unknown): WorkbenchRecordedMacroDraft {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid macro document.");
  }

  const candidate = value as { id?: unknown; steps?: unknown };
  const id = typeof candidate.id === "string" && candidate.id.trim() ? candidate.id : "macro/imported";
  const steps = Array.isArray(candidate.steps) ? candidate.steps : null;

  if (!steps || steps.length === 0) {
    throw new Error("Macro document does not contain any steps.");
  }

  const normalizedSteps = steps.map((step) => {
    if (!isWorkbenchMacroStep(step)) {
      throw new Error("Macro document contains an invalid step.");
    }
    return {
      action: step.action,
      ...(step.payload ? { payload: step.payload } : {}),
    };
  });

  return {
    id,
    steps: normalizedSteps,
  };
}

export function serializeWorkbenchRecordedMacroDraft(macro: WorkbenchRecordedMacroDraft): string {
  return JSON.stringify(macro, null, 2);
}

export function serializeWorkbenchMacroPythonSnippet(macro: WorkbenchRecordedMacroDraft): string {
  return `recorded_macro = ${JSON.stringify(macro, null, 2)}\n\nawait ky.run_macro_definition(recorded_macro)\n`;
}

function safeReadWorkbenchMacroPresetRecords(): WorkbenchMacroPresetRecord[] {
  if (typeof window === "undefined") return [];

  try {
    const raw = window.localStorage.getItem(WORKBENCH_MACRO_PRESETS_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    if (!Array.isArray(parsed)) return [];

    return parsed.flatMap((entry) => {
      if (!entry || typeof entry !== "object") return [];
      const candidate = entry as Partial<WorkbenchMacroPresetRecord>;
      if (
        typeof candidate.presetId !== "string" ||
        typeof candidate.projectId !== "string" ||
        typeof candidate.name !== "string" ||
        typeof candidate.updatedAt !== "string"
      ) {
        return [];
      }

      try {
        return [
          {
            presetId: candidate.presetId,
            projectId: candidate.projectId,
            name: candidate.name,
            macro: parseWorkbenchRecordedMacroDraft(candidate.macro),
            updatedAt: candidate.updatedAt,
          },
        ];
      } catch {
        return [];
      }
    });
  } catch {
    return [];
  }
}

function writeWorkbenchMacroPresetRecords(records: WorkbenchMacroPresetRecord[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(WORKBENCH_MACRO_PRESETS_KEY, JSON.stringify(records));
}

function normalizeWorkbenchMacroPresetName(name: string) {
  return name.trim().replace(/\s+/g, " ").slice(0, 64);
}

export function listWorkbenchMacroPresets(projectId: string | null): WorkbenchMacroPresetRecord[] {
  if (!projectId) return [];

  return safeReadWorkbenchMacroPresetRecords()
    .filter((entry) => entry.projectId === projectId)
    .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

export function saveWorkbenchMacroPreset(params: {
  projectId: string;
  name: string;
  macro: WorkbenchRecordedMacroDraft;
  presetId?: string;
}): WorkbenchMacroPresetRecord {
  const projectId = params.projectId.trim();
  const name = normalizeWorkbenchMacroPresetName(params.name);

  if (!projectId) {
    throw new Error("A project must be selected before saving a macro preset.");
  }

  if (!name) {
    throw new Error("Macro preset name cannot be empty.");
  }

  const records = safeReadWorkbenchMacroPresetRecords();
  const now = new Date().toISOString();
  const presetId = params.presetId?.trim() || `preset_${Math.random().toString(36).slice(2, 10)}`;
  const nextRecord: WorkbenchMacroPresetRecord = {
    presetId,
    projectId,
    name,
    macro: parseWorkbenchRecordedMacroDraft(params.macro),
    updatedAt: now,
  };

  const nextRecords = [...records.filter((entry) => entry.presetId !== presetId), nextRecord];
  writeWorkbenchMacroPresetRecords(nextRecords);
  return nextRecord;
}

export function deleteWorkbenchMacroPreset(presetId: string) {
  const normalizedPresetId = presetId.trim();
  if (!normalizedPresetId) return;
  const records = safeReadWorkbenchMacroPresetRecords();
  writeWorkbenchMacroPresetRecords(records.filter((entry) => entry.presetId !== normalizedPresetId));
}

function resolveWorkbenchMacroTemplateString(
  value: string,
  payload: Record<string, unknown>,
  snapshot: WorkbenchScriptSnapshot,
): unknown {
  const exact = value.match(MACRO_TEMPLATE_EXACT_RE);

  if (exact) {
    const [, source, key] = exact;
    return source === "payload"
      ? payload[key]
      : snapshot[key as keyof WorkbenchScriptSnapshot];
  }

  return value.replaceAll(MACRO_TEMPLATE_INLINE_RE, (_full, source: string, key: string) => {
    const resolved =
      source === "payload"
        ? payload[key]
        : snapshot[key as keyof WorkbenchScriptSnapshot];
    return resolved === undefined || resolved === null ? "" : String(resolved);
  });
}

export function resolveWorkbenchMacroPayloadTemplates(
  value: unknown,
  payload: Record<string, unknown>,
  snapshot: WorkbenchScriptSnapshot,
): unknown {
  if (typeof value === "string") {
    return resolveWorkbenchMacroTemplateString(value, payload, snapshot);
  }

  if (Array.isArray(value)) {
    return value.map((entry) => resolveWorkbenchMacroPayloadTemplates(entry, payload, snapshot));
  }

  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, resolveWorkbenchMacroPayloadTemplates(entry, payload, snapshot)]),
    );
  }

  return value;
}

type PyodideInterface = {
  runPythonAsync<T = unknown>(code: string): Promise<T>;
};

type LoadPyodideFunction = (options?: {
  indexURL?: string;
}) => Promise<PyodideInterface>;

declare global {
  interface Window {
    loadPyodide?: LoadPyodideFunction;
    __kyuubikiPyodidePromise?: Promise<PyodideInterface>;
    __kyuubikiBridge?: {
      invoke: (action: string, payloadJson?: string) => Promise<string>;
      state_json: () => string;
      actions_json: () => string;
      macros_json: () => string;
      log: (message: string) => void;
      sleep: (seconds?: number) => Promise<void>;
    };
  }
}

const PYODIDE_VERSION = "0.27.7";
const PYODIDE_SCRIPT_URL = `https://cdn.jsdelivr.net/pyodide/v${PYODIDE_VERSION}/full/pyodide.js`;
const PYODIDE_INDEX_URL = `https://cdn.jsdelivr.net/pyodide/v${PYODIDE_VERSION}/full/`;

let pyodideScriptPromise: Promise<void> | null = null;

function loadPyodideBrowserScript(): Promise<void> {
  if (typeof window === "undefined") {
    return Promise.reject(new Error("Pyodide can only load in the browser."));
  }

  if (window.loadPyodide) {
    return Promise.resolve();
  }

  if (pyodideScriptPromise) {
    return pyodideScriptPromise;
  }

  pyodideScriptPromise = new Promise((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>('script[data-pyodide="true"]');
    if (existing) {
      existing.addEventListener("load", () => resolve(), { once: true });
      existing.addEventListener("error", () => reject(new Error("Unable to load the Pyodide runtime.")), {
        once: true,
      });
      return;
    }

    const script = document.createElement("script");
    script.src = PYODIDE_SCRIPT_URL;
    script.async = true;
    script.dataset.pyodide = "true";
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("Unable to load the Pyodide runtime."));
    document.head.appendChild(script);
  });

  return pyodideScriptPromise;
}

export async function ensurePyodideRuntime(): Promise<PyodideInterface> {
  if (typeof window === "undefined") {
    throw new Error("Pyodide can only initialize in the browser.");
  }

  await loadPyodideBrowserScript();

  if (!window.loadPyodide) {
    throw new Error("Pyodide loader did not become available.");
  }

  if (!window.__kyuubikiPyodidePromise) {
    window.__kyuubikiPyodidePromise = window.loadPyodide({
      indexURL: PYODIDE_INDEX_URL,
    });
  }

  return window.__kyuubikiPyodidePromise;
}

export const DEFAULT_WORKBENCH_PYTHON = `# Kyuubiki frontend automation
# Available helpers:
# - state: current frontend snapshot (dict)
# - actions: action catalog (list[dict])
# - macros: macro catalog (list[dict])
# - ky.log(*parts)
# - await ky.invoke("action/id", payload_dict)
# - await ky.run_macro("macro/id", payload_dict)
# - await ky.run_macro_definition(macro_dict)
# - await ky.sleep(seconds)
# - await ky.wait_until(...)
# - await ky.wait_for_job_done()
# - await ky.wait_for_message("completed")

ky.log("Current study:", state["studyKind"])
ky.log("Current sidebar:", state["sidebarSection"])

# Example: refresh runtime surfaces first.
await ky.invoke("runtime/refreshAll")

# Example: branch your automation on the current study kind.
if state["studyKind"] == "axial_bar_1d":
    await ky.invoke("nav/setStudyKind", {"studyKind": "truss_2d"})
    await ky.invoke("state/setParametric", {"bays": 6, "span": 18, "height": 3.5, "loadY": -1500})
    await ky.invoke("model/generateTruss")

ky.log("Submitting study...")
await ky.invoke("job/run")
`;

export function buildWorkbenchPythonPrelude(): string {
  return `
import json
from js import __kyuubikiBridge

class _KyuubikiBridge:
    def state(self):
        return json.loads(__kyuubikiBridge.state_json())

    def actions(self):
        return json.loads(__kyuubikiBridge.actions_json())

    def macros(self):
        return json.loads(__kyuubikiBridge.macros_json())

    def log(self, *parts):
        __kyuubikiBridge.log(" ".join(str(part) for part in parts))

    async def invoke(self, action, payload=None):
        if payload is None:
            payload = {}
        result = await __kyuubikiBridge.invoke(action, json.dumps(payload))
        return json.loads(result)

    async def run_macro(self, macro, payload=None):
        if payload is None:
            payload = {}
        return await self.invoke("macro/run", {"macroId": macro, **payload})

    async def run_steps(self, steps):
        results = []
        for step in steps:
            action = step.get("action")
            payload = step.get("payload", {})
            results.append(await self.invoke(action, payload))
        return results

    async def run_macro_definition(self, macro):
        return await self.run_steps(macro.get("steps", []))

    async def sleep(self, seconds=0.0):
        await __kyuubikiBridge.sleep(seconds)

    async def wait_until(self, predicate, timeout=30.0, interval=0.25):
        elapsed = 0.0
        while elapsed <= timeout:
            current = self.state()
            if predicate(current):
                return current
            await self.sleep(interval)
            elapsed += interval
        raise TimeoutError(f"Condition not met within {timeout} seconds")

    async def wait_for_job_done(self, timeout=90.0, interval=0.5):
        terminal = {"completed", "failed", "cancelled"}
        return await self.wait_until(
            lambda current: current.get("jobStatus") in terminal,
            timeout=timeout,
            interval=interval,
        )

    async def wait_for_message(self, text, timeout=30.0, interval=0.25):
        needle = str(text)
        return await self.wait_until(
            lambda current: needle in str(current.get("message", "")),
            timeout=timeout,
            interval=interval,
        )

ky = _KyuubikiBridge()
state = ky.state()
actions = ky.actions()
macros = ky.macros()
`;
}
