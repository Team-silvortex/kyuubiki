import test from "node:test";
import assert from "node:assert/strict";
import { ORCHESTRATOR_URL, loadSampleModel, runKyuubiki, waitFor } from "./support.mjs";
test("local workstation stack can run a workflow graph end-to-end", async () => {
  try {
    runKyuubiki("restart-local");

    const health = await waitFor(
      `${ORCHESTRATOR_URL}/api/health`,
      (payload) =>
        payload?.status === "ok" &&
        Array.isArray(payload?.solver_agents) &&
        payload.solver_agents.length >= 1,
      120_000,
    );

    assert.equal(health.status, "ok");

    const heatModel = loadSampleModel("heat-plane-quad-2d.json");
    const thermoSeedModel = loadSampleModel("thermal-plane-quad-2d.json");

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/workflows/graph/run`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        graph: {
          schema_version: "kyuubiki.workflow-graph/v1",
          id: "workflow.heat-to-thermo-quad-2d",
          name: "Heat to thermo quad",
          version: "1.0.0",
          entry_nodes: ["heat_model"],
          output_nodes: ["json_output"],
          defaults: {
            cache_policy: "cached",
            orchestrated: true,
          },
          nodes: [
            {
              id: "heat_model",
              kind: "input",
              outputs: [{ id: "model", artifact_type: "study_model/heat_plane_quad_2d" }],
            },
            {
              id: "solve_heat",
              kind: "solve",
              operator_id: "solve.heat_plane_quad_2d",
              inputs: [{ id: "model", artifact_type: "study_model/heat_plane_quad_2d" }],
              outputs: [{ id: "result", artifact_type: "result/heat_plane_quad_2d" }],
            },
            {
              id: "bridge_temperature",
              kind: "transform",
              operator_id: "bridge.temperature_field_to_thermo_quad_2d",
              config: thermoSeedModel,
              inputs: [{ id: "heat_result", artifact_type: "result/heat_plane_quad_2d" }],
              outputs: [{ id: "thermo_model", artifact_type: "study_model/thermal_plane_quad_2d" }],
            },
            {
              id: "solve_thermo",
              kind: "solve",
              operator_id: "solve.thermal_plane_quad_2d",
              inputs: [{ id: "model", artifact_type: "study_model/thermal_plane_quad_2d" }],
              outputs: [{ id: "result", artifact_type: "result/thermal_plane_quad_2d" }],
            },
            {
              id: "extract_summary",
              kind: "extract",
              operator_id: "extract.result_summary",
              inputs: [{ id: "result", artifact_type: "result/thermal_plane_quad_2d" }],
              outputs: [{ id: "summary", artifact_type: "report/summary" }],
            },
            {
              id: "export_json",
              kind: "export",
              operator_id: "export.summary_json",
              inputs: [{ id: "summary", artifact_type: "report/summary" }],
              outputs: [{ id: "json", artifact_type: "export/json" }],
            },
            {
              id: "json_output",
              kind: "output",
              inputs: [{ id: "json", artifact_type: "export/json" }],
              outputs: [],
            },
          ],
          edges: [
            {
              id: "e0",
              from: { node: "heat_model", port: "model" },
              to: { node: "solve_heat", port: "model" },
              artifact_type: "study_model/heat_plane_quad_2d",
            },
            {
              id: "e1",
              from: { node: "solve_heat", port: "result" },
              to: { node: "bridge_temperature", port: "heat_result" },
              artifact_type: "result/heat_plane_quad_2d",
            },
            {
              id: "e2",
              from: { node: "bridge_temperature", port: "thermo_model" },
              to: { node: "solve_thermo", port: "model" },
              artifact_type: "study_model/thermal_plane_quad_2d",
            },
            {
              id: "e3",
              from: { node: "solve_thermo", port: "result" },
              to: { node: "extract_summary", port: "result" },
              artifact_type: "result/thermal_plane_quad_2d",
            },
            {
              id: "e4",
              from: { node: "extract_summary", port: "summary" },
              to: { node: "export_json", port: "summary" },
              artifact_type: "report/summary",
            },
            {
              id: "e5",
              from: { node: "export_json", port: "json" },
              to: { node: "json_output", port: "json" },
              artifact_type: "export/json",
            },
          ],
        },
        input_artifacts: {
          heat_model: heatModel,
        },
      }),
    });

    assert.equal(submitResponse.status, 200);
    const payload = await submitResponse.json();
    assert.equal(payload.workflow_id, "workflow.heat-to-thermo-quad-2d");
    assert.equal(payload.completed_nodes.length, 7);
    const exportArtifact = payload.artifacts["json_output.json"];
    assert.equal(exportArtifact.format, "json");
    const summary = JSON.parse(exportArtifact.content);
    assert.equal(summary.max_temperature_delta, 100);
    assert.ok(Math.abs(summary.max_stress - 61293532.33830845) < 1.0e-6);
    assert.ok(Math.abs(summary.max_displacement - 0.0) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 180_000 });
