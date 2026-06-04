"use client";

import type {
  WorkbenchScriptActionDefinition,
  WorkbenchScriptMacroDefinition,
} from "./workbench-script-runtime-types";

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

