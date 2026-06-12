"use client";

import { serializeWorkbenchPythonLiteral } from "./workbench-script-python-format";
import type {
  WorkbenchScriptLanguage,
  WorkbenchScriptSnippetParameterValue,
  WorkbenchScriptSnippetParameters,
} from "./workbench-script-runtime-types";

export type WorkbenchScriptSnippetParameter = {
  key: string;
  defaultValue: WorkbenchScriptSnippetParameterValue;
};

export type WorkbenchScriptSnippetDefinition = {
  id: string;
  category: "runtime" | "navigation" | "workflow" | "inspection";
  title: {
    en: string;
    zh: string;
  };
  summary: {
    en: string;
    zh: string;
  };
  parameters?: WorkbenchScriptSnippetParameter[];
  code: string;
};

export const WORKBENCH_SCRIPT_SNIPPETS: WorkbenchScriptSnippetDefinition[] = [
  {
    id: "snippet/runtime/open-control-panel",
    category: "runtime",
    title: {
      en: "Open runtime control panel",
      zh: "打开运行时控制面",
    },
    summary: {
      en: "Switch to the system sidebar, move into runtime control, and assert the built-in control window exists.",
      zh: "切到 system 侧栏，进入 runtime control，并断言内建 control window 已出现。",
    },
    parameters: [
      { key: "section", defaultValue: "system" },
      { key: "systemPanelTab", defaultValue: "runtime" },
      { key: "runtimeTab", defaultValue: "control" },
      { key: "settleSeconds", defaultValue: 0.1 },
    ],
    code: `await ky.invoke("nav/setSidebarSection", {"section": snippet_params["section"]})
await ky.invoke("nav/setTabs", {"systemPanelTab": snippet_params["systemPanelTab"]})

runtime_panel = ky.query_selector("runtimePanel")
if runtime_panel is None:
    raise RuntimeError("Runtime panel not found from UI contract")

control_tab = ky.query_selector("runtimeTab", snippet_params["runtimeTab"])
if control_tab is None:
    raise RuntimeError("Runtime control tab not found from UI contract")

control_tab.click()
await ky.sleep(snippet_params["settleSeconds"])

control_window = ky.query_selector("controlWindow")
if control_window is None:
    raise RuntimeError("Control window did not appear")

ky.log("Control window ready")`,
  },
  {
    id: "snippet/runtime/export-topology-snapshot",
    category: "runtime",
    title: {
      en: "Export topology snapshot",
      zh: "导出拓扑快照",
    },
    summary: {
      en: "Open the control page and trigger the built-in snapshot export button through stable selectors.",
      zh: "打开 control 页面，并通过稳定选择器触发内建快照导出按钮。",
    },
    parameters: [
      { key: "section", defaultValue: "system" },
      { key: "systemPanelTab", defaultValue: "runtime" },
      { key: "runtimeTab", defaultValue: "control" },
      { key: "settleSeconds", defaultValue: 0.1 },
    ],
    code: `await ky.invoke("nav/setSidebarSection", {"section": snippet_params["section"]})
await ky.invoke("nav/setTabs", {"systemPanelTab": snippet_params["systemPanelTab"]})

control_tab = ky.query_selector("runtimeTab", snippet_params["runtimeTab"])
if control_tab is not None:
    control_tab.click()
    await ky.sleep(snippet_params["settleSeconds"])

export_button = ky.query_selector("controlWindowExportButton")
if export_button is None:
    raise RuntimeError("Topology export button not found")

export_button.click()
ky.log("Triggered topology snapshot export")`,
  },
  {
    id: "snippet/workflow/focus-workflow-surface",
    category: "workflow",
    title: {
      en: "Focus workflow surface",
      zh: "聚焦工作流界面",
    },
    summary: {
      en: "Move to the workflow sidebar and verify the built-in viewport and inspector surfaces are both mounted.",
      zh: "切到 workflow 侧栏，并验证内建 viewport 与 inspector 面都已挂载。",
    },
    parameters: [{ key: "section", defaultValue: "workflow" }],
    code: `await ky.invoke("nav/setSidebarSection", {"section": snippet_params["section"]})

workflow_sidebar = ky.query_selector("sidebarSection", snippet_params["section"])
viewport_panel = ky.query_selector("viewportPanel")
inspector_panel = ky.query_selector("inspector")

if workflow_sidebar is None:
    raise RuntimeError("Workflow sidebar surface missing")
if viewport_panel is None:
    raise RuntimeError("Viewport panel missing")
if inspector_panel is None:
    raise RuntimeError("Inspector panel missing")

ky.log("Workflow surface ready for scripted inspection")`,
  },
  {
    id: "snippet/inspection/assert-built-in-shell",
    category: "inspection",
    title: {
      en: "Assert built-in shell anchors",
      zh: "断言内建壳层锚点",
    },
    summary: {
      en: "Check that the fixed built-in shell anchors are present before running a larger frontend automation flow.",
      zh: "在执行较大前端自动化流程前，先检查固定内建壳层锚点是否存在。",
    },
    parameters: [
      { key: "requiredKeys", defaultValue: "shell,sidebar,console,inspector,viewportPanel,viewportStage" },
    ],
    code: `required_keys = [
    key.strip()
    for key in str(snippet_params["requiredKeys"]).split(",")
    if key.strip()
]

missing = [key for key in required_keys if not ky.selector_exists(key)]
if missing:
    raise RuntimeError(f"Missing built-in shell anchors: {', '.join(missing)}")

ky.log("Workbench UI contract version:", ui_contract["contractVersion"])
ky.log("All required built-in shell anchors are present")`,
  },
];

export function getWorkbenchScriptSnippetLabel(
  snippet: WorkbenchScriptSnippetDefinition,
  language: WorkbenchScriptLanguage,
) {
  return language === "zh" ? snippet.title.zh : snippet.title.en;
}

export function getWorkbenchScriptSnippetSummary(
  snippet: WorkbenchScriptSnippetDefinition,
  language: WorkbenchScriptLanguage,
) {
  return language === "zh" ? snippet.summary.zh : snippet.summary.en;
}

export function getWorkbenchScriptSnippetParameterDefaults(
  snippet: WorkbenchScriptSnippetDefinition,
) : WorkbenchScriptSnippetParameters {
  return Object.fromEntries((snippet.parameters ?? []).map((parameter) => [parameter.key, parameter.defaultValue]));
}

export function renderWorkbenchScriptSnippet(
  snippet: WorkbenchScriptSnippetDefinition,
  parameters?: WorkbenchScriptSnippetParameters,
) {
  const parameterDefaults = {
    ...getWorkbenchScriptSnippetParameterDefaults(snippet),
    ...(parameters ?? {}),
  };
  const parameterBlock =
    snippet.parameters && snippet.parameters.length > 0
      ? `snippet_params = ${serializeWorkbenchPythonLiteral(parameterDefaults)}\n# Edit snippet_params before running if this flow needs different targets.\n\n`
      : "";
  return `# ${snippet.id}\n${parameterBlock}${snippet.code}\n`;
}
