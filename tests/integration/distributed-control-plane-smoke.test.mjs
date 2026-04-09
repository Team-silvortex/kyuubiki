import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

const ROOT = "/Users/Shared/chroot/dev/kyuubiki";
const ENTRYPOINT = `${ROOT}/scripts/kyuubiki`;
const ORCHESTRATOR_URL = "http://127.0.0.1:4000";
const CONTROL_TOKEN = "integration-control-token";
const CLUSTER_TOKEN = "integration-cluster-token";
const AGENT_ID = "integration-remote-a";
const CLUSTER_ID = "integration-lan";
const FINGERPRINT = "sha256:integration-remote-a";

const SECURITY_ENV = {
  KYUUBIKI_API_TOKEN: CONTROL_TOKEN,
  KYUUBIKI_CLUSTER_API_TOKEN: CLUSTER_TOKEN,
  KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT: "true",
  KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS: AGENT_ID,
  KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS: CLUSTER_ID,
};

function runKyuubiki(args, extraEnv = {}) {
  return execFileSync("zsh", [ENTRYPOINT, ...args], {
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
      // wait for boot
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${url}`);
}

function clusterHeaders(extra = {}) {
  return {
    "content-type": "application/json",
    "x-kyuubiki-token": CLUSTER_TOKEN,
    "x-kyuubiki-agent-id": AGENT_ID,
    "x-kyuubiki-cluster-id": CLUSTER_ID,
    "x-kyuubiki-agent-fingerprint": FINGERPRINT,
    "x-kyuubiki-cluster-ts": `${Date.now()}`,
    ...extra,
  };
}

test("local control plane accepts protected cluster register, heartbeat, and unregister flows", async () => {
  try {
    runKyuubiki(["restart-local"]);

    const health = await waitFor(
      `${ORCHESTRATOR_URL}/api/health`,
      (payload) =>
        payload?.status === "ok" &&
        payload?.security?.cluster_fingerprint_required === true &&
        payload?.security?.cluster_agent_allowlist_count === 1,
      60_000,
    );

    assert.equal(health.status, "ok");
    assert.equal(health.security.cluster_token_configured, true);
    assert.equal(health.security.cluster_cluster_allowlist_count, 1);

    const unauthorized = await fetch(`${ORCHESTRATOR_URL}/api/v1/agents/register`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        id: AGENT_ID,
        host: "127.0.0.1",
        port: 6501,
        role: "solver",
        cluster_id: CLUSTER_ID,
        fingerprint: FINGERPRINT,
        tags: ["solver", "mesh"],
      }),
    });

    assert.equal(unauthorized.status, 401);

    const registerResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/agents/register`, {
      method: "POST",
      headers: clusterHeaders(),
      body: JSON.stringify({
        id: AGENT_ID,
        host: "127.0.0.1",
        port: 6501,
        role: "solver",
        cluster_id: CLUSTER_ID,
        fingerprint: FINGERPRINT,
        tags: ["solver", "mesh"],
        capacity: 4,
      }),
    });

    assert.equal(registerResponse.status, 201);
    const registered = await registerResponse.json();
    assert.equal(registered.agent.id, AGENT_ID);
    assert.equal(registered.agent.cluster_id, CLUSTER_ID);
    assert.equal(registered.agent.fingerprint, FINGERPRINT);

    const heartbeatResponse = await fetch(
      `${ORCHESTRATOR_URL}/api/v1/agents/${AGENT_ID}/heartbeat`,
      {
        method: "POST",
        headers: clusterHeaders(),
        body: JSON.stringify({
          host: "127.0.0.1",
          port: 6501,
          role: "solver",
          cluster_id: CLUSTER_ID,
          fingerprint: FINGERPRINT,
          tags: ["solver", "mesh", "heartbeat"],
          capacity: 6,
        }),
      },
    );

    assert.equal(heartbeatResponse.status, 200);
    const heartbeat = await heartbeatResponse.json();
    assert.equal(heartbeat.agent.capacity, 6);
    assert.deepEqual(heartbeat.agent.tags, ["solver", "mesh", "heartbeat"]);

    const agentsResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/agents`);
    assert.equal(agentsResponse.status, 200);
    const agentsPayload = await agentsResponse.json();
    assert.equal(agentsPayload.summary.total_agents, 1);
    assert.equal(agentsPayload.summary.active_agents, 1);
    assert.equal(agentsPayload.agents[0].id, AGENT_ID);

    const removeResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/agents/${AGENT_ID}`, {
      method: "DELETE",
      headers: clusterHeaders(),
    });

    assert.equal(removeResponse.status, 200);
    const removed = await removeResponse.json();
    assert.equal(removed.status, "removed");

    const afterRemove = await fetch(`${ORCHESTRATOR_URL}/api/v1/agents`);
    assert.equal(afterRemove.status, 200);
    const afterRemovePayload = await afterRemove.json();
    assert.equal(afterRemovePayload.summary.total_agents, 0);
    assert.deepEqual(afterRemovePayload.agents, []);
  } finally {
    try {
      runKyuubiki(["stop"]);
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });
