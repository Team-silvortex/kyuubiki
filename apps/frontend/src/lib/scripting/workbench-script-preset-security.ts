"use client";

const SENSITIVE_SNIPPET_ERROR_PREFIX =
  "Snippet presets cannot persist sensitive fields:";
const SENSITIVE_MACRO_ERROR_PREFIX =
  "Macro presets cannot persist sensitive payload fields:";

const SENSITIVE_KEY_FRAGMENTS = [
  "token",
  "secret",
  "password",
  "passwd",
  "apikey",
  "authorization",
  "authheader",
  "bearer",
  "cookie",
  "credential",
  "sessionkey",
];

function normalizeSensitiveKey(key: string) {
  return key.replace(/[^a-z0-9]/gi, "").toLowerCase();
}

function isSensitiveKey(key: string) {
  const normalized = normalizeSensitiveKey(key);
  return SENSITIVE_KEY_FRAGMENTS.some((fragment) => normalized.includes(fragment));
}

function appendSensitivePath(target: string[], path: string) {
  if (!target.includes(path)) {
    target.push(path);
  }
}

export function listSensitiveSnippetParameterKeys(parameters: Record<string, unknown>) {
  return Object.keys(parameters)
    .filter((key) => isSensitiveKey(key))
    .sort((left, right) => left.localeCompare(right));
}

export function listSensitiveMacroPayloadPaths(value: unknown, path = "macro"): string[] {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return [];
  }

  const paths: string[] = [];
  for (const [key, nestedValue] of Object.entries(value)) {
    const nextPath = `${path}.${key}`;
    if (isSensitiveKey(key)) {
      appendSensitivePath(paths, nextPath);
    }
    if (nestedValue && typeof nestedValue === "object" && !Array.isArray(nestedValue)) {
      listSensitiveMacroPayloadPaths(nestedValue, nextPath).forEach((entry) =>
        appendSensitivePath(paths, entry),
      );
    }
  }
  return paths.sort((left, right) => left.localeCompare(right));
}

export function assertSnippetPresetIsSafe(parameters: Record<string, unknown>) {
  const sensitiveKeys = listSensitiveSnippetParameterKeys(parameters);
  if (sensitiveKeys.length === 0) return;
  throw new Error(
    `${SENSITIVE_SNIPPET_ERROR_PREFIX} ${sensitiveKeys.join(", ")}.`,
  );
}

export function assertMacroPresetIsSafe(
  steps: Array<{ payload?: Record<string, unknown> }>,
) {
  const sensitivePaths = steps.flatMap((step, index) =>
    listSensitiveMacroPayloadPaths(step.payload, `macro.steps[${index}].payload`),
  );
  if (sensitivePaths.length === 0) return;
  throw new Error(
    `${SENSITIVE_MACRO_ERROR_PREFIX} ${sensitivePaths.join(", ")}.`,
  );
}

export function isSensitivePresetSaveError(error: unknown) {
  if (!(error instanceof Error)) return false;
  return (
    error.message.startsWith(SENSITIVE_SNIPPET_ERROR_PREFIX) ||
    error.message.startsWith(SENSITIVE_MACRO_ERROR_PREFIX)
  );
}

export function describeSensitivePresetSaveError(error: unknown) {
  if (!(error instanceof Error)) return null;
  if (error.message.startsWith(SENSITIVE_SNIPPET_ERROR_PREFIX)) {
    return {
      kind: "snippet" as const,
      details: error.message.slice(SENSITIVE_SNIPPET_ERROR_PREFIX.length).trim(),
    };
  }
  if (error.message.startsWith(SENSITIVE_MACRO_ERROR_PREFIX)) {
    return {
      kind: "macro" as const,
      details: error.message.slice(SENSITIVE_MACRO_ERROR_PREFIX.length).trim(),
    };
  }
  return null;
}
