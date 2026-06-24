import test from "node:test";
import assert from "node:assert/strict";
import { ORCHESTRATOR_URL, loadSampleModel, runKyuubiki, waitFor } from "./support.mjs";
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
