import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const ENTRYPOINT = `${ROOT}/scripts/kyuubiki-runtime.mjs`;
const ORCHESTRATOR_URL = "http://127.0.0.1:4000";

function runKyuubiki(...args) {
  return execFileSync("node", [ENTRYPOINT, ...args], {
    cwd: ROOT,
    stdio: "pipe",
    encoding: "utf8",
  });
}

function loadSampleModel(filename) {
  return JSON.parse(readFileSync(`${ROOT}/apps/frontend/public/models/${filename}`, "utf8"));
}

async function waitFor(url, predicate, timeoutMs = 30_000, intervalMs = 500) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        const payload = await response.json();
        if (predicate(payload)) {
          return payload;
        }
      }
    } catch {
      // wait for service boot
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${url}`);
}

function buildHeatToThermoChainGraph(passThroughCount, thermoSeedModel) {
  const passNodes = Array.from({ length: passThroughCount }, (_, index) => ({
    id: `pass_${String(index).padStart(3, "0")}`,
    kind: "transform",
    operator_id: "transform.first_available",
    inputs: [{ id: "input", artifact_type: "result/heat_plane_quad_2d" }],
    outputs: [{ id: "result", artifact_type: "result/heat_plane_quad_2d" }],
  }));

  const passEdges = Array.from({ length: passThroughCount }, (_, index) => {
    const nodeId = `pass_${String(index).padStart(3, "0")}`;
    const fromNode = index === 0 ? "solve_heat" : `pass_${String(index - 1).padStart(3, "0")}`;

    return {
      id: `ep_${index}`,
      from: { node: fromNode, port: "result" },
      to: { node: nodeId, port: "input" },
      artifact_type: "result/heat_plane_quad_2d",
    };
  });

  return {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: `workflow.heat-to-thermo-quad-chain-${passThroughCount}`,
    name: "Heat to thermo quad chain",
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
      ...passNodes,
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
      ...passEdges,
      {
        id: "e_bridge",
        from: { node: `pass_${String(passThroughCount - 1).padStart(3, "0")}`, port: "result" },
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
  };
}

test("local workstation stack can run a 48-hop coupled workflow graph end-to-end", async () => {
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
        graph: buildHeatToThermoChainGraph(48, thermoSeedModel),
        input_artifacts: {
          heat_model: heatModel,
        },
      }),
    });

    assert.equal(submitResponse.status, 200);
    const payload = await submitResponse.json();
    const summary = JSON.parse(payload.artifacts["json_output.json"].content);

    assert.equal(payload.workflow_id, "workflow.heat-to-thermo-quad-chain-48");
    assert.equal(payload.completed_nodes.length, 55);
    assert.equal(payload.performance.loop_passes, 1);
    assert.equal(payload.performance.node_kind_breakdown.solve.count, 2);
    assert.equal(payload.performance.node_kind_breakdown.transform.count, 49);
    assert.equal(payload.performance.node_kind_breakdown.extract.count, 1);
    assert.equal(payload.performance.node_kind_breakdown.export.count, 1);
    assert.equal(payload.performance.node_kind_breakdown.output.count, 1);
    assert.equal(summary.max_temperature_delta, 100);
    assert.ok(Math.abs(summary.max_stress - 61293532.33830845) < 1.0e-6);
    assert.ok(Math.abs(summary.max_displacement - 0.0) < 1.0e-12);
    assert.ok(payload.performance.total_elapsed_ms >= 0.0);
    assert.ok(Array.isArray(payload.performance.slowest_nodes));
    assert.ok(payload.performance.slowest_nodes.length > 0);
    assert.equal(payload.performance.slowest_nodes[0].node_id, "solve_heat");
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 180_000 });
