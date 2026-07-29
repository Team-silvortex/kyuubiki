"use client";

import type { WorkbenchRecordedMacroDraft, WorkbenchScriptActionLogEntry } from "./workbench-script-runtime-types.ts";
import {
  getWorkbenchScriptActionDefinition,
  getWorkbenchScriptMacroDefinition,
} from "./workbench-script-runtime-catalog.ts";
import { getWorkbenchScriptRecipeDefinition } from "./workbench-script-runtime-recipes.ts";
import {
  isVariableReference,
  VARIABLE_IDENTIFIER_RE,
  type WorkbenchFrontendDslVarReference,
} from "./workbench-script-dsl-expressions.ts";
import {
  buildClosedLoopTrussWorkbenchFrontendDslDocument,
  buildDefaultWorkbenchFrontendDslDocument,
  buildHeatToThermoQuadWorkbenchFrontendDslDocument,
} from "./workbench-script-dsl-templates.ts";

export type WorkbenchFrontendDslStep =
  | { kind: "invoke"; action: string; payload?: Record<string, unknown> }
  | { kind: "macro"; macroId: string; payload?: Record<string, unknown> }
  | { kind: "run_recipe"; recipeId: string; payload?: Record<string, unknown>; assign?: string; message?: string }
  | { kind: "expect_action"; action: string; message?: string }
  | { kind: "expect_macro"; macroId: string; message?: string }
  | { kind: "expect_recipe"; recipeId: string; message?: string }
  | { kind: "capture_action_catalog"; assign: string; category?: string; risk?: string; message?: string }
  | { kind: "capture_recipe_catalog"; assign: string; category?: string; risk?: string; message?: string }
  | { kind: "emit_parity_report"; assign?: string; message?: string }
  | { kind: "log"; message: string }
  | { kind: "sleep"; seconds: number }
  | { kind: "capture_now"; assign: string; message?: string }
  | { kind: "capture_state"; key: string; assign: string; message?: string }
  | { kind: "assert_selector"; selector: string; value?: string | WorkbenchFrontendDslVarReference; message?: string }
  | { kind: "capture_selector_count"; selector: string; assign: string; value?: string | WorkbenchFrontendDslVarReference; message?: string }
  | { kind: "capture_selector_text"; selector: string; assign: string; value?: string | WorkbenchFrontendDslVarReference; message?: string }
  | { kind: "expect_selector_text"; selector: string; value?: string | WorkbenchFrontendDslVarReference; equals?: string | WorkbenchFrontendDslVarReference; includes?: string | WorkbenchFrontendDslVarReference; message?: string }
  | { kind: "expect_selector_count"; selector: string; value?: string | WorkbenchFrontendDslVarReference; equals?: number | WorkbenchFrontendDslVarReference; minimum?: number | WorkbenchFrontendDslVarReference; message?: string }
  | { kind: "expect_selector_exists_all"; selectors: Array<{ selector: string; value?: string | WorkbenchFrontendDslVarReference }>; message?: string }
  | { kind: "expect_state"; key: string; equals?: string | number | boolean | null; includes?: string; message?: string }
  | { kind: "branch_equals"; key: string; equals: string | number | boolean | null; then: WorkbenchFrontendDslStep[]; else?: WorkbenchFrontendDslStep[] }
  | { kind: "foreach_state_list"; key: string; item: string; steps: WorkbenchFrontendDslStep[]; else?: WorkbenchFrontendDslStep[] }
  | { kind: "wait_for_message"; text: string; timeout?: number; interval?: number }
  | { kind: "wait_for_job_done"; timeout?: number; interval?: number };

export type WorkbenchFrontendDslDocument = {
  dsl_version: "kyuubiki.frontend-dsl/v1";
  name: string;
  steps: WorkbenchFrontendDslStep[];
};

const DSL_VERSION = "kyuubiki.frontend-dsl/v1";
export const WORKBENCH_FRONTEND_DSL_REPORT_PREFIX = "[layout-report]";

export const DEFAULT_WORKBENCH_FRONTEND_DSL = serializeWorkbenchFrontendDslDocument(
  buildDefaultWorkbenchFrontendDslDocument(),
);
export const CLOSED_LOOP_TRUSS_WORKBENCH_FRONTEND_DSL = serializeWorkbenchFrontendDslDocument(
  buildClosedLoopTrussWorkbenchFrontendDslDocument(),
);
export const HEAT_TO_THERMO_QUAD_WORKBENCH_FRONTEND_DSL = serializeWorkbenchFrontendDslDocument(
  buildHeatToThermoQuadWorkbenchFrontendDslDocument(),
);

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function parsePayload(value: unknown) {
  if (value === undefined) return undefined;
  if (!isPlainObject(value)) {
    throw new Error("DSL step payload must be an object.");
  }
  return value;
}

function parseAssign(value: unknown, context: string) {
  if (typeof value !== "string" || !VARIABLE_IDENTIFIER_RE.test(value)) {
    throw new Error(`${context} requires a valid assign identifier.`);
  }
  return value;
}

function parseOptionalMessage(value: unknown) {
  return typeof value === "string" && value.trim() ? { message: value } : {};
}

function parseKnownAction(value: unknown, context: string) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${context} requires a non-empty action.`);
  }
  if (!getWorkbenchScriptActionDefinition(value)) {
    throw new Error(`${context} references unknown Workbench action "${value}".`);
  }
  return value;
}

function parseKnownMacro(value: unknown, context: string) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${context} requires a non-empty macroId.`);
  }
  if (!getWorkbenchScriptMacroDefinition(value)) {
    throw new Error(`${context} references unknown Workbench macro "${value}".`);
  }
  return value;
}

function parseKnownRecipe(value: unknown, context: string) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${context} requires a non-empty recipeId.`);
  }
  if (!getWorkbenchScriptRecipeDefinition(value)) {
    throw new Error(`${context} references unknown Pwdt recipe "${value}".`);
  }
  return value;
}

function parseStep(value: unknown): WorkbenchFrontendDslStep {
  if (!isPlainObject(value)) {
    throw new Error("DSL step must be an object.");
  }

  const kind = typeof value.kind === "string" ? value.kind : "";

  if (kind === "invoke") {
    const payload = parsePayload(value.payload);
    return {
      kind,
      action: parseKnownAction(value.action, "DSL invoke step"),
      ...(payload ? { payload } : {}),
    };
  }

  if (kind === "macro") {
    const payload = parsePayload(value.payload);
    return {
      kind,
      macroId: parseKnownMacro(value.macroId, "DSL macro step"),
      ...(payload ? { payload } : {}),
    };
  }

  if (kind === "run_recipe") {
    const payload = parsePayload(value.payload);
    return {
      kind,
      recipeId: parseKnownRecipe(value.recipeId, "DSL run_recipe step"),
      ...(payload ? { payload } : {}),
      ...(value.assign === undefined ? {} : { assign: parseAssign(value.assign, "DSL run_recipe step") }),
      ...parseOptionalMessage(value.message),
    };
  }

  if (kind === "expect_action") {
    return {
      kind,
      action: parseKnownAction(value.action, "DSL expect_action step"),
      ...parseOptionalMessage(value.message),
    };
  }

  if (kind === "expect_macro") {
    return {
      kind,
      macroId: parseKnownMacro(value.macroId, "DSL expect_macro step"),
      ...parseOptionalMessage(value.message),
    };
  }

  if (kind === "expect_recipe") {
    return {
      kind,
      recipeId: parseKnownRecipe(value.recipeId, "DSL expect_recipe step"),
      ...parseOptionalMessage(value.message),
    };
  }

  if (kind === "capture_action_catalog") {
    return {
      kind,
      assign: parseAssign(value.assign, "DSL capture_action_catalog step"),
      ...(typeof value.category === "string" && value.category.trim() ? { category: value.category } : {}),
      ...(typeof value.risk === "string" && value.risk.trim() ? { risk: value.risk } : {}),
      ...parseOptionalMessage(value.message),
    };
  }

  if (kind === "capture_recipe_catalog") {
    return {
      kind,
      assign: parseAssign(value.assign, "DSL capture_recipe_catalog step"),
      ...(typeof value.category === "string" && value.category.trim() ? { category: value.category } : {}),
      ...(typeof value.risk === "string" && value.risk.trim() ? { risk: value.risk } : {}),
      ...parseOptionalMessage(value.message),
    };
  }

  if (kind === "emit_parity_report") {
    return {
      kind,
      ...(value.assign === undefined
        ? {}
        : { assign: parseAssign(value.assign, "DSL emit_parity_report step") }),
      ...parseOptionalMessage(value.message),
    };
  }

  if (kind === "log") {
    if (typeof value.message !== "string" || !value.message.trim()) {
      throw new Error("DSL log step requires a message.");
    }
    return { kind, message: value.message };
  }

  if (kind === "sleep") {
    if (typeof value.seconds !== "number" || Number.isNaN(value.seconds) || value.seconds < 0) {
      throw new Error("DSL sleep step requires a non-negative seconds value.");
    }
    return { kind, seconds: value.seconds };
  }

  if (kind === "capture_now") {
    if (typeof value.assign !== "string" || !VARIABLE_IDENTIFIER_RE.test(value.assign)) {
      throw new Error("DSL capture_now step requires a valid assign identifier.");
    }
    return {
      kind,
      assign: value.assign,
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "capture_state") {
    if (typeof value.key !== "string" || !value.key.trim()) {
      throw new Error("DSL capture_state step requires a state key.");
    }
    if (typeof value.assign !== "string" || !/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(value.assign)) {
      throw new Error("DSL capture_state step requires a valid assign identifier.");
    }
    return {
      kind,
      key: value.key,
      assign: value.assign,
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "assert_selector") {
    if (typeof value.selector !== "string" || !value.selector.trim()) {
      throw new Error("DSL assert_selector step requires a selector key.");
    }
    return {
      kind,
      selector: value.selector,
      ...(typeof value.value === "string" || isVariableReference(value.value) ? { value: value.value } : {}),
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "capture_selector_count") {
    if (typeof value.selector !== "string" || !value.selector.trim()) {
      throw new Error("DSL capture_selector_count step requires a selector key.");
    }
    if (typeof value.assign !== "string" || !VARIABLE_IDENTIFIER_RE.test(value.assign)) {
      throw new Error("DSL capture_selector_count step requires a valid assign identifier.");
    }
    return {
      kind,
      selector: value.selector,
      assign: value.assign,
      ...(typeof value.value === "string" || isVariableReference(value.value) ? { value: value.value } : {}),
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "capture_selector_text") {
    if (typeof value.selector !== "string" || !value.selector.trim()) {
      throw new Error("DSL capture_selector_text step requires a selector key.");
    }
    if (typeof value.assign !== "string" || !VARIABLE_IDENTIFIER_RE.test(value.assign)) {
      throw new Error("DSL capture_selector_text step requires a valid assign identifier.");
    }
    return {
      kind,
      selector: value.selector,
      assign: value.assign,
      ...(typeof value.value === "string" || isVariableReference(value.value) ? { value: value.value } : {}),
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "expect_selector_exists_all") {
    if (!Array.isArray(value.selectors) || value.selectors.length === 0) {
      throw new Error("DSL expect_selector_exists_all step requires at least one selector.");
    }
    const selectors = value.selectors.map((entry) => {
      if (!isPlainObject(entry) || typeof entry.selector !== "string" || !entry.selector.trim()) {
        throw new Error("DSL expect_selector_exists_all selector entries require a selector key.");
      }
      return {
        selector: entry.selector,
        ...(typeof entry.value === "string" || isVariableReference(entry.value) ? { value: entry.value } : {}),
      };
    });
    return {
      kind,
      selectors,
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "expect_selector_text") {
    if (typeof value.selector !== "string" || !value.selector.trim()) {
      throw new Error("DSL expect_selector_text step requires a selector key.");
    }
    const hasEquals = typeof value.equals === "string" || isVariableReference(value.equals);
    const hasIncludes = typeof value.includes === "string" || isVariableReference(value.includes);
    if (!hasEquals && !hasIncludes) {
      throw new Error("DSL expect_selector_text step requires either equals or includes.");
    }
    return {
      kind,
      selector: value.selector,
      ...(typeof value.value === "string" || isVariableReference(value.value) ? { value: value.value } : {}),
      ...(hasEquals ? { equals: value.equals as string | WorkbenchFrontendDslVarReference } : {}),
      ...(hasIncludes ? { includes: value.includes as string | WorkbenchFrontendDslVarReference } : {}),
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "expect_selector_count") {
    if (typeof value.selector !== "string" || !value.selector.trim()) {
      throw new Error("DSL expect_selector_count step requires a selector key.");
    }
    const hasEquals = typeof value.equals === "number" || isVariableReference(value.equals);
    const hasMinimum = typeof value.minimum === "number" || isVariableReference(value.minimum);
    if (!hasEquals && !hasMinimum) {
      throw new Error("DSL expect_selector_count step requires either equals or minimum.");
    }
    return {
      kind,
      selector: value.selector,
      ...(typeof value.value === "string" || isVariableReference(value.value) ? { value: value.value } : {}),
      ...(hasEquals ? { equals: value.equals as number | WorkbenchFrontendDslVarReference } : {}),
      ...(hasMinimum ? { minimum: value.minimum as number | WorkbenchFrontendDslVarReference } : {}),
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "expect_state") {
    if (typeof value.key !== "string" || !value.key.trim()) {
      throw new Error("DSL expect_state step requires a state key.");
    }
    const hasEquals = "equals" in value;
    const hasIncludes = typeof value.includes === "string";
    if (!hasEquals && !hasIncludes) {
      throw new Error("DSL expect_state step requires either equals or includes.");
    }
    return {
      kind,
      key: value.key,
      ...(hasEquals ? { equals: value.equals as string | number | boolean | null } : {}),
      ...(hasIncludes ? { includes: value.includes as string } : {}),
      ...(typeof value.message === "string" && value.message.trim() ? { message: value.message } : {}),
    };
  }

  if (kind === "branch_equals") {
    if (typeof value.key !== "string" || !value.key.trim()) {
      throw new Error("DSL branch_equals step requires a state key.");
    }
    if (!("equals" in value)) {
      throw new Error("DSL branch_equals step requires an equals value.");
    }
    if (!Array.isArray(value.then) || value.then.length === 0) {
      throw new Error("DSL branch_equals step requires at least one then step.");
    }
    return {
      kind,
      key: value.key,
      equals: value.equals as string | number | boolean | null,
      then: value.then.map(parseStep),
      ...(Array.isArray(value.else) && value.else.length > 0
        ? { else: value.else.map(parseStep) }
        : {}),
    };
  }

  if (kind === "foreach_state_list") {
    if (typeof value.key !== "string" || !value.key.trim()) {
      throw new Error("DSL foreach_state_list step requires a state key.");
    }
    if (typeof value.item !== "string" || !/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(value.item)) {
      throw new Error("DSL foreach_state_list step requires a valid item identifier.");
    }
    if (!Array.isArray(value.steps) || value.steps.length === 0) {
      throw new Error("DSL foreach_state_list step requires at least one nested step.");
    }
    return {
      kind,
      key: value.key,
      item: value.item,
      steps: value.steps.map(parseStep),
      ...(Array.isArray(value.else) && value.else.length > 0
        ? { else: value.else.map(parseStep) }
        : {}),
    };
  }

  if (kind === "wait_for_message") {
    if (typeof value.text !== "string" || !value.text.trim()) {
      throw new Error("DSL wait_for_message step requires text.");
    }
    return {
      kind,
      text: value.text,
      ...(typeof value.timeout === "number" ? { timeout: value.timeout } : {}),
      ...(typeof value.interval === "number" ? { interval: value.interval } : {}),
    };
  }

  if (kind === "wait_for_job_done") {
    return {
      kind,
      ...(typeof value.timeout === "number" ? { timeout: value.timeout } : {}),
      ...(typeof value.interval === "number" ? { interval: value.interval } : {}),
    };
  }

  throw new Error(`Unsupported DSL step kind: ${String(kind || "unknown")}`);
}

export function parseWorkbenchFrontendDslDocument(source: string): WorkbenchFrontendDslDocument {
  let parsed: unknown;
  try {
    parsed = JSON.parse(source) as unknown;
  } catch {
    throw new Error("Frontend DSL must be valid JSON.");
  }

  if (!isPlainObject(parsed)) {
    throw new Error("Frontend DSL document must be an object.");
  }

  if (parsed.dsl_version !== DSL_VERSION) {
    throw new Error(`Frontend DSL must declare dsl_version "${DSL_VERSION}".`);
  }

  if (typeof parsed.name !== "string" || !parsed.name.trim()) {
    throw new Error("Frontend DSL document requires a non-empty name.");
  }

  if (!Array.isArray(parsed.steps) || parsed.steps.length === 0) {
    throw new Error("Frontend DSL document requires at least one step.");
  }

  return {
    dsl_version: DSL_VERSION,
    name: parsed.name,
    steps: parsed.steps.map(parseStep),
  };
}

export function serializeWorkbenchFrontendDslDocument(document: WorkbenchFrontendDslDocument) {
  return JSON.stringify(document, null, 2);
}

export function buildWorkbenchFrontendDslFromMacroDraft(macro: WorkbenchRecordedMacroDraft): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: macro.id.replace(/^macro\//, ""),
    steps: macro.steps.map((step) => ({
      kind: "invoke",
      action: step.action,
      ...(step.payload ? { payload: step.payload } : {}),
    })),
  };
}

export function buildWorkbenchFrontendDslFromActionLogEntry(entry: WorkbenchScriptActionLogEntry): WorkbenchFrontendDslDocument {
  return {
    dsl_version: DSL_VERSION,
    name: `${entry.action.replaceAll("/", "-")}-replay`,
    steps: [
      {
        kind: "invoke",
        action: entry.action,
        ...(entry.payload ? { payload: entry.payload } : {}),
      },
    ],
  };
}
