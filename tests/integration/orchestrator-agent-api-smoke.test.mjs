import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

const ROOT = "/Users/Shared/chroot/dev/kyuubiki";
const ENTRYPOINT = `${ROOT}/scripts/kyuubiki`;
const ORCHESTRATOR_URL = "http://127.0.0.1:4000";

function runKyuubiki(...args) {
  return execFileSync("zsh", [ENTRYPOINT, ...args], {
    cwd: ROOT,
    stdio: "pipe",
    encoding: "utf8",
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
      // wait for service boot
    }

    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`timed out waiting for ${url}`);
}

test("local workstation stack can solve an axial-bar job end-to-end", async () => {
  try {
    runKyuubiki("restart-local");

    const health = await waitFor(
      `${ORCHESTRATOR_URL}/api/health`,
      (payload) =>
        payload?.status === "ok" &&
        Array.isArray(payload?.solver_agents) &&
        payload.solver_agents.length >= 1,
      60_000,
    );

    assert.equal(health.status, "ok");

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/axial-bar/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        length: 1.0,
        area: 0.01,
        youngs_modulus_gpa: 210,
        elements: 8,
        tip_force: 1000,
      }),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(finalPayload.result.max_displacement > 0);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.ok(Array.isArray(finalPayload.result.elements));
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });
