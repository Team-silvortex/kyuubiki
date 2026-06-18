function normalizeBooleanFlag(value, defaultValue = false) {
  if (value === undefined) return defaultValue;
  if (typeof value === "boolean") return value;
  const normalized = String(value).trim().toLowerCase();
  if (!normalized) return defaultValue;
  return normalized !== "false" && normalized !== "0" && normalized !== "no";
}

function shouldBlockStep(step, options) {
  if (!step.requires_confirmation) return false;
  if (step.risk === "sensitive" && options.allowSensitive) return false;
  if (step.risk === "destructive" && options.allowDestructive) return false;
  return !options.dryRun;
}

function resolveExecutionError(error) {
  const message = error instanceof Error ? error.message : String(error);
  const code = error && typeof error === "object" && typeof error.code === "string" ? error.code : null;
  return { message, code };
}

async function executeStep(step, options) {
  const startedAt = new Date().toISOString();
  const base = {
    index: step.index,
    action: step.action,
    risk: step.risk,
    requires_confirmation: step.requires_confirmation,
    payload: step.payload,
    started_at: startedAt,
  };

  if (shouldBlockStep(step, options)) {
    return {
      ...base,
      status: "blocked",
      completed_at: new Date().toISOString(),
      message: `Step requires ${step.risk} confirmation.`,
    };
  }

  if (options.dryRun) {
    return {
      ...base,
      status: "simulated",
      completed_at: new Date().toISOString(),
      message: "Dry-run simulation completed.",
    };
  }

  if (typeof options.executor !== "function") {
    return {
      ...base,
      status: "completed",
      completed_at: new Date().toISOString(),
      message: "No live executor configured; step acknowledged by runner stub.",
      executor: "stub",
    };
  }

  try {
    const execution = await options.executor(step, options.context ?? {});
    return {
      ...base,
      status: execution?.status === "blocked" ? "blocked" : "completed",
      completed_at: new Date().toISOString(),
      message: execution?.message ?? "Step executed.",
      executor: execution?.executor ?? "custom",
      result: execution?.result ?? null,
    };
  } catch (error) {
    const failure = resolveExecutionError(error);
    return {
      ...base,
      status: "failed",
      completed_at: new Date().toISOString(),
      message: failure.message,
      error_code: failure.code,
    };
  }
}

export async function runHeadlessAutomationEnvelope(envelope, options = {}) {
  if (!envelope || typeof envelope !== "object" || !envelope.plan || !Array.isArray(envelope.plan.steps)) {
    throw new Error("Invalid headless automation envelope.");
  }

  const normalizedOptions = {
    dryRun: normalizeBooleanFlag(options.dryRun, true),
    allowSensitive: normalizeBooleanFlag(options.allowSensitive, false),
    allowDestructive: normalizeBooleanFlag(options.allowDestructive, false),
    executor: typeof options.executor === "function" ? options.executor : null,
    context: options.context && typeof options.context === "object" ? options.context : {},
  };

  const startedAt = new Date().toISOString();
  const steps = [];
  let status = normalizedOptions.dryRun ? "simulated" : "completed";

  for (const step of envelope.plan.steps) {
    const result = await executeStep(step, normalizedOptions);
    steps.push(result);
    if (result.status === "blocked") {
      status = "blocked";
      break;
    }
    if (result.status === "failed") {
      status = "failed";
      break;
    }
  }

  return {
    schema_version: "kyuubiki.headless-automation-run/v1",
    source: envelope.source,
    metadata: {
      macro_id: envelope.metadata?.macro_id ?? envelope.plan?.id ?? null,
      step_count: envelope.metadata?.step_count ?? envelope.plan.steps.length,
      generated_at: envelope.metadata?.generated_at ?? null,
    },
    started_at: startedAt,
    completed_at: new Date().toISOString(),
    status,
    dry_run: normalizedOptions.dryRun,
    requested_capabilities: {
      allow_sensitive: normalizedOptions.allowSensitive,
      allow_destructive: normalizedOptions.allowDestructive,
    },
    risk_summary: envelope.risk_summary ?? null,
    blocked_by_confirmation: steps.find((step) => step.status === "blocked") ?? null,
    failed_step: steps.find((step) => step.status === "failed") ?? null,
    executed_step_count: steps.filter((step) => step.status === "completed" || step.status === "simulated").length,
    steps,
  };
}
