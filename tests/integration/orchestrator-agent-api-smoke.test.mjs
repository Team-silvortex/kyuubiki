import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const ENTRYPOINT = `${ROOT}/scripts/kyuubiki`;
const ORCHESTRATOR_URL = "http://127.0.0.1:4000";

function runKyuubiki(...args) {
  return execFileSync("zsh", [ENTRYPOINT, ...args], {
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
