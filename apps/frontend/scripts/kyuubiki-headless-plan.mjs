import { findAutomationActionContract } from "./kyuubiki-automation-actions.mjs";
import { validateHeadlessExecutionBatch } from "./kyuubiki-headless-batch.mjs";

function materializePlanValue(value) {
  if (!value || typeof value !== "object") return value;
  if (value.kind === "literal") return value.value;
  if (value.kind === "binding") {
    return `{{steps.${String(value.source?.step ?? 0)}.result.${String(value.source?.output ?? "value")}}}`;
  }
  if (value.kind === "array") return (value.items ?? []).map((entry) => materializePlanValue(entry));
  if (value.kind === "object") {
    return Object.fromEntries(
      Object.entries(value.fields ?? {}).map(([key, entry]) => [key, materializePlanValue(entry)]),
    );
  }
  return value;
}

function collectBindings(value, bindings = []) {
  if (!value || typeof value !== "object") return bindings;
  if (value.kind === "binding") {
    bindings.push({
      source_step: value.source?.step ?? null,
      output: value.source?.output ?? null,
    });
    return bindings;
  }
  if (value.kind === "array") {
    for (const entry of value.items ?? []) collectBindings(entry, bindings);
    return bindings;
  }
  if (value.kind === "object") {
    for (const entry of Object.values(value.fields ?? {})) collectBindings(entry, bindings);
  }
  return bindings;
}

function resolveStepConfirmation(step) {
  if (step.risk === "destructive") {
    return {
      required: true,
      flag: "--allow-destructive",
      reason: "destructive step must be explicitly allowed before live execution",
    };
  }
  if (step.risk === "sensitive") {
    return {
      required: true,
      flag: "--allow-sensitive",
      reason: "sensitive step must be explicitly allowed before live execution",
    };
  }
  return { required: false, flag: null, reason: "normal-risk step" };
}

function buildStepPlan(step) {
  const contract = findAutomationActionContract(step.action);
  const confirmation = resolveStepConfirmation(step);
  return {
    index: step.index,
    action: step.action,
    engine: contract?.engine ?? "unknown",
    category: contract?.category ?? "unknown",
    risk: step.risk,
    requires_confirmation: confirmation.required,
    confirmation_flag: confirmation.flag,
    confirmation_reason: confirmation.reason,
    payload: materializePlanValue(step.payload),
    bindings: collectBindings(step.payload),
    output_keys: Array.isArray(contract?.outputSchema)
      ? contract.outputSchema.map((entry) => entry.key)
      : [],
    guidance_notes: Array.isArray(step.guidanceNotes) ? step.guidanceNotes : [],
  };
}

function buildCompatibility(validation) {
  const policy = validation.policy ?? {};
  const requiredEngines = policy.required_engines ?? [];
  return {
    service_only: {
      ok: Boolean(policy.safe_for_service_only),
      reason: policy.safe_for_service_only
        ? "all steps are service-backed"
        : "workflow includes browser-backed steps",
    },
    browser_session_required: Boolean(policy.needs_desktop_browser),
    hybrid_required: requiredEngines.includes("browser") && requiredEngines.includes("service"),
  };
}

export function buildHeadlessExecutionPlan(batch) {
  const validation = validateHeadlessExecutionBatch(batch);
  const steps = (batch.steps ?? []).map(buildStepPlan);
  const confirmations = steps
    .filter((step) => step.requires_confirmation)
    .map((step) => ({
      index: step.index,
      action: step.action,
      risk: step.risk,
      flag: step.confirmation_flag,
      reason: step.confirmation_reason,
    }));
  return {
    schema_version: "kyuubiki.headless-plan/v1",
    workflow_id: batch.workflow_id,
    generated_at: new Date().toISOString(),
    ok: validation.ok,
    validation,
    policy: validation.policy,
    compatibility: buildCompatibility(validation),
    confirmation_count: confirmations.length,
    confirmations,
    steps,
  };
}
