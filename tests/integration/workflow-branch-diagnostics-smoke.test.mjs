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

function buildBranchingDiagnosticsGraph(thermoSeedModel) {
  return {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.coupled-diagnostics-branching-markdown",
    name: "Coupled diagnostics branching markdown",
    version: "1.0.0",
    entry_nodes: ["heat_model"],
    output_nodes: ["markdown_output"],
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
        id: "extract_thermal_diagnostics",
        kind: "extract",
        operator_id: "extract.thermal_result_diagnostics",
        inputs: [{ id: "result", artifact_type: "result/heat_plane_quad_2d" }],
        outputs: [{ id: "summary", artifact_type: "artifact/json" }],
      },
      {
        id: "extract_thermo_diagnostics",
        kind: "extract",
        operator_id: "extract.thermo_result_diagnostics",
        inputs: [{ id: "result", artifact_type: "result/thermal_plane_quad_2d" }],
        outputs: [{ id: "summary", artifact_type: "artifact/json" }],
      },
      {
        id: "bundle",
        kind: "transform",
        operator_id: "transform.compose_diagnostics_bundle",
        config: {},
        inputs: [
          { id: "thermal", artifact_type: "artifact/json" },
          { id: "thermo", artifact_type: "artifact/json" },
        ],
        outputs: [{ id: "result", artifact_type: "artifact/json" }],
      },
      {
        id: "guard",
        kind: "transform",
        operator_id: "transform.evaluate_diagnostics_bundle_guard",
        config: {
          rules: [
            {
              source: "thermal",
              field: "thermal_temperature_max",
              comparison: "gt",
              threshold: 90.0,
              severity: "warn",
              label: "thermal temperature",
            },
            {
              source: "thermo",
              field: "thermo_stress_peak",
              comparison: "gt",
              threshold: 1000000.0,
              severity: "block",
              label: "thermo stress peak",
            },
          ],
        },
        inputs: [{ id: "bundle", artifact_type: "artifact/json" }],
        outputs: [{ id: "result", artifact_type: "artifact/json" }],
      },
      {
        id: "report",
        kind: "transform",
        operator_id: "transform.compose_diagnostics_report_payload",
        config: {},
        inputs: [
          { id: "bundle", artifact_type: "artifact/json" },
          { id: "guard", artifact_type: "artifact/json" },
        ],
        outputs: [{ id: "result", artifact_type: "artifact/json" }],
      },
      {
        id: "route",
        kind: "condition",
        config: {
          predicate: {
            path: "report_guard_status",
            operator: "eq",
            value: "pass",
          },
        },
        inputs: [{ id: "value", artifact_type: "artifact/json" }],
        outputs: [
          { id: "if_true", artifact_type: "artifact/json" },
          { id: "if_false", artifact_type: "artifact/json" },
        ],
      },
      {
        id: "export_continue",
        kind: "export",
        operator_id: "export.diagnostics_bundle_markdown",
        config: { title: "Coupled Workflow Continue Report" },
        inputs: [{ id: "bundle", artifact_type: "artifact/json" }],
        outputs: [{ id: "markdown", artifact_type: "export/markdown" }],
      },
      {
        id: "export_blocked",
        kind: "export",
        operator_id: "export.alert_markdown",
        config: {
          title: "Coupled Workflow Blocked",
          summary: "Guard blocked the coupled workflow and routed execution into the alert branch.",
          severity_path: "report_guard_status",
          fields: ["report_guard_status", "report_guard_recommendation"],
        },
        inputs: [{ id: "alert", artifact_type: "artifact/json" }],
        outputs: [{ id: "markdown", artifact_type: "export/markdown" }],
      },
      {
        id: "merge_markdown",
        kind: "transform",
        operator_id: "transform.first_available",
        inputs: [
          { id: "left", artifact_type: "export/markdown" },
          { id: "right", artifact_type: "export/markdown" },
        ],
        outputs: [{ id: "markdown", artifact_type: "export/markdown" }],
      },
      {
        id: "markdown_output",
        kind: "output",
        inputs: [{ id: "markdown", artifact_type: "export/markdown" }],
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
        from: { node: "solve_heat", port: "result" },
        to: { node: "extract_thermal_diagnostics", port: "result" },
        artifact_type: "result/heat_plane_quad_2d",
      },
      {
        id: "e4",
        from: { node: "solve_thermo", port: "result" },
        to: { node: "extract_thermo_diagnostics", port: "result" },
        artifact_type: "result/thermal_plane_quad_2d",
      },
      {
        id: "e5",
        from: { node: "extract_thermal_diagnostics", port: "summary" },
        to: { node: "bundle", port: "thermal" },
        artifact_type: "artifact/json",
      },
      {
        id: "e6",
        from: { node: "extract_thermo_diagnostics", port: "summary" },
        to: { node: "bundle", port: "thermo" },
        artifact_type: "artifact/json",
      },
      {
        id: "e7",
        from: { node: "bundle", port: "result" },
        to: { node: "guard", port: "bundle" },
        artifact_type: "artifact/json",
      },
      {
        id: "e8",
        from: { node: "bundle", port: "result" },
        to: { node: "report", port: "bundle" },
        artifact_type: "artifact/json",
      },
      {
        id: "e9",
        from: { node: "guard", port: "result" },
        to: { node: "report", port: "guard" },
        artifact_type: "artifact/json",
      },
      {
        id: "e10",
        from: { node: "report", port: "result" },
        to: { node: "route", port: "value" },
        artifact_type: "artifact/json",
      },
      {
        id: "e11",
        from: { node: "route", port: "if_true" },
        to: { node: "export_continue", port: "bundle" },
        artifact_type: "artifact/json",
      },
      {
        id: "e12",
        from: { node: "route", port: "if_false" },
        to: { node: "export_blocked", port: "alert" },
        artifact_type: "artifact/json",
      },
      {
        id: "e13",
        from: { node: "export_continue", port: "markdown" },
        to: { node: "merge_markdown", port: "left" },
        artifact_type: "export/markdown",
      },
      {
        id: "e14",
        from: { node: "export_blocked", port: "markdown" },
        to: { node: "merge_markdown", port: "right" },
        artifact_type: "export/markdown",
      },
      {
        id: "e15",
        from: { node: "merge_markdown", port: "markdown" },
        to: { node: "markdown_output", port: "markdown" },
        artifact_type: "export/markdown",
      },
    ],
  };
}

test("local workstation stack can route a coupled diagnostics workflow through the blocked markdown branch", async () => {
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
        graph: buildBranchingDiagnosticsGraph(thermoSeedModel),
        input_artifacts: {
          heat_model: heatModel,
        },
      }),
    });

    assert.equal(submitResponse.status, 200);
    const payload = await submitResponse.json();
    const exported = payload.artifacts["markdown_output.markdown"];

    assert.equal(payload.workflow_id, "workflow.coupled-diagnostics-branching-markdown");
    assert.deepEqual(payload.branch_decisions, [
      {
        node_id: "route",
        chosen_output: "if_false",
        predicate_result: false,
      },
    ]);
    assert.ok(payload.completed_nodes.includes("export_blocked"));
    assert.ok(payload.completed_nodes.includes("merge_markdown"));
    assert.ok(payload.skipped_nodes.includes("export_continue"));
    assert.equal(payload.performance.node_kind_breakdown.solve.count, 2);
    assert.equal(payload.performance.node_kind_breakdown.extract.count, 2);
    assert.equal(payload.performance.node_kind_breakdown.transform.count, 5);
    assert.equal(payload.performance.node_kind_breakdown.condition.count, 1);
    assert.equal(payload.performance.node_kind_breakdown.export.count, 2);
    assert.equal(exported.format, "markdown");
    assert.ok(exported.content.includes("# Coupled Workflow Blocked"));
    assert.ok(exported.content.includes("Severity: block"));
    assert.ok(exported.content.includes("report_guard_status: block"));
    assert.ok(exported.content.includes("hold_and_review"));
    assert.equal(getIn(payload, ["artifacts", "guard.result", "guard_status"]), "block");
    assert.equal(getIn(payload, ["artifacts", "report.result", "report_guard_status"]), "block");
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 180_000 });

function getIn(value, pathParts) {
  return pathParts.reduce((current, part) => current?.[part], value);
}
