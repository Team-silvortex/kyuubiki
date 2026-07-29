"use client";

import {
  buildPythonExpression,
  type WorkbenchFrontendDslVarReference,
} from "./workbench-script-dsl-expressions.ts";
import type {
  WorkbenchFrontendDslDocument,
  WorkbenchFrontendDslStep,
} from "./workbench-script-dsl.ts";
import { getWorkbenchScriptRecipeDefinition } from "./workbench-script-runtime-recipes.ts";

function buildDslFailureMessage(
  code: "capability_mismatch" | "selector_mismatch" | "state_mismatch" | "timeout",
  message: string,
) {
  return `[dsl-code=${code}] ${message}`;
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

function compileRecipeResultContract(step: Extract<WorkbenchFrontendDslStep, { kind: "run_recipe" }>, assign: string) {
  const resultKeys = getWorkbenchScriptRecipeDefinition(step.recipeId)?.success?.resultKeys ?? [];
  if (resultKeys.length === 0) return "";
  return `
if not isinstance(${assign}, dict):
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("state_mismatch", `Recipe ${step.recipeId} did not return a result object.`))})
for key in ${buildPythonExpression(resultKeys)}:
    if key not in ${assign}:
        raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("state_mismatch", `Recipe ${step.recipeId} result is missing a required key.`))} + " " + str(key))`;
}

function compileStep(step: WorkbenchFrontendDslStep, stepLabel: string) {
  if (step.kind === "invoke") {
    return `${stepLabel}_payload = ${buildPythonExpression(step.payload ?? {})}
ky.require_action(${JSON.stringify(step.action)})
${stepLabel}_result = await ky.invoke(${JSON.stringify(step.action)}, ${stepLabel}_payload)
ky.log("DSL invoke:", ${JSON.stringify(step.action)}, ${stepLabel}_result)`;
  }

  if (step.kind === "macro") {
    return `ky.require_macro(${JSON.stringify(step.macroId)})
${stepLabel}_result = await ky.run_macro(${JSON.stringify(step.macroId)}, ${buildPythonExpression(step.payload ?? {})})
ky.log("DSL macro:", ${JSON.stringify(step.macroId)}, ${stepLabel}_result)`;
  }

  if (step.kind === "run_recipe") {
    const assign = step.assign ?? `${stepLabel}_recipe_result`;
    return `${assign} = await ky.run_recipe(${JSON.stringify(step.recipeId)}, ${buildPythonExpression(step.payload ?? {})})
${compileRecipeResultContract(step, assign)}
ky.log(${buildPythonExpression(step.message?.trim() || "DSL recipe completed.")}, ${assign})`;
  }

  if (step.kind === "expect_action") {
    const message = step.message?.trim() || `Workbench action "${step.action}" is not available.`;
    return `try:
    ky.require_action(${JSON.stringify(step.action)})
except Exception as error:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("capability_mismatch", message))}) from error
ky.log("DSL action available:", ${JSON.stringify(step.action)})`;
  }

  if (step.kind === "expect_macro") {
    const message = step.message?.trim() || `Workbench macro "${step.macroId}" is not available.`;
    return `try:
    ky.require_macro(${JSON.stringify(step.macroId)})
except Exception as error:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("capability_mismatch", message))}) from error
ky.log("DSL macro available:", ${JSON.stringify(step.macroId)})`;
  }

  if (step.kind === "expect_recipe") {
    const message = step.message?.trim() || `Pwdt recipe "${step.recipeId}" is not available.`;
    return `try:
    ky.require_recipe(${JSON.stringify(step.recipeId)})
except Exception as error:
    raise RuntimeError(${buildPythonExpression(buildDslFailureMessage("capability_mismatch", message))}) from error
ky.log("DSL recipe available:", ${JSON.stringify(step.recipeId)})`;
  }

  if (step.kind === "capture_action_catalog") {
    const filters = [
      step.category ? `category=${JSON.stringify(step.category)}` : null,
      step.risk ? `risk=${JSON.stringify(step.risk)}` : null,
    ].filter(Boolean).join(", ");
    return `${step.assign} = ky.actions_matching(${filters})
ky.log(${buildPythonExpression(step.message?.trim() || "DSL captured Workbench action catalog.")}, len(${step.assign}))`;
  }

  if (step.kind === "capture_recipe_catalog") {
    const filters = [
      step.category ? `category=${JSON.stringify(step.category)}` : null,
      step.risk ? `risk=${JSON.stringify(step.risk)}` : null,
    ].filter(Boolean).join(", ");
    return `${step.assign} = ky.recipes_matching(${filters})
ky.log(${buildPythonExpression(step.message?.trim() || "DSL captured Pwdt recipe catalog.")}, len(${step.assign}))`;
  }

  if (step.kind === "emit_parity_report") {
    const assign = step.assign ?? `${stepLabel}_parity_report`;
    return `${assign} = ky.automation_parity_report()
ky.log(${buildPythonExpression(step.message?.trim() || "DSL automation parity report.")}, json.dumps(${assign}, sort_keys=True))`;
  }

  if (step.kind === "log") return `ky.log(${buildPythonExpression(step.message)})`;
  if (step.kind === "sleep") return `await ky.sleep(${String(step.seconds)})`;

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
    const thenBlock = compileSteps(step.then, `${stepLabel}_then`).split("\n").map((line) => `    ${line}`).join("\n");
    const elseBlock = step.else && step.else.length > 0
      ? compileSteps(step.else, `${stepLabel}_else`).split("\n").map((line) => `    ${line}`).join("\n")
      : `    ky.log("DSL branch skipped else:", ${JSON.stringify(step.key)})`;
    return `${stepLabel}_state_value = ${buildStateReadExpression(step.key)}
if ${stepLabel}_state_value == ${buildPythonExpression(step.equals)}:
${thenBlock}
else:
${elseBlock}`;
  }

  if (step.kind === "foreach_state_list") {
    const iterVar = `${stepLabel}_items`;
    const loopBody = compileSteps(step.steps, `${stepLabel}_loop`).split("\n").map((line) => `    ${line}`).join("\n");
    const elseBlock = step.else && step.else.length > 0
      ? compileSteps(step.else, `${stepLabel}_empty`).split("\n").map((line) => `    ${line}`).join("\n")
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
