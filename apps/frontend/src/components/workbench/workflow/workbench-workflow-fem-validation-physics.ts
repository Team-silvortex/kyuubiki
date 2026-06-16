"use client";

import type { WorkflowFemValidationIssue } from "@/components/workbench/workflow/workbench-workflow-fem-validation-types";
import { createWorkflowFemValidationIssue } from "@/components/workbench/workflow/workbench-workflow-fem-validation-types";

export function validateWorkflowFemPhysicsSignals(
  artifactType: string,
  payload: unknown,
): WorkflowFemValidationIssue[] {
  if (typeof payload !== "object" || payload === null) return [];

  if (artifactType.startsWith("study_model/electrostatic_")) {
    return validateElectrostaticSignals(payload as Record<string, unknown>);
  }
  if (artifactType.startsWith("study_model/heat_")) {
    return validateHeatSignals(payload as Record<string, unknown>);
  }
  if (artifactType.startsWith("study_model/thermal_")) {
    return validateThermoMechanicalSignals(payload as Record<string, unknown>);
  }
  if (
    artifactType === "study_model/bar_1d" ||
    artifactType.startsWith("study_model/plane_")
  ) {
    return validateMechanicalSignals(payload as Record<string, unknown>);
  }
  return [];
}

function validateElectrostaticSignals(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  if (nodes.length === 0) return [];

  const hasAnchoredPotential = nodes.some((node) => node.fix_potential === true);
  if (hasAnchoredPotential) return [];
  return [
    createWorkflowFemValidationIssue(
      "physics",
      "fix_potential",
      "Electrostatic studies should usually anchor at least one fixed-potential boundary.",
      "boundary",
    ),
  ];
}

function validateHeatSignals(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  if (nodes.length === 0) return [];

  const hasFixedTemperature = nodes.some((node) => node.fix_temperature === true);
  if (hasFixedTemperature) return [];
  return [
    createWorkflowFemValidationIssue(
      "physics",
      "fix_temperature",
      "Heat studies should usually anchor at least one fixed-temperature boundary.",
      "boundary",
    ),
  ];
}

function validateMechanicalSignals(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const issues: WorkflowFemValidationIssue[] = [];
  const nodes = readCollection(payload.nodes);

  if (nodes.length === 0) {
    const hasTipForce = isNonZeroNumber(payload.tip_force);
    if (!hasTipForce) {
      issues.push(
        createWorkflowFemValidationIssue(
          "physics",
          "tip_force",
          "Mechanical studies usually need a non-zero load or excitation to produce a meaningful response.",
          "load",
        ),
      );
    }
    return issues;
  }

  const hasSupport = nodes.some(
    (node) => node.fix_x === true || node.fix_y === true,
  );
  if (!hasSupport) {
    issues.push(
      createWorkflowFemValidationIssue(
        "physics",
        "fix_x",
        "Mechanical studies should usually constrain at least one displacement boundary.",
        "boundary",
      ),
    );
  }

  const hasLoad = nodes.some(
    (node) => isNonZeroNumber(node.load_x) || isNonZeroNumber(node.load_y),
  );
  if (!hasLoad) {
    issues.push(
      createWorkflowFemValidationIssue(
        "physics",
        "load_x",
        "Mechanical studies usually need a non-zero load or excitation to produce a meaningful response.",
        "load",
      ),
    );
  }

  return issues;
}

function validateThermoMechanicalSignals(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const issues = validateMechanicalSignals(payload);
  const nodes = readCollection(payload.nodes);
  const elements = readCollection(payload.elements);

  const hasTemperatureDelta = nodes.some((node) =>
    isNonZeroNumber(node.temperature_delta),
  );
  const hasThermalExpansion = elements.some((element) =>
    isNonZeroNumber(element.thermal_expansion),
  );

  if (hasTemperatureDelta && !hasThermalExpansion) {
    issues.push(
      createWorkflowFemValidationIssue(
        "physics",
        "thermal_expansion",
        "Temperature-driven thermo-mechanical studies should usually include non-zero thermal expansion.",
        "material",
      ),
    );
  }

  return issues;
}

function readCollection(value: unknown): Array<Record<string, unknown>> {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (entry): entry is Record<string, unknown> =>
      typeof entry === "object" && entry !== null,
  );
}

function isNonZeroNumber(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) && value !== 0;
}
