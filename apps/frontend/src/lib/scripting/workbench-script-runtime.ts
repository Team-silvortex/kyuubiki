"use client";

export type WorkbenchScriptLanguage = "en" | "zh";

export type WorkbenchScriptSnapshot = {
  studyKind: string;
  sidebarSection: string;
  studyTab: string;
  modelTab: string;
  libraryTab: string;
  systemPanelTab: string;
  language: WorkbenchScriptLanguage;
  theme: string;
  frontendRuntimeMode: string;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  selectedVersionId: string | null;
  loadedModelName: string;
  activeMaterial: string;
  selectedNode: number | null;
  selectedElement: number | null;
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
  status: "started" | "completed" | "failed";
  at: string;
  summary: string;
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

export function getWorkbenchScriptActionDefinition(actionId: string) {
  return WORKBENCH_SCRIPT_ACTIONS.find((entry) => entry.id === actionId) ?? null;
}

export function isWorkbenchScriptActionHighRisk(actionId: string) {
  return Boolean(getWorkbenchScriptActionDefinition(actionId)?.requiresConfirmation);
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
# - ky.log(*parts)
# - await ky.invoke("action/id", payload_dict)
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

    def log(self, *parts):
        __kyuubikiBridge.log(" ".join(str(part) for part in parts))

    async def invoke(self, action, payload=None):
        if payload is None:
            payload = {}
        result = await __kyuubikiBridge.invoke(action, json.dumps(payload))
        return json.loads(result)

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
`;
}
