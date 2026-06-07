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

export function parseWorkflowInputArtifactTexts(inputTexts: Record<string, string>): {
  inputArtifacts: Record<string, unknown>;
  invalidKeys: string[];
} {
  const inputArtifacts: Record<string, unknown> = {};
  const invalidKeys: string[] = [];

  for (const [key, raw] of Object.entries(inputTexts)) {
    const trimmed = raw.trim();
    if (!trimmed) {
      invalidKeys.push(key);
      continue;
    }
    try {
      inputArtifacts[key] = JSON.parse(trimmed) as unknown;
    } catch {
      invalidKeys.push(key);
    }
  }

  return { inputArtifacts, invalidKeys };
}
