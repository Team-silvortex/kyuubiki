"use client";

const PYTHON_INDENT = "  ";

function buildIndent(depth: number) {
  return PYTHON_INDENT.repeat(depth);
}

export function serializeWorkbenchPythonLiteral(value: unknown, depth = 0): string {
  if (value === null || value === undefined) return "None";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : "None";
  if (typeof value === "string") return JSON.stringify(value);

  if (Array.isArray(value)) {
    if (value.length === 0) return "[]";
    const nextDepth = depth + 1;
    return `[\n${value.map((entry) => `${buildIndent(nextDepth)}${serializeWorkbenchPythonLiteral(entry, nextDepth)}`).join(",\n")}\n${buildIndent(depth)}]`;
  }

  if (typeof value === "object") {
    const entries = Object.entries(value);
    if (entries.length === 0) return "{}";
    const nextDepth = depth + 1;
    return `{\n${entries
      .map(([key, entryValue]) => `${buildIndent(nextDepth)}${JSON.stringify(key)}: ${serializeWorkbenchPythonLiteral(entryValue, nextDepth)}`)
      .join(",\n")}\n${buildIndent(depth)}}`;
  }

  return JSON.stringify(String(value));
}
