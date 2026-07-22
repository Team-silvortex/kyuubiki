import test from "node:test";
import assert from "node:assert/strict";

import { validateRuntimeSupport } from "../../src/components/workbench/workflow/workbench-workflow-validation-runtime.ts";
import { isWorkflowOperatorSupportedInRuntime } from "../../src/components/workbench/workflow/workbench-workflow-runtime-support.ts";

test("runtime support includes advanced and magnetostatic solver operators", () => {
  const supported = [
    "solve.magnetostatic_bar_1d",
    "solve.magnetostatic_plane_triangle_2d",
    "solve.magnetostatic_plane_quad_2d",
    "solve.acoustic_bar_1d",
    "solve.stokes_flow_quad_2d",
    "solve.stokes_flow_triangle_2d",
    "solve.stokes_flow_plane_quad_2d",
    "solve.stokes_flow_plane_triangle_2d",
    "solve.nonlinear_spring_1d",
    "solve.contact_gap_1d",
    "solve.modal_frame_2d",
    "solve.buckling_beam_1d",
    "solve.buckling_frame_2d",
    "solve.frame_2d_p_delta",
    "solve.modal_frame_3d",
  ];

  for (const operatorId of supported) {
    assert.equal(isWorkflowOperatorSupportedInRuntime(operatorId), true, operatorId);
  }
});

test("runtime support includes shipped template chain operators", () => {
  const supported = [
    "bridge.magnetostatic_field_to_heat_quad_2d",
    "transform.validate_electrostatic_heat_bridge",
    "transform.validate_heat_thermo_bridge",
    "transform.compose_diagnostics_bundle",
    "transform.evaluate_diagnostics_bundle_guard",
    "transform.compose_diagnostics_report_payload",
    "transform.select_focus_payload",
    "transform.execute_focus_bridge_execution",
    "transform.evaluate_magnetostatic_guard",
    "transform.evaluate_cfd_guard",
    "transform.evaluate_material_margins",
    "transform.rank_material_candidates",
    "transform.extract_material_pareto_frontier",
    "transform.benchmark_magnetostatic_pair",
    "transform.benchmark_cfd_pair",
    "extract.electrostatic_result_diagnostics",
    "extract.magnetostatic_result_diagnostics",
    "extract.stokes_flow_result_diagnostics",
    "extract.thermal_result_diagnostics",
    "extract.thermo_result_diagnostics",
    "extract.bridge_integrity_diagnostics",
    "export.diagnostics_bundle_markdown",
  ];

  for (const operatorId of supported) {
    assert.equal(isWorkflowOperatorSupportedInRuntime(operatorId), true, operatorId);
  }
});

test("validateRuntimeSupport flags unsupported operator ids", () => {
  const graph = {
    nodes: [
      {
        id: "transform_1",
        kind: "transform",
        operator_id: "transform.not_supported_yet",
      },
    ],
  };
  const issues = validateRuntimeSupport(graph as never);

  assert.equal(issues.length, 1);
  assert.equal(issues[0]?.id, "runtime:unsupported:transform_1");
});
