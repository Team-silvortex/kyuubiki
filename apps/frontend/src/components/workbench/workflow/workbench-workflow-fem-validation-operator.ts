"use client";

import type { WorkflowFemValidationIssue } from "@/components/workbench/workflow/workbench-workflow-fem-validation-types";
import { createWorkflowFemValidationIssue } from "@/components/workbench/workflow/workbench-workflow-fem-validation-types";

export function validateWorkflowFemOperatorShape(
  artifactType: string,
  payload: unknown,
): WorkflowFemValidationIssue[] {
  if (typeof payload !== "object" || payload === null) return [];

  const record = payload as Record<string, unknown>;
  switch (artifactType) {
    case "study_model/electrostatic_plane_triangle_2d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j", "node_k"], 3),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k"]),
        ...validateElectrostaticBoundaryContrast(record),
      ];
    case "study_model/heat_plane_triangle_2d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j", "node_k"], 3),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k"]),
        ...validateHeatBoundaryContrast(record),
      ];
    case "study_model/plane_triangle_2d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j", "node_k"], 3),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k"]),
        ...validatePlanarConstraintCoverage(record),
      ];
    case "study_model/thermal_plane_triangle_2d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j", "node_k"], 3),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k"]),
        ...validatePlanarConstraintCoverage(record),
      ];
    case "study_model/electrostatic_plane_quad_2d":
      return [
        ...validateIndexedElementShape(
          record,
          ["node_i", "node_j", "node_k", "node_l"],
          4,
        ),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k", "node_l"]),
        ...validateElectrostaticBoundaryContrast(record),
      ];
    case "study_model/heat_plane_quad_2d":
      return [
        ...validateIndexedElementShape(
          record,
          ["node_i", "node_j", "node_k", "node_l"],
          4,
        ),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k", "node_l"]),
        ...validateHeatBoundaryContrast(record),
      ];
    case "study_model/plane_quad_2d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j", "node_k", "node_l"], 4),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k", "node_l"]),
        ...validatePlanarConstraintCoverage(record),
      ];
    case "study_model/thermal_plane_quad_2d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j", "node_k", "node_l"], 4),
        ...validatePlanarElementGeometry(record, ["node_i", "node_j", "node_k", "node_l"]),
        ...validatePlanarConstraintCoverage(record),
      ];
    case "study_model/heat_bar_1d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j"], 2),
        ...validateBarElementGeometry(record),
        ...validateHeatBoundaryContrast(record),
      ];
    case "study_model/thermal_bar_1d":
      return [
        ...validateIndexedElementShape(record, ["node_i", "node_j"], 2),
        ...validateBarElementGeometry(record),
        ...validateAxialConstraintCoverage(record),
      ];
    default:
      return [];
  }
}

function validateIndexedElementShape(
  payload: Record<string, unknown>,
  nodeFields: string[],
  minimumNodeCount: number,
): WorkflowFemValidationIssue[] {
  const issues: WorkflowFemValidationIssue[] = [];
  const nodes = readCollection(payload.nodes);
  const elements = readCollection(payload.elements);

  if (nodes.length < minimumNodeCount) {
    issues.push(
      createWorkflowFemValidationIssue(
        "contract",
        nodeFields[0] ?? "node_i",
        `This operator shape expects at least ${minimumNodeCount} nodes in the model payload.`,
        "control",
      ),
    );
  }

  elements.forEach((element, index) => {
    const indices = nodeFields.map((field) => element[field]);
    const validIndices = indices.filter(
      (value): value is number => typeof value === "number" && Number.isInteger(value),
    );

    if (validIndices.length !== nodeFields.length) {
      issues.push(
        createWorkflowFemValidationIssue(
          "contract",
          nodeFields[0] ?? "node_i",
          `Element ${index + 1} is missing one or more integer node references required by this operator shape.`,
          "control",
        ),
      );
      return;
    }

    const uniqueIndices = new Set(validIndices);
    if (uniqueIndices.size !== nodeFields.length) {
      issues.push(
        createWorkflowFemValidationIssue(
          "contract",
          nodeFields[0] ?? "node_i",
          `Element ${index + 1} reuses the same node more than once, which degenerates the operator shape.`,
          "control",
        ),
      );
    }

    const hasOutOfRange = validIndices.some(
      (value) => value < 0 || value >= nodes.length,
    );
    if (hasOutOfRange) {
      issues.push(
        createWorkflowFemValidationIssue(
          "contract",
          nodeFields[0] ?? "node_i",
          `Element ${index + 1} references node indices outside the available node list.`,
          "control",
        ),
      );
    }
  });

  return issues;
}

function validatePlanarElementGeometry(
  payload: Record<string, unknown>,
  nodeFields: string[],
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  const elements = readCollection(payload.elements);
  const issues: WorkflowFemValidationIssue[] = [];

  elements.forEach((element, index) => {
    const points = nodeFields
      .map((field) => readNodePoint(nodes, element[field]))
      .filter((point): point is { x: number; y: number } => Boolean(point));
    if (points.length !== nodeFields.length) return;

    const area = Math.abs(computePolygonArea(points));
    if (area > 1e-12) return;
    issues.push(
      createWorkflowFemValidationIssue(
        "physics",
        nodeFields[0] ?? "node_i",
        `Element ${index + 1} collapses to near-zero planar area, so this operator shape is geometrically degenerate.`,
        "control",
      ),
    );
  });

  return issues;
}

function validateBarElementGeometry(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  const elements = readCollection(payload.elements);
  const issues: WorkflowFemValidationIssue[] = [];

  elements.forEach((element, index) => {
    const from = readNodeCoordinate(nodes, element.node_i);
    const to = readNodeCoordinate(nodes, element.node_j);
    if (!from || !to) return;
    if (Math.abs(to - from) > 1e-12) return;
    issues.push(
      createWorkflowFemValidationIssue(
        "physics",
        "node_i",
        `Element ${index + 1} has zero axial span, so the 1D operator geometry is degenerate.`,
        "control",
      ),
    );
  });

  return issues;
}

function validateElectrostaticBoundaryContrast(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  const anchoredPotentials = nodes
    .filter((node) => node.fix_potential === true && typeof node.potential === "number")
    .map((node) => node.potential as number);
  const hasChargeDrive = nodes.some((node) => isNonZeroNumber(node.charge_density));

  if (anchoredPotentials.length < 2 || hasChargeDrive) return [];
  if (new Set(anchoredPotentials).size > 1) return [];
  return [
    createWorkflowFemValidationIssue(
      "physics",
      "potential",
      "Electrostatic operator inputs use anchored potentials with no contrast and no charge drive, so the field response may collapse to a trivial state.",
      "boundary",
    ),
  ];
}

function validateHeatBoundaryContrast(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  const anchoredTemperatures = nodes
    .filter(
      (node) => node.fix_temperature === true && typeof node.temperature === "number",
    )
    .map((node) => node.temperature as number);
  const hasHeatDrive = nodes.some((node) => isNonZeroNumber(node.heat_load));

  if (anchoredTemperatures.length < 2 || hasHeatDrive) return [];
  if (new Set(anchoredTemperatures).size > 1) return [];
  return [
    createWorkflowFemValidationIssue(
      "physics",
      "temperature",
      "Heat operator inputs use fixed temperatures with no contrast and no heat source, so the temperature field may remain trivial.",
      "boundary",
    ),
  ];
}

function validateAxialConstraintCoverage(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  if (nodes.length === 0) return [];
  if (nodes.some((node) => node.fix_x === true)) return [];
  return [
    createWorkflowFemValidationIssue(
      "physics",
      "fix_x",
      "Axial operator inputs should usually anchor at least one x displacement to avoid rigid-body drift.",
      "boundary",
    ),
  ];
}

function validatePlanarConstraintCoverage(
  payload: Record<string, unknown>,
): WorkflowFemValidationIssue[] {
  const nodes = readCollection(payload.nodes);
  if (nodes.length === 0) return [];

  const hasXConstraint = nodes.some((node) => node.fix_x === true);
  const hasYConstraint = nodes.some((node) => node.fix_y === true);
  const issues: WorkflowFemValidationIssue[] = [];

  if (!hasXConstraint) {
    issues.push(
      createWorkflowFemValidationIssue(
        "physics",
        "fix_x",
        "Planar operator inputs should usually constrain at least one x displacement to avoid rigid-body drift.",
        "boundary",
      ),
    );
  }

  if (!hasYConstraint) {
    issues.push(
      createWorkflowFemValidationIssue(
        "physics",
        "fix_y",
        "Planar operator inputs should usually constrain at least one y displacement to avoid rigid-body drift.",
        "boundary",
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

function readNodePoint(
  nodes: Array<Record<string, unknown>>,
  indexValue: unknown,
): { x: number; y: number } | null {
  if (typeof indexValue !== "number" || !Number.isInteger(indexValue)) return null;
  const node = nodes[indexValue];
  if (!node) return null;
  if (typeof node.x !== "number" || typeof node.y !== "number") return null;
  return { x: node.x, y: node.y };
}

function readNodeCoordinate(
  nodes: Array<Record<string, unknown>>,
  indexValue: unknown,
): number | null {
  if (typeof indexValue !== "number" || !Number.isInteger(indexValue)) return null;
  const node = nodes[indexValue];
  if (!node || typeof node.x !== "number") return null;
  return node.x;
}

function computePolygonArea(points: Array<{ x: number; y: number }>) {
  let twiceArea = 0;
  for (let index = 0; index < points.length; index += 1) {
    const current = points[index];
    const next = points[(index + 1) % points.length];
    twiceArea += current.x * next.y - next.x * current.y;
  }
  return twiceArea / 2;
}

function isNonZeroNumber(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) && value !== 0;
}
