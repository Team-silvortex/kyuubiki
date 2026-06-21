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

test("local workstation stack can solve an axial-bar job end-to-end", async () => {
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
}, { timeout: 180_000 });

test("local workstation stack can solve a thermal-bar-1d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-bar-1d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-bar-1d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal bar job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_stress - 100800000.0) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.max_axial_force - 1008000.0) < 1.0e-6);
    assert.equal(finalPayload.result.max_temperature_delta, 40);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 2);
    assert.ok(Math.abs(finalPayload.result.nodes[0].ux - 0.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.nodes[1].ux - 0.0) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
    assert.ok(Math.abs(finalPayload.result.elements[0].stress + 100800000.0) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[0].axial_force + 1008000.0) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[0].average_temperature_delta - 40.0) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a heat-bar-1d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/heat-bar-1d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("heat-bar-1d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a heat bar job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_temperature - 100.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_heat_flux - 1800.0) < 1.0e-9);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 3);
    assert.ok(Math.abs(finalPayload.result.nodes[1].temperature - 60.0) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 2);
    assert.ok(Math.abs(finalPayload.result.elements[0].temperature_gradient + 40.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[1].temperature_gradient + 40.0) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a thermal-beam-1d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-beam-1d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-beam-1d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal beam job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.005184000000000001) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.max_rotation - 0.004320000000000001) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.max_moment - 7.275957614183426e-12) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.max_stress - 6.614506921984932e-9) < 1.0e-15);
    assert.equal(finalPayload.result.max_temperature_gradient, 45);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 2);
    assert.ok(Math.abs(finalPayload.result.nodes[1].uy - 0.005184000000000001) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.nodes[1].rz - 0.004320000000000001) < 1.0e-15);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
    assert.ok(Math.abs(finalPayload.result.elements[0].temperature_gradient_y - 45.0) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a heat-plane-quad-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/heat-plane-quad-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("heat-plane-quad-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a heat plane quad job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_temperature - 100.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_heat_flux - 2846.0498941515416) < 1.0e-9);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[1].temperature - 60.0) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
    assert.ok(Math.abs(finalPayload.result.elements[0].temperature_gradient_x + 20.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[0].temperature_gradient_y + 60.0) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a heat-plane-triangle-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/heat-plane-triangle-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("heat-plane-triangle-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a heat plane triangle job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_temperature - 100.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_heat_flux - 3600.0) < 1.0e-9);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[1].temperature - 60.0) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 2);
    assert.ok(Math.abs(finalPayload.result.elements[0].temperature_gradient_x + 40.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[0].temperature_gradient_y + 40.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[1].temperature_gradient_x - 0.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[1].temperature_gradient_y + 80.0) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a truss-3d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/truss-3d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("space-frame-pyramid-3d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a truss 3d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.0000015799074540869988) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.max_stress - 74386.37868140468) < 1.0e-9);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[3].ux - 2.897530666749509e-7) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.nodes[3].uy - 2.897530666749509e-7) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.nodes[3].uz + 0.0000015258420246488773) < 1.0e-18);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 6);
    assert.ok(Math.abs(finalPayload.result.elements[3].stress + 74386.37868140468) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[4].stress + 63387.6959669619) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[5].stress + 63387.6959669619) < 1.0e-9);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a frame-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/frame-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("portal-frame-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a frame 2d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.000007804733100069413) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.max_rotation - 0.0000028537985619530678) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.max_moment - 22.473663675380564) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_stress - 666276.4988945248) < 1.0e-6);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[2].uy + 0.0000076145885587945695) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.nodes[3].ux + 0.000005136837411515516) < 1.0e-15);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 3);
    assert.ok(Math.abs(finalPayload.result.elements[0].max_combined_stress - 14436.207569254871) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[1].max_combined_stress - 20430.603341254886) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[2].max_combined_stress - 666276.4988945248) < 1.0e-6);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a truss-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/truss-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("braced-truss-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a truss 2d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.0000010994861883006246) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.max_stress - 58962.38207535379) < 1.0e-9);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 3);
    assert.ok(Math.abs(finalPayload.result.nodes[1].ux - 4.4642857142857147e-7) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.nodes[2].ux - 2.232142857142857e-7) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.nodes[2].uy + 0.0000010765896436975871) < 1.0e-18);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 3);
    assert.ok(Math.abs(finalPayload.result.elements[0].stress + 58962.38207535379) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[1].stress + 58962.38207535378) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[2].stress - 31250.000000000004) < 1.0e-9);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a plane-triangle-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/plane-triangle-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("cantilever-plate-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a plane triangle job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.000002356099519632326) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.max_stress - 336671.8739122107) < 1.0e-9);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[1].ux + 4.4825506780263774e-7) < 1.0e-18);
    assert.ok(Math.abs(finalPayload.result.nodes[2].uy + 0.0000023112942546863956) < 1.0e-18);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 2);
    assert.ok(Math.abs(finalPayload.result.elements[0].von_mises - 143522.0261933109) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[1].von_mises - 336671.8739122107) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.elements[1].tau_xy + 186681.4590323627) < 1.0e-9);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a spring-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/spring-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("spring-grid-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a spring 2d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.06339734949589224) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.max_force - 1120.754716981132) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[2].ux - 0.0509433962264151) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.nodes[2].uy + 0.03773584905660377) < 1.0e-15);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 5);
    assert.ok(Math.abs(finalPayload.result.elements[2].force - 1120.754716981132) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[1].force + 679.2452830188679) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[4].force - 112.06975399937737) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a spring-1d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/spring-1d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("spring-chain-1d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a spring 1d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.09428571428571428) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.max_force - 1200.0) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 3);
    assert.ok(Math.abs(finalPayload.result.nodes[1].ux - 0.03428571428571429) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.nodes[2].ux - 0.09428571428571428) < 1.0e-15);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 2);
    assert.ok(Math.abs(finalPayload.result.elements[0].force - 1200.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[1].force - 1199.9999999999998) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a torsion-1d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/torsion-1d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("torsion-shaft-1d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a torsion 1d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_rotation - 0.026371308016877638) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.max_torque - 2500.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_stress - 20833333.333333332) < 1.0e-9);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 2);
    assert.ok(Math.abs(finalPayload.result.nodes[1].rz - 0.026371308016877638) < 1.0e-15);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
    assert.ok(Math.abs(finalPayload.result.elements[0].torque - 2500.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[0].twist - 0.026371308016877638) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.elements[0].shear_stress - 20833333.333333332) < 1.0e-9);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a spring-3d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/spring-3d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("spring-cage-3d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a spring 3d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.05955868626521211) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.max_force - 803.0108273796119) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[3].ux - 0.037134189113355795) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.nodes[3].uy - 0.03445543981481482) < 1.0e-15);
    assert.ok(Math.abs(finalPayload.result.nodes[3].uz + 0.03132270383761861) < 1.0e-15);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 6);
    assert.ok(Math.abs(finalPayload.result.elements[1].force + 803.0108273796119) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[2].force + 474.11760144504234) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[0].force + 82.59674462242567) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a frame-3d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/frame-3d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("frame-3d-cantilever.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a frame 3d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(finalPayload.result.max_displacement > 0);
    assert.ok(finalPayload.result.max_rotation > 0);
    assert.ok(finalPayload.result.max_moment > 0);
    assert.ok(finalPayload.result.max_stress > 0);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 2);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a thermal-frame-3d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-frame-3d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-frame-3d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal frame 3d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(finalPayload.result.max_displacement <= 1.0e-9);
    assert.ok(finalPayload.result.max_axial_force > 0);
    assert.ok(finalPayload.result.max_moment > 0);
    assert.ok(finalPayload.result.max_stress > 0);
    assert.equal(finalPayload.result.max_temperature_delta, 35);
    assert.equal(finalPayload.result.max_temperature_gradient, 30);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 2);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a plane-quad-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/plane-quad-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("quad-plate-patch-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a plane quad job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 5.333507749004975e-7) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_stress - 126981.38527836032) < 1.0e-6);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[2].ux - 2.576145151695419e-7) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.nodes[2].uy + 4.6700943316053366e-7) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
    assert.ok(Math.abs(finalPayload.result.elements[0].stress_x - 12500.0) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[0].stress_y + 120000.0) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[0].tau_xy - 3048.7804878048746) < 1.0e-9);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a thermal-truss-3d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-truss-3d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-truss-3d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal truss 3d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 4.7438412716850235e-4) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_axial_force - 294000.0) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.max_stress - 29400000.0) < 1.0e-3);
    assert.equal(finalPayload.result.max_temperature_delta, 35);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 6);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a thermal-truss-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-truss-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-truss-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal truss 2d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 4.801785714285713e-4) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_axial_force - 235.84952830143558) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.max_stress - 23584.952830143557) < 1.0e-6);
    assert.equal(finalPayload.result.max_temperature_delta, 40);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 3);
    assert.ok(Math.abs(finalPayload.result.nodes[1].ux - 4.801785714285713e-4) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.nodes[2].uy - 2.834443641425211e-4) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 3);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a thermal-plane-triangle-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-plane-triangle-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-plane-triangle-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal plane triangle job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_stress - 50149253.731343284) < 1.0e-6);
    assert.equal(finalPayload.result.max_temperature_delta, 40);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 2);
    assert.ok(Math.abs(finalPayload.result.elements[0].stress_x + 50149253.731343284) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[1].stress_y + 50149253.731343284) < 1.0e-6);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

test("local workstation stack can solve a thermal-plane-quad-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-plane-quad-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-plane-quad-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal plane quad job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.0) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_stress - 34477611.940298505) < 1.0e-6);
    assert.equal(finalPayload.result.max_temperature_delta, 30);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 1);
    assert.ok(Math.abs(finalPayload.result.elements[0].stress_x + 34477611.940298505) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[0].stress_y + 34477611.940298505) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[0].mechanical_strain_x + 3.3e-4) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.elements[0].mechanical_strain_y + 3.3e-4) < 1.0e-12);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });

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

test("local workstation stack can solve a thermal-frame-2d sample end-to-end", async () => {
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

    const submitResponse = await fetch(`${ORCHESTRATOR_URL}/api/v1/fem/thermal-frame-2d/jobs`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(loadSampleModel("thermal-frame-2d.json")),
    });

    assert.equal(submitResponse.status, 202);
    const submitted = await submitResponse.json();
    const jobId = submitted?.job?.job_id;
    assert.ok(jobId, "expected a thermal frame 2d job_id from the orchestrator");

    const finalPayload = await waitFor(
      `${ORCHESTRATOR_URL}/api/v1/jobs/${jobId}`,
      (payload) => payload?.job?.status === "completed",
      60_000,
      750,
    );

    assert.equal(finalPayload.job.status, "completed");
    assert.match(finalPayload.job.worker_id, /rust-agent-rpc/);
    assert.ok(Math.abs(finalPayload.result.max_displacement - 0.0010408174194986581) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_rotation - 0.0006805479452054797) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.max_axial_force - 24164.383561644005) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.max_moment - 42915.94520547945) < 1.0e-9);
    assert.ok(Math.abs(finalPayload.result.max_stress - 36971506.84931508) < 1.0e-6);
    assert.equal(finalPayload.result.max_temperature_delta, 35);
    assert.equal(finalPayload.result.max_temperature_gradient, 30);
    assert.ok(Array.isArray(finalPayload.result.nodes));
    assert.equal(finalPayload.result.nodes.length, 4);
    assert.ok(Math.abs(finalPayload.result.nodes[1].ux + 0.0008284931506849309) < 1.0e-12);
    assert.ok(Math.abs(finalPayload.result.nodes[1].uy - 0.00063) < 1.0e-12);
    assert.ok(Array.isArray(finalPayload.result.elements));
    assert.equal(finalPayload.result.elements.length, 3);
    assert.ok(Math.abs(finalPayload.result.elements[1].axial_stress - 1208219.1780822002) < 1.0e-6);
    assert.ok(Math.abs(finalPayload.result.elements[1].max_combined_stress - 36971506.84931508) < 1.0e-6);
  } finally {
    try {
      runKyuubiki("stop");
    } catch {
      // keep cleanup best-effort for local integration runs
    }
  }
}, { timeout: 120_000 });
