const ACTION_CONTRACTS = [
  {
    id: "open_page",
    aliases: ["goto", "navigate", "browser_open_page"],
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
    category: "artifact",
    risk: "sensitive",
    requiresConfirmation: true,
    summary: "Capture a page screenshot into the artifacts directory.",
    requiredPayloadKeys: [],
    optionalPayloadKeys: ["file", "filename", "name", "fullPage", "full_page"],
    examples: [{ file: "result.png", fullPage: true }],
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
