import { readFile } from "node:fs/promises";
import path from "node:path";
import { findAutomationActionContract, validateAutomationStep } from "./kyuubiki-automation-actions.mjs";

const STEP_RESULT_TEMPLATE_RE = /^\{\{\s*steps\.(\d+)\.result\.([a-zA-Z0-9_]+)\s*\}\}$/;
const HEADLESS_SIMULATED_OUTPUT_KEYS = {
  service_health: ["service", "status"],
  project_create: ["project_id", "name"],
  model_create: ["model_id", "kind"],
  model_version_create: ["model_version_id", "kind"],
  workflow_submit_catalog: ["job_id", "status"],
  workflow_submit_graph: ["job_id", "status"],
  job_fetch: ["job_id", "status", "progress"],
  job_wait: ["job_id", "status", "progress"],
  result_fetch: ["job_id", "result"],
  direct_mesh_solve: ["job_id", "status", "endpoint"],
  solve_from_model_version: ["job_id", "status", "model_version_id", "endpoint"],
  solve_and_wait_from_model_version: ["job_id", "status", "model_version_id", "endpoint", "result"],
  open_page: ["url", "status", "ok"],
  click: ["selector"],
  type: ["selector", "value"],
  press: ["key"],
  select: ["selector", "values"],
  wait: ["timeout_ms"],
  assert_text: ["selector", "text"],
  snapshot: ["path"],
};

function compileExecutionValue(value, warnings) {
  if (typeof value === "string") {
    const exact = value.match(STEP_RESULT_TEMPLATE_RE);
    if (exact) {
      return {
        kind: "binding",
        source: {
          kind: "step_result",
          step: Number(exact[1]),
          output: exact[2],
        },
      };
    }
    if (value.includes("{{")) warnings.push(`Unresolved inline template kept as literal: ${value}`);
    return { kind: "literal", value };
  }
  if (Array.isArray(value)) {
    return { kind: "array", items: value.map((entry) => compileExecutionValue(entry, warnings)) };
  }
  if (value && typeof value === "object") {
    return {
      kind: "object",
      fields: Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, compileExecutionValue(entry, warnings)])),
    };
  }
  return { kind: "literal", value };
}

function compileWorkflowToExecutionBatch(candidate) {
  const warnings = [];
  const workflow = candidate.workflow && typeof candidate.workflow === "object" ? candidate.workflow : candidate;
  if (typeof workflow?.id !== "string" || !Array.isArray(workflow?.steps) || workflow.steps.length === 0) {
    throw new Error("Headless workflow document does not contain a valid workflow draft.");
  }
  return {
    schema_version: "kyuubiki.headless-execution-batch/v1",
    exported_at: typeof candidate.exported_at === "string" ? candidate.exported_at : new Date().toISOString(),
    language: typeof candidate.language === "string" ? candidate.language : "en",
    workflow_id: workflow.id.trim() || "macro/imported-headless-workflow",
    steps: workflow.steps.map((step, index) => {
      if (!step || typeof step !== "object" || typeof step.action !== "string" || !step.action.trim()) {
        throw new Error(`Headless workflow step ${index + 1} is invalid.`);
      }
      const contract = findAutomationActionContract(step.action);
      return {
        index: index + 1,
        action: step.action,
        risk: contract?.risk ?? "normal",
        payload: compileExecutionValue(step.payload ?? {}, warnings),
        guidanceNotes: Array.isArray(contract?.guidanceNotes) ? contract.guidanceNotes : [],
      };
    }),
    warnings,
  };
}

export async function loadHeadlessInputDocument(inputPath) {
  return JSON.parse(await readFile(path.resolve(inputPath), "utf8"));
}

export function normalizeHeadlessExecutionBatch(value) {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid headless document payload.");
  }
  const candidate = value;
  if (candidate.schema_version === "kyuubiki.headless-execution-batch/v1") {
    if (typeof candidate.workflow_id !== "string" || !Array.isArray(candidate.steps) || candidate.steps.length === 0) {
      throw new Error("Invalid headless execution batch.");
    }
    return candidate;
  }
  if (
    candidate.schema_version === "kyuubiki.headless-workflow/v1" ||
    (typeof candidate.id === "string" && Array.isArray(candidate.steps))
  ) {
    return compileWorkflowToExecutionBatch(candidate);
  }
  throw new Error("Unsupported headless document schema.");
}

function resolveExecutionValue(value, results) {
  if (!value || typeof value !== "object") return value;
  if (value.kind === "literal") return value.value;
  if (value.kind === "array") return value.items.map((entry) => resolveExecutionValue(entry, results));
  if (value.kind === "object") {
    return Object.fromEntries(Object.entries(value.fields).map(([key, entry]) => [key, resolveExecutionValue(entry, results)]));
  }
  if (value.kind === "binding") {
    const stepResult = results.get(value.source.step);
    if (!stepResult) throw new Error(`Missing result for step ${value.source.step}.`);
    if (!(value.source.output in stepResult)) throw new Error(`Step ${value.source.step} does not expose output ${value.source.output}.`);
    return stepResult[value.source.output];
  }
  return value;
}

function shouldBlockRisk(step, options) {
  if (step.risk === "sensitive" && options.allowSensitive) return false;
  if (step.risk === "destructive" && options.allowDestructive) return false;
  return step.risk === "sensitive" || step.risk === "destructive";
}

export function summarizeHeadlessExecutionBatch(batch) {
  return {
    schema_version: batch.schema_version,
    workflow_id: batch.workflow_id,
    exported_at: batch.exported_at,
    language: batch.language,
    step_count: batch.steps.length,
    warning_count: Array.isArray(batch.warnings) ? batch.warnings.length : 0,
    actions: batch.steps.map((step) => step.action),
  };
}

function materializeValidationPayload(value) {
  if (!value || typeof value !== "object") return value;
  if (value.kind === "literal") return value.value;
  if (value.kind === "binding") {
    return `{{steps.${String(value.source?.step ?? 0)}.result.${String(value.source?.output ?? "value")}}}`;
  }
  if (value.kind === "array") return (value.items ?? []).map((entry) => materializeValidationPayload(entry));
  if (value.kind === "object") {
    return Object.fromEntries(Object.entries(value.fields ?? {}).map(([key, entry]) => [key, materializeValidationPayload(entry)]));
  }
  return value;
}

function collectBindingIssues(value, stepIndex, knownOutputs, issues) {
  if (!value || typeof value !== "object") return;
  if (value.kind === "array") {
    for (const entry of value.items ?? []) collectBindingIssues(entry, stepIndex, knownOutputs, issues);
    return;
  }
  if (value.kind === "object") {
    for (const entry of Object.values(value.fields ?? {})) collectBindingIssues(entry, stepIndex, knownOutputs, issues);
    return;
  }
  if (value.kind !== "binding") return;

  const referencedStep = value.source?.step;
  const referencedOutput = value.source?.output;
  if (!Number.isInteger(referencedStep) || referencedStep < 1) {
    issues.push(`step ${stepIndex} references an invalid source step index`);
    return;
  }
  if (referencedStep >= stepIndex) {
    issues.push(`step ${stepIndex} cannot bind to future-or-self step ${referencedStep}`);
    return;
  }
  const outputs = knownOutputs.get(referencedStep);
  if (!outputs) {
    issues.push(`step ${stepIndex} references missing source step ${referencedStep}`);
    return;
  }
  if (!outputs.has(referencedOutput)) {
    issues.push(`step ${stepIndex} references unavailable output "${referencedOutput}" from step ${referencedStep}`);
  }
}

function resolveKnownOutputKeys(step) {
  const contract = findAutomationActionContract(step.action);
  if (Array.isArray(contract?.outputSchema) && contract.outputSchema.length > 0) {
    return contract.outputSchema.map((output) => output.key);
  }
  return HEADLESS_SIMULATED_OUTPUT_KEYS[step.action] ?? [];
}

function buildHeadlessPolicySummary(batch) {
  const contracts = (batch.steps ?? []).map((step) => findAutomationActionContract(step.action)).filter(Boolean);
  const engineCounts = {
    browser: contracts.filter((contract) => contract.engine === "browser").length,
    service: contracts.filter((contract) => contract.engine === "service").length,
  };
  const riskCounts = {
    normal: contracts.filter((contract) => contract.risk === "normal").length,
    sensitive: contracts.filter((contract) => contract.risk === "sensitive").length,
    destructive: contracts.filter((contract) => contract.risk === "destructive").length,
  };
  const requiredEngines = Array.from(
    new Set(contracts.map((contract) => contract.engine).filter((engine) => typeof engine === "string")),
  ).sort();
  const recommendedRuntime =
    engineCounts.browser > 0 && engineCounts.service > 0 ? "hybrid"
    : engineCounts.browser > 0 ? "browser_only"
    : engineCounts.service > 0 ? "service_only"
    : "unknown";
  const notes = [];
  if (engineCounts.browser > 0) {
    notes.push("Includes browser-backed steps; live execution needs a desktop session that can launch a local browser.");
  }
  if (riskCounts.sensitive > 0) {
    notes.push("Includes sensitive steps; live execution should pass --allow-sensitive after review.");
  }
  if (riskCounts.destructive > 0) {
    notes.push("Includes destructive steps; live execution should pass --allow-destructive only after explicit confirmation.");
  }
  return {
    required_engines: requiredEngines,
    engine_counts: engineCounts,
    risk_counts: riskCounts,
    recommended_runtime: recommendedRuntime,
    needs_desktop_browser: engineCounts.browser > 0,
    safe_for_service_only: engineCounts.browser === 0,
    notes,
  };
}

export function validateHeadlessExecutionBatch(batch) {
  const issues = [];
  if (!batch || typeof batch !== "object") {
    return { ok: false, issue_count: 1, issues: ["headless batch is not an object"], warning_count: 0, warnings: [], summary: null, policy: null };
  }
  if (batch.schema_version !== "kyuubiki.headless-execution-batch/v1") {
    issues.push(`unsupported schema_version: ${String(batch.schema_version ?? "unknown")}`);
  }
  if (typeof batch.workflow_id !== "string" || !batch.workflow_id.trim()) {
    issues.push("workflow_id is missing");
  }
  if (!Array.isArray(batch.steps) || batch.steps.length === 0) {
    issues.push("headless batch has no steps");
  }

  const knownOutputs = new Map();
  for (const [arrayIndex, step] of (batch.steps ?? []).entries()) {
    const stepNumber = arrayIndex + 1;
    if (!step || typeof step !== "object") {
      issues.push(`step ${stepNumber} is not an object`);
      continue;
    }
    if (step.index !== stepNumber) {
      issues.push(`step ${stepNumber} index should be ${stepNumber}, received ${String(step.index)}`);
    }
    const validation = validateAutomationStep(
      { action: step.action, payload: materializeValidationPayload(step.payload) ?? {} },
      arrayIndex,
    );
    issues.push(...validation.issues.map((issue) => issue.replace(`step ${arrayIndex}`, `step ${stepNumber}`)));
    collectBindingIssues(step.payload, stepNumber, knownOutputs, issues);
    knownOutputs.set(stepNumber, new Set(resolveKnownOutputKeys(step)));
  }

  return {
    ok: issues.length === 0,
    issue_count: issues.length,
    issues,
    warning_count: Array.isArray(batch.warnings) ? batch.warnings.length : 0,
    warnings: Array.isArray(batch.warnings) ? batch.warnings : [],
    summary: summarizeHeadlessExecutionBatch(batch),
    policy: buildHeadlessPolicySummary(batch),
  };
}

function buildSimulatedStepResult(step) {
  const contract = findAutomationActionContract(step.action);
  const outputKeys = Array.isArray(contract?.outputSchema)
    ? contract.outputSchema.map((output) => output.key)
    : HEADLESS_SIMULATED_OUTPUT_KEYS[step.action] ?? [];
  return Object.fromEntries(
    outputKeys.map((key) => [
      key,
      key.endsWith("_id")
        ? `simulated-${step.action}-${step.index}-${key}`
        : `simulated:${step.action}:${step.index}:${key}`,
    ]),
  );
}

export async function runHeadlessExecutionBatch(batch, options = {}) {
  const results = new Map();
  const steps = [];
  const dryRun = options.dryRun !== false;
  let status = dryRun ? "simulated" : "completed";

  for (const step of batch.steps) {
    const base = {
      index: step.index,
      action: step.action,
      risk: step.risk,
      payload: resolveExecutionValue(step.payload, results),
      started_at: new Date().toISOString(),
    };

    options.onEvent?.({ message: `[step ${step.index}] start ${step.action}` });

    if (!dryRun && shouldBlockRisk(step, options)) {
      const blocked = {
        ...base,
        status: "blocked",
        completed_at: new Date().toISOString(),
        message: `Step requires ${step.risk} confirmation.`,
      };
      steps.push(blocked);
      status = "blocked";
      break;
    }

    if (dryRun || typeof options.executor !== "function") {
      const simulatedResult = buildSimulatedStepResult(step);
      const simulated = {
        ...base,
        status: "simulated",
        completed_at: new Date().toISOString(),
        message: dryRun ? "Dry-run simulation completed." : "No live executor configured; step acknowledged by runner stub.",
        result: simulatedResult,
      };
      steps.push(simulated);
      results.set(step.index, simulatedResult);
      options.onEvent?.({ message: `[step ${step.index}] done ${step.action}` });
      continue;
    }

    try {
      const execution = await options.executor(
        {
          index: step.index,
          action: step.action,
          risk: step.risk,
          requires_confirmation: step.risk !== "normal",
          payload: base.payload,
        },
        options.context ?? {},
      );
      const completed = {
        ...base,
        status: execution?.status === "blocked" ? "blocked" : "completed",
        completed_at: new Date().toISOString(),
        message: execution?.message ?? "Step executed.",
        executor: execution?.executor ?? "custom",
        result: execution?.result ?? null,
      };
      steps.push(completed);
      results.set(step.index, execution?.result ?? {});
      options.onEvent?.({ message: `[step ${step.index}] done ${step.action}` });
      if (completed.status === "blocked") {
        status = "blocked";
        break;
      }
    } catch (error) {
      const failed = {
        ...base,
        status: "failed",
        completed_at: new Date().toISOString(),
        message: error instanceof Error ? error.message : String(error),
        error_code: error && typeof error === "object" && typeof error.code === "string" ? error.code : null,
      };
      steps.push(failed);
      status = "failed";
      break;
    }
  }

  return {
    schema_version: "kyuubiki.headless-execution-run/v1",
    workflow_id: batch.workflow_id,
    started_at: steps[0]?.started_at ?? new Date().toISOString(),
    completed_at: new Date().toISOString(),
    status,
    dry_run: dryRun,
    requested_capabilities: {
      allow_sensitive: Boolean(options.allowSensitive),
      allow_destructive: Boolean(options.allowDestructive),
    },
    warning_count: Array.isArray(batch.warnings) ? batch.warnings.length : 0,
    warnings: Array.isArray(batch.warnings) ? batch.warnings : [],
    executed_step_count: steps.filter((step) => step.status === "completed" || step.status === "simulated").length,
    blocked_by_confirmation: steps.find((step) => step.status === "blocked") ?? null,
    failed_step: steps.find((step) => step.status === "failed") ?? null,
    steps,
  };
}
