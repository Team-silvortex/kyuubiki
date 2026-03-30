"use client";

import { useEffect, useState, useTransition, type PointerEvent as ReactPointerEvent } from "react";
import { MATERIAL_PRESETS } from "@/lib/materials";
import { parsePlaygroundModel } from "@/lib/model-import";
import { exportStudyModel, generatePrattTruss, type ParametricTrussConfig } from "@/lib/modeler";
import { SAMPLE_LIBRARY } from "@/lib/sample-library";
import {
  createAxialBarJob,
  createPlaneTriangle2dJob,
  createTruss2dJob,
  fetchHealth,
  fetchJobHistory,
  fetchJobStatus,
  type AxialBarJobInput,
  type AxialBarResult,
  type HealthPayload,
  type JobEnvelope,
  type JobState,
  type PlaneTriangle2dJobInput,
  type PlaneTriangle2dResult,
  type Truss2dJobInput,
  type Truss2dResult,
} from "@/lib/api";

type Language = "en" | "zh";
type Theme = "linen" | "marine" | "graphite";
type SidebarSection = "study" | "model" | "library" | "system";
type StudyKind = "axial_bar_1d" | "truss_2d" | "plane_triangle_2d";

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

type SelectionKind = "node" | "element";

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

const SETTINGS_KEY = "kyuubiki-workbench-settings";

const copy = {
  en: {
    brand: "Kyuubiki",
    title: "Structural Workbench",
    subtitle: "A formal front-end workbench for modeling, orchestration, and solver review.",
    rail: { study: "Study", model: "Model", library: "History", system: "System" },
    sections: { study: "Study Setup", model: "Model Studio", library: "Job History", system: "System" },
    kinds: { axial_bar_1d: "1D axial bar", truss_2d: "2D truss", plane_triangle_2d: "2D plane triangle" },
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
    modelTools: "Editing Tools",
    dragHint: "Drag truss nodes directly in the viewport to reshape geometry.",
    parametric: "Parametric Generator",
    generate: "Generate",
    download: "Download JSON",
    saveForSolver: "Use current model",
    bays: "Bays",
    height: "Height (m)",
    nodeTable: "Nodes",
    dragNode: "Selected node",
    noNodeSelected: "No node selected",
    loadCase: "Load case",
    modelStudioHint: "Modeling is currently enabled for 2D truss studies.",
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
    sampleLibrary: "Sample Library",
  objectTree: "Object Tree",
    properties: "Properties",
    addNode: "Add Node",
    deleteNode: "Delete Node",
    addMember: "Add Member",
    deleteMember: "Delete Member",
    selectTwoNodes: "Select two nodes to create a member.",
    memberCreated: "Member created.",
    nodeCreated: "Node created.",
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
    none: "None",
  },
  zh: {
    brand: "Kyuubiki",
    title: "结构分析工作台",
    subtitle: "更正式的前端工作台，统一建模、编排与求解回看。",
    rail: { study: "研究", model: "建模", library: "历史", system: "系统" },
    sections: { study: "研究设置", model: "建模工作室", library: "任务历史", system: "系统" },
    kinds: { axial_bar_1d: "一维轴向杆", truss_2d: "二维桁架", plane_triangle_2d: "二维三角形单元" },
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
    modelTools: "编辑工具",
    dragHint: "直接在视图区拖拽桁架节点来修改几何。",
    parametric: "参数化生成",
    generate: "生成模型",
    download: "下载 JSON",
    saveForSolver: "使用当前模型",
    bays: "跨数",
    height: "高度 (m)",
    nodeTable: "节点列表",
    dragNode: "当前节点",
    noNodeSelected: "未选择节点",
    loadCase: "载荷工况",
    modelStudioHint: "当前建模页先支持二维桁架。",
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
    sampleLibrary: "样板库",
    objectTree: "对象树",
    properties: "属性",
    addNode: "新增节点",
    deleteNode: "删除节点",
    addMember: "新增杆件",
    deleteMember: "删除杆件",
    selectTwoNodes: "请选择两个节点来创建杆件。",
    memberCreated: "杆件已创建。",
    nodeCreated: "节点已创建。",
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
    none: "无",
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
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && !("tip_displacement" in value);
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
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

export function Workbench() {
  const [studyKind, setStudyKind] = useState<StudyKind>("axial_bar_1d");
  const [axialForm, setAxialForm] = useState<AxialFormState>(defaultAxial);
  const [trussModel, setTrussModel] = useState<Truss2dJobInput>(defaultTruss);
  const [planeModel, setPlaneModel] = useState<PlaneTriangle2dJobInput>(defaultPlane);
  const [parametric, setParametric] = useState<ParametricTrussConfig>(defaultParametric);
  const [activeMaterial, setActiveMaterial] = useState("210");
  const [result, setResult] = useState<AxialBarResult | Truss2dResult | PlaneTriangle2dResult | null>(null);
  const [job, setJob] = useState<JobEnvelope["job"] | null>(null);
  const [jobHistory, setJobHistory] = useState<JobState[]>([]);
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
  const [isPending, startTransition] = useTransition();
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
    void refreshHealth();
    void refreshJobHistory();
  }, []);

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

  const runAnalysis = () => {
    setMessage(t.dispatching);
    setResult(null);

    startTransition(async () => {
      try {
        const created =
          studyKind === "axial_bar_1d"
            ? await createAxialBarJob(toAxialInput(axialForm))
            : studyKind === "truss_2d"
              ? await createTruss2dJob(trussModel)
              : await createPlaneTriangle2dJob(planeModel);

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
            : await fetchJobStatus<PlaneTriangle2dResult>(jobId);

      setJob(payload.job);

      if (payload.result) {
        setResult(payload.result);
      }

      setMessage(`${jobId} ${payload.job.status}`);

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
        const payload = await fetchJobStatus<AxialBarResult | Truss2dResult | PlaneTriangle2dResult>(jobId);
        setJob(payload.job);

        if (payload.result) {
          setResult(payload.result);

          if (isAxialResult(payload.result)) {
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
            setStudyKind("truss_2d");
            setTrussModel(payload.result.input);
            setSidebarSection("study");
          }

          if (isPlaneTriangleResult(payload.result)) {
            setStudyKind("plane_triangle_2d");
            setPlaneModel(payload.result.input);
            setSidebarSection("study");
          }
        }

        setMessage(t.historyLoaded);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const importModel = async (file: File | undefined) => {
    if (!file) return;

    try {
      const imported = parsePlaygroundModel(await file.text());
      setLoadedModelName(imported.name);
      setMessage(`${t.importedModel}: ${imported.name}`);

      if (imported.kind === "truss_2d") {
        setStudyKind("truss_2d");
        setTrussModel(imported.model);
        setActiveMaterial(imported.material);
        setParametric((current) => ({
          ...current,
          youngsModulusGpa: imported.youngsModulusGpa,
        }));
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
        setLoadedModelName(imported.name);

        if (imported.kind === "plane_triangle_2d") {
          setStudyKind("plane_triangle_2d");
          setPlaneModel(imported.model);
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
    setAxialForm((current) => ({ ...current, [key]: value }));
  };

  const handleMaterialChange = (value: string) => {
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
    setParametric((current) => ({ ...current, [key]: value }));
  };

  const generateModel = () => {
    const nextModel = generatePrattTruss(parametric);
    setStudyKind("truss_2d");
    setTrussModel(nextModel);
    setSelectedNode(null);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setLoadedModelName("parametric-pratt-truss");
    setMessage(t.generatedModel);
    setSidebarSection("model");
  };

  const downloadModel = () => {
    const contents = exportStudyModel(studyKind, {
      name: loadedModelName,
      material: activeMaterial,
      youngsModulusGpa:
        studyKind === "axial_bar_1d"
          ? axialForm.youngsModulusGpa
          : studyKind === "truss_2d"
            ? parametric.youngsModulusGpa
            : round((planeModel.elements[0]?.youngs_modulus ?? 0) / 1.0e9),
      axial: studyKind === "axial_bar_1d" ? toAxialInput(axialForm) : undefined,
      truss: studyKind === "truss_2d" ? trussModel : undefined,
      plane: studyKind === "plane_triangle_2d" ? planeModel : undefined,
    });

    downloadTextFile(`${loadedModelName || "kyuubiki-model"}.json`, contents);
    setMessage(t.modelDownloaded);
  };

  const railItems: Array<{ key: SidebarSection; label: string; symbol: string }> = [
    { key: "study", label: t.rail.study, symbol: "S" },
    { key: "model", label: t.rail.model, symbol: "M" },
    { key: "library", label: t.rail.library, symbol: "H" },
    { key: "system", label: t.rail.system, symbol: "Y" },
  ];

  const isAxial = studyKind === "axial_bar_1d";
  const isTruss = studyKind === "truss_2d";
  const isPlane = studyKind === "plane_triangle_2d";
  const axialResult = isAxial && isAxialResult(result) ? result : null;
  const trussResult = isTruss && isTrussResult(result) ? result : null;
  const planeResult = isPlane && isPlaneTriangleResult(result) ? result : null;
  const axialNodes = axialResult?.nodes ?? [];
  const axialElements = axialResult?.elements ?? [];
  const axialLength = axialResult?.input.length ?? axialForm.length;
  const axialScale = axialResult?.max_displacement ? 140 / axialResult.max_displacement : 1;
  const displayTrussNodes = buildDisplayTrussNodes(trussModel, trussResult);
  const displayTrussElements = buildDisplayTrussElements(trussModel, trussResult);
  const trussBounds = getTrussBounds(displayTrussNodes);
  const planeNodes = planeResult?.nodes ?? planeModel.nodes.map((node, index) => ({ ...node, index, ux: 0, uy: 0 }));
  const planeElements = planeResult?.elements ?? planeModel.elements.map((element, index) => ({ ...element, index, area: 0, strain_x: 0, strain_y: 0, gamma_xy: 0, stress_x: 0, stress_y: 0, tau_xy: 0, von_mises: 0 }));
  const planeBounds = getTrussBounds(planeNodes);
  const selectedNodeData = selectedNode !== null ? displayTrussNodes[selectedNode] : null;
  const selectedElementData = selectedElement !== null ? displayTrussElements[selectedElement] : null;

  const updateSelectedNode = (key: keyof Truss2dJobInput["nodes"][number], value: number | boolean) => {
    if (selectedNode === null) return;
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
    setTrussModel((current) => ({
      ...current,
      elements: current.elements.map((element, index) =>
        index === selectedElement ? { ...element, [key]: value } : element,
      ),
    }));
  };

  const addNode = () => {
    setStudyKind("truss_2d");
    setSidebarSection("model");
    setTrussModel((current) => {
      const id = `n${current.nodes.length}`;
      const nextNode = {
        id,
        x: round((current.nodes[current.nodes.length - 1]?.x ?? 0) + 1),
        y: round(current.nodes[current.nodes.length - 1]?.y ?? 0.5),
        fix_x: false,
        fix_y: false,
        load_x: 0,
        load_y: 0,
      };
      return { ...current, nodes: [...current.nodes, nextNode] };
    });
    setSelectedNode(trussModel.nodes.length);
    setSelectedElement(null);
    setMessage(t.nodeCreated);
  };

  const deleteSelectedNode = () => {
    if (selectedNode === null) return;
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

  const addMemberFromDraft = () => {
    if (memberDraftNodes.length !== 2) {
      setMessage(t.selectTwoNodes);
      return;
    }

    const [nodeI, nodeJ] = memberDraftNodes;
    if (nodeI === nodeJ) return;

    setTrussModel((current) => {
      const exists = current.elements.some(
        (element) =>
          (element.node_i === nodeI && element.node_j === nodeJ) ||
          (element.node_i === nodeJ && element.node_j === nodeI),
      );

      if (exists) return current;

      const next = {
        id: `e${current.elements.length}`,
        node_i: nodeI,
        node_j: nodeJ,
        area: parametric.area,
        youngs_modulus: parametric.youngsModulusGpa * 1.0e9,
      };

      return { ...current, elements: [...current.elements, next] };
    });

    setSelectedElement(trussModel.elements.length);
    setMemberDraftNodes([]);
    setMessage(t.memberCreated);
  };

  const deleteSelectedElement = () => {
    if (selectedElement === null) return;
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
    const rect = event.currentTarget.getBoundingClientRect();
    const position = fromSvgPoint(event.clientX, event.clientY, rect, trussBounds);

    setTrussModel((current) => ({
      ...current,
      nodes: current.nodes.map((node, index) =>
        index === draggingNode ? { ...node, x: position.x, y: position.y } : node,
      ),
    }));
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
                  <select value={studyKind} onChange={(event) => setStudyKind(event.target.value as StudyKind)}>
                    <option value="axial_bar_1d">{t.kinds.axial_bar_1d}</option>
                    <option value="truss_2d">{t.kinds.truss_2d}</option>
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
                  <strong>{isAxial ? axialForm.elements : isTruss ? trussModel.elements.length : planeModel.elements.length}</strong>
                </div>
                <div>
                  <span>{t.load}</span>
                  <strong>
                    {isAxial
                      ? `${fixed(axialForm.tipForce, 0)} N`
                      : isTruss
                        ? `${fixed(trussModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`
                        : `${fixed(planeModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`}
                  </strong>
                </div>
                <div>
                  <span>{t.support}</span>
                  <strong>{isAxial ? "Node 0" : "Pinned base"}</strong>
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
              <p className="card-copy">{t.modelStudioHint}</p>
              <div className="button-row">
                <button className="ghost-button" onClick={addNode} type="button">
                  {t.addNode}
                </button>
                <button className="ghost-button" disabled={selectedNode === null} onClick={deleteSelectedNode} type="button">
                  {t.deleteNode}
                </button>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={addMemberFromDraft} type="button">
                  {t.addMember}
                </button>
                <button className="ghost-button" disabled={selectedElement === null} onClick={deleteSelectedElement} type="button">
                  {t.deleteMember}
                </button>
              </div>
              <div className="button-row">
                <button className="ghost-button" onClick={downloadModel} type="button">
                  {t.download}
                </button>
                <button
                  className="ghost-button"
                  onClick={() => {
                    setStudyKind("truss_2d");
                    setSidebarSection("study");
                    setMessage(t.dragHint);
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
                <h2>{t.parametric}</h2>
                <span>{t.modelTools}</span>
              </div>
              <div className="form-grid compact">
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
              </div>
              <button className="solve-button" onClick={generateModel} type="button">
                {t.generate}
              </button>
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.objectTree}</h2>
                <span>{memberDraftNodes.length}/2</span>
              </div>
              <p className="card-copy">{t.dragHint}</p>
              <div className="table-scroll small-table">
                <table>
                  <thead>
                    <tr>
                      <th>ID</th>
                      <th>X</th>
                      <th>Y</th>
                    </tr>
                  </thead>
                  <tbody>
                    {trussModel.nodes.map((node, index) => (
                      <tr
                        key={node.id}
                        className={selectedNode === index ? "table-row--active" : ""}
                        onClick={() => toggleDraftNode(index)}
                      >
                        <td>{node.id}</td>
                        <td>{fixed(node.x, 2)}</td>
                        <td>{fixed(node.y, 2)}</td>
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
                      <th>{t.nodeJ}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {displayTrussElements.map((element, index) => (
                      <tr
                        key={element.id}
                        className={selectedElement === index ? "table-row--active" : ""}
                        onClick={() => {
                          setSelectedElement(index);
                          setSelectedNode(null);
                          setMemberDraftNodes([]);
                        }}
                      >
                        <td>{element.id}</td>
                        <td>{element.node_i}</td>
                        <td>{element.node_j}</td>
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
                <button className="link-button" onClick={() => void refreshJobHistory()} type="button">
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
              <div className="history-list">
                {SAMPLE_LIBRARY.map((sample) => (
                  <button key={sample.id} className="history-item" onClick={() => openSample(sample.href)} type="button">
                    <strong>{sample.name}</strong>
                    <span>{t.kinds[sample.kind]}</span>
                    <small>{sample.summary}</small>
                  </button>
                ))}
              </div>
            </section>

            <section className="sidebar-card">
              <div className="card-head">
                <h2>{t.sections.library}</h2>
                <span>{jobHistory.length}</span>
              </div>
              <div className="history-list">
                {jobHistory.length === 0 ? <p className="card-copy">{t.historyEmpty}</p> : null}
                {jobHistory.map((historyJob) => (
                  <button
                    key={historyJob.job_id}
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
                ))}
              </div>
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
          {isAxial ? (
            <svg viewBox="0 0 980 460" className="viewport-svg" aria-label="Axial bar response">
              <defs>
                <linearGradient id="beamGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                  <stop offset="0%" stopColor="var(--accent-cool)" />
                  <stop offset="100%" stopColor="var(--accent)" />
                </linearGradient>
              </defs>
              <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
              <text x="48" y="58" className="svg-title">
                {t.kinds.axial_bar_1d}
              </text>
              <line x1="80" y1="160" x2="880" y2="160" className="guide" />
              <line x1="80" y1="360" x2="880" y2="360" className="guide guide--soft" />
              {axialNodes.length > 0 ? (
                <>
                  <polyline
                    points={axialNodes.map((node) => `${80 + (node.x / axialLength) * 800},160`).join(" ")}
                    className="bar bar--base"
                  />
                  <polyline
                    points={axialNodes
                      .map(
                        (node) =>
                          `${80 + (node.x / axialLength) * 800 + node.displacement * axialScale},360`,
                      )
                      .join(" ")}
                    className="bar bar--deformed"
                  />
                </>
              ) : null}
            </svg>
          ) : isTruss ? (
            <svg
              viewBox="0 0 980 460"
              className="viewport-svg"
              aria-label="2d truss response"
              onPointerLeave={() => setDraggingNode(null)}
              onPointerMove={handleTrussPointerMove}
              onPointerUp={() => setDraggingNode(null)}
            >
              <defs>
                <linearGradient id="beamGradientTruss" x1="0%" y1="0%" x2="100%" y2="0%">
                  <stop offset="0%" stopColor="var(--accent-cool)" />
                  <stop offset="100%" stopColor="var(--accent)" />
                </linearGradient>
              </defs>
              <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
              <text x="48" y="58" className="svg-title">
                {sidebarSection === "model" ? t.sections.model : t.kinds.truss_2d}
              </text>
              {displayTrussElements.map((element) => {
                const i = displayTrussNodes[element.node_i];
                const j = displayTrussNodes[element.node_j];
                const start = toSvgPoint(i, trussBounds);
                const end = toSvgPoint(j, trussBounds);

                return (
                  <line
                    key={`base-${element.id}`}
                    x1={start.x}
                    y1={start.y}
                    x2={end.x}
                    y2={end.y}
                    className="bar bar--base"
                  />
                );
              })}

              {trussResult
                ? displayTrussElements.map((element) => {
                    const i = displayTrussNodes[element.node_i];
                    const j = displayTrussNodes[element.node_j];
                    const start = toSvgPoint({ x: i.x + i.ux * 10000, y: i.y + i.uy * 10000 }, trussBounds);
                    const end = toSvgPoint({ x: j.x + j.ux * 10000, y: j.y + j.uy * 10000 }, trussBounds);

                    return (
                      <line
                        key={`def-${element.id}`}
                        x1={start.x}
                        y1={start.y}
                        x2={end.x}
                        y2={end.y}
                        className="bar bar--deformed-truss"
                      />
                    );
                  })
                : null}

              {displayTrussNodes.map((node, index) => {
                const point = toSvgPoint(node, trussBounds);
                const deformed = toSvgPoint({ x: node.x + node.ux * 10000, y: node.y + node.uy * 10000 }, trussBounds);

                return (
                  <g key={node.id}>
                    <circle
                      cx={point.x}
                      cy={point.y}
                      r={sidebarSection === "model" ? 10 : 7}
                      className={`node-base${selectedNode === index ? " node-base--active" : ""}${memberDraftNodes.includes(index) ? " node-base--draft" : ""}`}
                      onPointerDown={() => {
                        if (sidebarSection === "model") {
                          setDraggingNode(index);
                          toggleDraftNode(index);
                        }
                      }}
                    />
                    <text x={point.x + 12} y={point.y - 10} className="node-label">
                      {node.id}
                    </text>
                    {trussResult ? <circle cx={deformed.x} cy={deformed.y} r={5} className="node-deformed" /> : null}
                  </g>
                );
              })}
            </svg>
          ) : (
            <svg viewBox="0 0 980 460" className="viewport-svg" aria-label="2d plane triangle response">
              <rect x="16" y="16" width="948" height="428" rx="26" className="viewport-frame" />
              <text x="48" y="58" className="svg-title">
                {t.kinds.plane_triangle_2d}
              </text>
              {planeElements.map((element) => {
                const i = toSvgPoint(planeNodes[element.node_i], planeBounds);
                const j = toSvgPoint(planeNodes[element.node_j], planeBounds);
                const k = toSvgPoint(planeNodes[element.node_k], planeBounds);
                return (
                  <polygon
                    key={`plane-${element.id}`}
                    points={`${i.x},${i.y} ${j.x},${j.y} ${k.x},${k.y}`}
                    className="plane-triangle"
                  />
                );
              })}
              {planeResult
                ? planeElements.map((element) => {
                    const i = toSvgPoint({ x: planeNodes[element.node_i].x + planeNodes[element.node_i].ux * 5000, y: planeNodes[element.node_i].y + planeNodes[element.node_i].uy * 5000 }, planeBounds);
                    const j = toSvgPoint({ x: planeNodes[element.node_j].x + planeNodes[element.node_j].ux * 5000, y: planeNodes[element.node_j].y + planeNodes[element.node_j].uy * 5000 }, planeBounds);
                    const k = toSvgPoint({ x: planeNodes[element.node_k].x + planeNodes[element.node_k].ux * 5000, y: planeNodes[element.node_k].y + planeNodes[element.node_k].uy * 5000 }, planeBounds);
                    return (
                      <polygon
                        key={`plane-def-${element.id}`}
                        points={`${i.x},${i.y} ${j.x},${j.y} ${k.x},${k.y}`}
                        className="plane-triangle plane-triangle--deformed"
                      />
                    );
                  })
                : null}
              {planeNodes.map((node) => {
                const point = toSvgPoint(node, planeBounds);
                return (
                  <g key={node.id}>
                    <circle cx={point.x} cy={point.y} r={6} className="node-base" />
                    <text x={point.x + 10} y={point.y - 10} className="node-label">
                      {node.id}
                    </text>
                  </g>
                );
              })}
            </svg>
          )}
        </section>

        <section className="panel console-panel">
          <div className="panel-head">
            <h2>{sidebarSection === "model" ? t.nodeTable : t.report}</h2>
            <span>{message}</span>
          </div>
          <div className="console-grid">
            <div className="console-card">
              <h3>{sidebarSection === "model" ? t.dragNode : t.messages}</h3>
              {sidebarSection === "model" ? (
                <div className="metric-grid">
                  <div>
                    <span>ID</span>
                    <strong>{selectedNodeData?.id ?? t.noNodeSelected}</strong>
                  </div>
                  <div>
                    <span>X</span>
                    <strong>{fixed(selectedNodeData?.x)}</strong>
                  </div>
                  <div>
                    <span>Y</span>
                    <strong>{fixed(selectedNodeData?.y)}</strong>
                  </div>
                  <div>
                    <span>{t.loadCase}</span>
                    <strong>{fixed(selectedNodeData?.load_y, 0)} N</strong>
                  </div>
                </div>
              ) : (
                <p>{message}</p>
              )}
            </div>
            <div className="console-card">
              <h3>{isAxial ? t.axialElements : isTruss ? t.trussElements : t.planeElements}</h3>
              <div className="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>{t.span}</th>
                      <th>{t.stress}</th>
                      <th>{t.axialForce}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(isAxial ? axialElements : isTruss ? displayTrussElements : planeElements).map((element) => (
                      <tr key={element.index}>
                        <td>{element.index}</td>
                        <td>
                          {"x1" in element
                            ? `${fixed(element.x1, 2)} - ${fixed(element.x2, 2)}`
                            : "node_k" in element
                              ? `${element.node_i} - ${element.node_j} - ${element.node_k}`
                              : `${element.node_i} - ${element.node_j}`}
                        </td>
                        <td>{scientific("von_mises" in element ? element.von_mises : element.stress)}</td>
                        <td>{scientific("axial_force" in element ? element.axial_force : undefined)}</td>
                      </tr>
                    ))}
                    {(isAxial ? axialElements.length : isTruss ? displayTrussElements.length : planeElements.length) === 0 ? (
                      <tr>
                        <td colSpan={4}>--</td>
                      </tr>
                    ) : null}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </section>
      </main>

      <aside className="workspace-inspector panel">
        <div className="panel-head">
          <h2>{t.overview}</h2>
          <span>{isPending ? t.busy : t.ready}</span>
        </div>
        <div className="inspector-stack">
          {sidebarSection === "model" ? (
            <section className="info-card">
              <h3>{t.properties}</h3>
              {selectedNodeData ? (
                <div className="form-grid compact">
                  <label>
                    <span>{t.dragNode}</span>
                    <input value={selectedNodeData.id} readOnly />
                  </label>
                  <label>
                    <span>{t.nodeX}</span>
                    <input
                      type="number"
                      step={0.1}
                      value={selectedNodeData.x}
                      onChange={(event) => updateSelectedNode("x", Number(event.target.value))}
                    />
                  </label>
                  <label>
                    <span>{t.nodeY}</span>
                    <input
                      type="number"
                      step={0.1}
                      value={selectedNodeData.y}
                      onChange={(event) => updateSelectedNode("y", Number(event.target.value))}
                    />
                  </label>
                  <label>
                    <span>{t.loadX}</span>
                    <input
                      type="number"
                      step={100}
                      value={selectedNodeData.load_x}
                      onChange={(event) => updateSelectedNode("load_x", Number(event.target.value))}
                    />
                  </label>
                  <label>
                    <span>{t.loadY}</span>
                    <input
                      type="number"
                      step={100}
                      value={selectedNodeData.load_y}
                      onChange={(event) => updateSelectedNode("load_y", Number(event.target.value))}
                    />
                  </label>
                  <label className="toggle-row">
                    <span>{t.fixX}</span>
                    <input
                      type="checkbox"
                      checked={selectedNodeData.fix_x}
                      onChange={(event) => updateSelectedNode("fix_x", event.target.checked)}
                    />
                  </label>
                  <label className="toggle-row">
                    <span>{t.fixY}</span>
                    <input
                      type="checkbox"
                      checked={selectedNodeData.fix_y}
                      onChange={(event) => updateSelectedNode("fix_y", event.target.checked)}
                    />
                  </label>
                </div>
              ) : selectedElementData ? (
                <div className="form-grid compact">
                  <label>
                    <span>{t.memberSelection}</span>
                    <input value={selectedElementData.id} readOnly />
                  </label>
                  <label>
                    <span>{t.nodeI}</span>
                    <input value={selectedElementData.node_i} readOnly />
                  </label>
                  <label>
                    <span>{t.nodeJ}</span>
                    <input value={selectedElementData.node_j} readOnly />
                  </label>
                  <label>
                    <span>{t.area}</span>
                    <input
                      type="number"
                      step={0.0001}
                      value={selectedElementData ? trussModel.elements[selectedElementData.index]?.area ?? 0 : 0}
                      onChange={(event) => updateSelectedElement("area", Number(event.target.value))}
                    />
                  </label>
                  <label>
                    <span>{t.modulus}</span>
                    <input
                      type="number"
                      step={0.1}
                      value={
                        selectedElementData
                          ? round((trussModel.elements[selectedElementData.index]?.youngs_modulus ?? 0) / 1.0e9)
                          : 0
                      }
                      onChange={(event) =>
                        updateSelectedElement("youngs_modulus", Number(event.target.value) * 1.0e9)
                      }
                    />
                  </label>
                </div>
              ) : (
                <p className="card-copy">{t.selectionHint}</p>
              )}
            </section>
          ) : null}
          <section className="info-card">
            <h3>{t.metrics}</h3>
            <div className="metric-grid">
              <div>
                <span>{t.status}</span>
                <strong>{job?.status ?? "--"}</strong>
              </div>
              <div>
                <span>{t.worker}</span>
                <strong>{job?.worker_id ?? "--"}</strong>
              </div>
              <div>
                <span>{t.progress}</span>
                <strong>{typeof job?.progress === "number" ? `${Math.round(job.progress * 100)}%` : "--"}</strong>
              </div>
              <div>
                <span>{t.iteration}</span>
                <strong>{job?.iteration ?? "--"}</strong>
              </div>
              <div>
                <span>{t.residual}</span>
                <strong>{scientific(job?.residual)}</strong>
              </div>
              <div>
                <span>{t.nodes}</span>
                <strong>{isAxial ? axialNodes.length : isTruss ? displayTrussNodes.length : planeNodes.length}</strong>
              </div>
            </div>
          </section>
          <section className="info-card">
            <h3>{t.report}</h3>
            <div className="metric-grid">
              <div>
                <span>{t.tipDisp}</span>
                <strong>{isAxial ? scientific(axialResult?.tip_displacement) : isTruss ? scientific(trussResult?.max_displacement) : scientific(planeResult?.max_displacement)}</strong>
              </div>
              <div>
                <span>{t.maxStress}</span>
                <strong>{scientific(isAxial ? axialResult?.max_stress : isTruss ? trussResult?.max_stress : planeResult?.max_stress)}</strong>
              </div>
              <div>
                <span>{t.reaction}</span>
                <strong>{isAxial ? scientific(axialResult?.reaction_force) : "--"}</strong>
              </div>
              <div>
                <span>{t.createdAt}</span>
                <strong>{formatTime(job?.created_at, language)}</strong>
              </div>
              <div>
                <span>{t.updatedAt}</span>
                <strong>{formatTime(job?.updated_at, language)}</strong>
              </div>
              <div>
                <span>{t.hasResult}</span>
                <strong>{job?.has_result ? t.yes : t.no}</strong>
              </div>
            </div>
          </section>
        </div>
      </aside>
    </div>
  );
}
