"use client";

import type { WorkflowGraphNode, WorkflowOperatorDescriptor } from "@/lib/api";

const SUPPORTED_WORKFLOW_OPERATOR_IDS = new Set([
  "solve.bar_1d",
  "solve.thermal_bar_1d",
  "solve.heat_bar_1d",
  "solve.electrostatic_bar_1d",
  "solve.magnetostatic_bar_1d",
  "solve.electrostatic_plane_triangle_2d",
  "solve.electrostatic_plane_quad_2d",
  "solve.magnetostatic_plane_triangle_2d",
  "solve.magnetostatic_plane_quad_2d",
  "solve.acoustic_bar_1d",
  "solve.heat_plane_triangle_2d",
  "solve.heat_plane_quad_2d",
  "solve.stokes_flow_quad_2d",
  "solve.stokes_flow_plane_quad_2d",
  "solve.thermal_truss_2d",
  "solve.frame_3d",
  "solve.modal_frame_3d",
  "solve.plane_triangle_2d",
  "solve.thermal_plane_triangle_2d",
  "solve.plane_quad_2d",
  "solve.thermal_frame_3d",
  "solve.thermal_plane_quad_2d",
  "solve.thermal_truss_3d",
  "solve.torsion_1d",
  "solve.spring_1d",
  "solve.nonlinear_spring_1d",
  "solve.contact_gap_1d",
  "solve.spring_2d",
  "solve.spring_3d",
  "solve.truss_2d",
  "solve.truss_3d",
  "solve.frame_2d",
  "solve.modal_frame_2d",
  "solve.beam_1d",
  "solve.thermal_beam_1d",
  "solve.thermal_frame_2d",
  "bridge.temperature_field_to_thermo_quad_2d",
  "bridge.temperature_field_to_thermo_triangle_2d",
  "bridge.electrostatic_field_to_heat_quad_2d",
  "bridge.electrostatic_field_to_heat_triangle_2d",
  "bridge.magnetostatic_field_to_heat_quad_2d",
  "transform.first_available",
  "transform.merge_summary_pair",
  "transform.compare_summary_pair",
  "transform.aggregate_summary_collection",
  "transform.normalize_summary_fields",
  "transform.select_best_summary",
  "transform.evaluate_material_margins",
  "transform.rank_material_candidates",
  "transform.extract_material_pareto_frontier",
  "transform.validate_electrostatic_heat_bridge",
  "transform.validate_heat_thermo_bridge",
  "transform.compose_diagnostics_bundle",
  "transform.compose_diagnostics_report_payload",
  "transform.evaluate_diagnostics_bundle_guard",
  "transform.select_focus_payload",
  "transform.compose_focus_chain_input",
  "transform.compose_focus_bridge_request",
  "transform.resolve_focus_bridge_execution",
  "transform.execute_focus_bridge_execution",
  "transform.evaluate_thermal_guard",
  "transform.evaluate_electrostatic_guard",
  "transform.evaluate_magnetostatic_guard",
  "transform.evaluate_cfd_guard",
  "transform.benchmark_coupled_heat_pair",
  "transform.benchmark_electrostatic_pair",
  "transform.benchmark_magnetostatic_pair",
  "transform.benchmark_cfd_pair",
  "extract.result_summary",
  "extract.field_statistics",
  "extract.field_hotspots",
  "extract.electrostatic_result_diagnostics",
  "extract.electrostatic_peak_field",
  "extract.magnetostatic_result_diagnostics",
  "extract.magnetostatic_peak_field",
  "extract.stokes_flow_result_diagnostics",
  "extract.thermal_result_diagnostics",
  "extract.heat_peak_flux",
  "extract.thermo_result_diagnostics",
  "extract.thermo_peak_response",
  "extract.bridge_integrity_diagnostics",
  "export.summary_json",
  "export.summary_csv",
  "export.alert_markdown",
  "export.diagnostics_bundle_markdown",
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
