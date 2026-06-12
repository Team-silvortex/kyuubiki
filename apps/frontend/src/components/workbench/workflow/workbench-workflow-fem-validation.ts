"use client";

import type { WorkflowCatalogEntryArtifact } from "@/lib/api";
import type { WorkflowFemInputSection } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";
import { resolveWorkflowFemInputProfile } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";

export type WorkflowFemValidationIssue = {
  category: "physics" | "contract";
  field: string;
  message: string;
  severity: "warning";
  sectionKey: WorkflowFemInputSection["key"];
};

export function validateWorkflowFemInputPayload(
  artifactType: string,
  payload: unknown,
): WorkflowFemValidationIssue[] {
  const profile = resolveWorkflowFemInputProfile(artifactType);
  if (!profile || typeof payload !== "object" || payload === null) return [];

  const issues: WorkflowFemValidationIssue[] = [];
  for (const section of profile.sections) {
    if (section.target === "root") {
      validateRecord(section, payload as Record<string, unknown>, issues);
      continue;
    }

    const collection = (payload as Record<string, unknown>)[section.target];
    if (!Array.isArray(collection)) continue;
    collection.forEach((entry) => {
      if (typeof entry !== "object" || entry === null) return;
      validateRecord(section, entry as Record<string, unknown>, issues);
    });
  }
  return dedupeIssues(issues);
}

export function collectWorkflowInputArtifactContractWarnings(params: {
  entryInputs: WorkflowCatalogEntryArtifact[];
  inputArtifactTexts?: Record<string, string>;
}) {
  const warnings: Record<string, string[]> = {};

  for (const artifact of params.entryInputs) {
    const raw = params.inputArtifactTexts?.[artifact.node_id];
    if (!raw?.trim()) {
      warnings[artifact.node_id] = [`${artifact.artifact_type}: missing input artifact payload.`];
      continue;
    }

    try {
      const payload = JSON.parse(raw) as unknown;
      const issues = validateWorkflowFemInputPayload(artifact.artifact_type, payload)
        .filter((entry) => entry.category === "contract")
        .map((entry) => `${entry.sectionKey}.${entry.field}: ${entry.message}`);
      if (issues.length > 0) warnings[artifact.node_id] = issues;
    } catch {
      warnings[artifact.node_id] = [`${artifact.artifact_type}: input artifact payload is not valid JSON.`];
    }
  }

  return warnings;
}

export function summarizeWorkflowInputArtifactContractHealth(params: {
  entryInputs: WorkflowCatalogEntryArtifact[];
  inputArtifactTexts?: Record<string, string>;
}) {
  const warningCount = Object.values(collectWorkflowInputArtifactContractWarnings(params)).reduce(
    (total, lines) => total + lines.length,
    0,
  );
  if (warningCount === 0) return { level: "clean", warningCount, tags: ["contract_health:clean"] };
  if (warningCount <= 3) return { level: "manageable", warningCount, tags: ["contract_health:manageable"] };
  return { level: "review", warningCount, tags: ["contract_health:review"] };
}

function validateRecord(
  section: WorkflowFemInputSection,
  record: Record<string, unknown>,
  issues: WorkflowFemValidationIssue[],
) {
  for (const field of section.fields) {
    const value = record[field];
    if (value === undefined || value === null) continue;
    const numericIssue = validateNumericField(section.key, field, value);
    if (numericIssue) issues.push(numericIssue);
  }

  if (section.key === "boundary") {
    if (record.fix_potential === true && typeof record.potential !== "number") {
      issues.push(issue("contract", "potential", "Fixed electric boundaries should include a numeric potential value.", section.key));
    }
    if (record.fix_temperature === true && typeof record.temperature !== "number") {
      issues.push(issue("contract", "temperature", "Fixed thermal boundaries should include a numeric temperature value.", section.key));
    }
    if (record.fix_x === true && typeof record.load_x === "number" && record.load_x !== 0) {
      issues.push(issue("contract", "load_x", "A fixed x boundary still carries a non-zero x load. Confirm this overlap is intentional.", section.key));
    }
    if (record.fix_y === true && typeof record.load_y === "number" && record.load_y !== 0) {
      issues.push(issue("contract", "load_y", "A fixed y boundary still carries a non-zero y load. Confirm this overlap is intentional.", section.key));
    }
  }
}

function validateNumericField(
  sectionKey: WorkflowFemInputSection["key"],
  field: string,
  value: unknown,
) {
  if (typeof value !== "number" || Number.isNaN(value)) return issue("contract", field, "Expected a numeric value.", sectionKey);
  if (field === "poisson_ratio" && (value <= -1 || value >= 0.5)) {
    return issue("physics", field, "Poisson ratio is usually expected between -1 and 0.5.", sectionKey);
  }
  if (field === "thermal_expansion" && value < 0) {
    return issue("physics", field, "Thermal expansion is usually non-negative for this workflow.", sectionKey);
  }
  if (field === "elements" && (!Number.isInteger(value) || value < 1)) {
    return issue("contract", field, "Element count should be an integer greater than or equal to 1.", sectionKey);
  }
  if (
    ["youngs_modulus", "conductivity", "thickness", "area", "length", "permittivity"].includes(field) &&
    value <= 0
  ) {
    return issue("physics", field, "This field is expected to be greater than 0.", sectionKey);
  }
  return null;
}

function issue(
  category: WorkflowFemValidationIssue["category"],
  field: string,
  message: string,
  sectionKey: WorkflowFemInputSection["key"],
): WorkflowFemValidationIssue {
  return { category, field, message, severity: "warning", sectionKey };
}

function dedupeIssues(issues: WorkflowFemValidationIssue[]) {
  const seen = new Set<string>();
  return issues.filter((entry) => {
    const key = `${entry.category}:${entry.sectionKey}:${entry.field}:${entry.message}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
