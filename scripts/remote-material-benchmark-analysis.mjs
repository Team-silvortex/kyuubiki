export function summarizeStageRows(rows) {
  const groups = new Map();
  for (const row of rows) {
    const key = `${row.matrix}/${row.profile}/${row.stage}`;
    const current = groups.get(key) || {
      case_count: 0,
      elapsed_ms_total: 0,
      matrix: row.matrix,
      max_elapsed_ms: 0,
      non_zero_elapsed_ms_total: 0,
      non_zero_visit_count: 0,
      profile: row.profile,
      stage: row.stage,
      stage_share_pct_total: 0,
      stage_share_sample_count: 0,
    };
    current.case_count += 1;
    current.elapsed_ms_total += row.elapsed_ms;
    current.max_elapsed_ms = Math.max(current.max_elapsed_ms, row.elapsed_ms);
    const nonZeroVisits = stageNonZeroVisits(row);
    if (nonZeroVisits > 0) {
      current.non_zero_elapsed_ms_total += row.elapsed_ms;
      current.non_zero_visit_count += nonZeroVisits;
    }
    if (Number.isFinite(row.stage_share_pct)) {
      current.stage_share_pct_total += row.stage_share_pct;
      current.stage_share_sample_count += 1;
    }
    groups.set(key, current);
  }
  return [...groups.values()]
    .map((item) => ({
      average_stage_share_pct:
        item.stage_share_sample_count > 0
          ? item.stage_share_pct_total / item.stage_share_sample_count
          : null,
      case_count: item.case_count,
      elapsed_ms_total: item.elapsed_ms_total,
      matrix: item.matrix,
      max_elapsed_ms: item.max_elapsed_ms,
      ms_per_million_non_zero_visits:
        item.non_zero_visit_count > 0
          ? item.non_zero_elapsed_ms_total / (item.non_zero_visit_count / 1_000_000)
          : null,
      non_zero_elapsed_ms_total: item.non_zero_elapsed_ms_total,
      non_zero_visit_count: item.non_zero_visit_count,
      profile: item.profile,
      stage: item.stage,
    }))
    .sort((left, right) => right.elapsed_ms_total - left.elapsed_ms_total);
}

export function optimizationTargets(stageSummary) {
  return stageSummary
    .map((item) => ({
      average_stage_share_pct: item.average_stage_share_pct,
      case_count: item.case_count,
      elapsed_ms_total: item.elapsed_ms_total,
      focus: optimizationFocus(item.stage),
      matrix: item.matrix,
      ms_per_million_non_zero_visits: item.ms_per_million_non_zero_visits,
      non_zero_visit_count: item.non_zero_visit_count,
      priority_score: optimizationPriorityScore(item),
      profile: item.profile,
      stage: item.stage,
    }))
    .sort((left, right) => right.priority_score - left.priority_score);
}

export function sparseMatvecThroughput(stageSummary) {
  return stageSummary
    .filter(
      (item) =>
        item.stage === "solve_spd_matvec" &&
        Number.isFinite(item.ms_per_million_non_zero_visits),
    )
    .sort(
      (left, right) =>
        right.non_zero_visit_count - left.non_zero_visit_count ||
        left.ms_per_million_non_zero_visits - right.ms_per_million_non_zero_visits,
    );
}

export function preconditionerEconomics(comparisons, stageRows) {
  const stageByCase = stageRowsByCase(stageRows);
  return comparisons.map((comparison) => {
    const winner = comparison.compared.find(
      (item) => item.solver_preconditioner === comparison.winner_preconditioner,
    );
    const slowest = comparison.compared.at(-1);
    const winnerCaseId = `${comparison.base_case_id}#${comparison.winner_preconditioner}`;
    const slowestCaseId = `${comparison.base_case_id}#${slowest?.solver_preconditioner}`;
    const winnerStages = stageByCase.get(stageCaseKey(comparison, winnerCaseId)) || new Map();
    const slowestStages = stageByCase.get(stageCaseKey(comparison, slowestCaseId)) || new Map();
    const elapsedSavedMs =
      Number.isFinite(winner?.median_ms) && Number.isFinite(slowest?.median_ms)
        ? slowest.median_ms - winner.median_ms
        : null;
    const iterationsSaved =
      Number.isFinite(winner?.solver_iterations) && Number.isFinite(slowest?.solver_iterations)
        ? slowest.solver_iterations - winner.solver_iterations
        : null;
    const winnerPreconditionerMs = winnerStages.get("solve_spd_preconditioner") ?? null;
    const slowestPreconditionerMs = slowestStages.get("solve_spd_preconditioner") ?? null;
    const preconditionerExtraMs =
      Number.isFinite(winnerPreconditionerMs) && Number.isFinite(slowestPreconditionerMs)
        ? winnerPreconditionerMs - slowestPreconditionerMs
        : null;
    return {
      base_case_id: comparison.base_case_id,
      elapsed_saved_ms: elapsedSavedMs,
      extra_pre_ms_per_iteration_saved: ratio(preconditionerExtraMs, iterationsSaved),
      gross_non_preconditioner_saved_ms:
        Number.isFinite(elapsedSavedMs) && Number.isFinite(preconditionerExtraMs)
          ? elapsedSavedMs + preconditionerExtraMs
          : null,
      iterations_saved: iterationsSaved,
      matrix: comparison.matrix,
      ms_saved_per_iteration_saved: ratio(elapsedSavedMs, iterationsSaved),
      preconditioner_extra_ms: preconditionerExtraMs,
      profile: comparison.profile,
      slowest_matvec_ms: slowestStages.get("solve_spd_matvec") ?? null,
      slowest_matvec_ms_per_iteration: msPerIteration(
        slowestStages.get("solve_spd_matvec"),
        slowest?.solver_iterations,
      ),
      slowest_preconditioner: slowest?.solver_preconditioner || null,
      slowest_preconditioner_ms: slowestPreconditionerMs,
      slowest_pre_ms_per_iteration: msPerIteration(
        slowestPreconditionerMs,
        slowest?.solver_iterations,
      ),
      winner_matvec_ms: winnerStages.get("solve_spd_matvec") ?? null,
      winner_matvec_ms_per_iteration: msPerIteration(
        winnerStages.get("solve_spd_matvec"),
        winner?.solver_iterations,
      ),
      winner_preconditioner: comparison.winner_preconditioner,
      winner_preconditioner_ms: winnerPreconditionerMs,
      winner_pre_ms_per_iteration: msPerIteration(
        winnerPreconditionerMs,
        winner?.solver_iterations,
      ),
      winner_speedup_ratio: comparison.winner_speedup_ratio,
    };
  });
}

export function solverTuningNotes(economicsRows) {
  return economicsRows
    .filter(
      (item) =>
        Number.isFinite(item.winner_pre_ms_per_iteration) &&
        Number.isFinite(item.winner_matvec_ms_per_iteration),
    )
    .map((item) => ({
      case_id: item.base_case_id,
      focus:
        item.winner_pre_ms_per_iteration > item.winner_matvec_ms_per_iteration
          ? "sgs-sweep-cost"
          : "matvec-or-vector-cost",
      matrix: item.matrix,
      profile: item.profile,
      reason:
        item.winner_pre_ms_per_iteration > item.winner_matvec_ms_per_iteration
          ? "winner preconditioner cost per iteration is above matvec cost per iteration"
          : "winner matvec/vector work remains comparable to preconditioner cost",
      winner_matvec_ms_per_iteration: item.winner_matvec_ms_per_iteration,
      winner_pre_ms_per_iteration: item.winner_pre_ms_per_iteration,
    }))
    .sort(
      (left, right) =>
        right.winner_pre_ms_per_iteration - right.winner_matvec_ms_per_iteration -
        (left.winner_pre_ms_per_iteration - left.winner_matvec_ms_per_iteration),
    );
}

function stageNonZeroVisits(row) {
  if (row.stage !== "solve_spd_matvec") return 0;
  if (!Number.isFinite(row.solver_matrix_non_zero_count)) return 0;
  if (!Number.isFinite(row.solver_iterations)) return 0;
  return row.solver_matrix_non_zero_count * row.solver_iterations;
}

function optimizationPriorityScore(item) {
  const share = Number.isFinite(item.average_stage_share_pct) ? item.average_stage_share_pct : 0;
  return item.elapsed_ms_total * (1.0 + share / 100.0);
}

function optimizationFocus(stage) {
  if (stage === "solve_spd_matvec") return "sparse-matvec-throughput";
  if (stage === "solve_spd_preconditioner") return "preconditioner-sweep-throughput";
  if (stage === "solve_spd_dot") return "vector-reduction-throughput";
  if (stage === "solve_spd_vector_update" || stage === "solve_spd_direction_update") {
    return "vector-update-throughput";
  }
  if (stage === "solve_spd_residual_recompute") return "residual-refresh-cost";
  if (stage === "assemble_global") return "assembly-throughput";
  return "stage-throughput";
}

function stageRowsByCase(stageRows) {
  const groups = new Map();
  for (const row of stageRows) {
    const key = `${row.matrix}/${row.profile}/${row.case_id}`;
    const stages = groups.get(key) || new Map();
    stages.set(row.stage, row.elapsed_ms);
    groups.set(key, stages);
  }
  return groups;
}

function stageCaseKey(comparison, caseId) {
  return `${comparison.matrix}/${comparison.profile}/${caseId}`;
}

function msPerIteration(elapsedMs, iterations) {
  return ratio(elapsedMs, iterations);
}

function ratio(numerator, denominator) {
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator <= 0) return null;
  return numerator / denominator;
}
