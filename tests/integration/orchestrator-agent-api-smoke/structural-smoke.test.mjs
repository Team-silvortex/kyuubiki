import test from "node:test";
import assert from "node:assert/strict";
import { ORCHESTRATOR_URL, loadSampleModel, runKyuubiki, waitFor } from "./support.mjs";
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
