import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const ENTRYPOINT = `${ROOT}/scripts/kyuubiki`;
const FRONTEND_URL = "http://127.0.0.1:3000";
const DIRECT_MESH_TOKEN = "integration-direct-mesh-token";
const DIRECT_MESH_ENDPOINTS = ["127.0.0.1:5001", "127.0.0.1:5002"];

const DIRECT_MESH_ENV = {
  KYUUBIKI_DIRECT_MESH_TOKEN: DIRECT_MESH_TOKEN,
  KYUUBIKI_DIRECT_MESH_ENDPOINTS: DIRECT_MESH_ENDPOINTS.join(","),
};

function runKyuubiki(args, extraEnv = {}) {
  return execFileSync("zsh", [ENTRYPOINT, ...args], {
    cwd: ROOT,
    stdio: "pipe",
    encoding: "utf8",
    env: {
      ...process.env,
      ...DIRECT_MESH_ENV,
      ...extraEnv,
    },
  });
}

async function waitFor(url, predicate, init = {}, timeoutMs = 30_000, intervalMs = 500) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url, init);
      const contentType = response.headers.get("content-type") ?? "";
      const payload = contentType.includes("application/json")
        ? await response.json()
        : await response.text();

      if (predicate(response, payload)) {
        return { response, payload };
      }
    } catch {
      // wait for frontend boot
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${url}`);
}

function directMeshHeaders() {
  return {
    "content-type": "application/json",
    "x-kyuubiki-token": DIRECT_MESH_TOKEN,
  };
}

function loadSampleModel(filename) {
  return JSON.parse(readFileSync(`${ROOT}/apps/frontend/public/models/${filename}`, "utf8"));
}

test("direct mesh gui can inspect LAN agents, solve directly, and fetch result chunks", async () => {
  try {
    runKyuubiki(["restart-local"]);

    const agentsReady = await waitFor(
      `${FRONTEND_URL}/api/direct-mesh/agents`,
      (response, payload) =>
        response.status === 200 &&
        payload?.mode === "direct_mesh_gui" &&
        payload?.endpoint_count >= 1 &&
        Array.isArray(payload?.agents) &&
        payload.agents.length >= 1,
      {
        method: "POST",
        headers: directMeshHeaders(),
        body: JSON.stringify({
          endpoints: DIRECT_MESH_ENDPOINTS,
        }),
      },
      60_000,
    );

    assert.equal(agentsReady.response.status, 200);
    assert.equal(agentsReady.payload.mode, "direct_mesh_gui");
    assert.ok(agentsReady.payload.agents.some((agent) => !agent.descriptor_error));

    const solveResponse = await fetch(`${FRONTEND_URL}/api/direct-mesh/solve`, {
      method: "POST",
      headers: directMeshHeaders(),
      body: JSON.stringify({
        endpoints: DIRECT_MESH_ENDPOINTS,
        selection_mode: "healthiest",
        study_kind: "axial_bar_1d",
        input: {
          length: 20.0,
          area: 0.01,
          youngs_modulus_gpa: 70,
          elements: 60,
          tip_force: 1800,
        },
      }),
    });

    assert.equal(solveResponse.status, 200);
    const solved = await solveResponse.json();
    assert.equal(solved.job.status, "completed");
    assert.match(solved.job.worker_id, /^direct-mesh@/);
    assert.ok(Array.isArray(solved.result.nodes));
    assert.equal(solved.result.nodes.length, 61);
    assert.ok(Array.isArray(solved.result.elements));
    assert.equal(solved.result.elements.length, 60);
    assert.ok(solved.direct_mesh.endpoint);

    const jobId = solved.job.job_id;
    assert.ok(jobId);

    const nodesChunkResponse = await fetch(
      `${FRONTEND_URL}/api/direct-mesh/results/${jobId}/chunks/nodes?offset=20&limit=10`,
      {
        headers: {
          "x-kyuubiki-token": DIRECT_MESH_TOKEN,
        },
      },
    );

    assert.equal(nodesChunkResponse.status, 200);
    const nodesChunk = await nodesChunkResponse.json();
    assert.equal(nodesChunk.kind, "nodes");
    assert.equal(nodesChunk.offset, 20);
    assert.equal(nodesChunk.limit, 10);
    assert.equal(nodesChunk.returned, 10);
    assert.equal(nodesChunk.total, 61);
    assert.equal(nodesChunk.items.length, 10);

    const elementsChunkResponse = await fetch(
      `${FRONTEND_URL}/api/direct-mesh/results/${jobId}/chunks/elements?offset=15&limit=8`,
      {
        headers: {
          "x-kyuubiki-token": DIRECT_MESH_TOKEN,
        },
      },
    );

    assert.equal(elementsChunkResponse.status, 200);
    const elementsChunk = await elementsChunkResponse.json();
    assert.equal(elementsChunk.kind, "elements");
    assert.equal(elementsChunk.offset, 15);
    assert.equal(elementsChunk.limit, 8);
    assert.equal(elementsChunk.returned, 8);
    assert.equal(elementsChunk.total, 60);
    assert.equal(elementsChunk.items.length, 8);
  } finally {
    try {
      runKyuubiki(["stop"]);
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("direct mesh gui can solve official spring samples for 2d and 3d studies", async () => {
  try {
    runKyuubiki(["restart-local"]);

    const agentsReady = await waitFor(
      `${FRONTEND_URL}/api/direct-mesh/agents`,
      (response, payload) =>
        response.status === 200 &&
        payload?.mode === "direct_mesh_gui" &&
        payload?.endpoint_count >= 1 &&
        Array.isArray(payload?.agents) &&
        payload.agents.length >= 1,
      {
        method: "POST",
        headers: directMeshHeaders(),
        body: JSON.stringify({
          endpoints: DIRECT_MESH_ENDPOINTS,
        }),
      },
      60_000,
    );

    assert.equal(agentsReady.response.status, 200);
    assert.ok(agentsReady.payload.agents.some((agent) => !agent.descriptor_error));

    const spring2dResponse = await fetch(`${FRONTEND_URL}/api/direct-mesh/solve`, {
      method: "POST",
      headers: directMeshHeaders(),
      body: JSON.stringify({
        endpoints: DIRECT_MESH_ENDPOINTS,
        selection_mode: "healthiest",
        study_kind: "spring_2d",
        input: loadSampleModel("spring-grid-2d.json"),
      }),
    });

    assert.equal(spring2dResponse.status, 200);
    const spring2dSolved = await spring2dResponse.json();
    assert.equal(spring2dSolved.job.status, "completed");
    assert.equal(spring2dSolved.job.worker_id, "direct-mesh@127.0.0.1:5001");
    assert.equal(spring2dSolved.result.nodes.length, 4);
    assert.equal(spring2dSolved.result.elements.length, 5);
    assert.ok(Math.abs(spring2dSolved.result.max_displacement - 0.06339734949589224) < 1e-12);
    assert.ok(Math.abs(spring2dSolved.result.max_force - 1120.754716981132) < 1e-9);

    const spring3dResponse = await fetch(`${FRONTEND_URL}/api/direct-mesh/solve`, {
      method: "POST",
      headers: directMeshHeaders(),
      body: JSON.stringify({
        endpoints: DIRECT_MESH_ENDPOINTS,
        selection_mode: "healthiest",
        study_kind: "spring_3d",
        input: loadSampleModel("spring-cage-3d.json"),
      }),
    });

    assert.equal(spring3dResponse.status, 200);
    const spring3dSolved = await spring3dResponse.json();
    assert.equal(spring3dSolved.job.status, "completed");
    assert.equal(spring3dSolved.job.worker_id, "direct-mesh@127.0.0.1:5001");
    assert.equal(spring3dSolved.result.nodes.length, 4);
    assert.equal(spring3dSolved.result.elements.length, 6);
    assert.ok(Math.abs(spring3dSolved.result.max_displacement - 0.05955868626521211) < 1e-12);
    assert.ok(Math.abs(spring3dSolved.result.max_force - 803.0108273796119) < 1e-9);
  } finally {
    try {
      runKyuubiki(["stop"]);
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });
