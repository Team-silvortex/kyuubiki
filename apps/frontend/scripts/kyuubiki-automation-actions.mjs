const ACTION_CONTRACTS = [
  {
    id: "open_page",
    aliases: ["goto", "navigate", "browser_open_page"],
    engine: "browser",
    category: "navigation",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Open a URL in the active browser page.",
    requiredPayloadKeys: ["url"],
    optionalPayloadKeys: ["href", "waitUntil", "wait_until", "timeout", "timeoutMs", "timeout_ms"],
    examples: [{ url: "https://example.com", waitUntil: "networkidle" }],
  },
  {
    id: "click",
    aliases: ["browser_click"],
    engine: "browser",
    category: "interaction",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Click a DOM target matched by selector.",
    requiredPayloadKeys: ["selector"],
    optionalPayloadKeys: ["target", "timeout", "timeoutMs", "timeout_ms"],
    examples: [{ selector: "#submit" }],
  },
  {
    id: "type",
    aliases: ["fill", "input", "browser_type"],
    engine: "browser",
    category: "interaction",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Fill text into an input-like field.",
    requiredPayloadKeys: ["selector", "value"],
    optionalPayloadKeys: ["target", "text", "input", "timeout", "timeoutMs", "timeout_ms"],
    examples: [{ selector: "#name", value: "tamamono" }],
  },
  {
    id: "press",
    aliases: ["keyboard_press", "browser_press"],
    engine: "browser",
    category: "interaction",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Send a keyboard key or shortcut to the page or a focused locator.",
    requiredPayloadKeys: ["key"],
    optionalPayloadKeys: ["selector", "target", "timeout", "timeoutMs", "timeout_ms"],
    examples: [{ key: "Enter" }, { selector: "#name", key: "Meta+A" }],
  },
  {
    id: "select",
    aliases: ["select_option", "browser_select"],
    engine: "browser",
    category: "interaction",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Select one or more option values from a select element.",
    requiredPayloadKeys: ["selector", "value"],
    optionalPayloadKeys: ["target", "values", "timeout", "timeoutMs", "timeout_ms"],
    examples: [{ selector: "#country", value: "jp" }],
  },
  {
    id: "wait",
    aliases: ["sleep", "wait_for", "browser_wait"],
    engine: "browser",
    category: "timing",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Wait by duration, selector state, or matched text.",
    requiredPayloadKeys: [],
    optionalPayloadKeys: ["selector", "target", "text", "value", "state", "timeout", "timeoutMs", "timeout_ms", "duration", "durationMs"],
    examples: [{ selector: "#result", text: "done", timeout: 1500 }, { duration: 500 }],
  },
  {
    id: "assert_text",
    aliases: ["expect_text", "browser_assert_text"],
    engine: "browser",
    category: "assertion",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Assert that a selector contains the expected text.",
    requiredPayloadKeys: ["selector", "text"],
    optionalPayloadKeys: ["target", "timeout", "timeoutMs", "timeout_ms"],
    examples: [{ selector: "#result", text: "tamamono" }],
  },
  {
    id: "snapshot",
    aliases: ["screenshot", "export_snapshot", "browser_snapshot"],
    engine: "browser",
    category: "artifact",
    risk: "sensitive",
    requiresConfirmation: true,
    summary: "Capture a page screenshot into the artifacts directory.",
    requiredPayloadKeys: [],
    optionalPayloadKeys: ["file", "filename", "name", "fullPage", "full_page"],
    examples: [{ file: "result.png", fullPage: true }],
  },
  {
    id: "service_health",
    aliases: ["health_check", "api_health"],
    engine: "service",
    category: "service",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Fetch the platform health payload directly from the service API.",
    requiredPayloadKeys: [],
    optionalPayloadKeys: ["path"],
    examples: [{}],
  },
  {
    id: "project_create",
    aliases: ["api_project_create"],
    engine: "service",
    category: "project",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Create a project through the service API without using the frontend UI.",
    requiredPayloadKeys: ["name"],
    optionalPayloadKeys: ["description"],
    examples: [{ name: "Headless Project", description: "created by macro" }],
  },
  {
    id: "project_update",
    aliases: ["api_project_update"],
    engine: "service",
    category: "project",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Patch a project record directly through the service API.",
    requiredPayloadKeys: ["project_id"],
    optionalPayloadKeys: ["name", "description"],
    examples: [{ project_id: "proj_123", name: "Renamed Project" }],
  },
  {
    id: "project_delete",
    aliases: ["api_project_delete"],
    engine: "service",
    category: "project",
    risk: "destructive",
    requiresConfirmation: true,
    summary: "Delete a project directly through the service API.",
    requiredPayloadKeys: ["project_id"],
    optionalPayloadKeys: [],
    examples: [{ project_id: "proj_123" }],
  },
  {
    id: "model_create",
    aliases: ["api_model_create"],
    engine: "service",
    category: "model",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Create a model directly under a project through the service API.",
    requiredPayloadKeys: ["project_id", "name", "kind", "payload"],
    optionalPayloadKeys: ["material", "model_schema_version"],
    examples: [{ project_id: "proj_123", name: "truss-demo", kind: "truss_2d", payload: { nodes: [], elements: [] } }],
  },
  {
    id: "model_version_create",
    aliases: ["api_model_version_create"],
    engine: "service",
    category: "model",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Create a saved model version directly through the service API.",
    requiredPayloadKeys: ["model_id", "payload"],
    optionalPayloadKeys: ["name", "kind", "material", "model_schema_version"],
    examples: [{ model_id: "model_123", name: "v2", payload: { nodes: [], elements: [] } }],
  },
  {
    id: "workflow_submit_catalog",
    aliases: ["api_workflow_submit_catalog"],
    engine: "service",
    category: "workflow",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Submit a named workflow job directly to the workflow service API.",
    requiredPayloadKeys: ["workflow_id"],
    optionalPayloadKeys: ["input_artifacts"],
    examples: [{ workflow_id: "wf_demo", input_artifacts: {} }],
  },
  {
    id: "workflow_submit_graph",
    aliases: ["api_workflow_submit_graph"],
    engine: "service",
    category: "workflow",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Submit an ad hoc workflow graph directly to the workflow service API.",
    requiredPayloadKeys: ["graph"],
    optionalPayloadKeys: ["input_artifacts"],
    examples: [{ graph: { nodes: [], edges: [] }, input_artifacts: {} }],
  },
  {
    id: "job_wait",
    aliases: ["api_job_wait", "job_poll"],
    engine: "service",
    category: "job",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Poll a job until it reaches a terminal status directly from the service API.",
    requiredPayloadKeys: ["job_id"],
    optionalPayloadKeys: ["interval_ms", "timeout_ms"],
    examples: [{ job_id: "job_123", interval_ms: 1000, timeout_ms: 60000 }],
  },
  {
    id: "job_fetch",
    aliases: ["api_job_fetch", "job_status"],
    engine: "service",
    category: "job",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Fetch a job envelope directly from the service API.",
    requiredPayloadKeys: ["job_id"],
    optionalPayloadKeys: [],
    examples: [{ job_id: "job_123" }],
  },
  {
    id: "result_fetch",
    aliases: ["api_result_fetch", "job_fetch_result"],
    engine: "service",
    category: "result",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Fetch the result payload for a job without going through the frontend UI state.",
    requiredPayloadKeys: ["job_id"],
    optionalPayloadKeys: ["prefer_job_result", "direct_mesh"],
    examples: [{ job_id: "job_123" }],
  },
  {
    id: "direct_mesh_solve",
    aliases: ["api_direct_mesh_solve"],
    engine: "service",
    category: "solve",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Submit a direct-mesh solve request directly to headless solver agents.",
    requiredPayloadKeys: ["endpoints"],
    optionalPayloadKeys: ["study_kind", "input", "model_payload", "model_id", "model_version_id", "materials", "selection_mode", "project_id"],
    examples: [{ study_kind: "truss_3d", input: { nodes: [], elements: [] }, endpoints: ["http://127.0.0.1:7001"] }],
  },
  {
    id: "solve_from_model_version",
    aliases: ["api_solve_from_model_version"],
    engine: "service",
    category: "solve",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Resolve study kind and solve input from a saved model version, then submit a direct-mesh solve.",
    requiredPayloadKeys: ["model_version_id", "endpoints"],
    optionalPayloadKeys: ["selection_mode", "project_id"],
    examples: [{ model_version_id: "ver_123", endpoints: ["http://127.0.0.1:7001"] }],
  },
  {
    id: "solve_and_wait_from_model_version",
    aliases: ["api_solve_and_wait_from_model_version"],
    engine: "service",
    category: "solve",
    risk: "normal",
    requiresConfirmation: false,
    summary: "Resolve from a saved model version, submit direct-mesh solve, wait for completion, and fetch the result payload.",
    requiredPayloadKeys: ["model_version_id", "endpoints"],
    optionalPayloadKeys: ["selection_mode", "project_id", "interval_ms", "timeout_ms", "prefer_job_result", "direct_mesh"],
    examples: [{ model_version_id: "ver_123", endpoints: ["http://127.0.0.1:7001"], timeout_ms: 60000 }],
  },
];

const ACTION_CONTRACT_BY_NAME = new Map();
for (const contract of ACTION_CONTRACTS) {
  ACTION_CONTRACT_BY_NAME.set(contract.id, contract);
  for (const alias of contract.aliases) ACTION_CONTRACT_BY_NAME.set(alias, contract);
}

export function listAutomationActionContracts() {
  return ACTION_CONTRACTS.map((contract) => ({
    ...contract,
    aliases: [...contract.aliases],
    requiredPayloadKeys: [...contract.requiredPayloadKeys],
    optionalPayloadKeys: [...contract.optionalPayloadKeys],
    examples: contract.examples.map((entry) => ({ ...entry })),
  }));
}

export function findAutomationActionContract(action) {
  if (typeof action !== "string" || !action.trim()) return null;
  return ACTION_CONTRACT_BY_NAME.get(action.trim()) ?? null;
}

function hasPresentValue(payload, key) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) return false;
  const value = payload[key];
  if (typeof value === "string") return value.trim().length > 0;
  return value !== undefined && value !== null;
}

function missingRequiredKeys(contract, payload) {
  if (contract.id === "type") {
    const hasSelector = hasPresentValue(payload, "selector") || hasPresentValue(payload, "target");
    const hasValue = hasPresentValue(payload, "value") || hasPresentValue(payload, "text") || hasPresentValue(payload, "input");
    return [...(!hasSelector ? ["selector"] : []), ...(!hasValue ? ["value"] : [])];
  }
  if (contract.id === "click") {
    return hasPresentValue(payload, "selector") || hasPresentValue(payload, "target") ? [] : ["selector"];
  }
  if (contract.id === "open_page") {
    return hasPresentValue(payload, "url") || hasPresentValue(payload, "href") ? [] : ["url"];
  }
  if (contract.id === "wait") {
    const hasSelector = hasPresentValue(payload, "selector") || hasPresentValue(payload, "target");
    const hasDuration = hasPresentValue(payload, "duration") || hasPresentValue(payload, "durationMs") || hasPresentValue(payload, "timeout");
    return hasSelector || hasDuration ? [] : ["selector|duration"];
  }
  if (contract.id === "select") {
    const hasSelector = hasPresentValue(payload, "selector") || hasPresentValue(payload, "target");
    const hasValue = hasPresentValue(payload, "value") || hasPresentValue(payload, "values");
    return [...(!hasSelector ? ["selector"] : []), ...(!hasValue ? ["value"] : [])];
  }
  if (contract.id === "press") {
    return hasPresentValue(payload, "key") ? [] : ["key"];
  }
  if (contract.id === "assert_text") {
    const hasSelector = hasPresentValue(payload, "selector") || hasPresentValue(payload, "target");
    const hasText = hasPresentValue(payload, "text");
    return [...(!hasSelector ? ["selector"] : []), ...(!hasText ? ["text"] : [])];
  }
  if (contract.id === "project_update") {
    const hasPatch = hasPresentValue(payload, "name") || hasPresentValue(payload, "description");
    return [...(!hasPresentValue(payload, "project_id") ? ["project_id"] : []), ...(!hasPatch ? ["name|description"] : [])];
  }
  if (contract.id === "model_create") {
    return [
      ...(!hasPresentValue(payload, "project_id") ? ["project_id"] : []),
      ...(!hasPresentValue(payload, "name") ? ["name"] : []),
      ...(!hasPresentValue(payload, "kind") ? ["kind"] : []),
      ...(!hasPresentValue(payload, "payload") ? ["payload"] : []),
    ];
  }
  if (contract.id === "model_version_create") {
    return [...(!hasPresentValue(payload, "model_id") ? ["model_id"] : []), ...(!hasPresentValue(payload, "payload") ? ["payload"] : [])];
  }
  if (contract.id === "workflow_submit_catalog") {
    return hasPresentValue(payload, "workflow_id") ? [] : ["workflow_id"];
  }
  if (contract.id === "workflow_submit_graph") {
    return hasPresentValue(payload, "graph") ? [] : ["graph"];
  }
  if (contract.id === "job_wait") {
    return hasPresentValue(payload, "job_id") ? [] : ["job_id"];
  }
  if (contract.id === "job_fetch") {
    return hasPresentValue(payload, "job_id") ? [] : ["job_id"];
  }
  if (contract.id === "result_fetch") {
    return hasPresentValue(payload, "job_id") ? [] : ["job_id"];
  }
  if (contract.id === "direct_mesh_solve") {
    const hasStudy =
      hasPresentValue(payload, "study_kind") ||
      hasPresentValue(payload, "model_id") ||
      hasPresentValue(payload, "model_version_id");
    const hasSource =
      hasPresentValue(payload, "input") ||
      hasPresentValue(payload, "model_payload") ||
      hasPresentValue(payload, "model_id") ||
      hasPresentValue(payload, "model_version_id");
    return [
      ...(!hasStudy ? ["study_kind|model_id|model_version_id"] : []),
      ...(!hasSource ? ["input|model_payload|model_id|model_version_id"] : []),
      ...(!hasPresentValue(payload, "endpoints") ? ["endpoints"] : []),
    ];
  }
  if (contract.id === "solve_from_model_version") {
    return [
      ...(!hasPresentValue(payload, "model_version_id") ? ["model_version_id"] : []),
      ...(!hasPresentValue(payload, "endpoints") ? ["endpoints"] : []),
    ];
  }
  if (contract.id === "solve_and_wait_from_model_version") {
    return [
      ...(!hasPresentValue(payload, "model_version_id") ? ["model_version_id"] : []),
      ...(!hasPresentValue(payload, "endpoints") ? ["endpoints"] : []),
    ];
  }
  return contract.requiredPayloadKeys.filter((key) => !hasPresentValue(payload, key));
}

export function validateAutomationStep(step, index = 0) {
  const issues = [];
  if (!step || typeof step !== "object") {
    return { ok: false, issues: [`step ${index} is not an object`], contract: null };
  }
  if (typeof step.action !== "string" || !step.action.trim()) {
    return { ok: false, issues: [`step ${index} is missing action`], contract: null };
  }
  if (step.payload !== undefined && (!step.payload || typeof step.payload !== "object" || Array.isArray(step.payload))) {
    return { ok: false, issues: [`step ${index} has an invalid payload`], contract: null };
  }

  const contract = findAutomationActionContract(step.action);
  if (!contract) {
    issues.push(`step ${index} uses unsupported action "${step.action}"`);
    return { ok: false, issues, contract: null };
  }

  const missingKeys = missingRequiredKeys(contract, step.payload ?? {});
  if (missingKeys.length > 0) {
    issues.push(`step ${index} action "${step.action}" is missing payload keys: ${missingKeys.join(", ")}`);
  }

  return {
    ok: issues.length === 0,
    issues,
    contract,
  };
}
