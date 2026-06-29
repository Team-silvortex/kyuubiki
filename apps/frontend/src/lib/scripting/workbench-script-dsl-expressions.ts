"use client";

import { serializeWorkbenchPythonLiteral } from "./workbench-script-python-format.ts";

export type WorkbenchFrontendDslVarReference = { $var: string };

export const VARIABLE_IDENTIFIER_RE = /^[a-zA-Z_][a-zA-Z0-9_]*$/;
const VARIABLE_TEMPLATE_RE = /\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}/g;

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

export function isVariableReference(value: unknown): value is WorkbenchFrontendDslVarReference {
  return (
    isPlainObject(value) &&
    Object.keys(value).length === 1 &&
    typeof value.$var === "string" &&
    VARIABLE_IDENTIFIER_RE.test(value.$var)
  );
}

function buildPythonInterpolatedStringExpression(value: string) {
  const matches = [...value.matchAll(VARIABLE_TEMPLATE_RE)];
  if (matches.length === 0) {
    return JSON.stringify(value);
  }

  let cursor = 0;
  const parts: string[] = [];
  for (const match of matches) {
    const [fullMatch, varName] = match;
    const startIndex = match.index ?? 0;
    if (startIndex > cursor) {
      parts.push(JSON.stringify(value.slice(cursor, startIndex)));
    }
    parts.push(`str(${varName})`);
    cursor = startIndex + fullMatch.length;
  }
  if (cursor < value.length) {
    parts.push(JSON.stringify(value.slice(cursor)));
  }
  return parts.join(" + ");
}

export function buildPythonExpression(value: unknown): string {
  if (isVariableReference(value)) {
    return value.$var;
  }

  if (typeof value === "string") {
    return buildPythonInterpolatedStringExpression(value);
  }

  if (Array.isArray(value)) {
    return `[${value.map((entry) => buildPythonExpression(entry)).join(", ")}]`;
  }

  if (isPlainObject(value)) {
    return `{${Object.entries(value)
      .map(([key, entry]) => `${JSON.stringify(key)}: ${buildPythonExpression(entry)}`)
      .join(", ")}}`;
  }

  return serializeWorkbenchPythonLiteral(value);
}
