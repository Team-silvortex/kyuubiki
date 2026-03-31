"use client";

import {
  useDeferredValue,
  useEffect,
  useRef,
  useState,
  useTransition,
  type Dispatch,
  type PointerEvent as ReactPointerEvent,
  type SetStateAction,
} from "react";
import { VirtualList } from "@/components/virtual-list";
import { WorkbenchConsole } from "@/components/workbench-console";
import { WorkbenchInspector } from "@/components/workbench-inspector";
import { WorkbenchViewport } from "@/components/workbench-viewport";
import { MATERIAL_PRESETS } from "@/lib/materials";
import { parsePlaygroundModel } from "@/lib/model-import";
import { exportProjectBundleZip, parseProjectBundleFile } from "@/lib/project-format";
import {
  buildStudyModelPayload,
  exportProjectBundle,
  exportStudyModel,
  generatePrattTruss,
  generateRectangularPanelMesh,
  type ParametricPanelConfig,
  type ParametricTrussConfig,
} from "@/lib/modeler";
import { SAMPLE_LIBRARY } from "@/lib/sample-library";
import {
  createAxialBarJob,
  createPlaneTriangle2dJob,
  createModel,
  createModelVersion,
  createProject,
  createTruss2dJob,
  createTruss3dJob,
  deleteModel,
  deleteModelVersion,
  deleteProject,
  fetchModel,
  fetchModelVersion,
  fetchModelVersions,
  fetchHealth,
  fetchJobHistory,
  fetchJobStatus,
  fetchProjects,
  type AxialBarJobInput,
  type AxialBarResult,
  type HealthPayload,
  type JobEnvelope,
  type JobResultRecord,
  type JobState,
  type ModelRecord,
  type ModelVersionRecord,
  type PlaneTriangle2dJobInput,
  type PlaneTriangle2dResult,
  type ProjectRecord,
  type Truss2dJobInput,
  type Truss2dResult,
  type Truss3dJobInput,
  type Truss3dResult,
  updateModel,
  updateModelVersion,
  updateProject,
} from "@/lib/api";

type Language = "en" | "zh";
type Theme = "linen" | "marine" | "graphite";
type SidebarSection = "study" | "model" | "library" | "system";
type StudyKind = "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d";

type AxialFormState = {
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

type DisplayTrussNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  ux: number;
  uy: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

type DisplayTrussElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
};

type DisplayTruss3dNode = {
  index: number;
  id: string;
  x: number;
  y: number;
  z: number;
  ux: number;
  uy: number;
  uz: number;
};

type DisplayTruss3dElement = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  length: number;
  strain: number;
  stress: number;
  axial_force: number;
};

type SelectionKind = "node" | "element";

type TrussSuggestion =
  | { id: string; kind: "fix_support"; axis: "x" | "y"; nodeIndex: number; label: string }
  | { id: string; kind: "connect_nearest"; nodeIndex: number; label: string };

type TrussDiagnostics = {
  blockingMessages: string[];
  nodeIssues: Record<number, string[]>;
  suggestions: TrussSuggestion[];
};

type StabilitySummary = {
  score: number;
  tone: "good" | "watch" | "risk";
  hotspotNodes: number[];
};

type WorkbenchSnapshot = {
  studyKind: StudyKind;
  axialForm: AxialFormState;
  trussModel: Truss2dJobInput;
  truss3dModel: Truss3dJobInput;
  planeModel: PlaneTriangle2dJobInput;
  parametric: ParametricTrussConfig;
  panelParametric: ParametricPanelConfig;
  activeMaterial: string;
  loadedModelName: string;
  sidebarSection: SidebarSection;
  selectedNode: number | null;
  selectedElement: number | null;
  memberDraftNodes: number[];
};

type HistoryEntry = {
  label: string;
  snapshot: WorkbenchSnapshot;
};

const defaultAxial: AxialFormState = {
  length: 1.2,
  area: 0.01,
  elements: 6,
  tipForce: 1800,
  material: "210",
  youngsModulusGpa: 210,
};

const defaultTruss: Truss2dJobInput = {
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 0.5, y: 0.8, fix_x: false, fix_y: false, load_x: 0, load_y: -1000 },
  ],
  elements: [
    { id: "e0", node_i: 0, node_j: 2, area: 0.01, youngs_modulus: 70e9 },
    { id: "e1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9 },
    { id: "e2", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9 },
  ],
};

const defaultParametric: ParametricTrussConfig = {
  bays: 4,
  span: 12,
  height: 3,
  area: 0.01,
  youngsModulusGpa: 70,
  loadY: -1200,
};

const defaultPanelParametric: ParametricPanelConfig = {
  width: 3.2,
  height: 1.8,
  divisionsX: 4,
  divisionsY: 3,
  thickness: 0.02,
  youngsModulusGpa: 70,
  poissonRatio: 0.33,
  loadY: -1200,
};

const defaultPlane: PlaneTriangle2dJobInput = {
  nodes: [
    { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
    { id: "n2", x: 1, y: 1, fix_x: false, fix_y: false, load_x: 0, load_y: -800 },
    { id: "n3", x: 0, y: 1, fix_x: true, fix_y: false, load_x: 0, load_y: -800 },
  ],
  elements: [
    { id: "p0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33 },
    { id: "p1", node_i: 0, node_j: 2, node_k: 3, thickness: 0.02, youngs_modulus: 70e9, poisson_ratio: 0.33 },
  ],
};

const defaultTruss3d: Truss3dJobInput = {
  nodes: [
    { id: "b0", x: 0, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "b1", x: 1.2, y: 0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "b2", x: 0.1, y: 1.0, z: 0, fix_x: true, fix_y: true, fix_z: true, load_x: 0, load_y: 0, load_z: 0 },
    { id: "top", x: 0.35, y: 0.3, z: 1.0, fix_x: false, fix_y: false, fix_z: false, load_x: 0, load_y: 0, load_z: -1500 },
  ],
  elements: [
    { id: "e0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 70e9 },
    { id: "e1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 70e9 },
    { id: "e2", node_i: 2, node_j: 0, area: 0.01, youngs_modulus: 70e9 },
    { id: "e3", node_i: 0, node_j: 3, area: 0.01, youngs_modulus: 70e9 },
    { id: "e4", node_i: 1, node_j: 3, area: 0.01, youngs_modulus: 70e9 },
    { id: "e5", node_i: 2, node_j: 3, area: 0.01, youngs_modulus: 70e9 },
  ],
};

const SETTINGS_KEY = "kyuubiki-workbench-settings";

const copy = {
  en: {
    brand: "Kyuubiki",
    title: "Structural Workbench",
    subtitle: "A formal front-end workbench for modeling, orchestration, and solver review.",
    rail: { study: "Study", model: "Model", library: "History", system: "System" },
    sections: { study: "Study Setup", model: "Model Studio", library: "Job History", system: "System" },
    kinds: { axial_bar_1d: "1D axial bar", truss_2d: "2D truss", truss_3d: "3D space truss", plane_triangle_2d: "2D plane triangle" },
    importModel: "Import model",
    importHint: "Load a JSON model for 1D or 2D studies.",
    axialSample: "Open 1D sample",
    trussSample: "Open 2D sample",
    modelName: "Model",
    material: "Material",
    mesh: "Mesh",
    load: "Load",
    support: "Support",
    viewport: "Viewport",
    report: "Report",
    metrics: "Solver Metrics",
    messages: "Messages",
    failureReason: "Failure reason",
    historyPanel: "Operation History",
    undo: "Undo",
    redo: "Redo",
    noOperations: "No reversible operations yet.",
    undoApplied: "Rolled back the last change.",
    redoApplied: "Re-applied the last rolled-back change.",
    changeStudyType: "Changed study type",
    editAxialField: "Edited axial study input",
    editMaterial: "Changed material preset",
    editParametric: "Edited parametric generator",
    importAction: "Imported model file",
    sampleAction: "Loaded sample model",
    historyAction: "Opened historical job",
    generateAction: "Generated parametric truss",
    applySuggestionAction: "Applied diagnostic fix",
    addNodeAction: "Added node",
    deleteNodeAction: "Deleted node",
    toggleMemberAction: "Changed member connectivity",
    deleteMemberAction: "Deleted member",
    dragNodeAction: "Dragged node",
    editNodeAction: "Edited node properties",
    editMemberAction: "Edited member properties",
    overview: "Overview",
    controls: "Controls",
    settings: "Settings",
    backend: "Backend",
    orchestrator: "Elixir orchestrator",
    solverAgent: "Rust solver agent",
    ui: "Next.js UI",
    theme: "Theme",
    language: "Language",
    themes: { linen: "Linen Draft", marine: "Marine Grid", graphite: "Graphite Lab" },
    languages: { en: "English", zh: "中文" },
    length: "Span (m)",
    area: "Area (m²)",
    modulus: "Young's modulus (GPa)",
    elements: "Elements",
    tipForce: "Tip force (N)",
    run: "Run Study",
    running: "Running...",
    ready: "ready",
    busy: "busy",
    tipDisp: "Tip displacement",
    maxStress: "Max stress",
    reaction: "Reaction",
    progress: "Progress",
    iteration: "Iteration",
    residual: "Residual",
    worker: "Worker",
    status: "Status",
    nodes: "Nodes",
    axialElements: "Element results",
    trussElements: "Truss members",
    spatialTrussElements: "Space-truss members",
    span: "Span",
    stress: "Stress (Pa)",
    axialForce: "Axial force (N)",
    importedModel: "Imported model",
    importFailed: "Import failed",
    initialLoaded: "Workbench connected to the orchestrator.",
    initialFailed: "Unable to reach the orchestrator.",
    dispatching: "Submitting a study to the orchestrator.",
    defaultModel: "manual-study",
    online: "online",
    offline: "offline",
    historyHint: "Durable jobs survive orchestrator restarts.",
    historyEmpty: "No jobs yet.",
    openJob: "Open",
    refresh: "Refresh",
    projectLibrary: "Project Library",
    projectNameField: "Project name",
    projectDescriptionField: "Description",
    createProject: "Create project",
    updateProject: "Rename project",
    deleteProject: "Delete project",
    exportProject: "Export project",
    exportProjectJson: "Project JSON",
    exportProjectZip: "Project ZIP",
    importProject: "Import project",
    importProjectHint: "Load a .kyuubiki.json or .kyuubiki archive into the project library.",
    projectEmpty: "No projects yet.",
    savedModels: "Saved Models",
    versions: "Versions",
    save: "Save",
    saveAs: "Save As",
    deleteSavedModel: "Delete model",
    renameVersion: "Rename version",
    deleteVersion: "Delete version",
    exportData: "Export Data",
    exportJson: "JSON",
    exportCsv: "CSV",
    noSavedModels: "No saved models in this project yet.",
    noVersions: "No saved versions yet.",
    defaultProject: "Workspace",
    projectCreated: "Project created.",
    projectUpdated: "Project updated.",
    projectDeleted: "Project deleted.",
    projectRequired: "Create or select a project before saving models.",
    projectExported: "Project bundle exported.",
    projectImported: "Project bundle imported.",
    modelSaved: "Saved a new model version.",
    modelCreated: "Saved a new model.",
    modelDeletedStored: "Deleted the saved model.",
    versionLoaded: "Loaded a saved version into the workbench.",
    persistedModelLoaded: "Loaded a saved model from the library.",
    versionRenamed: "Version renamed.",
    versionDeleted: "Version deleted.",
    resultJsonDownloaded: "Analysis result JSON downloaded.",
    resultCsvDownloaded: "Analysis result CSV downloaded.",
    noResultToExport: "Run a study first before exporting analysis data.",
    modelTools: "Editing Tools",
    dragHint: "Drag truss nodes directly in the viewport to reshape geometry.",
    planeHint: "Select plane nodes and triangles to edit supports, loads, and material thickness.",
    parametric: "Parametric Generator",
    panelGenerator: "Panel Generator",
    generate: "Generate",
    generatePanel: "Generate Panel",
    download: "Download JSON",
    saveForSolver: "Use current model",
    bays: "Bays",
    height: "Height (m)",
    divisionsX: "Divisions X",
    divisionsY: "Divisions Y",
    nodeTable: "Nodes",
    dragNode: "Selected node",
    noNodeSelected: "No node selected",
    loadCase: "Load case",
    modelStudioHint: "2D truss and 2D plane studies can be edited directly here. 3D studies can already be imported, solved, and reviewed.",
    sourceModel: "Source model",
    createdAt: "Created",
    updatedAt: "Updated",
    hasResult: "Result",
    yes: "Yes",
    no: "No",
    dragToEdit: "Drag to edit",
    historyLoaded: "Loaded a persisted study from history.",
    modelDownloaded: "Model JSON downloaded.",
    generatedModel: "Generated a parametric truss model.",
    planeElements: "Plane elements",
    thickness: "Thickness",
    poisson: "Poisson ratio",
    poissonRatio: "Poisson ratio",
    sampleLibrary: "Sample Library",
    objectTree: "Object Tree",
    properties: "Properties",
    diagnostics: "Diagnostics",
    diagnosticsClear: "No blocking issues detected.",
    suggestedFixes: "Suggested fixes",
    stabilityScore: "Stability score",
    hotspotNodes: "Hotspot nodes",
    stabilityGood: "Stable",
    stabilityWatch: "Watch",
    stabilityRisk: "At risk",
    translatedSmallDeformation:
      "The structure is behaving too softly for a small-deformation solve. Add supports or reinforce the weak span.",
    translatedConnectivity:
      "The truss likely has a weak or disconnected region. Check supports, member links, and isolated nodes.",
    translatedSingular:
      "The stiffness matrix is singular. The model is still acting like a mechanism, so it needs more restraints or diagonal bracing.",
    mechanismRisk: "This topology still looks mechanism-prone. Add more triangulation or extra supports before solving.",
    addNode: "Add Node",
    addBranchNode: "Branch Node",
    deleteNode: "Delete Node",
    addMember: "Add Member",
    toggleMember: "Toggle Link",
    deleteMember: "Delete Member",
    selectTwoNodes: "Select two nodes to create a member.",
    memberCreated: "Member created.",
    memberRemoved: "Member removed.",
    nodeCreated: "Node created.",
    branchCreated: "Node created and linked.",
    nodeDeleted: "Node deleted.",
    memberDeleted: "Member deleted.",
    selectionHint: "Use the object tree or viewport to pick nodes and members.",
    memberSelection: "Member selection",
    noElementSelected: "No member selected",
    nodeX: "Node X",
    nodeY: "Node Y",
    fixX: "Fix X",
    fixY: "Fix Y",
    loadX: "Load X",
    loadY: "Load Y",
    nodeI: "Node I",
    nodeJ: "Node J",
    nodeK: "Node K",
    none: "None",
    precheckPrefix: "Precheck failed",
    unstableSupport: "Add at least two restrained degrees of freedom to stabilize the truss.",
    isolatedNode: "Every node should connect to at least one member before solving.",
    underconnected: "This truss is under-connected for a stable 2D solve.",
    freeRigidBody: "Prevent rigid-body motion by constraining both translation directions across the model.",
    supportXMissing: "Add at least one X restraint to prevent sideways drift.",
    supportYMissing: "Add at least one Y restraint to prevent vertical drift.",
    fixCurrentNodeXAction: "Fix current node in X",
    fixCurrentNodeYAction: "Fix current node in Y",
    fixNodeXAction: "Fix flagged node in X",
    fixNodeYAction: "Fix flagged node in Y",
    connectCurrentNodeAction: "Connect current node to nearest node",
    connectNodeAction: "Connect flagged node to nearest node",
    suggestionAppliedSupportX: "Added an X restraint to the suggested node.",
    suggestionAppliedSupportY: "Added a Y restraint to the suggested node.",
    suggestionAppliedLink: "Connected the node to its nearest available neighbor.",
    suggestionNoLinkTarget: "No valid nearby node was available for an automatic link.",
    panelGenerated: "Generated a rectangular plane mesh.",
    planeThickness: "Thickness",
  },
  zh: {
    brand: "Kyuubiki",
    title: "结构分析工作台",
    subtitle: "更正式的前端工作台，统一建模、编排与求解回看。",
    rail: { study: "研究", model: "建模", library: "历史", system: "系统" },
    sections: { study: "研究设置", model: "建模工作室", library: "任务历史", system: "系统" },
    kinds: { axial_bar_1d: "一维轴向杆", truss_2d: "二维桁架", truss_3d: "三维空间桁架", plane_triangle_2d: "二维三角形单元" },
    importModel: "导入模型",
    importHint: "导入 1D 或 2D 研究 JSON 模型。",
    axialSample: "打开 1D 样例",
    trussSample: "打开 2D 样例",
    modelName: "模型",
    material: "材料",
    mesh: "网格",
    load: "载荷",
    support: "约束",
    viewport: "视图区",
    report: "报告",
    metrics: "求解指标",
    messages: "消息",
    failureReason: "失败原因",
    historyPanel: "操作历史",
    undo: "撤销",
    redo: "重做",
    noOperations: "当前还没有可回滚的操作。",
    undoApplied: "已回滚上一个改动。",
    redoApplied: "已恢复刚才回滚的改动。",
    changeStudyType: "切换研究类型",
    editAxialField: "修改轴向研究参数",
    editMaterial: "切换材料预设",
    editParametric: "修改参数化生成器",
    importAction: "导入模型文件",
    sampleAction: "加载样板模型",
    historyAction: "打开历史任务",
    generateAction: "生成参数化桁架",
    applySuggestionAction: "应用诊断修复",
    addNodeAction: "新增节点",
    deleteNodeAction: "删除节点",
    toggleMemberAction: "修改杆件连接",
    deleteMemberAction: "删除杆件",
    dragNodeAction: "拖拽节点",
    editNodeAction: "编辑节点属性",
    editMemberAction: "编辑杆件属性",
    overview: "概览",
    controls: "控制",
    settings: "设置",
    backend: "后端",
    orchestrator: "Elixir 编排器",
    solverAgent: "Rust 求解代理",
    ui: "Next.js 界面",
    theme: "主题",
    language: "语言",
    themes: { linen: "纸面浅色", marine: "海图网格", graphite: "石墨实验室" },
    languages: { en: "English", zh: "中文" },
    length: "跨度 (m)",
    area: "截面积 (m²)",
    modulus: "弹性模量 (GPa)",
    elements: "单元数",
    tipForce: "端部载荷 (N)",
    run: "运行研究",
    running: "运行中...",
    ready: "就绪",
    busy: "处理中",
    tipDisp: "端部位移",
    maxStress: "最大应力",
    reaction: "反力",
    progress: "进度",
    iteration: "迭代",
    residual: "残差",
    worker: "执行器",
    status: "状态",
    nodes: "节点",
    axialElements: "单元结果",
    trussElements: "桁架杆件",
    spatialTrussElements: "空间桁架杆件",
    span: "区间",
    stress: "应力 (Pa)",
    axialForce: "轴力 (N)",
    importedModel: "已导入模型",
    importFailed: "导入失败",
    initialLoaded: "工作台已连接到编排器。",
    initialFailed: "无法连接到编排器。",
    dispatching: "正在向编排器提交研究任务。",
    defaultModel: "手动研究",
    online: "在线",
    offline: "离线",
    historyHint: "任务历史会在编排器重启后保留。",
    historyEmpty: "暂时还没有任务。",
    openJob: "打开",
    refresh: "刷新",
    projectLibrary: "项目库",
    projectNameField: "项目名",
    projectDescriptionField: "描述",
    createProject: "新建项目",
    updateProject: "更新项目",
    deleteProject: "删除项目",
    exportProject: "导出项目",
    exportProjectJson: "项目 JSON",
    exportProjectZip: "项目 ZIP",
    importProject: "导入项目",
    importProjectHint: "导入 .kyuubiki.json 或 .kyuubiki 项目包到项目库。",
    projectEmpty: "还没有项目。",
    savedModels: "已保存模型",
    versions: "版本",
    save: "保存",
    saveAs: "另存为",
    deleteSavedModel: "删除模型",
    renameVersion: "重命名版本",
    deleteVersion: "删除版本",
    exportData: "导出数据",
    exportJson: "JSON",
    exportCsv: "CSV",
    noSavedModels: "这个项目里还没有保存的模型。",
    noVersions: "还没有版本记录。",
    defaultProject: "工作区",
    projectCreated: "项目已创建。",
    projectUpdated: "项目已更新。",
    projectDeleted: "项目已删除。",
    projectRequired: "保存模型前请先创建或选择一个项目。",
    projectExported: "项目包已导出。",
    projectImported: "项目包已导入。",
    modelSaved: "已保存新模型版本。",
    modelCreated: "已保存新模型。",
    modelDeletedStored: "已删除保存的模型。",
    versionLoaded: "已将保存版本加载到工作台。",
    persistedModelLoaded: "已从库中加载保存模型。",
    versionRenamed: "版本已重命名。",
    versionDeleted: "版本已删除。",
    resultJsonDownloaded: "分析结果 JSON 已下载。",
    resultCsvDownloaded: "分析结果 CSV 已下载。",
    noResultToExport: "请先运行一次分析再导出结果数据。",
    modelTools: "编辑工具",
    dragHint: "直接在视图区拖拽桁架节点来修改几何。",
    planeHint: "选择平面节点和三角形单元，直接修改约束、载荷和厚度材料。",
    parametric: "参数化生成",
    panelGenerator: "面板生成器",
    generate: "生成模型",
    generatePanel: "生成面板",
    download: "下载 JSON",
    saveForSolver: "使用当前模型",
    bays: "跨数",
    height: "高度 (m)",
    divisionsX: "X 分段",
    divisionsY: "Y 分段",
    nodeTable: "节点列表",
    dragNode: "当前节点",
    noNodeSelected: "未选择节点",
    loadCase: "载荷工况",
    modelStudioHint: "当前建模页支持二维桁架和二维平面单元。三维研究已经支持导入、求解和结果回看。",
    sourceModel: "来源模型",
    createdAt: "创建时间",
    updatedAt: "更新时间",
    hasResult: "结果",
    yes: "是",
    no: "否",
    dragToEdit: "拖拽编辑",
    historyLoaded: "已从历史记录加载持久化任务。",
    modelDownloaded: "模型 JSON 已下载。",
    generatedModel: "已生成参数化桁架模型。",
    planeElements: "平面单元",
    thickness: "厚度",
    poisson: "泊松比",
    poissonRatio: "泊松比",
    sampleLibrary: "样板库",
    objectTree: "对象树",
    properties: "属性",
    diagnostics: "诊断",
    diagnosticsClear: "当前没有阻塞性的预检查问题。",
    suggestedFixes: "建议修复",
    stabilityScore: "稳定性评分",
    hotspotNodes: "危险热区节点",
    stabilityGood: "稳定",
    stabilityWatch: "需关注",
    stabilityRisk: "高风险",
    translatedSmallDeformation: "这个结构看起来过软，已经超出小变形求解范围。请补约束或增强薄弱跨段。",
    translatedConnectivity: "桁架里可能有连接偏弱或断开的区域。请检查约束、杆件连接和孤立节点。",
    translatedSingular: "刚度矩阵是奇异的。说明模型仍然像机构一样可动，需要更多约束或对角支撑。",
    mechanismRisk: "这个拓扑仍然很像机构。建议先增加三角化斜撑或额外支座再求解。",
    addNode: "新增节点",
    addBranchNode: "分支节点",
    deleteNode: "删除节点",
    addMember: "新增杆件",
    toggleMember: "切换连接",
    deleteMember: "删除杆件",
    selectTwoNodes: "请选择两个节点来创建杆件。",
    memberCreated: "杆件已创建。",
    memberRemoved: "杆件已断开。",
    nodeCreated: "节点已创建。",
    branchCreated: "节点已创建并连接。",
    nodeDeleted: "节点已删除。",
    memberDeleted: "杆件已删除。",
    selectionHint: "可以通过对象树或视图区选择节点和杆件。",
    memberSelection: "杆件选择",
    noElementSelected: "未选择杆件",
    nodeX: "节点 X",
    nodeY: "节点 Y",
    fixX: "固定 X",
    fixY: "固定 Y",
    loadX: "载荷 X",
    loadY: "载荷 Y",
    nodeI: "起点节点",
    nodeJ: "终点节点",
    nodeK: "第三节点",
    none: "无",
    precheckPrefix: "预检查失败",
    unstableSupport: "请至少提供两个受限自由度来稳定桁架。",
    isolatedNode: "每个节点在求解前都应该至少连接一根杆件。",
    underconnected: "这个桁架连接数量偏少，可能无法稳定求解。",
    freeRigidBody: "请通过约束两个平移方向来避免整体刚体运动。",
    supportXMissing: "至少增加一个 X 方向约束，避免侧向漂移。",
    supportYMissing: "至少增加一个 Y 方向约束，避免竖向漂移。",
    fixCurrentNodeXAction: "固定当前节点 X",
    fixCurrentNodeYAction: "固定当前节点 Y",
    fixNodeXAction: "固定问题节点 X",
    fixNodeYAction: "固定问题节点 Y",
    connectCurrentNodeAction: "将当前节点连接到最近节点",
    connectNodeAction: "将问题节点连接到最近节点",
    suggestionAppliedSupportX: "已为建议节点补上 X 约束。",
    suggestionAppliedSupportY: "已为建议节点补上 Y 约束。",
    suggestionAppliedLink: "已将该节点连接到最近的可用邻点。",
    suggestionNoLinkTarget: "附近没有可自动连接的有效节点。",
    panelGenerated: "已生成矩形平面网格。",
    planeThickness: "厚度",
  },
} as const;

function safeStorageGet(): { theme?: Theme; language?: Language } {
  if (typeof window === "undefined") return {};
  try {
    const raw = window.localStorage.getItem(SETTINGS_KEY);
    return raw ? (JSON.parse(raw) as { theme?: Theme; language?: Language }) : {};
  } catch {
    return {};
  }
}

function serializeCurrentModel(
  studyKind: StudyKind,
  loadedModelName: string,
  activeMaterial: string,
  axialForm: AxialFormState,
  trussModel: Truss2dJobInput,
  truss3dModel: Truss3dJobInput,
  planeModel: PlaneTriangle2dJobInput,
  parametric: ParametricTrussConfig,
): Record<string, unknown> {
  return buildStudyModelPayload(studyKind, {
    name: loadedModelName,
    material: activeMaterial,
    youngsModulusGpa:
      studyKind === "axial_bar_1d"
        ? axialForm.youngsModulusGpa
        : studyKind === "truss_2d"
          ? parametric.youngsModulusGpa
          : studyKind === "truss_3d"
            ? round((truss3dModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9)
            : round((planeModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9),
    axial: studyKind === "axial_bar_1d" ? toAxialInput(axialForm) : undefined,
    truss: studyKind === "truss_2d" ? trussModel : undefined,
    truss3d: studyKind === "truss_3d" ? truss3dModel : undefined,
    plane: studyKind === "plane_triangle_2d" ? planeModel : undefined,
  });
}

function toCsvRow(values: Array<string | number | boolean | null | undefined>) {
  return values
    .map((value) => {
      const text = value === null || value === undefined ? "" : String(value);
      if (text.includes(",") || text.includes("\"") || text.includes("\n")) {
        return `"${text.replaceAll("\"", "\"\"")}"`;
      }
      return text;
    })
    .join(",");
}

function serializeResultCsv(
  studyKind: StudyKind,
  job: JobEnvelope["job"] | null,
  result: AxialBarResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | null,
) {
  if (!result) return "";

  const lines: string[] = [];
  lines.push("meta");
  lines.push(toCsvRow(["study_kind", studyKind]));
  lines.push(toCsvRow(["job_id", job?.job_id]));
  lines.push(toCsvRow(["status", job?.status]));
  lines.push(toCsvRow(["worker_id", job?.worker_id]));
  lines.push("");

  if (isAxialResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "x", "displacement"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.x, node.displacement])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "x1", "x2", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.x1, element.x2, element.strain, element.stress, element.axial_force])),
    );
    return lines.join("\n");
  }

  if (isTruss3dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "z", "ux", "uy", "uz"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.z, node.ux, node.uy, node.uz])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.strain, element.stress, element.axial_force]),
      ),
    );
    return lines.join("\n");
  }

  if (isTrussResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.strain, element.stress, element.axial_force]),
      ),
    );
    return lines.join("\n");
  }

  lines.push("nodes");
  lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy"]));
  result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy])));
  lines.push("");
  lines.push("elements");
  lines.push(
    toCsvRow([
      "index",
      "id",
      "node_i",
      "node_j",
      "node_k",
      "area",
      "strain_x",
      "strain_y",
      "gamma_xy",
      "stress_x",
      "stress_y",
      "tau_xy",
      "von_mises",
    ]),
  );
  result.elements.forEach((element) =>
    lines.push(
      toCsvRow([
        element.index,
        element.id,
        element.node_i,
        element.node_j,
        element.node_k,
        element.area,
        element.strain_x,
        element.strain_y,
        element.gamma_xy,
        element.stress_x,
        element.stress_y,
        element.tau_xy,
        element.von_mises,
      ]),
    ),
  );
  return lines.join("\n");
}

function toAxialInput(form: AxialFormState): AxialBarJobInput {
  return {
    length: form.length,
    area: form.area,
    elements: form.elements,
    tip_force: form.tipForce,
    youngs_modulus_gpa: form.youngsModulusGpa,
  };
}

function scientific(value: number | null | undefined, digits = 3): string {
  return typeof value === "number" ? value.toExponential(digits) : "--";
}

function fixed(value: number | null | undefined, digits = 2): string {
  return typeof value === "number" ? value.toFixed(digits) : "--";
}

function humanizeSolverFailure(message: string | null | undefined, languageCopy: (typeof copy)[Language]) {
  if (!message) return null;

  if (message.includes("small-deformation limit")) {
    return `${languageCopy.translatedSmallDeformation} ${languageCopy.translatedConnectivity}`;
  }

  if (message.includes("system is singular")) {
    return `${languageCopy.translatedSingular} ${languageCopy.translatedConnectivity}`;
  }

  if (message.includes("supports or connectivity")) {
    return languageCopy.translatedConnectivity;
  }

  return message;
}

function formatJobMessage(job: JobEnvelope["job"] | null, fallback: string, languageCopy: (typeof copy)[Language]) {
  if (!job) return fallback;
  if (job.status === "failed" && job.message) {
    return `${job.job_id} failed: ${humanizeSolverFailure(job.message, languageCopy) ?? job.message}`;
  }
  return `${job.job_id} ${job.status}`;
}

function summarizeTrussStability(model: Truss2dJobInput, diagnostics: TrussDiagnostics): StabilitySummary {
  const nodeIssueEntries = Object.entries(diagnostics.nodeIssues);
  const issueCount = diagnostics.blockingMessages.length + nodeIssueEntries.reduce((sum, [, issues]) => sum + issues.length, 0);
  const supportCount = model.nodes.reduce((sum, node) => sum + (node.fix_x ? 1 : 0) + (node.fix_y ? 1 : 0), 0);
  const structuralScore = Math.max(0, 100 - issueCount * 14 - Math.max(0, model.nodes.length - model.elements.length - 1) * 8);
  const supportBoost = Math.min(12, supportCount * 3);
  const score = Math.max(0, Math.min(100, structuralScore + supportBoost));
  const hotspotNodes = nodeIssueEntries
    .sort((left, right) => right[1].length - left[1].length)
    .slice(0, 3)
    .map(([nodeIndex]) => Number(nodeIndex));

  if (score >= 80) return { score, tone: "good", hotspotNodes };
  if (score >= 55) return { score, tone: "watch", hotspotNodes };
  return { score, tone: "risk", hotspotNodes };
}

function isTruss3dResult(value: unknown): value is Truss3dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && Array.isArray((value as Truss3dResult).nodes) && (value as Truss3dResult).nodes.some((node) => "z" in node);
}

function buildDisplayTruss3dNodes(model: Truss3dJobInput, result: Truss3dResult | null): DisplayTruss3dNode[] {
  if (result) {
    return result.nodes.map((node, index) => ({
      index,
      id: node.id,
      x: node.x,
      y: node.y,
      z: node.z,
      ux: node.ux,
      uy: node.uy,
      uz: node.uz,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    z: node.z,
    ux: 0,
    uy: 0,
    uz: 0,
  }));
}

function buildDisplayTruss3dElements(model: Truss3dJobInput, result: Truss3dResult | null): DisplayTruss3dElement[] {
  if (result) {
    return result.elements.map((element) => ({ ...element }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);
    const dz = (nodeJ?.z ?? 0) - (nodeI?.z ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy + dz * dz),
      strain: 0,
      stress: 0,
      axial_force: 0,
    };
  });
}

function projectTruss3dPoint(node: { x: number; y: number; z: number }, bounds: ReturnType<typeof getTrussBounds>) {
  const isoX = node.x - node.y * 0.55;
  const isoY = node.z + node.y * 0.35;
  return toSvgPoint({ x: isoX, y: isoY }, bounds);
}

function localMaterialLabel(value: string, language: Language): string {
  const labels = {
    en: {
      "210": "Steel",
      "70": "Aluminum",
      "116": "Titanium",
      "30": "Concrete",
      "135": "Carbon fiber",
      custom: "Custom",
    },
    zh: {
      "210": "钢",
      "70": "铝",
      "116": "钛",
      "30": "混凝土",
      "135": "碳纤维",
      custom: "自定义",
    },
  } as const;

  return labels[language][value as keyof (typeof labels)[Language]] ?? labels[language].custom;
}

function formatTime(value: string | undefined, language: Language): string {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function isAxialResult(value: unknown): value is AxialBarResult {
  return typeof value === "object" && value !== null && "tip_displacement" in value;
}

function isTrussResult(value: unknown): value is Truss2dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && !("tip_displacement" in value) && Array.isArray((value as Truss2dResult).nodes) && !(value as Truss2dResult).nodes.some((node) => "z" in node);
}

function isPlaneTriangleResult(value: unknown): value is PlaneTriangle2dResult {
  return typeof value === "object" && value !== null && "elements" in value && "nodes" in value && "input" in value && Array.isArray((value as PlaneTriangle2dResult).elements) && (value as PlaneTriangle2dResult).elements.some((element) => "node_k" in element);
}

function buildDisplayTrussNodes(model: Truss2dJobInput, result: Truss2dResult | null): DisplayTrussNode[] {
  if (result) {
    return result.nodes.map((node, index) => ({
      index,
      id: node.id,
      x: node.x,
      y: node.y,
      ux: node.ux,
      uy: node.uy,
      fix_x: model.nodes[index]?.fix_x ?? false,
      fix_y: model.nodes[index]?.fix_y ?? false,
      load_x: model.nodes[index]?.load_x ?? 0,
      load_y: model.nodes[index]?.load_y ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    ux: 0,
    uy: 0,
    fix_x: node.fix_x,
    fix_y: node.fix_y,
    load_x: node.load_x,
    load_y: node.load_y,
  }));
}

function buildDisplayTrussElements(model: Truss2dJobInput, result: Truss2dResult | null): DisplayTrussElement[] {
  if (result) {
    return result.elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.strain,
      stress: element.stress,
      axial_force: element.axial_force,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);

    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy),
      strain: 0,
      stress: 0,
      axial_force: 0,
    };
  });
}

function getTrussBounds(nodes: Array<{ x: number; y: number }>) {
  const xs = nodes.map((node) => node.x);
  const ys = nodes.map((node) => node.y);
  const minX = Math.min(...xs, 0);
  const maxX = Math.max(...xs, 1);
  const minY = Math.min(...ys, 0);
  const maxY = Math.max(...ys, 1);
  return {
    minX,
    maxX,
    minY,
    maxY,
    width: Math.max(maxX - minX, 1),
    height: Math.max(maxY - minY, 1),
  };
}

function toSvgPoint(node: { x: number; y: number }, bounds: ReturnType<typeof getTrussBounds>) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;

  return {
    x: paddingX + ((node.x - bounds.minX) / bounds.width) * usableWidth,
    y: 460 - paddingY - ((node.y - bounds.minY) / bounds.height) * usableHeight,
  };
}

function fromSvgPoint(clientX: number, clientY: number, rect: DOMRect, bounds: ReturnType<typeof getTrussBounds>) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;
  const x = ((clientX - rect.left) / rect.width) * 980;
  const y = ((clientY - rect.top) / rect.height) * 460;

  const normalizedX = Math.min(Math.max((x - paddingX) / usableWidth, 0), 1);
  const normalizedY = Math.min(Math.max((460 - paddingY - y) / usableHeight, 0), 1);

  return {
    x: round(bounds.minX + normalizedX * bounds.width),
    y: round(bounds.minY + normalizedY * bounds.height),
  };
}

function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}

function downloadTextFile(filename: string, contents: string) {
  const blob = new Blob([contents], { type: "application/json;charset=utf-8" });
  downloadBlobFile(filename, blob);
}

function downloadBlobFile(filename: string, blob: Blob) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

function resetActiveResult(
  setResult: Dispatch<SetStateAction<AxialBarResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | null>>,
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>,
) {
  setResult(null);
  setJob(null);
}

function pushNodeIssue(nodeIssues: Record<number, string[]>, nodeIndex: number, issue: string) {
  const issues = nodeIssues[nodeIndex] ?? [];
  if (!issues.includes(issue)) {
    nodeIssues[nodeIndex] = [...issues, issue];
  }
}

function findNearestConnectableNode(model: Truss2dJobInput, nodeIndex: number): number | null {
  const origin = model.nodes[nodeIndex];
  if (!origin) return null;

  let bestIndex: number | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;

  for (const [candidateIndex, candidate] of model.nodes.entries()) {
    if (candidateIndex === nodeIndex) continue;

    const alreadyLinked = model.elements.some(
      (element) =>
        (element.node_i === nodeIndex && element.node_j === candidateIndex) ||
        (element.node_i === candidateIndex && element.node_j === nodeIndex),
    );

    if (alreadyLinked) continue;

    const distance = Math.hypot(candidate.x - origin.x, candidate.y - origin.y);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestIndex = candidateIndex;
    }
  }

  return bestIndex;
}

function analyzeTrussModel(
  model: Truss2dJobInput,
  languageCopy: (typeof copy)[Language],
  selectedNode: number | null,
): TrussDiagnostics {
  const nodeCount = model.nodes.length;
  const elementCount = model.elements.length;
  const fixedXCount = model.nodes.filter((node) => node.fix_x).length;
  const fixedYCount = model.nodes.filter((node) => node.fix_y).length;
  const constrainedDofs = model.nodes.reduce(
    (count, node) => count + (node.fix_x ? 1 : 0) + (node.fix_y ? 1 : 0),
    0,
  );
  const blockingMessages: string[] = [];
  const nodeIssues: Record<number, string[]> = {};
  const suggestions: TrussSuggestion[] = [];
  const suggestionIds = new Set<string>();
  const connectionCounts = new Array(nodeCount).fill(0);
  const supportTarget = selectedNode ?? 0;

  const addSuggestion = (suggestion: TrussSuggestion) => {
    if (!suggestionIds.has(suggestion.id)) {
      suggestionIds.add(suggestion.id);
      suggestions.push(suggestion);
    }
  };

  if (constrainedDofs < 2) {
    blockingMessages.push(languageCopy.unstableSupport);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.unstableSupport);
    }
  }

  if (fixedXCount === 0) {
    blockingMessages.push(languageCopy.supportXMissing);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.supportXMissing);
      addSuggestion({
        id: `fix-x-${supportTarget}`,
        kind: "fix_support",
        axis: "x",
        nodeIndex: supportTarget,
        label: selectedNode !== null ? languageCopy.fixCurrentNodeXAction : languageCopy.fixNodeXAction,
      });
    }
  }

  if (fixedYCount === 0) {
    blockingMessages.push(languageCopy.supportYMissing);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.supportYMissing);
      addSuggestion({
        id: `fix-y-${supportTarget}`,
        kind: "fix_support",
        axis: "y",
        nodeIndex: supportTarget,
        label: selectedNode !== null ? languageCopy.fixCurrentNodeYAction : languageCopy.fixNodeYAction,
      });
    }
  }

  if (fixedXCount === 0 || fixedYCount === 0) {
    blockingMessages.push(languageCopy.freeRigidBody);
  }

  if (elementCount < nodeCount - 1) {
    blockingMessages.push(languageCopy.underconnected);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, selectedNode ?? 0, languageCopy.underconnected);
    }
  }

  if (elementCount + constrainedDofs < nodeCount * 2) {
    blockingMessages.push(languageCopy.mechanismRisk);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, selectedNode ?? 0, languageCopy.mechanismRisk);
    }
  }

  for (const element of model.elements) {
    if (element.node_i < nodeCount) connectionCounts[element.node_i] += 1;
    if (element.node_j < nodeCount) connectionCounts[element.node_j] += 1;
  }

  const isolatedNodes = connectionCounts.flatMap((count, index) => (count === 0 ? [index] : []));
  for (const nodeIndex of isolatedNodes) {
    pushNodeIssue(nodeIssues, nodeIndex, languageCopy.isolatedNode);
  }

  if (isolatedNodes.length > 0) {
    blockingMessages.push(languageCopy.isolatedNode);
  }

  const connectTarget =
    (selectedNode !== null && (isolatedNodes.includes(selectedNode) || connectionCounts[selectedNode] < 2)
      ? selectedNode
      : isolatedNodes[0] ?? selectedNode) ?? null;

  if (connectTarget !== null && nodeCount > 1) {
    addSuggestion({
      id: `connect-${connectTarget}`,
      kind: "connect_nearest",
      nodeIndex: connectTarget,
      label: selectedNode !== null ? languageCopy.connectCurrentNodeAction : languageCopy.connectNodeAction,
    });
  }

  return {
    blockingMessages: [...new Set(blockingMessages)],
    nodeIssues,
    suggestions,
  };
}

function planeStressFill(value: number, maxValue: number): string {
  const normalized = maxValue > 0 ? Math.max(0, Math.min(1, value / maxValue)) : 0;
  const hue = 205 - normalized * 180;
  const lightness = 72 - normalized * 22;
  return `hsla(${hue}, 72%, ${lightness}%, 0.72)`;
}

function renderSupportGlyph(
  point: { x: number; y: number },
  constraints: { fix_x: boolean; fix_y: boolean },
  key: string,
) {
  if (!constraints.fix_x && !constraints.fix_y) return null;

  return (
    <g key={key} className="support-glyph">
      {constraints.fix_y ? <line x1={point.x - 12} y1={point.y + 14} x2={point.x + 12} y2={point.y + 14} /> : null}
      {constraints.fix_x ? <line x1={point.x - 14} y1={point.y - 12} x2={point.x - 14} y2={point.y + 12} /> : null}
    </g>
  );
}

function renderLoadGlyph(
  point: { x: number; y: number },
  load: { load_x: number; load_y: number },
  key: string,
) {
  if (Math.abs(load.load_x) < 1.0e-9 && Math.abs(load.load_y) < 1.0e-9) return null;

  const scale = 0.01;
  const x2 = point.x + load.load_x * scale;
  const y2 = point.y - load.load_y * scale;

  return (
    <g key={key} className="load-glyph">
      <line x1={point.x} y1={point.y} x2={x2} y2={y2} />
      <circle cx={x2} cy={y2} r={3.5} />
    </g>
  );
}

export function Workbench() {
  const [studyKind, setStudyKind] = useState<StudyKind>("axial_bar_1d");
  const [axialForm, setAxialForm] = useState<AxialFormState>(defaultAxial);
  const [trussModel, setTrussModel] = useState<Truss2dJobInput>(defaultTruss);
  const [truss3dModel, setTruss3dModel] = useState<Truss3dJobInput>(defaultTruss3d);
  const [planeModel, setPlaneModel] = useState<PlaneTriangle2dJobInput>(defaultPlane);
  const [parametric, setParametric] = useState<ParametricTrussConfig>(defaultParametric);
  const [panelParametric, setPanelParametric] = useState<ParametricPanelConfig>(defaultPanelParametric);
  const [activeMaterial, setActiveMaterial] = useState("210");
  const [result, setResult] = useState<AxialBarResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult | null>(null);
  const [job, setJob] = useState<JobEnvelope["job"] | null>(null);
  const [jobHistory, setJobHistory] = useState<JobState[]>([]);
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [selectedVersionId, setSelectedVersionId] = useState<string | null>(null);
  const [modelVersions, setModelVersions] = useState<ModelVersionRecord[]>([]);
  const [projectNameDraft, setProjectNameDraft] = useState<string>(copy.en.defaultProject);
  const [projectDescriptionDraft, setProjectDescriptionDraft] = useState<string>("");
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [loadedModelName, setLoadedModelName] = useState<string>(copy.en.defaultModel);
  const [message, setMessage] = useState<string>(copy.en.initialLoaded);
  const [language, setLanguage] = useState<Language>("en");
  const [theme, setTheme] = useState<Theme>("linen");
  const [sidebarSection, setSidebarSection] = useState<SidebarSection>("study");
  const [draggingNode, setDraggingNode] = useState<number | null>(null);
  const [selectedNode, setSelectedNode] = useState<number | null>(null);
  const [selectedElement, setSelectedElement] = useState<number | null>(null);
  const [memberDraftNodes, setMemberDraftNodes] = useState<number[]>([]);
  const [undoStack, setUndoStack] = useState<HistoryEntry[]>([]);
  const [redoStack, setRedoStack] = useState<HistoryEntry[]>([]);
  const [isPending, startTransition] = useTransition();
  const dragHistoryCapturedRef = useRef(false);
  const dragFrameRef = useRef<number | null>(null);
  const pendingDragPointRef = useRef<{ x: number; y: number } | null>(null);
  const t = copy[language];

  useEffect(() => {
    const stored = safeStorageGet();
    if (stored.theme) setTheme(stored.theme);
    if (stored.language) {
      setLanguage(stored.language);
      setLoadedModelName(copy[stored.language].defaultModel);
      setMessage(copy[stored.language].initialLoaded);
    }
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    if (typeof window !== "undefined") {
      window.localStorage.setItem(SETTINGS_KEY, JSON.stringify({ theme, language }));
    }
  }, [theme, language]);

  useEffect(() => {
    return () => {
      if (dragFrameRef.current !== null) {
        window.cancelAnimationFrame(dragFrameRef.current);
      }
    };
  }, []);

  useEffect(() => {
    void refreshHealth();
    void refreshJobHistory();
    void refreshProjects(true);
  }, []);

  useEffect(() => {
    const selectedProject = projects.find((project) => project.project_id === selectedProjectId) ?? null;

    if (selectedProject) {
      setProjectNameDraft(selectedProject.name);
      setProjectDescriptionDraft(selectedProject.description ?? "");
    } else if (projects.length === 0) {
      setProjectNameDraft(t.defaultProject);
      setProjectDescriptionDraft("");
    }
  }, [projects, selectedProjectId, t.defaultProject]);

  useEffect(() => {
    if (!selectedModelId) {
      setModelVersions([]);
      return;
    }

    void refreshVersions(selectedModelId);
  }, [selectedModelId]);

  async function refreshHealth() {
    try {
      setHealth(await fetchHealth());
    } catch {
      setHealth(null);
    }
  }

  async function refreshJobHistory() {
    try {
      const payload = await fetchJobHistory();
      setJobHistory(payload.jobs);
    } catch {
      setJobHistory([]);
    }
  }

  async function refreshProjects(bootstrap = false) {
    try {
      const payload = await fetchProjects();
      let nextProjects = payload.projects;

      if (bootstrap && nextProjects.length === 0) {
        const created = await createProject({ name: copy.en.defaultProject, description: "Local workspace" });
        nextProjects = [created.project];
      }

      setProjects(nextProjects);

      const nextProjectId =
        selectedProjectId && nextProjects.some((project) => project.project_id === selectedProjectId)
          ? selectedProjectId
          : nextProjects[0]?.project_id ?? null;

      setSelectedProjectId(nextProjectId);

      const nextModelId =
        selectedModelId &&
        nextProjects.some((project) => (project.models ?? []).some((model) => model.model_id === selectedModelId))
          ? selectedModelId
          : (nextProjects.find((project) => project.project_id === nextProjectId)?.models ?? [])[0]?.model_id ?? null;

      setSelectedModelId(nextModelId);
      if (!nextModelId) {
        setSelectedVersionId(null);
      }
    } catch {
      setProjects([]);
    }
  }

  async function refreshVersions(modelId: string) {
    try {
      const payload = await fetchModelVersions(modelId);
      setModelVersions(payload.versions);
    } catch {
      setModelVersions([]);
    }
  }

  const runAnalysis = () => {
    if (studyKind === "truss_2d") {
      const precheckErrors = trussDiagnostics?.blockingMessages ?? [];
      if (precheckErrors.length > 0) {
        setMessage(`${t.precheckPrefix}: ${precheckErrors[0]}`);
        resetActiveResult(setResult, setJob);
        return;
      }
    }

    setMessage(t.dispatching);
    setResult(null);

    startTransition(async () => {
      try {
        const jobContext = {
          ...(selectedProjectId ? { project_id: selectedProjectId } : {}),
          ...(selectedVersionId ? { model_version_id: selectedVersionId } : {}),
        };

        const created =
          studyKind === "axial_bar_1d"
            ? await createAxialBarJob({ ...toAxialInput(axialForm), ...jobContext })
            : studyKind === "truss_2d"
              ? await createTruss2dJob({ ...trussModel, ...jobContext })
              : studyKind === "truss_3d"
                ? await createTruss3dJob({ ...truss3dModel, ...jobContext })
              : await createPlaneTriangle2dJob({ ...planeModel, ...jobContext });

        setJob(created.job);
        await refreshJobHistory();
        await pollJob(created.job.job_id, studyKind);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const pollJob = async (jobId: string, kind: StudyKind) => {
    for (let attempt = 0; attempt < 40; attempt += 1) {
      const payload =
        kind === "axial_bar_1d"
          ? await fetchJobStatus<AxialBarResult>(jobId)
          : kind === "truss_2d"
            ? await fetchJobStatus<Truss2dResult>(jobId)
            : kind === "truss_3d"
              ? await fetchJobStatus<Truss3dResult>(jobId)
            : await fetchJobStatus<PlaneTriangle2dResult>(jobId);

      setJob(payload.job);

      if (payload.result) {
        setResult(payload.result);
      }

      setMessage(formatJobMessage(payload.job, `${jobId} ${payload.job.status}`, t));

      if (payload.job.status === "completed" || payload.job.status === "failed") {
        await refreshJobHistory();
        return;
      }

      await new Promise((resolve) => window.setTimeout(resolve, 250));
    }
  };

  const openHistoryJob = (jobId: string) => {
    startTransition(async () => {
      try {
        const payload = await fetchJobStatus<AxialBarResult | Truss2dResult | Truss3dResult | PlaneTriangle2dResult>(jobId);
        setJob(payload.job);

        if (payload.result) {
          setResult(payload.result);

          if (isAxialResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("axial_bar_1d");
            setAxialForm({
              length: payload.result.input.length,
              area: payload.result.input.area,
              elements: payload.result.input.elements,
              tipForce: payload.result.input.tip_force,
              material: activeMaterial,
              youngsModulusGpa: round(payload.result.input.youngs_modulus / 1.0e9),
            });
          }

          if (isTrussResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("truss_2d");
            setTrussModel(payload.result.input);
            setSidebarSection("study");
          }

          if (isTruss3dResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("truss_3d");
            setTruss3dModel(payload.result.input);
            setSidebarSection("study");
          }

          if (isPlaneTriangleResult(payload.result)) {
            recordHistory(t.historyAction);
            setStudyKind("plane_triangle_2d");
            setPlaneModel(payload.result.input);
            setSidebarSection("study");
          }
        }

        setMessage(payload.job.status === "failed" ? formatJobMessage(payload.job, t.historyLoaded, t) : t.historyLoaded);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const importModel = async (file: File | undefined) => {
    if (!file) return;

    try {
      const imported = parsePlaygroundModel(await file.text());
      recordHistory(t.importAction);
      setLoadedModelName(imported.name);
      setSelectedModelId(null);
      setSelectedVersionId(null);
      setModelVersions([]);
      setMessage(`${t.importedModel}: ${imported.name}`);

      if (imported.kind === "truss_2d") {
        setStudyKind("truss_2d");
        setTrussModel(imported.model);
        setActiveMaterial(imported.material);
        setParametric((current) => ({
          ...current,
          youngsModulusGpa: imported.youngsModulusGpa,
        }));
      } else if (imported.kind === "truss_3d") {
        setStudyKind("truss_3d");
        setTruss3dModel(imported.model);
        setActiveMaterial(imported.material);
      } else if (imported.kind === "plane_triangle_2d") {
        setStudyKind("plane_triangle_2d");
        setPlaneModel(imported.model);
        setActiveMaterial(imported.material);
      } else {
        setStudyKind("axial_bar_1d");
        setAxialForm({
          length: imported.length,
          area: imported.area,
          elements: imported.elements,
          tipForce: imported.tipForce,
          material: imported.material,
          youngsModulusGpa: imported.youngsModulusGpa,
        });
        setActiveMaterial(imported.material);
      }
    } catch (error) {
      setMessage(error instanceof Error ? `${t.importFailed}: ${error.message}` : t.importFailed);
    }
  };

  const openSample = (href: string) => {
    startTransition(async () => {
      try {
        const response = await fetch(href, { cache: "no-store" });
        const text = await response.text();
        const imported = parsePlaygroundModel(text);
        recordHistory(t.sampleAction);
        setLoadedModelName(imported.name);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);

        if (imported.kind === "plane_triangle_2d") {
          setStudyKind("plane_triangle_2d");
          setPlaneModel(imported.model);
        } else if (imported.kind === "truss_3d") {
          setStudyKind("truss_3d");
          setTruss3dModel(imported.model);
        } else if (imported.kind === "truss_2d") {
          setStudyKind("truss_2d");
          setTrussModel(imported.model);
        } else {
          setStudyKind("axial_bar_1d");
          setAxialForm({
            length: imported.length,
            area: imported.area,
            elements: imported.elements,
            tipForce: imported.tipForce,
            material: imported.material,
            youngsModulusGpa: imported.youngsModulusGpa,
          });
        }

        setMessage(`${t.importedModel}: ${imported.name}`);
      } catch (error) {
        setMessage(error instanceof Error ? `${t.importFailed}: ${error.message}` : t.importFailed);
      }
    });
  };

  const handleAxialFieldChange = (key: keyof AxialFormState, value: number | string) => {
    recordHistory(t.editAxialField);
    setAxialForm((current) => ({ ...current, [key]: value }));
  };

  const handleMaterialChange = (value: string) => {
    recordHistory(t.editMaterial);
    const preset = MATERIAL_PRESETS.find((item) => item.value === value);
    setActiveMaterial(value);
    setAxialForm((current) => ({
      ...current,
      material: value,
      youngsModulusGpa: preset?.modulusGpa ?? current.youngsModulusGpa,
    }));
    setParametric((current) => ({
      ...current,
      youngsModulusGpa: preset?.modulusGpa ?? current.youngsModulusGpa,
    }));
  };

  const handleLanguageChange = (nextLanguage: Language) => {
    setLanguage(nextLanguage);
    setLoadedModelName((current) =>
      current === copy.en.defaultModel || current === copy.zh.defaultModel
        ? copy[nextLanguage].defaultModel
        : current,
    );
  };

  const handleParametricChange = (key: keyof ParametricTrussConfig, value: number) => {
    recordHistory(t.editParametric);
    setParametric((current) => ({ ...current, [key]: value }));
  };

  const handlePanelParametricChange = (key: keyof ParametricPanelConfig, value: number) => {
    recordHistory(t.editParametric);
    setPanelParametric((current) => ({ ...current, [key]: value }));
  };

  const generateModel = () => {
    recordHistory(t.generateAction);
    const nextModel = generatePrattTruss(parametric);
    setStudyKind("truss_2d");
    setTrussModel(nextModel);
    setSelectedNode(null);
    setSelectedElement(null);
    setSelectedModelId(null);
    setSelectedVersionId(null);
    setModelVersions([]);
    setMemberDraftNodes([]);
    setLoadedModelName("parametric-pratt-truss");
    setMessage(t.generatedModel);
    setSidebarSection("model");
  };

  const generatePanelModel = () => {
    recordHistory(t.generateAction);
    const nextModel = generateRectangularPanelMesh(panelParametric);
    setStudyKind("plane_triangle_2d");
    setPlaneModel(nextModel);
    setSelectedNode(null);
    setSelectedElement(null);
    setSelectedModelId(null);
    setSelectedVersionId(null);
    setModelVersions([]);
    setMemberDraftNodes([]);
    setLoadedModelName("parametric-panel-mesh");
    setMessage(t.panelGenerated);
    setSidebarSection("model");
    resetActiveResult(setResult, setJob);
  };

  const downloadModel = () => {
    const contents = JSON.stringify(
      serializeCurrentModel(
        studyKind,
        loadedModelName,
        activeMaterial,
        axialForm,
        trussModel,
        truss3dModel,
        planeModel,
        parametric,
      ),
      null,
      2,
    );

    downloadTextFile(`${loadedModelName || "kyuubiki-model"}.json`, contents);
    setMessage(t.modelDownloaded);
  };

  const downloadResultJson = () => {
    if (!result) {
      setMessage(t.noResultToExport);
      return;
    }

    const payload = {
      exported_at: new Date().toISOString(),
      study_kind: studyKind,
      model_name: loadedModelName,
      job,
      result,
    };

    downloadTextFile(`${loadedModelName || "kyuubiki-study"}-result.json`, JSON.stringify(payload, null, 2));
    setMessage(t.resultJsonDownloaded);
  };

  const downloadResultCsv = () => {
    if (!result) {
      setMessage(t.noResultToExport);
      return;
    }

    const csv = serializeResultCsv(studyKind, job, result);
    downloadTextFile(`${loadedModelName || "kyuubiki-study"}-result.csv`, csv);
    setMessage(t.resultCsvDownloaded);
  };

  const buildProjectBundleJson = async () => {
    if (!selectedProject) {
      throw new Error(t.projectRequired);
    }

    const modelDetails = await Promise.all(
      selectedProjectModels.map(async (model) => {
        const modelEnvelope = await fetchModel(model.model_id);
        const versionsEnvelope = await fetchModelVersions(model.model_id);
        return {
          model: modelEnvelope.model,
          versions: versionsEnvelope.versions,
        };
      }),
    );

    const resultCandidates: Array<JobResultRecord | null> = await Promise.all(
      jobHistory
        .filter((historyJob) => historyJob.has_result)
        .map(async (historyJob) => {
          try {
            const payload = await fetchJobStatus(historyJob.job_id);

            if (!payload.result) {
              return null;
            }

            return {
              job_id: historyJob.job_id,
              status: payload.job.status,
              worker_id: payload.job.worker_id,
              result: payload.result,
            };
          } catch {
            return null;
          }
        }),
    );

    const results = resultCandidates.filter((entry): entry is JobResultRecord => entry !== null);

    return exportProjectBundle({
      project: selectedProject,
      models: modelDetails.map((entry) => entry.model),
      modelVersions: modelDetails.flatMap((entry) => entry.versions),
      activeModelId: selectedModelId,
      activeVersionId: selectedVersionId,
      workspaceSnapshot: serializeCurrentModel(
        studyKind,
        loadedModelName,
        activeMaterial,
        axialForm,
        trussModel,
        truss3dModel,
        planeModel,
        parametric,
      ),
      jobs: jobHistory,
      results,
    });
  };

  const downloadProjectBundleJson = async () => {
    try {
      const bundle = await buildProjectBundleJson();
      downloadTextFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki.json`, bundle);
      setMessage(t.projectExported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const downloadProjectBundleZip = async () => {
    try {
      const bundle = await buildProjectBundleJson();
      const blob = await exportProjectBundleZip(bundle);
      downloadBlobFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki`, blob);
      setMessage(t.projectExported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.initialFailed);
    }
  };

  const importProjectBundle = async (file: File | undefined) => {
    if (!file) return;

    try {
      const bundle = await parseProjectBundleFile(file);
      const createdProject = await createProject({
        name: bundle.project.name,
        description: bundle.project.description ?? "",
      });

      const modelIdMap = new Map<string, string>();

      for (const bundledModel of bundle.models) {
        const bundledVersions = bundle.model_versions
          .filter((version) => version.model_id === bundledModel.model_id)
          .sort((left, right) => left.version_number - right.version_number);

        const baseVersion = bundledVersions[0];
        const createdModel = await createModel(createdProject.project.project_id, {
          name: baseVersion?.name || bundledModel.name,
          kind: bundledModel.kind,
          material: bundledModel.material ?? undefined,
          model_schema_version: bundledModel.model_schema_version,
          payload: (baseVersion?.payload ?? bundledModel.payload) as Record<string, unknown>,
        });

        const newModelId = createdModel.model.model_id;
        modelIdMap.set(bundledModel.model_id, newModelId);

        const initialVersionId = createdModel.model.versions?.[0]?.version_id;
        if (initialVersionId && baseVersion?.name) {
          await updateModelVersion(initialVersionId, { name: baseVersion.name });
        }

        for (const extraVersion of bundledVersions.slice(1)) {
          await createModelVersion(newModelId, {
            name: extraVersion.name,
            kind: extraVersion.kind,
            material: extraVersion.material ?? undefined,
            model_schema_version: extraVersion.model_schema_version,
            payload: extraVersion.payload,
          });
        }
      }

      await refreshProjects();

      const importedActiveModelId =
        (bundle.active_model_id && modelIdMap.get(bundle.active_model_id)) ||
        [...modelIdMap.values()][0] ||
        null;

      setSelectedProjectId(createdProject.project.project_id);
      setSelectedModelId(importedActiveModelId);

      if (bundle.workspace_snapshot) {
        recordHistory(t.importAction);
        applyPersistedPayload(bundle.workspace_snapshot, bundle.project.name);
      }

      if (importedActiveModelId) {
        await refreshVersions(importedActiveModelId);
      } else {
        setModelVersions([]);
      }

      setMessage(t.projectImported);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : t.importFailed);
    }
  };

  const applyPersistedPayload = (payload: Record<string, unknown>, fallbackName?: string) => {
    const imported = parsePlaygroundModel(JSON.stringify(payload));

    if (imported.kind === "plane_triangle_2d") {
      setStudyKind("plane_triangle_2d");
      setPlaneModel(imported.model);
    } else if (imported.kind === "truss_3d") {
      setStudyKind("truss_3d");
      setTruss3dModel(imported.model);
    } else if (imported.kind === "truss_2d") {
      setStudyKind("truss_2d");
      setTrussModel(imported.model);
    } else {
      setStudyKind("axial_bar_1d");
      setAxialForm({
        length: imported.length,
        area: imported.area,
        elements: imported.elements,
        tipForce: imported.tipForce,
        material: imported.material,
        youngsModulusGpa: imported.youngsModulusGpa,
      });
    }

    setLoadedModelName(fallbackName ?? imported.name);
    setActiveMaterial(imported.material);
    resetActiveResult(setResult, setJob);
  };

  const createProjectRecord = () => {
    startTransition(async () => {
      try {
        const payload = await createProject({
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        setSelectedProjectId(payload.project.project_id);
        await refreshProjects();
        setMessage(t.projectCreated);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const updateProjectRecord = () => {
    if (!selectedProjectId) return;

    startTransition(async () => {
      try {
        await updateProject(selectedProjectId, {
          name: projectNameDraft || t.defaultProject,
          description: projectDescriptionDraft,
        });
        await refreshProjects();
        setMessage(t.projectUpdated);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteProjectRecord = () => {
    if (!selectedProjectId) return;
    if (typeof window !== "undefined" && !window.confirm(projectNameDraft)) return;

    startTransition(async () => {
      try {
        await deleteProject(selectedProjectId);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        await refreshProjects();
        setMessage(t.projectDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const saveModelVersion = (saveAs: boolean) => {
    if (!selectedProjectId) {
      setMessage(t.projectRequired);
      return;
    }

    const payload = serializeCurrentModel(
      studyKind,
      loadedModelName,
      activeMaterial,
      axialForm,
      trussModel,
      truss3dModel,
      planeModel,
      parametric,
    );

    startTransition(async () => {
      try {
        if (!selectedModelId || saveAs) {
          const created = await createModel(selectedProjectId, {
            name: loadedModelName,
            kind: studyKind,
            material: activeMaterial,
            model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
            payload,
          });
          setSelectedModelId(created.model.model_id);
          setSelectedVersionId(created.model.latest_version_id ?? null);
          await refreshProjects();
          await refreshVersions(created.model.model_id);
          setMessage(t.modelCreated);
          return;
        }

        await updateModel(selectedModelId, {
          name: loadedModelName,
          kind: studyKind,
          material: activeMaterial,
          model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
          payload,
        });

        const version = await createModelVersion(selectedModelId, {
          name: loadedModelName,
          kind: studyKind,
          material: activeMaterial,
          model_schema_version: String(payload.model_schema_version ?? "kyuubiki.model/v1"),
          payload,
        });

        setSelectedVersionId(version.version.version_id);
        await refreshProjects();
        await refreshVersions(selectedModelId);
        setMessage(t.modelSaved);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const openSavedModel = (model: ModelRecord) => {
    startTransition(async () => {
      try {
        const payload = await fetchModel(model.model_id);
        recordHistory(t.historyAction);
        applyPersistedPayload(payload.model.payload, payload.model.name);
        setSelectedProjectId(payload.model.project_id);
        setSelectedModelId(payload.model.model_id);
        setSelectedVersionId(payload.model.latest_version_id ?? null);
        await refreshVersions(payload.model.model_id);
        setMessage(t.persistedModelLoaded);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const openSavedVersion = (version: ModelVersionRecord) => {
    startTransition(async () => {
      try {
        const payload = await fetchModelVersion(version.version_id);
        recordHistory(t.historyAction);
        applyPersistedPayload(payload.version.payload, payload.version.name);
        setSelectedModelId(payload.version.model_id);
        setSelectedProjectId(payload.version.project_id);
        setSelectedVersionId(payload.version.version_id);
        setMessage(t.versionLoaded);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const renameSelectedVersion = () => {
    if (!selectedVersionId) return;

    startTransition(async () => {
      try {
        await updateModelVersion(selectedVersionId, { name: loadedModelName });
        await refreshVersions(selectedModelId ?? "");
        setMessage(t.versionRenamed);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteSelectedVersion = () => {
    if (!selectedVersionId) return;

    startTransition(async () => {
      try {
        await deleteModelVersion(selectedVersionId);
        setSelectedVersionId(null);
        if (selectedModelId) {
          await refreshVersions(selectedModelId);
        }
        await refreshProjects();
        setMessage(t.versionDeleted);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const deleteSavedModelRecord = () => {
    if (!selectedModelId) return;

    startTransition(async () => {
      try {
        await deleteModel(selectedModelId);
        setSelectedModelId(null);
        setSelectedVersionId(null);
        setModelVersions([]);
        await refreshProjects();
        setMessage(t.modelDeletedStored);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const railItems: Array<{ key: SidebarSection; label: string; symbol: string }> = [
    { key: "study", label: t.rail.study, symbol: "S" },
    { key: "model", label: t.rail.model, symbol: "M" },
    { key: "library", label: t.rail.library, symbol: "H" },
    { key: "system", label: t.rail.system, symbol: "Y" },
  ];

  const selectedProject = projects.find((project) => project.project_id === selectedProjectId) ?? null;
  const selectedProjectModels = selectedProject?.models ?? [];
  const deferredProjectModels = useDeferredValue(selectedProjectModels);
  const deferredModelVersions = useDeferredValue(modelVersions);
  const deferredJobHistory = useDeferredValue(jobHistory);

  const isAxial = studyKind === "axial_bar_1d";
  const isTruss = studyKind === "truss_2d";
  const isTruss3d = studyKind === "truss_3d";
  const isPlane = studyKind === "plane_triangle_2d";
  const axialResult = isAxial && isAxialResult(result) ? result : null;
  const trussResult = isTruss && isTrussResult(result) ? result : null;
  const truss3dResult = isTruss3d && isTruss3dResult(result) ? result : null;
  const planeResult = isPlane && isPlaneTriangleResult(result) ? result : null;
  const trussDiagnostics = isTruss ? analyzeTrussModel(trussModel, t, selectedNode) : null;
  const trussStability = isTruss && trussDiagnostics ? summarizeTrussStability(trussModel, trussDiagnostics) : null;
  const axialNodes = axialResult?.nodes ?? [];
  const axialElements = axialResult?.elements ?? [];
  const axialLength = axialResult?.input.length ?? axialForm.length;
  const axialScale = axialResult?.max_displacement ? 140 / axialResult.max_displacement : 1;
  const displayTrussNodes = buildDisplayTrussNodes(trussModel, trussResult);
  const displayTrussElements = buildDisplayTrussElements(trussModel, trussResult);
  const trussBounds = getTrussBounds(displayTrussNodes);
  const displayTruss3dNodes = buildDisplayTruss3dNodes(truss3dModel, truss3dResult);
  const displayTruss3dElements = buildDisplayTruss3dElements(truss3dModel, truss3dResult);
  const truss3dProjectedBounds = getTrussBounds(
    displayTruss3dNodes.map((node) => ({ x: node.x - node.y * 0.55, y: node.z + node.y * 0.35 })),
  );
  const planeNodes =
    planeResult?.nodes.map((node, index) => ({
      ...planeModel.nodes[index],
      ...node,
      fix_x: planeModel.nodes[index]?.fix_x ?? false,
      fix_y: planeModel.nodes[index]?.fix_y ?? false,
      load_x: planeModel.nodes[index]?.load_x ?? 0,
      load_y: planeModel.nodes[index]?.load_y ?? 0,
    })) ??
    planeModel.nodes.map((node, index) => ({ ...node, index, ux: 0, uy: 0 }));
  const planeElements = planeResult?.elements ?? planeModel.elements.map((element, index) => ({ ...element, index, area: 0, strain_x: 0, strain_y: 0, gamma_xy: 0, stress_x: 0, stress_y: 0, tau_xy: 0, von_mises: 0 }));
  const planeBounds = getTrussBounds(planeNodes);
  const selectedNodeData = selectedNode !== null ? displayTrussNodes[selectedNode] : null;
  const selectedElementData = selectedElement !== null ? displayTrussElements[selectedElement] : null;
  const selectedPlaneNodeData = selectedNode !== null ? planeNodes[selectedNode] : null;
  const selectedPlaneElementData = selectedElement !== null ? planeElements[selectedElement] : null;
  const selectedNodeIssues =
    selectedNode !== null && trussDiagnostics ? trussDiagnostics.nodeIssues[selectedNode] ?? [] : [];
  const translatedFailureReason = humanizeSolverFailure(job?.message, t);
  const planeMaxVonMises = Math.max(...planeElements.map((element) => element.von_mises ?? 0), 0);

  const buildSnapshot = (): WorkbenchSnapshot => ({
    studyKind,
    axialForm,
    trussModel,
    truss3dModel,
    planeModel,
    parametric,
    panelParametric,
    activeMaterial,
    loadedModelName,
    sidebarSection,
    selectedNode,
    selectedElement,
    memberDraftNodes,
  });

  const restoreSnapshot = (snapshot: WorkbenchSnapshot) => {
    setStudyKind(snapshot.studyKind);
    setAxialForm(snapshot.axialForm);
    setTrussModel(snapshot.trussModel);
    setTruss3dModel(snapshot.truss3dModel);
    setPlaneModel(snapshot.planeModel);
    setParametric(snapshot.parametric);
    setPanelParametric(snapshot.panelParametric);
    setActiveMaterial(snapshot.activeMaterial);
    setLoadedModelName(snapshot.loadedModelName);
    setSidebarSection(snapshot.sidebarSection);
    setSelectedNode(snapshot.selectedNode);
    setSelectedElement(snapshot.selectedElement);
    setMemberDraftNodes(snapshot.memberDraftNodes);
    resetActiveResult(setResult, setJob);
  };

  const recordHistory = (label: string) => {
    const snapshot = buildSnapshot();
    setUndoStack((current) => [...current.slice(-39), { label, snapshot }]);
    setRedoStack([]);
  };

  const handleUndo = () => {
    const entry = undoStack.at(-1);
    if (!entry) return;
    const currentSnapshot = buildSnapshot();
    setUndoStack((current) => current.slice(0, -1));
    setRedoStack((current) => [...current.slice(-39), { label: entry.label, snapshot: currentSnapshot }]);
    restoreSnapshot(entry.snapshot);
    setMessage(t.undoApplied);
  };

  const handleRedo = () => {
    const entry = redoStack.at(-1);
    if (!entry) return;
    const currentSnapshot = buildSnapshot();
    setRedoStack((current) => current.slice(0, -1));
    setUndoStack((current) => [...current.slice(-39), { label: entry.label, snapshot: currentSnapshot }]);
    restoreSnapshot(entry.snapshot);
    setMessage(t.redoApplied);
  };

  const applyTrussSuggestion = (suggestion: TrussSuggestion) => {
    recordHistory(t.applySuggestionAction);
    resetActiveResult(setResult, setJob);
    setStudyKind("truss_2d");
    setSidebarSection("model");
    setSelectedElement(null);
    setSelectedNode(suggestion.nodeIndex);
    setMemberDraftNodes([]);

    if (suggestion.kind === "fix_support") {
      setTrussModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === suggestion.nodeIndex
            ? { ...node, [suggestion.axis === "x" ? "fix_x" : "fix_y"]: true }
            : node,
        ),
      }));
      setMessage(suggestion.axis === "x" ? t.suggestionAppliedSupportX : t.suggestionAppliedSupportY);
      return;
    }

    let connected = false;
    setTrussModel((current) => {
      const nearestIndex = findNearestConnectableNode(current, suggestion.nodeIndex);
      if (nearestIndex === null) return current;
      connected = true;
      return {
        ...current,
        elements: [
          ...current.elements,
          {
            id: `e${current.elements.length}`,
            node_i: suggestion.nodeIndex,
            node_j: nearestIndex,
            area: parametric.area,
            youngs_modulus: parametric.youngsModulusGpa * 1.0e9,
          },
        ],
      };
    });
    setMessage(connected ? t.suggestionAppliedLink : t.suggestionNoLinkTarget);
  };

  const updateSelectedNode = (key: keyof Truss2dJobInput["nodes"][number], value: number | boolean) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => ({
      ...current,
      nodes: current.nodes.map((node, index) =>
        index === selectedNode ? { ...node, [key]: value } : node,
      ),
    }));
  };

  const updateSelectedElement = (
    key: keyof Truss2dJobInput["elements"][number],
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => ({
      ...current,
      elements: current.elements.map((element, index) =>
        index === selectedElement ? { ...element, [key]: value } : element,
      ),
    }));
  };

  const updateSelectedPlaneNode = (
    key: keyof PlaneTriangle2dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(t.editNodeAction);
    resetActiveResult(setResult, setJob);
    setPlaneModel((current) => ({
      ...current,
      nodes: current.nodes.map((node, index) =>
        index === selectedNode ? { ...node, [key]: value } : node,
      ),
    }));
  };

  const updateSelectedPlaneElement = (
    key: keyof PlaneTriangle2dJobInput["elements"][number],
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(t.editMemberAction);
    resetActiveResult(setResult, setJob);
    setPlaneModel((current) => ({
      ...current,
      elements: current.elements.map((element, index) =>
        index === selectedElement ? { ...element, [key]: value } : element,
      ),
    }));
  };

  const addNode = (connectToSelected: boolean) => {
    recordHistory(t.addNodeAction);
    setStudyKind("truss_2d");
    setSidebarSection("model");
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => {
      const anchorIndex = connectToSelected ? selectedNode : null;
      const anchor =
        anchorIndex !== null ? current.nodes[anchorIndex] : current.nodes[current.nodes.length - 1];
      const id = `n${current.nodes.length}`;
      const nextNode = {
        id,
        x: round((anchor?.x ?? 0) + 1),
        y: round((anchor?.y ?? 0) + (anchorIndex !== null ? 0.4 : 0.5)),
        fix_x: false,
        fix_y: false,
        load_x: 0,
        load_y: 0,
      };
      const nodes = [...current.nodes, nextNode];
      const elements =
        anchorIndex !== null
          ? [
              ...current.elements,
              {
                id: `e${current.elements.length}`,
                node_i: anchorIndex,
                node_j: nodes.length - 1,
                area: parametric.area,
                youngs_modulus: parametric.youngsModulusGpa * 1.0e9,
              },
            ]
          : current.elements;

      return { ...current, nodes, elements };
    });
    setSelectedNode(trussModel.nodes.length);
    setSelectedElement(connectToSelected && selectedNode !== null ? trussModel.elements.length : null);
    setMemberDraftNodes([]);
    setMessage(connectToSelected && selectedNode !== null ? t.branchCreated : t.nodeCreated);
  };

  const deleteSelectedNode = () => {
    if (selectedNode === null) return;
    recordHistory(t.deleteNodeAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => {
      const nodes = current.nodes.filter((_, index) => index !== selectedNode);
      const elements = current.elements
        .filter((element) => element.node_i !== selectedNode && element.node_j !== selectedNode)
        .map((element, index) => ({
          ...element,
          id: `e${index}`,
          node_i: element.node_i > selectedNode ? element.node_i - 1 : element.node_i,
          node_j: element.node_j > selectedNode ? element.node_j - 1 : element.node_j,
        }));
      return { ...current, nodes, elements };
    });
    setSelectedNode(null);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setMessage(t.nodeDeleted);
  };

  const toggleDraftNode = (index: number) => {
    setSelectedNode(index);
    setSelectedElement(null);
    setMemberDraftNodes((current) => {
      if (current.includes(index)) {
        return current.filter((value) => value !== index);
      }
      return [...current, index].slice(-2);
    });
  };

  const toggleMemberFromDraft = () => {
    if (memberDraftNodes.length !== 2) {
      setMessage(t.selectTwoNodes);
      return;
    }
    recordHistory(t.toggleMemberAction);

    const [nodeI, nodeJ] = memberDraftNodes;
    if (nodeI === nodeJ) return;

    let removedExisting = false;
    let nextSelectedElement: number | null = null;

    resetActiveResult(setResult, setJob);
    setTrussModel((current) => {
      const existingIndex = current.elements.findIndex(
        (element) =>
          (element.node_i === nodeI && element.node_j === nodeJ) ||
          (element.node_i === nodeJ && element.node_j === nodeI),
      );

      if (existingIndex >= 0) {
        removedExisting = true;
        return {
          ...current,
          elements: current.elements
            .filter((_, index) => index !== existingIndex)
            .map((element, index) => ({ ...element, id: `e${index}` })),
        };
      }

      const next = {
        id: `e${current.elements.length}`,
        node_i: nodeI,
        node_j: nodeJ,
        area: parametric.area,
        youngs_modulus: parametric.youngsModulusGpa * 1.0e9,
      };

      nextSelectedElement = current.elements.length;
      return { ...current, elements: [...current.elements, next] };
    });

    setSelectedElement(removedExisting ? null : nextSelectedElement);
    setMemberDraftNodes([]);
    setMessage(removedExisting ? t.memberRemoved : t.memberCreated);
  };

  const deleteSelectedElement = () => {
    if (selectedElement === null) return;
    recordHistory(t.deleteMemberAction);
    resetActiveResult(setResult, setJob);
    setTrussModel((current) => ({
      ...current,
      elements: current.elements
        .filter((_, index) => index !== selectedElement)
        .map((element, index) => ({ ...element, id: `e${index}` })),
    }));
    setSelectedElement(null);
    setMessage(t.memberDeleted);
  };

  const handleTrussPointerMove = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (draggingNode === null || studyKind !== "truss_2d") return;
    if (!dragHistoryCapturedRef.current) {
      recordHistory(t.dragNodeAction);
      dragHistoryCapturedRef.current = true;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const position = fromSvgPoint(event.clientX, event.clientY, rect, trussBounds);
    pendingDragPointRef.current = position;

    if (dragFrameRef.current !== null) {
      return;
    }

    dragFrameRef.current = window.requestAnimationFrame(() => {
      dragFrameRef.current = null;
      const nextPoint = pendingDragPointRef.current;
      if (!nextPoint) return;

      resetActiveResult(setResult, setJob);
      setTrussModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === draggingNode ? { ...node, x: nextPoint.x, y: nextPoint.y } : node,
        ),
      }));
    });
  };

  const stopDraggingNode = () => {
    setDraggingNode(null);
    dragHistoryCapturedRef.current = false;
    pendingDragPointRef.current = null;
    if (dragFrameRef.current !== null) {
      window.cancelAnimationFrame(dragFrameRef.current);
      dragFrameRef.current = null;
    }
  };

  return (
    <div className="workbench-shell">
      <aside className="app-rail panel">
        <div className="rail-brand">
          <strong>{t.brand}</strong>
          <span>v0.3</span>
        </div>
        <div className="rail-nav">
          {railItems.map((item) => (
            <button
              key={item.key}
              className={`rail-button${sidebarSection === item.key ? " rail-button--active" : ""}`}
              onClick={() => setSidebarSection(item.key)}
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
          <p className="eyebrow">{t.brand}</p>
          <h1>{t.title}</h1>
          <p>{t.subtitle}</p>
        </div>

        {sidebarSection === "study" ? (
          <div className="sidebar-stack">
            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.sections.study}</h2>
                <span>{loadedModelName}</span>
              </div>
              <div className="form-grid compact">
                <label>
                  <span>Study Type</span>
                  <select
                    value={studyKind}
                    onChange={(event) => {
                      recordHistory(t.changeStudyType);
                      setStudyKind(event.target.value as StudyKind);
                    }}
                  >
                    <option value="axial_bar_1d">{t.kinds.axial_bar_1d}</option>
                    <option value="truss_2d">{t.kinds.truss_2d}</option>
                    <option value="truss_3d">{t.kinds.truss_3d}</option>
                    <option value="plane_triangle_2d">{t.kinds.plane_triangle_2d}</option>
                  </select>
                </label>
              </div>
              <div className="sidebar-list">
                <div>
                  <span>{t.modelName}</span>
                  <strong>{loadedModelName}</strong>
                </div>
                <div>
                  <span>{t.material}</span>
                  <strong>{localMaterialLabel(activeMaterial, language)}</strong>
                </div>
                <div>
                  <span>{t.mesh}</span>
                  <strong>{isAxial ? axialForm.elements : isTruss ? trussModel.elements.length : isTruss3d ? truss3dModel.elements.length : planeModel.elements.length}</strong>
                </div>
                <div>
                  <span>{t.load}</span>
                  <strong>
                    {isAxial
                      ? `${fixed(axialForm.tipForce, 0)} N`
                      : isTruss
                        ? `${fixed(trussModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`
                        : isTruss3d
                          ? `${fixed(truss3dModel.nodes.reduce((sum, node) => sum + node.load_z, 0), 0)} N`
                        : `${fixed(planeModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`}
                  </strong>
                </div>
                <div>
                  <span>{t.support}</span>
                  <strong>{isAxial ? "Node 0" : isTruss3d ? "Fixed tripod" : "Pinned base"}</strong>
                </div>
              </div>
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.controls}</h2>
                <span>{isPending ? t.busy : t.ready}</span>
              </div>
              {isAxial ? (
                <div className="form-grid compact">
                  <label>
                    <span>{t.length}</span>
                    <input
                      type="number"
                      value={axialForm.length}
                      min={0.1}
                      step={0.1}
                      onChange={(event) => handleAxialFieldChange("length", Number(event.target.value))}
                    />
                  </label>
                  <label>
                    <span>{t.area}</span>
                    <input
                      type="number"
                      value={axialForm.area}
                      min={0.0001}
                      step={0.0001}
                      onChange={(event) => handleAxialFieldChange("area", Number(event.target.value))}
                    />
                  </label>
                  <label>
                    <span>{t.material}</span>
                    <select value={axialForm.material} onChange={(event) => handleMaterialChange(event.target.value)}>
                      {MATERIAL_PRESETS.map((preset) => (
                        <option key={preset.value} value={preset.value}>
                          {localMaterialLabel(preset.value, language)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    <span>{t.modulus}</span>
                    <input
                      type="number"
                      value={axialForm.youngsModulusGpa}
                      min={0.1}
                      step={0.1}
                      onChange={(event) =>
                        handleAxialFieldChange("youngsModulusGpa", Number(event.target.value))
                      }
                    />
                  </label>
                  <label>
                    <span>{t.elements}</span>
                    <input
                      type="number"
                      value={axialForm.elements}
                      min={1}
                      max={120}
                      step={1}
                      onChange={(event) => handleAxialFieldChange("elements", Number(event.target.value))}
                    />
                  </label>
                  <label>
                    <span>{t.tipForce}</span>
                    <input
                      type="number"
                      value={axialForm.tipForce}
                      step={100}
                      onChange={(event) => handleAxialFieldChange("tipForce", Number(event.target.value))}
                    />
                  </label>
                </div>
              ) : isTruss ? (
                <div className="sidebar-list">
                  <div>
                    <span>{t.nodes}</span>
                    <strong>{trussModel.nodes.length}</strong>
                  </div>
                  <div>
                    <span>{t.trussElements}</span>
                    <strong>{trussModel.elements.length}</strong>
                  </div>
                  <div>
                    <span>{t.material}</span>
                    <strong>{localMaterialLabel(activeMaterial, language)}</strong>
                  </div>
                  <div>
                    <span>{t.sourceModel}</span>
                    <strong>{loadedModelName}</strong>
                  </div>
                </div>
              ) : isTruss3d ? (
                <div className="sidebar-list">
                  <div>
                    <span>{t.nodes}</span>
                    <strong>{truss3dModel.nodes.length}</strong>
                  </div>
                  <div>
                    <span>{t.spatialTrussElements}</span>
                    <strong>{truss3dModel.elements.length}</strong>
                  </div>
                  <div>
                    <span>{t.load}</span>
                    <strong>{fixed(truss3dModel.nodes.reduce((sum, node) => sum + node.load_z, 0), 0)} N</strong>
                  </div>
                  <div>
                    <span>{t.sourceModel}</span>
                    <strong>{loadedModelName}</strong>
                  </div>
                </div>
              ) : (
                <div className="sidebar-list">
                  <div>
                    <span>{t.nodes}</span>
                    <strong>{planeModel.nodes.length}</strong>
                  </div>
                  <div>
                    <span>{t.planeElements}</span>
                    <strong>{planeModel.elements.length}</strong>
                  </div>
                  <div>
                    <span>{t.thickness}</span>
                    <strong>{fixed(planeModel.elements[0]?.thickness, 3)}</strong>
                  </div>
                  <div>
                    <span>{t.sourceModel}</span>
                    <strong>{loadedModelName}</strong>
                  </div>
                </div>
              )}
              <button className="solve-button" disabled={isPending} onClick={runAnalysis} type="button">
                {isPending ? t.running : t.run}
              </button>
            </section>
          </div>
        ) : null}

        {sidebarSection === "model" ? (
          <div className="sidebar-stack">
            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.sections.model}</h2>
                <span>{t.dragToEdit}</span>
              </div>
              <p className="card-copy">{isPlane ? t.planeHint : t.modelStudioHint}</p>
              {isTruss ? (
                <>
                  <div className="button-row">
                    <button className="ghost-button" onClick={() => addNode(false)} type="button">
                      {t.addNode}
                    </button>
                    <button className="ghost-button" disabled={selectedNode === null} onClick={() => addNode(true)} type="button">
                      {t.addBranchNode}
                    </button>
                    <button className="ghost-button" disabled={selectedNode === null} onClick={deleteSelectedNode} type="button">
                      {t.deleteNode}
                    </button>
                  </div>
                  <div className="button-row">
                    <button className="ghost-button" onClick={toggleMemberFromDraft} type="button">
                      {t.toggleMember}
                    </button>
                    <button className="ghost-button" disabled={selectedElement === null} onClick={deleteSelectedElement} type="button">
                      {t.deleteMember}
                    </button>
                  </div>
                </>
              ) : null}
              <div className="button-row">
                <button className="ghost-button" disabled={undoStack.length === 0} onClick={handleUndo} type="button">
                  {t.undo}
                </button>
                <button className="ghost-button" disabled={redoStack.length === 0} onClick={handleRedo} type="button">
                  {t.redo}
                </button>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={downloadModel} type="button">
                  {t.download}
                </button>
                <button
                  className="ghost-button"
                  onClick={() => {
                    setStudyKind(isPlane ? "plane_triangle_2d" : "truss_2d");
                    setSidebarSection("study");
                    setMessage(isPlane ? t.planeHint : t.dragHint);
                  }}
                  type="button"
                >
                  {t.saveForSolver}
                </button>
              </div>
              <p className="card-copy">{t.selectionHint}</p>
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{isPlane ? t.panelGenerator : t.parametric}</h2>
                <span>{t.modelTools}</span>
              </div>
              <div className="form-grid compact">
                {isPlane ? (
                  <>
                    <label>
                      <span>{t.length}</span>
                      <input type="number" min={0.2} step={0.1} value={panelParametric.width} onChange={(event) => handlePanelParametricChange("width", Number(event.target.value))} />
                    </label>
                    <label>
                      <span>{t.height}</span>
                      <input type="number" min={0.2} step={0.1} value={panelParametric.height} onChange={(event) => handlePanelParametricChange("height", Number(event.target.value))} />
                    </label>
                    <label>
                      <span>{t.divisionsX}</span>
                      <input type="number" min={1} max={12} step={1} value={panelParametric.divisionsX} onChange={(event) => handlePanelParametricChange("divisionsX", Number(event.target.value))} />
                    </label>
                    <label>
                      <span>{t.divisionsY}</span>
                      <input type="number" min={1} max={12} step={1} value={panelParametric.divisionsY} onChange={(event) => handlePanelParametricChange("divisionsY", Number(event.target.value))} />
                    </label>
                    <label>
                      <span>{t.planeThickness}</span>
                      <input type="number" min={0.001} step={0.001} value={panelParametric.thickness} onChange={(event) => handlePanelParametricChange("thickness", Number(event.target.value))} />
                    </label>
                    <label>
                      <span>{t.modulus}</span>
                      <input type="number" min={0.1} step={0.1} value={panelParametric.youngsModulusGpa} onChange={(event) => handlePanelParametricChange("youngsModulusGpa", Number(event.target.value))} />
                    </label>
                    <label>
                      <span>{t.poissonRatio}</span>
                      <input type="number" min={0.01} max={0.49} step={0.01} value={panelParametric.poissonRatio} onChange={(event) => handlePanelParametricChange("poissonRatio", Number(event.target.value))} />
                    </label>
                    <label>
                      <span>{t.loadCase}</span>
                      <input type="number" step={100} value={panelParametric.loadY} onChange={(event) => handlePanelParametricChange("loadY", Number(event.target.value))} />
                    </label>
                  </>
                ) : (
                  <>
                    <label>
                      <span>{t.bays}</span>
                      <input
                        type="number"
                        min={2}
                        max={10}
                        step={1}
                        value={parametric.bays}
                        onChange={(event) => handleParametricChange("bays", Number(event.target.value))}
                      />
                    </label>
                    <label>
                      <span>{t.length}</span>
                      <input
                        type="number"
                        min={1}
                        step={0.5}
                        value={parametric.span}
                        onChange={(event) => handleParametricChange("span", Number(event.target.value))}
                      />
                    </label>
                    <label>
                      <span>{t.height}</span>
                      <input
                        type="number"
                        min={0.2}
                        step={0.1}
                        value={parametric.height}
                        onChange={(event) => handleParametricChange("height", Number(event.target.value))}
                      />
                    </label>
                    <label>
                      <span>{t.area}</span>
                      <input
                        type="number"
                        min={0.0001}
                        step={0.0001}
                        value={parametric.area}
                        onChange={(event) => handleParametricChange("area", Number(event.target.value))}
                      />
                    </label>
                    <label>
                      <span>{t.modulus}</span>
                      <input
                        type="number"
                        min={0.1}
                        step={0.1}
                        value={parametric.youngsModulusGpa}
                        onChange={(event) =>
                          handleParametricChange("youngsModulusGpa", Number(event.target.value))
                        }
                      />
                    </label>
                    <label>
                      <span>{t.loadCase}</span>
                      <input
                        type="number"
                        step={100}
                        value={parametric.loadY}
                        onChange={(event) => handleParametricChange("loadY", Number(event.target.value))}
                      />
                    </label>
                  </>
                )}
              </div>
              <button className="solve-button" onClick={isPlane ? generatePanelModel : generateModel} type="button">
                {isPlane ? t.generatePanel : t.generate}
              </button>
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.objectTree}</h2>
                <span>{isTruss ? `${memberDraftNodes.length}/2` : planeModel.elements.length}</span>
              </div>
              <p className="card-copy">{isPlane ? t.planeHint : t.dragHint}</p>
              <div className="table-scroll small-table">
                <table>
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>X</th>
                      <th>Y</th>
                      <th>{isTruss ? t.diagnostics : t.loadCase}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(isPlane ? planeModel.nodes : trussModel.nodes).map((node, index) => (
                      <tr
                        key={node.id}
                        className={[
                          selectedNode === index ? "table-row--active" : "",
                          isTruss && (trussDiagnostics?.nodeIssues[index] ?? []).length > 0 ? "table-row--warning" : "",
                        ]
                          .filter(Boolean)
                          .join(" ")}
                        onClick={() => {
                          if (isPlane) {
                            setSelectedNode(index);
                            setSelectedElement(null);
                          } else {
                            toggleDraftNode(index);
                          }
                        }}
                      >
                        <td>{node.id}</td>
                        <td>{fixed(node.x, 2)}</td>
                        <td>{fixed(node.y, 2)}</td>
                        <td>
                          {isTruss && (trussDiagnostics?.nodeIssues[index] ?? []).length > 0 ? (
                            <span className="issue-badge">{(trussDiagnostics?.nodeIssues[index] ?? []).length}</span>
                          ) : (
                            fixed(node.load_y, 0)
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="table-scroll small-table model-tree-spacer">
                <table>
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>{t.nodeI}</th>
                      <th>{isPlane ? t.nodeK : t.nodeJ}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(isPlane ? planeElements : displayTrussElements).map((element, index) => (
                      <tr
                        key={element.id}
                        className={selectedElement === index ? "table-row--active" : ""}
                        onClick={() => {
                          setSelectedElement(index);
                          setSelectedNode(null);
                          if (isTruss) setMemberDraftNodes([]);
                        }}
                      >
                        <td>{element.id}</td>
                        <td>{element.node_i}</td>
                        <td>{"node_k" in element ? element.node_k : element.node_j}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          </div>
        ) : null}

        {sidebarSection === "library" ? (
          <div className="sidebar-stack">
            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.sampleLibrary}</h2>
                <button
                  className="link-button"
                  onClick={() => {
                    void refreshJobHistory();
                    void refreshProjects();
                  }}
                  type="button"
                >
                  {t.refresh}
                </button>
              </div>
              <p className="card-copy">{t.historyHint}</p>
              <label className="import-box">
                <span>{t.importModel}</span>
                <small>{t.importHint}</small>
                <input
                  type="file"
                  accept=".json,application/json"
                  onChange={(event) => importModel(event.target.files?.[0])}
                />
              </label>
              <VirtualList
                className="history-list"
                items={SAMPLE_LIBRARY}
                itemHeight={102}
                maxHeight={328}
                itemKey={(sample) => sample.id}
                renderItem={(sample) => (
                  <button className="history-item" onClick={() => openSample(sample.href)} type="button">
                    <strong>{sample.name}</strong>
                    <span>{t.kinds[sample.kind]}</span>
                    <small>{sample.summary}</small>
                  </button>
                )}
              />
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.projectLibrary}</h2>
                <span>{projects.length}</span>
              </div>
              <div className="form-grid compact">
                <label>
                  <span>{t.projectNameField}</span>
                  <input value={projectNameDraft} onChange={(event) => setProjectNameDraft(event.target.value)} />
                </label>
                <label>
                  <span>{t.projectDescriptionField}</span>
                  <input value={projectDescriptionDraft} onChange={(event) => setProjectDescriptionDraft(event.target.value)} />
                </label>
                <label>
                  <span>{t.projectLibrary}</span>
                  <select
                    value={selectedProjectId ?? ""}
                    onChange={(event) => {
                      setSelectedProjectId(event.target.value || null);
                      setSelectedModelId(null);
                    }}
                  >
                    <option value="">{t.none}</option>
                    {projects.map((project) => (
                      <option key={project.project_id} value={project.project_id}>
                        {project.name}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={createProjectRecord} type="button">
                  {t.createProject}
                </button>
                <button className="ghost-button" disabled={!selectedProjectId} onClick={updateProjectRecord} type="button">
                  {t.updateProject}
                </button>
                <button className="ghost-button" disabled={!selectedProjectId} onClick={deleteProjectRecord} type="button">
                  {t.deleteProject}
                </button>
              </div>
              <div className="button-row">
                <button className="ghost-button" disabled={!selectedProjectId} onClick={() => void downloadProjectBundleJson()} type="button">
                  {t.exportProjectJson}
                </button>
                <button className="ghost-button" disabled={!selectedProjectId} onClick={() => void downloadProjectBundleZip()} type="button">
                  {t.exportProjectZip}
                </button>
              </div>
              <label className="import-box">
                <span>{t.importProject}</span>
                <small>{t.importProjectHint}</small>
                <input
                  type="file"
                  accept=".kyuubiki,.kyuubiki.json,application/json,application/zip"
                  onChange={(event) => void importProjectBundle(event.target.files?.[0])}
                />
              </label>
              {projects.length === 0 ? <p className="card-copy">{t.projectEmpty}</p> : null}
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.savedModels}</h2>
                <span>{selectedProjectModels.length}</span>
              </div>
              <div className="form-grid compact">
                <label>
                  <span>{t.modelName}</span>
                  <input value={loadedModelName} onChange={(event) => setLoadedModelName(event.target.value)} />
                </label>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={() => saveModelVersion(false)} type="button">
                  {t.save}
                </button>
                <button className="ghost-button" onClick={() => saveModelVersion(true)} type="button">
                  {t.saveAs}
                </button>
                <button className="ghost-button" disabled={!selectedModelId} onClick={deleteSavedModelRecord} type="button">
                  {t.deleteSavedModel}
                </button>
              </div>
              <VirtualList
                className="history-list"
                items={deferredProjectModels}
                itemHeight={112}
                maxHeight={344}
                emptyState={<p className="card-copy">{t.noSavedModels}</p>}
                itemKey={(model) => model.model_id}
                renderItem={(model) => (
                  <button
                    className={`history-item${selectedModelId === model.model_id ? " history-item--active" : ""}`}
                    onClick={() => openSavedModel(model)}
                    type="button"
                  >
                    <strong>{model.name}</strong>
                    <span>{t.kinds[model.kind as keyof typeof t.kinds] ?? model.kind}</span>
                    <small>
                      {t.updatedAt}: {formatTime(model.updated_at, language)}
                    </small>
                    <small>
                      v{model.latest_version_number ?? 1}
                    </small>
                  </button>
                )}
              />
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.versions}</h2>
                <span>{modelVersions.length}</span>
              </div>
              <div className="button-row">
                <button className="ghost-button" disabled={!selectedVersionId} onClick={renameSelectedVersion} type="button">
                  {t.renameVersion}
                </button>
                <button className="ghost-button" disabled={!selectedVersionId} onClick={deleteSelectedVersion} type="button">
                  {t.deleteVersion}
                </button>
              </div>
              <VirtualList
                className="history-list"
                items={deferredModelVersions}
                itemHeight={100}
                maxHeight={320}
                emptyState={<p className="card-copy">{t.noVersions}</p>}
                itemKey={(version) => version.version_id}
                renderItem={(version) => (
                  <button
                    className={`history-item${selectedVersionId === version.version_id ? " history-item--active" : ""}`}
                    onClick={() => openSavedVersion(version)}
                    type="button"
                  >
                    <strong>{version.name}</strong>
                    <span>v{version.version_number}</span>
                    <small>
                      {t.updatedAt}: {formatTime(version.updated_at, language)}
                    </small>
                  </button>
                )}
              />
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.sections.library}</h2>
                <span>{jobHistory.length}</span>
              </div>
              <VirtualList
                className="history-list"
                items={deferredJobHistory}
                itemHeight={112}
                maxHeight={360}
                emptyState={<p className="card-copy">{t.historyEmpty}</p>}
                itemKey={(historyJob) => historyJob.job_id}
                renderItem={(historyJob) => (
                  <button
                    className={`history-item${job?.job_id === historyJob.job_id ? " history-item--active" : ""}`}
                    onClick={() => openHistoryJob(historyJob.job_id)}
                    type="button"
                  >
                    <strong>{historyJob.job_id.slice(0, 8)}</strong>
                    <span>{historyJob.status}</span>
                    <small>
                      {t.updatedAt}: {formatTime(historyJob.updated_at, language)}
                    </small>
                    <small>
                      {t.hasResult}: {historyJob.has_result ? t.yes : t.no}
                    </small>
                  </button>
                )}
              />
            </section>
          </div>
        ) : null}

        {sidebarSection === "system" ? (
          <div className="sidebar-stack">
            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.settings}</h2>
                <span>{health?.status === "ok" ? t.online : t.offline}</span>
              </div>
              <div className="form-grid compact">
                <label>
                  <span>{t.theme}</span>
                  <select value={theme} onChange={(event) => setTheme(event.target.value as Theme)}>
                    <option value="linen">{t.themes.linen}</option>
                    <option value="marine">{t.themes.marine}</option>
                    <option value="graphite">{t.themes.graphite}</option>
                  </select>
                </label>
                <label>
                  <span>{t.language}</span>
                  <select value={language} onChange={(event) => handleLanguageChange(event.target.value as Language)}>
                    <option value="en">{t.languages.en}</option>
                    <option value="zh">{t.languages.zh}</option>
                  </select>
                </label>
              </div>
            </section>
            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.backend}</h2>
                <span>{health?.status ?? t.offline}</span>
              </div>
              <div className="sidebar-list">
                <div>
                  <span>{t.ui}</span>
                  <strong>3000</strong>
                </div>
                <div>
                  <span>{t.orchestrator}</span>
                  <strong>{health ? "4000" : t.offline}</strong>
                </div>
                <div>
                  <span>{t.solverAgent}</span>
                  <strong>{health?.transport?.solver_agent_tcp ?? 5001}</strong>
                </div>
              </div>
            </section>
          </div>
        ) : null}
      </aside>

      <main className="workspace-main">
        <section className="panel canvas-panel">
          <div className="panel-head">
            <h2>{sidebarSection === "model" ? t.sections.model : t.viewport}</h2>
            <span>{job?.status ?? "idle"}</span>
          </div>
          <WorkbenchViewport
            studyKind={studyKind}
            sidebarSection={sidebarSection}
            title={t.sections.model}
            axialTitle={t.kinds.axial_bar_1d}
            trussTitle={t.kinds.truss_2d}
            truss3dTitle={t.kinds.truss_3d}
            planeTitle={t.kinds.plane_triangle_2d}
            axialNodes={axialNodes}
            axialLength={axialLength}
            axialScale={axialScale}
            displayTrussNodes={displayTrussNodes}
            displayTrussElements={displayTrussElements}
            trussBounds={trussBounds}
            trussResult={Boolean(trussResult)}
            trussHotspotNodes={trussStability?.hotspotNodes ?? []}
            trussNodeIssues={trussDiagnostics?.nodeIssues ?? {}}
            selectedNode={selectedNode}
            selectedElement={selectedElement}
            memberDraftNodes={memberDraftNodes}
            onTrussPointerMove={handleTrussPointerMove}
            onStopDraggingNode={stopDraggingNode}
            onSelectTrussElement={(index) => {
              setSelectedElement(index);
              setSelectedNode(null);
              setMemberDraftNodes([]);
            }}
            onStartTrussNodeDrag={(index) => {
              dragHistoryCapturedRef.current = false;
              setDraggingNode(index);
              toggleDraftNode(index);
            }}
            displayTruss3dNodes={displayTruss3dNodes}
            displayTruss3dElements={displayTruss3dElements}
            truss3dProjectedBounds={truss3dProjectedBounds}
            planeNodes={planeNodes}
            planeElements={planeElements}
            planeBounds={planeBounds}
            planeResult={Boolean(planeResult)}
            planeMaxVonMises={planeMaxVonMises}
            selectedPlaneNodeId={selectedPlaneNodeData?.id ?? null}
            onSelectPlaneElement={(index) => {
              setSelectedElement(index);
              setSelectedNode(null);
            }}
            onSelectPlaneNode={(index) => {
              setSelectedNode(index);
              setSelectedElement(null);
            }}
          />
        </section>

        <WorkbenchConsole
          sidebarSection={sidebarSection}
          title={sidebarSection === "model" ? t.nodeTable : t.report}
          subtitle={message}
          modelMessageTitle={t.dragNode}
          reportMessageTitle={t.messages}
          message={message}
          dragNodeLabel={t.dragNode}
          noNodeSelectedLabel={t.noNodeSelected}
          loadCaseLabel={t.loadCase}
          diagnosticsLabel={t.diagnostics}
          selectedNodeId={isPlane ? selectedPlaneNodeData?.id ?? null : selectedNodeData?.id ?? null}
          selectedNodeX={isPlane ? selectedPlaneNodeData?.x : selectedNodeData?.x}
          selectedNodeY={isPlane ? selectedPlaneNodeData?.y : selectedNodeData?.y}
          selectedNodeLoadY={isPlane ? selectedPlaneNodeData?.load_y : selectedNodeData?.load_y}
          selectedNodeIssueCount={isPlane ? null : selectedNodeIssues.length > 0 ? selectedNodeIssues.length : null}
          elementTitle={isAxial ? t.axialElements : isTruss ? t.trussElements : isTruss3d ? t.spatialTrussElements : t.planeElements}
          spanLabel={t.span}
          stressLabel={t.stress}
          axialForceLabel={t.axialForce}
          elements={(isAxial ? axialElements : isTruss ? displayTrussElements : isTruss3d ? displayTruss3dElements : planeElements) as Array<{
            index: number;
            x1?: number;
            x2?: number;
            node_i?: number;
            node_j?: number;
            node_k?: number;
            stress?: number;
            axial_force?: number;
            von_mises?: number;
          }>}
        />
      </main>

      <WorkbenchInspector
        t={t}
        sidebarSection={sidebarSection}
        studyKind={studyKind}
        isPending={isPending}
        selectedNodeData={selectedNodeData ? { ...selectedNodeData } : null}
        selectedElementData={selectedElementData ? { ...selectedElementData } : null}
        selectedPlaneNodeData={selectedPlaneNodeData ? { ...selectedPlaneNodeData } : null}
        selectedPlaneElementData={selectedPlaneElementData ? { ...selectedPlaneElementData } : null}
        trussElementArea={selectedElementData ? trussModel.elements[selectedElementData.index]?.area ?? 0 : 0}
        trussElementModulusGpa={selectedElementData ? round((trussModel.elements[selectedElementData.index]?.youngs_modulus ?? 0) / 1.0e9) : 0}
        planeElementThickness={selectedPlaneElementData ? planeModel.elements[selectedPlaneElementData.index]?.thickness ?? 0 : 0}
        planeElementModulusGpa={selectedPlaneElementData ? round((planeModel.elements[selectedPlaneElementData.index]?.youngs_modulus ?? 0) / 1.0e9) : 0}
        planeElementPoissonRatio={selectedPlaneElementData ? planeModel.elements[selectedPlaneElementData.index]?.poisson_ratio ?? 0.33 : 0.33}
        onUpdateSelectedNode={updateSelectedNode}
        onUpdateSelectedElement={updateSelectedElement}
        onUpdateSelectedPlaneNode={updateSelectedPlaneNode}
        onUpdateSelectedPlaneElement={updateSelectedPlaneElement}
        trussDiagnostics={trussDiagnostics}
        trussStability={trussStability}
        hotspotNodeLabels={(trussStability?.hotspotNodes ?? []).map((nodeIndex) => trussModel.nodes[nodeIndex]?.id ?? nodeIndex).join(", ")}
        onApplyTrussSuggestion={(id) => {
          const suggestion = trussDiagnostics?.suggestions.find((entry) => entry.id === id);
          if (suggestion) applyTrussSuggestion(suggestion);
        }}
        undoStack={undoStack}
        redoStack={redoStack}
        onUndo={handleUndo}
        onRedo={handleRedo}
        job={job}
        nodeCount={isAxial ? axialNodes.length : isTruss ? displayTrussNodes.length : isTruss3d ? displayTruss3dNodes.length : planeNodes.length}
        tipDisplacement={isAxial ? scientific(axialResult?.tip_displacement) : isTruss ? scientific(trussResult?.max_displacement) : isTruss3d ? scientific(truss3dResult?.max_displacement) : scientific(planeResult?.max_displacement)}
        maxStressValue={scientific(isAxial ? axialResult?.max_stress : isTruss ? trussResult?.max_stress : isTruss3d ? truss3dResult?.max_stress : planeResult?.max_stress)}
        reactionValue={isAxial ? scientific(axialResult?.reaction_force) : "--"}
        createdAtValue={formatTime(job?.created_at, language)}
        updatedAtValue={formatTime(job?.updated_at, language)}
        failureReasonValue={translatedFailureReason ?? job?.message ?? "--"}
        onDownloadJson={downloadResultJson}
        onDownloadCsv={downloadResultCsv}
      />
    </div>
  );
}
