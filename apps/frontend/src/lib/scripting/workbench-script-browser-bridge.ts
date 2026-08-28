"use client";

import {
  getWorkbenchScriptActionDefinition,
  getWorkbenchScriptMacroDefinition,
  WORKBENCH_SCRIPT_ACTIONS,
  WORKBENCH_SCRIPT_MACROS,
} from "./workbench-script-runtime-catalog.ts";
import {
  CLOSED_LOOP_TRUSS_RECIPE_ID,
  ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID,
  ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID,
  HEAT_TO_THERMO_QUAD_RECIPE_ID,
  HEAT_TO_THERMO_TRIANGLE_RECIPE_ID,
  filterWorkbenchScriptRecipes,
  getWorkbenchScriptRecipeDefinition,
  type WorkbenchScriptRecipeFilters,
  WORKBENCH_SCRIPT_RECIPES,
} from "./workbench-script-runtime-recipes.ts";
import { buildWorkbenchUiAutomationContractSnapshot } from "./workbench-script-ui-automation.ts";
import type {
  WorkbenchScriptMacroStep,
  WorkbenchScriptSnapshot,
} from "./workbench-script-runtime-types.ts";

type ScriptActionSource = "script" | "assistant" | "hub-assistant" | "";

type InvokeAction = (
  action: string,
  payload?: Record<string, unknown>,
  source?: ScriptActionSource,
  note?: string,
) => Promise<unknown>;

export type WorkbenchPwdtWaitOptions = {
  timeoutMs?: number;
  intervalMs?: number;
};

export type WorkbenchPwdtTrussStudyParams = {
  projectName?: string;
  projectDescription?: string;
  modelName?: string;
  activeMaterial?: string | number;
  bays?: number;
  span?: number;
  height?: number;
  loadY?: number;
  timeoutMs?: number;
  timeoutSeconds?: number;
};

export type WorkbenchPwdtHeatThermoQuadParams = {
  projectName?: string;
  projectDescription?: string;
  heatModelName?: string;
  thermoModelName?: string;
  activeMaterial?: string | number;
  timeoutMs?: number;
  timeoutSeconds?: number;
};

export type WorkbenchPwdtHeatThermoTriangleParams = WorkbenchPwdtHeatThermoQuadParams;

export type WorkbenchPwdtElectrostaticHeatThermoQuadParams = WorkbenchPwdtHeatThermoQuadParams & {
  electrostaticModelName?: string;
};

export type WorkbenchPwdtElectrostaticHeatThermoTriangleParams = WorkbenchPwdtHeatThermoTriangleParams & {
  electrostaticModelName?: string;
};

export type WorkbenchPwdtRecipeParams =
  WorkbenchPwdtTrussStudyParams &
  WorkbenchPwdtHeatThermoQuadParams &
  WorkbenchPwdtHeatThermoTriangleParams &
  WorkbenchPwdtElectrostaticHeatThermoQuadParams &
  WorkbenchPwdtElectrostaticHeatThermoTriangleParams;

export type WorkbenchPwdtBrowserBridge = {
  version: "kyuubiki.pwdt.browser-bridge/v1";
  invoke: (action: string, payload?: Record<string, unknown>) => Promise<Record<string, unknown>>;
  runMacro: (macroId: string, payload?: Record<string, unknown>) => Promise<Record<string, unknown>>;
  runSteps: (steps: WorkbenchScriptMacroStep[]) => Promise<Record<string, unknown>[]>;
  state: () => unknown;
  stateJson: () => string;
  actions: () => typeof WORKBENCH_SCRIPT_ACTIONS;
  macros: () => typeof WORKBENCH_SCRIPT_MACROS;
  recipes: () => typeof WORKBENCH_SCRIPT_RECIPES;
  hasAction: (actionId: string) => boolean;
  hasMacro: (macroId: string) => boolean;
  hasRecipe: (recipeId: string) => boolean;
  actionsMatching: (filters?: { category?: string; risk?: string }) => typeof WORKBENCH_SCRIPT_ACTIONS;
  recipesMatching: (filters?: WorkbenchScriptRecipeFilters) => typeof WORKBENCH_SCRIPT_RECIPES;
  uiContract: () => ReturnType<typeof buildWorkbenchUiAutomationContractSnapshot>;
  uiSelector: (key: string, value?: string | number | boolean) => string;
  querySelector: (key: string, value?: string | number | boolean) => Element | null;
  querySelectorAll: (key: string, value?: string | number | boolean) => Element[];
  selectorExists: (key: string, value?: string | number | boolean) => boolean;
  parityReport: () => Record<string, unknown>;
  waitUntil: (
    predicate: (snapshot: Partial<WorkbenchScriptSnapshot>) => boolean,
    options?: WorkbenchPwdtWaitOptions,
  ) => Promise<Partial<WorkbenchScriptSnapshot>>;
  waitForState: (
    expected: Partial<WorkbenchScriptSnapshot>,
    options?: WorkbenchPwdtWaitOptions,
  ) => Promise<Partial<WorkbenchScriptSnapshot>>;
  waitForMessage: (text: string, options?: WorkbenchPwdtWaitOptions) => Promise<Partial<WorkbenchScriptSnapshot>>;
  waitForJobDone: (options?: WorkbenchPwdtWaitOptions) => Promise<Partial<WorkbenchScriptSnapshot>>;
  openSidebar: (section: string) => Promise<Record<string, unknown>>;
  openTabs: (tabs: Record<string, unknown>) => Promise<Record<string, unknown>>;
  configure: (settings: Record<string, unknown>) => Promise<Record<string, unknown>>;
  refreshAll: () => Promise<Record<string, unknown>>;
  stageStoreEntry: (kind: string, entryId: string) => Promise<Record<string, unknown>>;
  removeStoreEntry: (kind: string, entryId: string) => Promise<Record<string, unknown>>;
  exportStoreManifest: () => Promise<Record<string, unknown>>;
  ensureProject: (name?: string, description?: string) => Promise<string | null>;
  buildParametricTruss2d: (params?: WorkbenchPwdtTrussStudyParams) => Promise<Partial<WorkbenchScriptSnapshot>>;
  prepareElectrostaticPlaneTriangleStudy: (params?: WorkbenchPwdtElectrostaticHeatThermoTriangleParams) => Promise<Partial<WorkbenchScriptSnapshot>>;
  prepareElectrostaticPlaneQuadStudy: (params?: WorkbenchPwdtElectrostaticHeatThermoQuadParams) => Promise<Partial<WorkbenchScriptSnapshot>>;
  prepareHeatPlaneTriangleStudy: (params?: WorkbenchPwdtHeatThermoTriangleParams) => Promise<Partial<WorkbenchScriptSnapshot>>;
  prepareHeatPlaneQuadStudy: (params?: WorkbenchPwdtHeatThermoQuadParams) => Promise<Partial<WorkbenchScriptSnapshot>>;
  saveModel: (params?: {
    name?: string;
    material?: string | number;
    saveAs?: boolean;
  }) => Promise<Record<string, unknown>>;
  runCurrentStudy: (options?: WorkbenchPwdtWaitOptions) => Promise<Partial<WorkbenchScriptSnapshot>>;
  openResults: (filters?: { projectId?: string | null; modelVersionId?: string | null }) => Promise<Partial<WorkbenchScriptSnapshot>>;
  projectElectrostaticToHeatTriangleStudy: () => Promise<Record<string, unknown>>;
  projectElectrostaticToHeatQuadStudy: () => Promise<Record<string, unknown>>;
  projectHeatToThermoTriangleStudy: () => Promise<Record<string, unknown>>;
  projectHeatToThermoQuadStudy: () => Promise<Record<string, unknown>>;
  runRecipe: (recipeId: string, params?: WorkbenchPwdtRecipeParams) => Promise<Record<string, unknown>>;
  runClosedLoopTrussStudy: (params?: WorkbenchPwdtTrussStudyParams) => Promise<Record<string, unknown>>;
  runHeatToThermoTriangleStudy: (params?: WorkbenchPwdtHeatThermoTriangleParams) => Promise<Record<string, unknown>>;
  runHeatToThermoQuadStudy: (params?: WorkbenchPwdtHeatThermoQuadParams) => Promise<Record<string, unknown>>;
  runElectrostaticHeatThermoTriangleStudy: (params?: WorkbenchPwdtElectrostaticHeatThermoTriangleParams) => Promise<Record<string, unknown>>;
  runElectrostaticHeatThermoQuadStudy: (params?: WorkbenchPwdtElectrostaticHeatThermoQuadParams) => Promise<Record<string, unknown>>;
  sleep: (seconds?: number) => Promise<void>;
};

export type WorkbenchPwdtBrowserBridgeInput = {
  appendOutput?: (line: string) => void;
  getSnapshot: () => unknown;
  invokeAction: InvokeAction;
};

declare global {
  interface Window {
    __kyuubikiPwdt?: WorkbenchPwdtBrowserBridge;
  }
}

function asRecordResult(action: string, result: unknown): Record<string, unknown> {
  if (result && typeof result === "object" && !Array.isArray(result)) {
    return result as Record<string, unknown>;
  }
  return { ok: true, action, result: result ?? null };
}

function resolveUiSelector(key: string, value?: string | number | boolean) {
  const contract = buildWorkbenchUiAutomationContractSnapshot();
  const direct = contract.selectors[key];
  if (direct) return direct;
  const parameterized = contract.parameterizedSelectors.find((selector) => selector.key === key);
  if (!parameterized) throw new Error(`Unknown Workbench UI selector key: ${key}`);
  if (value === undefined || value === null) {
    throw new Error(`Workbench UI selector "${key}" requires ${parameterized.parameter}.`);
  }
  return parameterized.template.replace(`\${${parameterized.parameter}}`, String(value));
}

function safeQuerySelector(selector: string) {
  if (typeof document === "undefined") return null;
  try {
    return document.querySelector(selector);
  } catch {
    return null;
  }
}

function safeQuerySelectorAll(selector: string) {
  if (typeof document === "undefined") return [];
  try {
    return Array.from(document.querySelectorAll(selector));
  } catch {
    return [];
  }
}

function snapshotRecord(getSnapshot: () => unknown): Partial<WorkbenchScriptSnapshot> {
  const snapshot = getSnapshot();
  if (snapshot && typeof snapshot === "object" && !Array.isArray(snapshot)) {
    return snapshot as Partial<WorkbenchScriptSnapshot>;
  }
  return {};
}

function delay(ms: number) {
  return new Promise<void>((resolve) => {
    globalThis.setTimeout(resolve, Math.max(0, ms));
  });
}

function expectStateMatches(snapshot: Partial<WorkbenchScriptSnapshot>, expected: Partial<WorkbenchScriptSnapshot>) {
  return Object.entries(expected).every(([key, value]) => snapshot[key as keyof WorkbenchScriptSnapshot] === value);
}

function terminalJobStatus(status: unknown) {
  return status === "completed" || status === "failed" || status === "cancelled";
}

function recipeTimeoutOptions(params: { timeoutMs?: number; timeoutSeconds?: number }): WorkbenchPwdtWaitOptions {
  return {
    timeoutMs: params.timeoutMs ?? (params.timeoutSeconds === undefined ? undefined : params.timeoutSeconds * 1000),
  };
}

export function createWorkbenchPwdtBrowserBridge({
  appendOutput,
  getSnapshot,
  invokeAction,
}: WorkbenchPwdtBrowserBridgeInput): WorkbenchPwdtBrowserBridge {
  const bridge: WorkbenchPwdtBrowserBridge = {
    version: "kyuubiki.pwdt.browser-bridge/v1",
    async invoke(action, payload = {}) {
      if (!getWorkbenchScriptActionDefinition(action)) {
        throw new Error(`Unknown Workbench frontend action: ${action}`);
      }
      const result = await invokeAction(action, payload, "script", "Pwdt browser bridge");
      return asRecordResult(action, result);
    },
    async runMacro(macroId, payload = {}) {
      if (!getWorkbenchScriptMacroDefinition(macroId)) {
        throw new Error(`Unknown Workbench frontend macro: ${macroId}`);
      }
      return bridge.invoke("macro/run", { macroId, ...payload });
    },
    async runSteps(steps) {
      const results: Record<string, unknown>[] = [];
      for (const step of steps) {
        results.push(await bridge.invoke(step.action, step.payload ?? {}));
      }
      return results;
    },
    state: getSnapshot,
    stateJson: () => JSON.stringify(getSnapshot()),
    actions: () => WORKBENCH_SCRIPT_ACTIONS,
    macros: () => WORKBENCH_SCRIPT_MACROS,
    recipes: () => WORKBENCH_SCRIPT_RECIPES,
    hasAction: (actionId) => Boolean(getWorkbenchScriptActionDefinition(actionId)),
    hasMacro: (macroId) => Boolean(getWorkbenchScriptMacroDefinition(macroId)),
    hasRecipe: (recipeId) => Boolean(getWorkbenchScriptRecipeDefinition(recipeId)),
    actionsMatching: (filters = {}) =>
      WORKBENCH_SCRIPT_ACTIONS.filter((action) => {
        if (filters.category && action.category !== filters.category) return false;
        if (filters.risk && action.risk !== filters.risk) return false;
        return true;
      }),
    recipesMatching: filterWorkbenchScriptRecipes,
    uiContract: buildWorkbenchUiAutomationContractSnapshot,
    uiSelector: resolveUiSelector,
    querySelector: (key, value) => safeQuerySelector(resolveUiSelector(key, value)),
    querySelectorAll: (key, value) => safeQuerySelectorAll(resolveUiSelector(key, value)),
    selectorExists: (key, value) => Boolean(bridge.querySelector(key, value)),
    parityReport: () => {
      const snapshot = snapshotRecord(getSnapshot);
      const contract = buildWorkbenchUiAutomationContractSnapshot();
      return {
        version: bridge.version,
        action_count: WORKBENCH_SCRIPT_ACTIONS.length,
        macro_count: WORKBENCH_SCRIPT_MACROS.length,
        recipe_count: WORKBENCH_SCRIPT_RECIPES.length,
        normal_recipe_count: bridge.recipesMatching({ risk: "normal" }).length,
        selector_count: Object.keys(contract.selectors).length,
        parameterized_selector_count: contract.parameterizedSelectors.length,
        current_sidebar: snapshot.sidebarSection ?? null,
        current_study: snapshot.studyKind ?? null,
        product_owned_static_ui: contract.shellExtensible === false,
      };
    },
    async waitUntil(predicate, options = {}) {
      const timeoutMs = options.timeoutMs ?? 30_000;
      const intervalMs = options.intervalMs ?? 250;
      const startedAt = Date.now();
      let current = snapshotRecord(getSnapshot);
      while (Date.now() - startedAt <= timeoutMs) {
        current = snapshotRecord(getSnapshot);
        if (predicate(current)) return current;
        await delay(intervalMs);
      }
      throw new Error(`Pwdt wait timed out after ${timeoutMs}ms.`);
    },
    waitForState: (expected, options) =>
      bridge.waitUntil((current) => expectStateMatches(current, expected), options),
    waitForMessage: (text, options) =>
      bridge.waitUntil((current) => String(current.message ?? "").includes(text), options),
    waitForJobDone: (options) =>
      bridge.waitUntil((current) => terminalJobStatus(current.jobStatus), {
        timeoutMs: options?.timeoutMs ?? 90_000,
        intervalMs: options?.intervalMs ?? 500,
      }),
    openSidebar: (section) => bridge.invoke("nav/setSidebarSection", { section }),
    openTabs: (tabs) => bridge.invoke("nav/setTabs", tabs),
    configure: (settings) => bridge.invoke("settings/patch", settings),
    refreshAll: () => bridge.invoke("runtime/refreshAll"),
    stageStoreEntry: (kind, entryId) => bridge.invoke("store/stageEntry", { kind, entryId }),
    removeStoreEntry: (kind, entryId) => bridge.invoke("store/removeEntry", { kind, entryId }),
    exportStoreManifest: () => bridge.invoke("store/exportManifest"),
    async ensureProject(name = "Pwdt automation study", description = "Created from Pwdt") {
      const selectedProjectId = snapshotRecord(getSnapshot).selectedProjectId;
      if (selectedProjectId) return selectedProjectId;
      const result = await bridge.invoke("project/create", { name, description });
      return typeof result.projectId === "string" ? result.projectId : null;
    },
    async buildParametricTruss2d(params = {}) {
      await bridge.invoke("nav/setStudyKind", { studyKind: "truss_2d" });
      await bridge.openSidebar("model");
      await bridge.openTabs({ modelTab: "tools", modelToolsPage: "generate" });
      if (params.modelName !== undefined || params.activeMaterial !== undefined) {
        await bridge.invoke("model/setWorkspaceMeta", {
          ...(params.modelName !== undefined ? { loadedModelName: params.modelName } : {}),
          ...(params.activeMaterial !== undefined ? { activeMaterial: String(params.activeMaterial) } : {}),
        });
      }
      await bridge.invoke("state/setParametric", {
        bays: params.bays ?? 6,
        span: params.span ?? 18,
        height: params.height ?? 3.5,
        loadY: params.loadY ?? -1500,
      });
      await bridge.invoke("model/generateTruss");
      return snapshotRecord(getSnapshot);
    },
    async prepareElectrostaticPlaneTriangleStudy(params = {}) {
      await bridge.invoke("nav/setStudyKind", { studyKind: "electrostatic_plane_triangle_2d" });
      await bridge.openSidebar("model");
      await bridge.openTabs({ modelTab: "tools", modelToolsPage: "study" });
      if (params.electrostaticModelName !== undefined || params.activeMaterial !== undefined) {
        await bridge.invoke("model/setWorkspaceMeta", {
          ...(params.electrostaticModelName !== undefined ? { loadedModelName: params.electrostaticModelName } : {}),
          ...(params.activeMaterial !== undefined ? { activeMaterial: String(params.activeMaterial) } : {}),
        });
      }
      return snapshotRecord(getSnapshot);
    },
    async prepareElectrostaticPlaneQuadStudy(params = {}) {
      await bridge.invoke("nav/setStudyKind", { studyKind: "electrostatic_plane_quad_2d" });
      await bridge.openSidebar("model");
      await bridge.openTabs({ modelTab: "tools", modelToolsPage: "study" });
      if (params.electrostaticModelName !== undefined || params.activeMaterial !== undefined) {
        await bridge.invoke("model/setWorkspaceMeta", {
          ...(params.electrostaticModelName !== undefined ? { loadedModelName: params.electrostaticModelName } : {}),
          ...(params.activeMaterial !== undefined ? { activeMaterial: String(params.activeMaterial) } : {}),
        });
      }
      return snapshotRecord(getSnapshot);
    },
    async prepareHeatPlaneTriangleStudy(params = {}) {
      await bridge.invoke("nav/setStudyKind", { studyKind: "heat_plane_triangle_2d" });
      await bridge.openSidebar("model");
      await bridge.openTabs({ modelTab: "tools", modelToolsPage: "study" });
      if (params.heatModelName !== undefined || params.activeMaterial !== undefined) {
        await bridge.invoke("model/setWorkspaceMeta", {
          ...(params.heatModelName !== undefined ? { loadedModelName: params.heatModelName } : {}),
          ...(params.activeMaterial !== undefined ? { activeMaterial: String(params.activeMaterial) } : {}),
        });
      }
      return snapshotRecord(getSnapshot);
    },
    async prepareHeatPlaneQuadStudy(params = {}) {
      await bridge.invoke("nav/setStudyKind", { studyKind: "heat_plane_quad_2d" });
      await bridge.openSidebar("model");
      await bridge.openTabs({ modelTab: "tools", modelToolsPage: "study" });
      if (params.heatModelName !== undefined || params.activeMaterial !== undefined) {
        await bridge.invoke("model/setWorkspaceMeta", {
          ...(params.heatModelName !== undefined ? { loadedModelName: params.heatModelName } : {}),
          ...(params.activeMaterial !== undefined ? { activeMaterial: String(params.activeMaterial) } : {}),
        });
      }
      return snapshotRecord(getSnapshot);
    },
    async saveModel(params = {}) {
      if (params.name !== undefined || params.material !== undefined) {
        await bridge.invoke("model/setWorkspaceMeta", {
          ...(params.name !== undefined ? { loadedModelName: params.name } : {}),
          ...(params.material !== undefined ? { activeMaterial: String(params.material) } : {}),
        });
      }
      return bridge.invoke(params.saveAs ? "model/saveAs" : "model/save");
    },
    async runCurrentStudy(options) {
      await bridge.invoke("job/run");
      return bridge.waitForJobDone(options);
    },
    async openResults(filters = {}) {
      await bridge.invoke("data/setFilters", {
        activeTab: "results",
        ...(filters.projectId !== undefined ? { projectId: filters.projectId } : {}),
        ...(filters.modelVersionId !== undefined ? { modelVersionId: filters.modelVersionId } : {}),
      });
      return snapshotRecord(getSnapshot);
    },
    projectElectrostaticToHeatTriangleStudy: () => bridge.invoke("state/projectElectrostaticToHeat"),
    projectElectrostaticToHeatQuadStudy: () => bridge.invoke("state/projectElectrostaticToHeat"),
    projectHeatToThermoTriangleStudy: () => bridge.invoke("state/projectHeatToThermo"),
    projectHeatToThermoQuadStudy: () => bridge.invoke("state/projectHeatToThermo"),
    async runRecipe(recipeId, params = {}) {
      if (!getWorkbenchScriptRecipeDefinition(recipeId)) {
        throw new Error(`Unknown Pwdt recipe: ${recipeId}`);
      }
      if (recipeId === CLOSED_LOOP_TRUSS_RECIPE_ID) {
        return bridge.runClosedLoopTrussStudy(params);
      }
      if (recipeId === HEAT_TO_THERMO_QUAD_RECIPE_ID) {
        return bridge.runHeatToThermoQuadStudy(params);
      }
      if (recipeId === HEAT_TO_THERMO_TRIANGLE_RECIPE_ID) {
        return bridge.runHeatToThermoTriangleStudy(params);
      }
      if (recipeId === ELECTROSTATIC_HEAT_THERMO_QUAD_RECIPE_ID) {
        return bridge.runElectrostaticHeatThermoQuadStudy(params);
      }
      if (recipeId === ELECTROSTATIC_HEAT_THERMO_TRIANGLE_RECIPE_ID) {
        return bridge.runElectrostaticHeatThermoTriangleStudy(params);
      }
      throw new Error(`Pwdt recipe is registered but not executable yet: ${recipeId}`);
    },
    async runClosedLoopTrussStudy(params = {}) {
      const projectId = await bridge.ensureProject(params.projectName, params.projectDescription);
      await bridge.buildParametricTruss2d(params);
      const saveResult = await bridge.saveModel({
        name: params.modelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const runState = await bridge.runCurrentStudy({
        ...recipeTimeoutOptions(params),
      });
      await bridge.openResults({ projectId });
      return {
        ok: runState.jobStatus === "completed",
        projectId,
        saveResult,
        jobStatus: runState.jobStatus ?? null,
        resultCount: runState.resultCount ?? null,
      };
    },
    async runHeatToThermoTriangleStudy(params = {}) {
      const projectId = await bridge.ensureProject(params.projectName, params.projectDescription);
      await bridge.prepareHeatPlaneTriangleStudy(params);
      const heatSaveResult = await bridge.saveModel({
        name: params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const heatRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      const thermoProjection = await bridge.projectHeatToThermoTriangleStudy();
      const thermoSaveResult = await bridge.saveModel({
        name: params.thermoModelName ?? params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const thermoRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      await bridge.openResults({ projectId });
      return {
        ok:
          heatRunState.jobStatus === "completed" &&
          thermoProjection.studyKind === "thermal_plane_triangle_2d" &&
          thermoRunState.jobStatus === "completed",
        projectId,
        heatSaveResult,
        heatJobStatus: heatRunState.jobStatus ?? null,
        thermoProjection,
        thermoSaveResult,
        thermoJobStatus: thermoRunState.jobStatus ?? null,
        resultCount: thermoRunState.resultCount ?? null,
      };
    },
    async runHeatToThermoQuadStudy(params = {}) {
      const projectId = await bridge.ensureProject(params.projectName, params.projectDescription);
      await bridge.prepareHeatPlaneQuadStudy(params);
      const heatSaveResult = await bridge.saveModel({
        name: params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const heatRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      const thermoProjection = await bridge.projectHeatToThermoQuadStudy();
      const thermoSaveResult = await bridge.saveModel({
        name: params.thermoModelName ?? params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const thermoRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      await bridge.openResults({ projectId });
      return {
        ok:
          heatRunState.jobStatus === "completed" &&
          thermoProjection.studyKind === "thermal_plane_quad_2d" &&
          thermoRunState.jobStatus === "completed",
        projectId,
        heatSaveResult,
        heatJobStatus: heatRunState.jobStatus ?? null,
        thermoProjection,
        thermoSaveResult,
        thermoJobStatus: thermoRunState.jobStatus ?? null,
        resultCount: thermoRunState.resultCount ?? null,
      };
    },
    async runElectrostaticHeatThermoTriangleStudy(params = {}) {
      const projectId = await bridge.ensureProject(params.projectName, params.projectDescription);
      await bridge.prepareElectrostaticPlaneTriangleStudy(params);
      const electrostaticSaveResult = await bridge.saveModel({
        name: params.electrostaticModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const electrostaticRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      const heatProjection = await bridge.projectElectrostaticToHeatTriangleStudy();
      const heatSaveResult = await bridge.saveModel({
        name: params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const heatRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      const thermoProjection = await bridge.projectHeatToThermoTriangleStudy();
      const thermoSaveResult = await bridge.saveModel({
        name: params.thermoModelName ?? params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const thermoRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      await bridge.openResults({ projectId });
      return {
        ok:
          electrostaticRunState.jobStatus === "completed" &&
          heatProjection.studyKind === "heat_plane_triangle_2d" &&
          heatRunState.jobStatus === "completed" &&
          thermoProjection.studyKind === "thermal_plane_triangle_2d" &&
          thermoRunState.jobStatus === "completed",
        projectId,
        electrostaticSaveResult,
        electrostaticJobStatus: electrostaticRunState.jobStatus ?? null,
        heatProjection,
        heatSaveResult,
        heatJobStatus: heatRunState.jobStatus ?? null,
        thermoProjection,
        thermoSaveResult,
        thermoJobStatus: thermoRunState.jobStatus ?? null,
        resultCount: thermoRunState.resultCount ?? null,
      };
    },
    async runElectrostaticHeatThermoQuadStudy(params = {}) {
      const projectId = await bridge.ensureProject(params.projectName, params.projectDescription);
      await bridge.prepareElectrostaticPlaneQuadStudy(params);
      const electrostaticSaveResult = await bridge.saveModel({
        name: params.electrostaticModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const electrostaticRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      const heatProjection = await bridge.projectElectrostaticToHeatQuadStudy();
      const heatSaveResult = await bridge.saveModel({
        name: params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const heatRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      const thermoProjection = await bridge.projectHeatToThermoQuadStudy();
      const thermoSaveResult = await bridge.saveModel({
        name: params.thermoModelName ?? params.heatModelName,
        material: params.activeMaterial,
        saveAs: true,
      });
      const thermoRunState = await bridge.runCurrentStudy(recipeTimeoutOptions(params));
      await bridge.openResults({ projectId });
      return {
        ok:
          electrostaticRunState.jobStatus === "completed" &&
          heatProjection.studyKind === "heat_plane_quad_2d" &&
          heatRunState.jobStatus === "completed" &&
          thermoProjection.studyKind === "thermal_plane_quad_2d" &&
          thermoRunState.jobStatus === "completed",
        projectId,
        electrostaticSaveResult,
        electrostaticJobStatus: electrostaticRunState.jobStatus ?? null,
        heatProjection,
        heatSaveResult,
        heatJobStatus: heatRunState.jobStatus ?? null,
        thermoProjection,
        thermoSaveResult,
        thermoJobStatus: thermoRunState.jobStatus ?? null,
        resultCount: thermoRunState.resultCount ?? null,
      };
    },
    sleep: (seconds = 0) => delay(seconds * 1000),
  };
  appendOutput?.(`[pwdt] bridge installed ${bridge.version}`);
  return bridge;
}

export function buildWorkbenchPyodideBridge(input: WorkbenchPwdtBrowserBridgeInput) {
  const pwdt = createWorkbenchPwdtBrowserBridge(input);
  return {
    invoke: async (action: string, payloadJson?: string) => {
      const payload =
        payloadJson && payloadJson.trim().length > 0
          ? (JSON.parse(payloadJson) as Record<string, unknown>)
          : {};
      return JSON.stringify(await pwdt.invoke(action, payload));
    },
    state_json: pwdt.stateJson,
    actions_json: () => JSON.stringify(pwdt.actions()),
    macros_json: () => JSON.stringify(pwdt.macros()),
    recipes_json: () => JSON.stringify(pwdt.recipes()),
    ui_contract_json: () => JSON.stringify(pwdt.uiContract()),
    log: (message: string) => input.appendOutput?.(message),
    sleep: pwdt.sleep,
  };
}

export function installWorkbenchPwdtBrowserBridge(input: WorkbenchPwdtBrowserBridgeInput) {
  if (typeof window === "undefined") return () => {};
  const pwdt = createWorkbenchPwdtBrowserBridge(input);
  const pyodideBridge = {
    invoke: async (action: string, payloadJson?: string) => {
      const payload =
        payloadJson && payloadJson.trim().length > 0
          ? (JSON.parse(payloadJson) as Record<string, unknown>)
          : {};
      return JSON.stringify(await pwdt.invoke(action, payload));
    },
    state_json: pwdt.stateJson,
    actions_json: () => JSON.stringify(pwdt.actions()),
    macros_json: () => JSON.stringify(pwdt.macros()),
    recipes_json: () => JSON.stringify(pwdt.recipes()),
    ui_contract_json: () => JSON.stringify(pwdt.uiContract()),
    log: (message: string) => input.appendOutput?.(message),
    sleep: pwdt.sleep,
  };
  window.__kyuubikiPwdt = pwdt;
  window.__kyuubikiBridge = pyodideBridge;
  return () => {
    if (window.__kyuubikiPwdt === pwdt) delete window.__kyuubikiPwdt;
    if (window.__kyuubikiBridge === pyodideBridge) delete window.__kyuubikiBridge;
  };
}
