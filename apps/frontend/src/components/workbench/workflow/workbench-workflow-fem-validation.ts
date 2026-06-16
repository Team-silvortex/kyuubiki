"use client";

import type { WorkflowCatalogEntryArtifact } from "@/lib/api";
import type { WorkflowFemInputSection } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";
import { resolveWorkflowFemInputProfile } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";
import { validateWorkflowFemOperatorShape } from "@/components/workbench/workflow/workbench-workflow-fem-validation-operator";
import { validateWorkflowFemPhysicsSignals } from "@/components/workbench/workflow/workbench-workflow-fem-validation-physics";
import {
  createWorkflowFemValidationIssue,
  dedupeWorkflowFemValidationIssues,
  type WorkflowFemValidationIssue,
} from "@/components/workbench/workflow/workbench-workflow-fem-validation-types";

export type { WorkflowFemValidationIssue } from "@/components/workbench/workflow/workbench-workflow-fem-validation-types";

export function validateWorkflowFemInputPayload(
  artifactType: string,
  payload: unknown,
): WorkflowFemValidationIssue[] {
  const profile = resolveWorkflowFemInputProfile(artifactType);
  if (!profile || typeof payload !== "object" || payload === null) return [];

  const issues = validateWorkflowFemPhysicsSignals(
    artifactType,
    payload,
  ) as WorkflowFemValidationIssue[];
  issues.push(...validateWorkflowFemOperatorShape(artifactType, payload));
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
  return dedupeWorkflowFemValidationIssues(issues);
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
    if (BOOLEAN_FIELDS.has(field)) {
      if (typeof value !== "boolean") {
        issues.push(
          createWorkflowFemValidationIssue(
            "contract",
            field,
            "Expected a boolean value.",
            section.key,
          ),
        );
      }
      continue;
    }
    const numericIssue = validateNumericField(section.key, field, value);
    if (numericIssue) issues.push(numericIssue);
  }

  if (section.key === "boundary") {
    if (record.fix_potential === true && typeof record.potential !== "number") {
      issues.push(createWorkflowFemValidationIssue("contract", "potential", "Fixed electric boundaries should include a numeric potential value.", section.key));
    }
    if (record.fix_temperature === true && typeof record.temperature !== "number") {
      issues.push(createWorkflowFemValidationIssue("contract", "temperature", "Fixed thermal boundaries should include a numeric temperature value.", section.key));
    }
    if (record.fix_x === true && typeof record.load_x === "number" && record.load_x !== 0) {
      issues.push(createWorkflowFemValidationIssue("contract", "load_x", "A fixed x boundary still carries a non-zero x load. Confirm this overlap is intentional.", section.key));
    }
    if (record.fix_y === true && typeof record.load_y === "number" && record.load_y !== 0) {
      issues.push(createWorkflowFemValidationIssue("contract", "load_y", "A fixed y boundary still carries a non-zero y load. Confirm this overlap is intentional.", section.key));
    }
  }
}

const BOOLEAN_FIELDS = new Set([
  "fix_potential",
  "fix_temperature",
  "fix_x",
  "fix_y",
]);

function validateNumericField(
  sectionKey: WorkflowFemInputSection["key"],
  field: string,
  value: unknown,
) {
  if (typeof value !== "number" || Number.isNaN(value)) return createWorkflowFemValidationIssue("contract", field, "Expected a numeric value.", sectionKey);
  if (field === "poisson_ratio" && (value <= -1 || value >= 0.5)) {
    return createWorkflowFemValidationIssue("physics", field, "Poisson ratio is usually expected between -1 and 0.5.", sectionKey);
  }
  if (field === "thermal_expansion" && value < 0) {
    return createWorkflowFemValidationIssue("physics", field, "Thermal expansion is usually non-negative for this workflow.", sectionKey);
  }
  if (field === "elements" && (!Number.isInteger(value) || value < 1)) {
    return createWorkflowFemValidationIssue("contract", field, "Element count should be an integer greater than or equal to 1.", sectionKey);
  }
  if (
    ["youngs_modulus", "conductivity", "thickness", "area", "length", "permittivity"].includes(field) &&
    value <= 0
  ) {
    return createWorkflowFemValidationIssue("physics", field, "This field is expected to be greater than 0.", sectionKey);
  }
  return null;
}
