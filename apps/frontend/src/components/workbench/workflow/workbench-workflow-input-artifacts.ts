"use client";

import type { WorkflowCatalogEntryArtifact } from "@/lib/api";

export function buildWorkflowInputArtifactTexts(
  entryInputs: WorkflowCatalogEntryArtifact[],
  inputArtifacts: Record<string, unknown> | null | undefined,
): Record<string, string> {
  return Object.fromEntries(
    entryInputs.map((artifact) => [
      artifact.node_id,
      inputArtifacts && artifact.node_id in inputArtifacts
        ? `${JSON.stringify(inputArtifacts[artifact.node_id], null, 2)}\n`
        : "",
    ]),
  );
}

export function parseWorkflowInputArtifactTexts(
  inputTexts: Record<string, string>,
  entryInputs?: WorkflowCatalogEntryArtifact[],
): {
  inputArtifacts: Record<string, unknown>;
  invalidKeys: string[];
} {
  const parsedEntries: Array<[string, unknown]> = [];
  const invalidKeys: string[] = [];
  // The live contract, not stale editor state, defines which inputs are required.
  const keys = entryInputs
    ? [...new Set(entryInputs.map((artifact) => artifact.node_id))]
    : Object.keys(inputTexts);

  for (const key of keys) {
    const raw = Object.hasOwn(inputTexts, key) ? inputTexts[key] : "";
    const trimmed = typeof raw === "string" ? raw.trim() : "";
    if (!trimmed) {
      invalidKeys.push(key);
      continue;
    }
    try {
      parsedEntries.push([key, JSON.parse(trimmed) as unknown]);
    } catch {
      invalidKeys.push(key);
    }
  }

  return { inputArtifacts: Object.fromEntries(parsedEntries), invalidKeys };
}
