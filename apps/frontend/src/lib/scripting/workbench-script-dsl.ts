"use client";

import type { WorkbenchRecordedMacroDraft, WorkbenchScriptActionLogEntry } from "./workbench-script-runtime-types.ts";
import { serializeWorkbenchPythonLiteral } from "./workbench-script-python-format.ts";
import {
  buildPythonExpression,
  isVariableReference,
  VARIABLE_IDENTIFIER_RE,
  type WorkbenchFrontendDslVarReference,
} from "./workbench-script-dsl-expressions.ts";
import { buildDefaultWorkbenchFrontendDslDocument } from "./workbench-script-dsl-templates.ts";

export type WorkbenchFrontendDslStep =
  | { kind: "invoke"; action: string; payload?: Record<string, unknown> }
  | { kind: "macro"; macroId: string; payload?: Record<string, unknown> }
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

function buildDslFailureMessage(code: "selector_mismatch" | "state_mismatch" | "timeout", message: string) {
  return `[dsl-code=${code}] ${message}`;
}

export const DEFAULT_WORKBENCH_FRONTEND_DSL = serializeWorkbenchFrontendDslDocument(
  buildDefaultWorkbenchFrontendDslDocument(),
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

function parseStep(value: unknown): WorkbenchFrontendDslStep {
  if (!isPlainObject(value)) {
    throw new Error("DSL step must be an object.");
  }

  const kind = typeof value.kind === "string" ? value.kind : "";

  if (kind === "invoke") {
    if (typeof value.action !== "string" || !value.action.trim()) {
      throw new Error("DSL invoke step requires a non-empty action.");
    }
    return { kind, action: value.action, ...(parsePayload(value.payload) ? { payload: parsePayload(value.payload) } : {}) };
  }

  if (kind === "macro") {
    if (typeof value.macroId !== "string" || !value.macroId.trim()) {
      throw new Error("DSL macro step requires a non-empty macroId.");
    }
    return { kind, macroId: value.macroId, ...(parsePayload(value.payload) ? { payload: parsePayload(value.payload) } : {}) };
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

function buildSelectorExpression(selector: string, value?: string | WorkbenchFrontendDslVarReference) {
  return value !== undefined
    ? `ky.query_selector(${JSON.stringify(selector)}, ${buildPythonExpression(value)})`
    : `ky.query_selector(${JSON.stringify(selector)})`;
}

function buildStateReadExpression(key: string) {
  return `ky.state().get(${JSON.stringify(key)})`;
}

function compileSteps(steps: WorkbenchFrontendDslStep[], labelPrefix: string): string {
  return steps.map((step, index) => compileStep(step, `${labelPrefix}_${index + 1}`)).join("\n\n");
}

function compileStep(step: WorkbenchFrontendDslStep, stepLabel: string) {

  if (step.kind === "invoke") {
    return `${stepLabel}_payload = ${buildPythonExpression(step.payload ?? {})}
${stepLabel}_result = await ky.invoke(${JSON.stringify(step.action)}, ${stepLabel}_payload)
ky.log("DSL invoke:", ${JSON.stringify(step.action)}, ${stepLabel}_result)`;
  }

  if (step.kind === "macro") {
    return `${stepLabel}_result = await ky.run_macro(${JSON.stringify(step.macroId)}, ${buildPythonExpression(step.payload ?? {})})
ky.log("DSL macro:", ${JSON.stringify(step.macroId)}, ${stepLabel}_result)`;
  }

  if (step.kind === "log") {
    return `ky.log(${buildPythonExpression(step.message)})`;
  }

  if (step.kind === "sleep") {
    return `await ky.sleep(${String(step.seconds)})`;
  }

  if (step.kind === "capture_now") {
    return `from datetime import datetime, timezone
${step.assign} = datetime.now(timezone.utc).isoformat()
ky.log(${buildPythonExpression(step.message?.trim() || "DSL captured current report time.")}, ${step.assign})`;
  }

  if (step.kind === "capture_state") {
    return `${step.assign} = ${buildStateReadExpression(step.key)}
ky.log(${buildPythonExpression(step.message?.trim() || `DSL captured state "${step.key}".`)}, ${step.assign})`;
  }

  if (step.kind === "assert_selector") {
    const message = step.message?.trim() || `Selector "${step.selector}" was not found.`;
    return `${stepLabel}_node = ${buildSelectorExpression(step.selector, step.value)}
if ${stepLabel}_node is None:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", message))})
ky.log("DSL selector ready:", ${JSON.stringify(step.selector)})`;
  }

  if (step.kind === "capture_selector_count") {
    const queryExpression = step.value !== undefined
      ? `ky.query_selector_all(${JSON.stringify(step.selector)}, ${buildPythonExpression(step.value)})`
      : `ky.query_selector_all(${JSON.stringify(step.selector)})`;
    return `${stepLabel}_nodes = ${queryExpression}
${step.assign} = ${stepLabel}_nodes.length
ky.log(${buildPythonExpression(step.message?.trim() || `DSL captured selector count "${step.selector}".`)}, ${step.assign})`;
  }

  if (step.kind === "capture_selector_text") {
    return `${stepLabel}_node = ${buildSelectorExpression(step.selector, step.value)}
if ${stepLabel}_node is None:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", `Selector "${step.selector}" was not found.`))})
${step.assign} = (${stepLabel}_node.textContent or "").strip()
ky.log(${buildPythonExpression(step.message?.trim() || `DSL captured selector text "${step.selector}".`)}, ${step.assign})`;
  }

  if (step.kind === "expect_selector_text") {
    const message =
      step.message?.trim() ||
      (step.includes
        ? `Selector "${step.selector}" text must include the expected value.`
        : `Selector "${step.selector}" text did not match the expected value.`);
    const readText = `${stepLabel}_text = (${stepLabel}_node.textContent or "").strip()`;
    if (step.includes !== undefined) {
      return `${stepLabel}_node = ${buildSelectorExpression(step.selector, step.value)}
if ${stepLabel}_node is None:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", `Selector "${step.selector}" was not found.`))})
${readText}
if str(${buildPythonExpression(step.includes)}) not in ${stepLabel}_text:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", message))})
ky.log("DSL selector text includes:", ${JSON.stringify(step.selector)}, ${stepLabel}_text)`;
    }
    return `${stepLabel}_node = ${buildSelectorExpression(step.selector, step.value)}
if ${stepLabel}_node is None:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", `Selector "${step.selector}" was not found.`))})
${readText}
if ${stepLabel}_text != str(${buildPythonExpression(step.equals ?? "")}):
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", message))})
ky.log("DSL selector text matched:", ${JSON.stringify(step.selector)}, ${stepLabel}_text)`;
  }

  if (step.kind === "expect_selector_count") {
    const queryExpression = step.value !== undefined
      ? `ky.query_selector_all(${JSON.stringify(step.selector)}, ${buildPythonExpression(step.value)})`
      : `ky.query_selector_all(${JSON.stringify(step.selector)})`;
    const message =
      step.message?.trim() ||
      (step.minimum !== undefined
        ? `Selector "${step.selector}" count must satisfy the minimum requirement.`
        : `Selector "${step.selector}" count did not match the expected value.`);
    if (step.minimum !== undefined) {
      return `${stepLabel}_nodes = ${queryExpression}
${stepLabel}_count = ${stepLabel}_nodes.length
if ${stepLabel}_count < int(${buildPythonExpression(step.minimum)}):
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", message))})
ky.log("DSL selector count minimum satisfied:", ${JSON.stringify(step.selector)}, ${stepLabel}_count)`;
    }
    return `${stepLabel}_nodes = ${queryExpression}
${stepLabel}_count = ${stepLabel}_nodes.length
if ${stepLabel}_count != int(${buildPythonExpression(step.equals ?? 0)}):
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", message))})
ky.log("DSL selector count matched:", ${JSON.stringify(step.selector)}, ${stepLabel}_count)`;
  }

  if (step.kind === "expect_selector_exists_all") {
    const lines = step.selectors.flatMap((entry, index) => {
      const nodeLabel = `${stepLabel}_node_${index + 1}`;
      return [
        `${nodeLabel} = ${buildSelectorExpression(entry.selector, entry.value)}`,
        `if ${nodeLabel} is None:`,
        `    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("selector_mismatch", step.message?.trim() || `Required selector "${entry.selector}" was not found.`))})`,
      ];
    });
    return `${lines.join("\n")}
ky.log("DSL selector bundle ready:", ${step.selectors.length})`;
  }

  if (step.kind === "expect_state") {
    const expectedMessage =
      step.message?.trim() ||
      (typeof step.includes === "string"
        ? `State "${step.key}" must include "${step.includes}".`
        : `State "${step.key}" did not match the expected value.`);
    if (typeof step.includes === "string") {
      return `${stepLabel}_state_value = ${buildStateReadExpression(step.key)}
if str(${buildPythonExpression(step.includes)}) not in str(${stepLabel}_state_value):
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("state_mismatch", expectedMessage))})
ky.log("DSL state includes:", ${JSON.stringify(step.key)}, ${stepLabel}_state_value)`;
    }
    return `${stepLabel}_state_value = ${buildStateReadExpression(step.key)}
if ${stepLabel}_state_value != ${buildPythonExpression(step.equals ?? null)}:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("state_mismatch", expectedMessage))})
ky.log("DSL state matched:", ${JSON.stringify(step.key)}, ${stepLabel}_state_value)`;
  }

  if (step.kind === "branch_equals") {
    const thenBlock = compileSteps(step.then, `${stepLabel}_then`)
      .split("\n")
      .map((line) => `    ${line}`)
      .join("\n");
    const elseBlock = step.else && step.else.length > 0
      ? compileSteps(step.else, `${stepLabel}_else`)
          .split("\n")
          .map((line) => `    ${line}`)
          .join("\n")
      : `    ky.log("DSL branch skipped else:", ${JSON.stringify(step.key)})`;
    return `${stepLabel}_state_value = ${buildStateReadExpression(step.key)}
if ${stepLabel}_state_value == ${buildPythonExpression(step.equals)}:
${thenBlock}
else:
${elseBlock}`;
  }

  if (step.kind === "foreach_state_list") {
    const iterVar = `${stepLabel}_items`;
    const loopBody = compileSteps(step.steps, `${stepLabel}_loop`)
      .split("\n")
      .map((line) => `    ${line}`)
      .join("\n");
    const elseBlock = step.else && step.else.length > 0
      ? compileSteps(step.else, `${stepLabel}_empty`)
          .split("\n")
          .map((line) => `    ${line}`)
          .join("\n")
      : `    ky.log("DSL foreach_state_list found no items:", ${JSON.stringify(step.key)})`;
    return `${iterVar} = ${buildStateReadExpression(step.key)}
if isinstance(${iterVar}, list) and len(${iterVar}) > 0:
    for ${step.item} in ${iterVar}:
${loopBody}
else:
${elseBlock}`;
  }

  if (step.kind === "wait_for_message") {
    const timeout = typeof step.timeout === "number" ? step.timeout : 30;
    const interval = typeof step.interval === "number" ? step.interval : 0.25;
    return `try:
    await ky.wait_for_message(${buildPythonExpression(step.text)}, timeout=${String(timeout)}, interval=${String(interval)})
except Exception as error:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("timeout", `Timed out while waiting for message "${step.text}".`))}) from error`;
  }

  const timeout = typeof step.timeout === "number" ? step.timeout : 90;
  const interval = typeof step.interval === "number" ? step.interval : 0.5;
  return `try:
    await ky.wait_for_job_done(timeout=${String(timeout)}, interval=${String(interval)})
except Exception as error:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("timeout", "Timed out while waiting for job completion."))}) from error`;
}

export function compileWorkbenchFrontendDslToPython(document: WorkbenchFrontendDslDocument) {
  const compiledSteps = compileSteps(document.steps, "dsl_step");
  return `# frontend dsl: ${document.name}
# dsl_version: ${document.dsl_version}

${compiledSteps}
`;
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
