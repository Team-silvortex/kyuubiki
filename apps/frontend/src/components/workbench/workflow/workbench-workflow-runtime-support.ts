"use client";

import type { WorkflowGraphNode, WorkflowOperatorDescriptor } from "@/lib/api";

const SUPPORTED_WORKFLOW_OPERATOR_IDS = new Set([
  "solve.electrostatic_bar_1d",
  "solve.electrostatic_plane_triangle_2d",
  "solve.electrostatic_plane_quad_2d",
  "solve.heat_plane_quad_2d",
  "solve.frame_3d",
  "solve.thermal_frame_3d",
  "solve.thermal_plane_quad_2d",
  "solve.thermal_truss_3d",
  "solve.spring_1d",
  "solve.spring_2d",
  "solve.spring_3d",
  "solve.truss_2d",
  "solve.truss_3d",
  "solve.frame_2d",
  "solve.beam_1d",
  "solve.thermal_beam_1d",
  "solve.thermal_frame_2d",
  "bridge.temperature_field_to_thermo_quad_2d",
  "bridge.electrostatic_field_to_heat_quad_2d",
  "transform.first_available",
  "extract.result_summary",
  "export.summary_json",
  "export.summary_csv",
]);

const SUPPORTED_NODE_KINDS = new Set(["input", "solve", "transform", "extract", "export", "output", "condition"]);

export function isWorkflowOperatorSupportedInRuntime(operatorId?: string | null) {
  return Boolean(operatorId && SUPPORTED_WORKFLOW_OPERATOR_IDS.has(operatorId));
}

export function isWorkflowNodeSupportedInRuntime(node: WorkflowGraphNode) {
  if (!SUPPORTED_NODE_KINDS.has(node.kind)) return false;
  if (node.kind === "input" || node.kind === "output" || node.kind === "condition") return true;
  return isWorkflowOperatorSupportedInRuntime(node.operator_id);
}

export function isWorkflowDescriptorSupportedInRuntime(descriptor: WorkflowOperatorDescriptor) {
  return isWorkflowOperatorSupportedInRuntime(descriptor.id);
}
