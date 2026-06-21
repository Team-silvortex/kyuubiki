import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync, openSync, readFileSync } from "node:fs";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const ENTRYPOINT = `${ROOT}/scripts/kyuubiki-runtime.mjs`;
const RUST_DIR = path.join(ROOT, "workers/rust");
const RUN_DIR = path.join(ROOT, "tmp/run");
const AGENT_BIN = path.join(RUST_DIR, "target/debug/kyuubiki-cli");
const ORCHESTRATOR_URL = "http://127.0.0.1:4000";
const CONTROL_TOKEN = "integration-control-token";
const CLUSTER_TOKEN = "integration-cluster-token";
const CLUSTER_ID = "integration-offline-mesh-diagnostics";
const AGENTS = [
  {
    id: "integration-mesh-diag-a",
    port: 6521,
    fingerprint: "sha256:integration-mesh-diag-a",
  },
  {
    id: "integration-mesh-diag-b",
    port: 6522,
    fingerprint: "sha256:integration-mesh-diag-b",
  },
];

const SECURITY_ENV = {
  KYUUBIKI_API_TOKEN: CONTROL_TOKEN,
  KYUUBIKI_CLUSTER_API_TOKEN: CLUSTER_TOKEN,
  KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT: "true",
  KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS: AGENTS.map((agent) => agent.id).join(","),
  KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS: CLUSTER_ID,
  KYUUBIKI_AGENT_DISCOVERY: "registry",
  KYUUBIKI_AGENT_ENDPOINTS: "",
};

function runKyuubiki(args, extraEnv = {}) {
  return execFileSync("node", [ENTRYPOINT, ...args], {
    cwd: ROOT,
    stdio: "pipe",
    encoding: "utf8",
    env: {
      ...process.env,
      ...SECURITY_ENV,
      ...extraEnv,
    },
  });
}

function loadSampleModel(filename) {
  return JSON.parse(readFileSync(`${ROOT}/apps/frontend/public/models/${filename}`, "utf8"));
}

function controlHeaders(extra = {}) {
  return {
    "content-type": "application/json",
    "x-kyuubiki-token": CONTROL_TOKEN,
    ...extra,
  };
}

function clusterHeaders(agent, extra = {}) {
  const nonce = `nonce-${agent.id}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
  return {
    "content-type": "application/json",
    "x-kyuubiki-token": CLUSTER_TOKEN,
    "x-kyuubiki-agent-id": agent.id,
    "x-kyuubiki-cluster-id": CLUSTER_ID,
    "x-kyuubiki-agent-fingerprint": agent.fingerprint,
    "x-kyuubiki-cluster-ts": `${Date.now()}`,
    "x-kyuubiki-cluster-nonce": nonce,
    ...extra,
  };
}

async function waitFor(url, predicate, timeoutMs = 30_000, intervalMs = 500, options = {}) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url, options);
      if (response.ok) {
        const payload = await response.json();
        if (predicate(payload)) {
          return payload;
        }
      }
    } catch {
      // wait for boot
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${url}`);
}

async function waitForPort(port, timeoutMs = 30_000, intervalMs = 200) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    if (await isPortListening(port)) {
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for tcp://127.0.0.1:${port}`);
}

async function isPortListening(port) {
  return await new Promise((resolve) => {
    const socket = net.createConnection({ host: "127.0.0.1", port });
    socket.once("connect", () => {
      socket.destroy();
      resolve(true);
    });
    socket.once("error", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

function startRemoteAgent(agent) {
  ensureAgentBinary();
  mkdirSync(RUN_DIR, { recursive: true });
  const logPath = path.join(RUN_DIR, `integration-offline-mesh-diag-agent-${agent.port}.log`);
  const output = openSync(logPath, "a");
  const child = spawn(
    AGENT_BIN,
    ["agent", "--port", String(agent.port)],
    {
      cwd: RUST_DIR,
      env: process.env,
      detached: true,
      stdio: ["ignore", output, output],
      windowsHide: true,
    },
  );

  child.unref();
  return { ...agent, pid: child.pid, logPath };
}

let agentBinaryPrepared = false;

function ensureAgentBinary() {
  if (agentBinaryPrepared) {
    return;
  }

  execFileSync("cargo", ["build", "-p", "kyuubiki-cli", "--bin", "kyuubiki-cli"], {
    cwd: RUST_DIR,
    stdio: "pipe",
    encoding: "utf8",
    env: process.env,
  });

  agentBinaryPrepared = true;
}

function stopRemoteAgent(agentProcess) {
  if (!agentProcess?.pid) {
    return;
  }

  try {
    process.kill(-agentProcess.pid, "SIGTERM");
  } catch {
    try {
      process.kill(agentProcess.pid, "SIGTERM");
    } catch {
      // best effort
    }
  }
}

function buildBranchingDiagnosticsGraph(thermoSeedModel) {
  return {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.offline-mesh-coupled-diagnostics-branching-markdown",
    name: "Offline mesh coupled diagnostics branching markdown",
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
        required_capabilities: ["solver_rpc"],
        placement_tags: ["thermal", "mesh"],
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
        required_capabilities: ["solver_rpc"],
        placement_tags: ["thermal", "mesh"],
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
        config: { title: "Offline Mesh Coupled Workflow Continue Report" },
        inputs: [{ id: "bundle", artifact_type: "artifact/json" }],
        outputs: [{ id: "markdown", artifact_type: "export/markdown" }],
      },
      {
        id: "export_blocked",
        kind: "export",
        operator_id: "export.alert_markdown",
        config: {
          title: "Offline Mesh Coupled Workflow Blocked",
          summary: "Guard blocked the offline mesh coupled workflow and routed execution into the alert branch.",
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

function getIn(value, pathParts) {
  return pathParts.reduce((current, part) => current?.[part], value);
}

test("distributed orchestrator can route a coupled diagnostics workflow through offline mesh blocked markdown branch", async () => {
  const agentProcesses = [];

  try {
    for (const agent of AGENTS) {
      agentProcesses.push(startRemoteAgent(agent));
    }

    await Promise.all(agentProcesses.map((agent) => waitForPort(agent.port, 120_000)));
    runKyuubiki(["restart-distributed"]);

    const health = await waitFor(
      `${ORCHESTRATOR_URL}/api/health`,
      (payload) =>
        payload?.status === "ok" &&
        payload?.deployment?.mode === "distributed" &&
        payload?.deployment?.discovery === "registry",
      120_000,
      500,
      { headers: controlHeaders() },
    );

    assert.equal(health.status, "ok");

    for (const agent of AGENTS) {
      const registerResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/agents/register`, {
        method: "POST",
        headers: clusterHeaders(agent),
        body: JSON.stringify({
          id: agent.id,
          host: "127.0.0.1",
          port: agent.port,
          role: "solver",
          cluster_id: CLUSTER_ID,
          fingerprint: agent.fingerprint,
          control_mode: "offline_mesh",
          capacity: 8,
          tags: ["solver", "thermal", "mesh"],
          methods: ["solve_heat_plane_quad_2d", "solve_thermal_plane_quad_2d"],
          capabilities: [
            { id: "solver_rpc", role: "solver" },
            { id: "thermal_mesh", role: "solver" },
          ],
        }),
      });

      assert.equal(registerResponse.status, 201);
    }

    const agentsPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/agents`,
      (payload) => payload?.summary?.total_agents === 2 && payload?.summary?.active_agents === 2,
      60_000,
      500,
      { headers: controlHeaders() },
    );

    assert.equal(agentsPayload.summary.control_modes.offline_mesh, 2);

    const heatModel = loadSampleModel("heat-plane-quad-2d.json");
    const thermoSeedModel = loadSampleModel("thermal-plane-quad-2d.json");
    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/workflows/graph/jobs`, {
      method: "POST",
      headers: controlHeaders(),
      body: JSON.stringify({
        graph: buildBranchingDiagnosticsGraph(thermoSeedModel),
        input_artifacts: {
          heat_model: heatModel,
        },
        control_mode: "offline_mesh",
        cluster_id: CLUSTER_ID,
      }),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a workflow graph job id");

    const payload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (result) => result?.job?.status === "completed",
      180_000,
      750,
      { headers: controlHeaders() },
    );

    const exported = payload.result.artifacts["markdown_output.markdown"];

    assert.equal(payload.result.workflow_id, "workflow.offline-mesh-coupled-diagnostics-branching-markdown");
    assert.deepEqual(payload.result.branch_decisions, [
      {
        node_id: "route",
        chosen_output: "if_false",
        predicate_result: false,
      },
    ]);
    assert.ok(payload.result.completed_nodes.includes("export_blocked"));
    assert.ok(payload.result.completed_nodes.includes("merge_markdown"));
    assert.ok(payload.result.skipped_nodes.includes("export_continue"));
    assert.equal(payload.result.performance.node_kind_breakdown.solve.count, 2);
    assert.equal(payload.result.performance.node_kind_breakdown.extract.count, 2);
    assert.equal(payload.result.performance.node_kind_breakdown.transform.count, 5);
    assert.equal(payload.result.performance.node_kind_breakdown.condition.count, 1);
    assert.equal(payload.result.performance.node_kind_breakdown.export.count, 2);
    assert.equal(exported.format, "markdown");
    assert.ok(exported.content.includes("# Offline Mesh Coupled Workflow Blocked"));
    assert.ok(exported.content.includes("Severity: block"));
    assert.ok(exported.content.includes("report_guard_status: block"));
    assert.ok(exported.content.includes("hold_and_review"));
    assert.equal(getIn(payload.result, ["artifacts", "guard.result", "guard_status"]), "block");
    assert.equal(getIn(payload.result, ["artifacts", "report.result", "report_guard_status"]), "block");
  } finally {
    try {
      runKyuubiki(["stop"]);
    } catch {
      // best effort
    }

    for (const agent of agentProcesses.reverse()) {
      stopRemoteAgent(agent);
    }
  }
}, { timeout: 240_000 });
