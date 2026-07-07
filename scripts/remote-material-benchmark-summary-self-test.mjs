import assert from "node:assert/strict";

export function runRemoteMaterialBenchmarkSummarySelfTest(buildSummary) {
  const summary = buildSummary([
    fakeRun(
      "run-001",
      "mechanical-core",
      "400k",
      [
        fakeCase("truss-400k", 100, "symmetric-gauss-seidel"),
        fakeCase("panel-400k", 200, "symmetric-gauss-seidel"),
        fakeCase("panel-400k#jacobi", 300, "jacobi"),
        fakeCase("panel-400k#symmetric-gauss-seidel", 220, "symmetric-gauss-seidel"),
      ],
      [fakeComparison("panel-400k", 220, 300)],
    ),
    fakeRun("run-002", "mechanical-core", "400k", [
      fakeCase("panel-400k#jacobi", 330, "jacobi"),
    ]),
  ]);
  const latestIds = summary.latest_cases.map((item) => item.case_id);
  assert.deepEqual(latestIds, [
    "panel-400k",
    "panel-400k#jacobi",
    "panel-400k#symmetric-gauss-seidel",
    "truss-400k",
  ]);
  assert.equal(
    summary.latest_cases.find((item) => item.case_id === "panel-400k#jacobi")?.median_ms,
    330,
  );
  assert.equal(summary.latest_stage_hotspots[0].run, "run-002");
  assert.equal(summary.latest_stage_hotspots[0].stage_share_pct, 50);
  assert.equal(summary.latest_stage_hotspots.some((item) => item.case_id === "truss-400k"), true);
  assert.equal(summary.latest_stage_summary[0].stage, "solve_spd_matvec");
  assert.equal(summary.latest_stage_summary[0].case_count, 4);
  assert.equal(summary.latest_stage_summary[0].elapsed_ms_total, 425);
  assert.equal(summary.latest_stage_summary[0].ms_per_million_non_zero_visits, 500);
  assert.equal(summary.latest_sparse_matvec_throughput[0].stage, "solve_spd_matvec");
  assert.equal(summary.latest_sparse_matvec_throughput[0].ms_per_million_non_zero_visits, 500);
  assert.equal(summary.latest_optimization_targets[0].stage, "solve_spd_matvec");
  assert.equal(summary.latest_optimization_targets[0].focus, "sparse-matvec-throughput");
  assert.equal(summary.latest_preconditioner_economics[0].base_case_id, "panel-400k");
  assert.equal(summary.latest_preconditioner_economics[0].iterations_saved, 110);
  assert.equal(summary.latest_preconditioner_economics[0].winner_preconditioner_ms, 55);
  assert.equal(summary.latest_preconditioner_economics[0].slowest_preconditioner_ms, 82.5);
  assert.equal(summary.latest_preconditioner_economics[0].preconditioner_extra_ms, -27.5);
  assert.equal(summary.latest_preconditioner_economics[0].gross_non_preconditioner_saved_ms, 82.5);
  assert.equal(summary.latest_preconditioner_economics[0].winner_pre_ms_per_iteration, 0.25);
  assert.equal(summary.latest_preconditioner_economics[0].slowest_pre_ms_per_iteration, 0.25);
  assert.equal(summary.latest_preconditioner_economics[0].winner_matvec_ms_per_iteration, 0.5);
  assert.equal(summary.latest_preconditioner_economics[0].slowest_matvec_ms_per_iteration, 0.5);
  assert.equal(summary.latest_solver_tuning_notes[0].case_id, "panel-400k");
  assert.equal(summary.latest_solver_tuning_notes[0].focus, "matvec-or-vector-cost");
  assert.deepEqual(summary.latest_preconditioner_comparisons, [
    {
      base_case_id: "panel-400k",
      compared: [
        {
          median_ms: 220,
          solver_iterations: 220,
          solver_preconditioner: "symmetric-gauss-seidel",
        },
        {
          median_ms: 330,
          solver_iterations: 330,
          solver_preconditioner: "jacobi",
        },
      ],
      matrix: "mechanical-core",
      profile: "400k",
      winner_iteration_reduction_pct: 33.33333333333333,
      winner_median_ms: 220,
      winner_preconditioner: "symmetric-gauss-seidel",
      winner_solver_iterations: 220,
      winner_speedup_ratio: 1.5,
    },
  ]);
  console.log("remote material benchmark summary self-test passed");
}

function fakeRun(name, matrix, profile, cases, preconditionerComparisons = []) {
  return {
    benchmark: {
      cases,
      matrix,
      preconditioner_comparisons: preconditionerComparisons,
      profile,
      repeat: 1,
    },
    name,
    research: null,
  };
}

function fakeComparison(baseCaseId, sgsMs, jacobiMs) {
  return {
    base_case_id: baseCaseId,
    compared: [
      {
        median_ms: sgsMs,
        solver_iterations: sgsMs,
        solver_preconditioner: "symmetric-gauss-seidel",
      },
      {
        median_ms: jacobiMs,
        solver_iterations: jacobiMs,
        solver_preconditioner: "jacobi",
      },
    ],
    winner_iteration_reduction_pct: ((jacobiMs - sgsMs) / jacobiMs) * 100.0,
    winner_median_ms: sgsMs,
    winner_preconditioner: "symmetric-gauss-seidel",
    winner_solver_iterations: sgsMs,
    winner_speedup_ratio: jacobiMs / sgsMs,
  };
}

function fakeCase(id, medianMs, solverPreconditioner) {
  return {
    dof_count: 1000,
    id,
    median_ms: medianMs,
    memory_stages: [
      {
        elapsed_ms: medianMs / 2,
        label: "solve_spd_system",
        rss_kib: 1024,
      },
      {
        elapsed_ms: medianMs / 2,
        label: "solve_spd_matvec",
        rss_kib: 1024,
      },
      {
        elapsed_ms: medianMs / 4,
        label: "solve_spd_preconditioner",
        rss_kib: 1024,
      },
    ],
    ok: true,
    peak_rss_kib: 2048,
    solver_iterations: medianMs,
    solver_matrix_non_zero_count: 1000,
    solver_preconditioner: solverPreconditioner,
    solver_residual_norm: 1.0e-10,
  };
}
