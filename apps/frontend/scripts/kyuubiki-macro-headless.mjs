import { findAutomationActionContract, validateAutomationStep } from "./kyuubiki-automation-actions.mjs";

const MACRO_TEMPLATE_EXACT_RE = /^\{\{\s*(payload|state)\.([a-zA-Z0-9_]+)\s*\}\}$/;
const MACRO_TEMPLATE_INLINE_RE = /\{\{\s*(payload|state)\.([a-zA-Z0-9_]+)\s*\}\}/g;

function inferActionRisk(action) {
  const contract = findAutomationActionContract(action);
  if (contract) return contract.risk;
  if (/delete|destroy|remove/i.test(action)) return "destructive";
  if (/export|snapshot|database/i.test(action)) return "sensitive";
  return "normal";
}

function inferRequiresConfirmation(action, risk) {
  const contract = findAutomationActionContract(action);
  if (contract) return contract.requiresConfirmation;
  return risk !== "normal" || /delete|destroy|remove|export|snapshot|database/i.test(action);
}

export function normalizeMacroDraft(value) {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid macro document.");
  }

  const id = typeof value.id === "string" && value.id.trim() ? value.id : "macro/imported";
  const steps = Array.isArray(value.steps) ? value.steps : null;
  if (!steps || steps.length === 0) {
    throw new Error("Macro document does not contain any steps.");
  }

  return {
    id,
    steps: steps.map((step, index) => {
      const validation = validateAutomationStep(step, index);
      if (!validation.ok) throw new Error(validation.issues[0]);
      return {
        action: step.action,
        ...(step.payload ? { payload: step.payload } : {}),
      };
    }),
  };
}

function resolveTemplateString(value, payload, state) {
  const exact = value.match(MACRO_TEMPLATE_EXACT_RE);
  if (exact) {
    const [, source, key] = exact;
    return source === "payload" ? payload[key] : state[key];
  }

  return value.replaceAll(MACRO_TEMPLATE_INLINE_RE, (_full, source, key) => {
    const resolved = source === "payload" ? payload[key] : state[key];
    return resolved === undefined || resolved === null ? "" : String(resolved);
  });
}

export function resolveMacroPayloadTemplates(value, payload = {}, state = {}) {
  if (typeof value === "string") return resolveTemplateString(value, payload, state);
  if (Array.isArray(value)) return value.map((entry) => resolveMacroPayloadTemplates(entry, payload, state));
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, resolveMacroPayloadTemplates(entry, payload, state)]),
    );
  }
  return value;
}

export function renderMacroExecutionPlan(macro, options = {}) {
  const normalized = normalizeMacroDraft(macro);
  const payload = options.payload && typeof options.payload === "object" ? options.payload : {};
  const state = options.state && typeof options.state === "object" ? options.state : {};
  const steps = normalized.steps.map((step, index) => {
    const risk = inferActionRisk(step.action);
    return {
      index,
      action: step.action,
      risk,
      requires_confirmation: inferRequiresConfirmation(step.action, risk),
      payload: resolveMacroPayloadTemplates(step.payload ?? {}, payload, state),
    };
  });

  return {
    id: normalized.id,
    step_count: steps.length,
    actions: steps.map((step) => step.action),
    payload,
    state,
    steps,
  };
}

export function buildHeadlessAutomationEnvelope(source, macro, options = {}) {
  const plan = renderMacroExecutionPlan(macro, options);
  const highestRisk = plan.steps.some((step) => step.risk === "destructive")
    ? "destructive"
    : plan.steps.some((step) => step.risk === "sensitive")
      ? "sensitive"
      : "normal";

  return {
    schema_version: "kyuubiki.headless-automation-plan/v1",
    source,
    metadata: {
      macro_id: plan.id,
      generated_at: new Date().toISOString(),
      step_count: plan.step_count,
      action_count: plan.actions.length,
    },
    risk_summary: {
      highest_risk: highestRisk,
      sensitive_step_count: plan.steps.filter((step) => step.risk === "sensitive").length,
      destructive_step_count: plan.steps.filter((step) => step.risk === "destructive").length,
    },
    required_confirmations: plan.steps
      .filter((step) => step.requires_confirmation)
      .map((step) => ({ step_index: step.index, action: step.action, risk: step.risk })),
    plan,
  };
}
