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
const CLUSTER_ID = "integration-offline-mesh";
const AGENTS = [
  {
    id: "integration-mesh-a",
    port: 6511,
    fingerprint: "sha256:integration-mesh-a",
  },
  {
    id: "integration-mesh-b",
    port: 6512,
    fingerprint: "sha256:integration-mesh-b",
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
  const logPath = path.join(RUN_DIR, `integration-offline-mesh-agent-${agent.port}.log`);
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

function buildOfflineMeshHeatToThermoGraph(passThroughCount, thermoSeedModel) {
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
    id: `workflow.offline-mesh-heat-to-thermo-chain-${passThroughCount}`,
    name: "Offline mesh heat to thermo chain",
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
        required_capabilities: ["solver_rpc"],
        placement_tags: ["thermal", "mesh"],
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
        required_capabilities: ["solver_rpc"],
        placement_tags: ["thermal", "mesh"],
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

test("distributed orchestrator can run a complex coupled workflow through offline mesh agents", async () => {
  const agentProcesses = [];

  try {
    for (const agent of AGENTS) {
      const started = startRemoteAgent(agent);
      agentProcesses.push(started);
    }

    await Promise.all(agentProcesses.map((agent) => waitForPort(agent.port, 120_000)));
    runKyuubiki(["restart-distributed"]);

    const health = await waitFor(
      `${ORCHESTRATOR_URL}/api/health`,
      (payload) =>
        payload?.status === "ok" &&
        payload?.deployment?.mode === "distributed" &&
        payload?.deployment?.discovery === "registry" &&
        payload?.security?.api_token_configured === true &&
        payload?.security?.protect_reads === true,
      120_000,
      500,
      { headers: controlHeaders() },
    );

    assert.equal(health.status, "ok");
    assert.equal(health.deployment.mode, "distributed");
    assert.equal(health.deployment.discovery, "registry");

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

    assert.equal(agentsPayload.summary.control_modes.orch_managed, 0);
    assert.equal(agentsPayload.summary.control_modes.offline_mesh, 2);

    const heatModel = loadSampleModel("heat-plane-quad-2d.json");
    const thermoSeedModel = loadSampleModel("thermal-plane-quad-2d.json");
    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/workflows/graph/jobs`, {
      method: "POST",
      headers: controlHeaders(),
      body: JSON.stringify({
        graph: buildOfflineMeshHeatToThermoGraph(16, thermoSeedModel),
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

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      180_000,
      750,
      { headers: controlHeaders() },
    );

    const exported = finalPayload.result.artifacts["json_output.json"];
    const summary = JSON.parse(exported.content);

    assert.equal(finalPayload.job.status, "completed");
    assert.equal(finalPayload.result.workflow_id, "workflow.offline-mesh-heat-to-thermo-chain-16");
    assert.equal(finalPayload.result.completed_nodes.length, 23);
    assert.equal(finalPayload.result.performance.node_kind_breakdown.solve.count, 2);
    assert.equal(finalPayload.result.performance.node_kind_breakdown.transform.count, 17);
    assert.equal(finalPayload.result.performance.node_kind_breakdown.extract.count, 1);
    assert.equal(finalPayload.result.performance.node_kind_breakdown.export.count, 1);
    assert.equal(finalPayload.result.performance.node_kind_breakdown.output.count, 1);
    assert.equal(summary.max_temperature_delta, 100);
    assert.ok(Math.abs(summary.max_stress - 61293532.33830845) < 1.0e-6);
    assert.ok(Math.abs(summary.max_displacement - 0.0) < 1.0e-12);
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
