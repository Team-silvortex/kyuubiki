import fs from "node:fs";
import path from "node:path";

export function writeRemoteMaterialBenchmarkMarkdown(summary, outputPath) {
  const lines = [
    "# Remote Material Benchmark Summary",
    "",
    `Generated: ${summary.generated_at_utc}`,
    "",
    `Runs: ${summary.run_count}`,
    "",
    `Cases: ${summary.case_count}`,
    "",
    `Failed cases: ${summary.failed_cases.length}`,
    "",
    "## Latest Cases",
    "",
    "| Matrix | Profile | Case | Median ms | RSS MiB | DOF | Iterations | Residual |",
    "| --- | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ...summary.latest_cases.map((item) => caseRowWithoutRank(item)),
    "",
    "## Best By Case",
    "",
    "| Rank | Matrix | Profile | Case | Median ms | RSS MiB | DOF | Iterations | Residual |",
    "| ---: | --- | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ...summary.best_cases.map((item, index) => caseRow(index + 1, item)),
    "",
    "## Hottest Cases",
    "",
    "| Rank | Matrix | Profile | Case | Median ms | RSS MiB | DOF | Iterations | Residual |",
    "| ---: | --- | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ...summary.hottest_cases.map((item, index) => caseRow(index + 1, item)),
    "",
    "## Memory Heaviest Cases",
    "",
    "| Rank | Matrix | Profile | Case | Median ms | RSS MiB | DOF | Iterations | Residual |",
    "| ---: | --- | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ...summary.memory_heaviest_cases.map((item, index) => caseRow(index + 1, item)),
    "",
    "## Latest Preconditioner Comparison",
    "",
    "| Matrix | Profile | Case | Winner | Winner ms | Speedup | Iterations | Compared |",
    "| --- | --- | --- | --- | ---: | ---: | ---: | --- |",
    ...summary.latest_preconditioner_comparisons.map((item) => preconditionerComparisonRow(item)),
    "",
    "## Latest Preconditioner Economics",
    "",
    "| Matrix | Profile | Case | Winner | Slowest | Saved ms | Saved it | Winner pre ms/it | Slowest pre ms/it | Winner matvec ms/it | Slowest matvec ms/it | Extra pre ms | Gross non-pre saved ms |",
    "| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ...summary.latest_preconditioner_economics.map((item) => preconditionerEconomicsRow(item)),
    "",
    "## Latest Solver Tuning Notes",
    "",
    "| Matrix | Profile | Case | Focus | Winner pre ms/it | Winner matvec ms/it | Reason |",
    "| --- | --- | --- | --- | ---: | ---: | --- |",
    ...summary.latest_solver_tuning_notes.map((item) => solverTuningNoteRow(item)),
    "",
    "## Latest Optimization Targets",
    "",
    "| Rank | Matrix | Profile | Stage | Focus | Cases | Total ms | Avg Share | ms / M nnz-visits | Score |",
    "| ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ...summary.latest_optimization_targets.map((item, index) =>
      optimizationTargetRow(index + 1, item),
    ),
    "",
    "## Latest Sparse Matvec Throughput",
    "",
    "| Matrix | Profile | Cases | Total ms | nnz-visits | ms / M nnz-visits |",
    "| --- | --- | ---: | ---: | ---: | ---: |",
    ...summary.latest_sparse_matvec_throughput.map((item) => sparseMatvecThroughputRow(item)),
    "",
    "## Latest Stage Summary",
    "",
    "| Matrix | Profile | Stage | Cases | Total ms | Max ms | Avg Share | ms / M nnz-visits |",
    "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ...summary.latest_stage_summary.map((item) => stageSummaryRow(item)),
    "",
    "## Latest Stage Hotspots",
    "",
    "| Rank | Matrix | Profile | Case | Stage | Elapsed ms | Share | Stage RSS MiB |",
    "| ---: | --- | --- | --- | --- | ---: | ---: | ---: |",
    ...summary.latest_stage_hotspots.map((item, index) => stageRow(index + 1, item)),
    "",
    "## Stage Hotspots",
    "",
    "| Rank | Matrix | Profile | Case | Stage | Elapsed ms | Share | Stage RSS MiB |",
    "| ---: | --- | --- | --- | --- | ---: | ---: | ---: |",
    ...summary.stage_hotspots.map((item, index) => stageRow(index + 1, item)),
    "",
    "## Stage Summary",
    "",
    "| Matrix | Profile | Stage | Cases | Total ms | Max ms | Avg Share | ms / M nnz-visits |",
    "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ...summary.stage_summary.slice(0, 16).map((item) => stageSummaryRow(item)),
    "",
  ];
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${lines.join("\n")}\n`);
}

function caseRow(rank, item) {
  const rss = item.peak_rss_mib == null ? "n/a" : item.peak_rss_mib.toFixed(1);
  const iterations = item.solver_iterations == null ? "n/a" : String(item.solver_iterations);
  const residual = item.residual_norm == null ? "n/a" : Number(item.residual_norm).toExponential(2);
  return `| ${rank} | ${item.matrix} | ${formatProfile(item.profile)} | ${item.case_id} | ${item.median_ms.toFixed(2)} | ${rss} | ${item.dof_count} | ${iterations} | ${residual} |`;
}

function caseRowWithoutRank(item) {
  const rss = item.peak_rss_mib == null ? "n/a" : item.peak_rss_mib.toFixed(1);
  const iterations = item.solver_iterations == null ? "n/a" : String(item.solver_iterations);
  const residual = item.residual_norm == null ? "n/a" : Number(item.residual_norm).toExponential(2);
  return `| ${item.matrix} | ${formatProfile(item.profile)} | ${item.case_id} | ${item.median_ms.toFixed(2)} | ${rss} | ${item.dof_count} | ${iterations} | ${residual} |`;
}

function stageRow(rank, item) {
  const rss = item.stage_rss_mib == null ? "n/a" : item.stage_rss_mib.toFixed(1);
  const share = item.stage_share_pct == null ? "n/a" : `${item.stage_share_pct.toFixed(1)}%`;
  return `| ${rank} | ${item.matrix} | ${formatProfile(item.profile)} | ${item.case_id} | ${item.stage} | ${item.elapsed_ms.toFixed(2)} | ${share} | ${rss} |`;
}

function stageSummaryRow(item) {
  const share =
    item.average_stage_share_pct == null ? "n/a" : `${item.average_stage_share_pct.toFixed(1)}%`;
  return `| ${item.matrix} | ${formatProfile(item.profile)} | ${item.stage} | ${item.case_count} | ${item.elapsed_ms_total.toFixed(2)} | ${item.max_elapsed_ms.toFixed(2)} | ${share} | ${formatNullableFixed(item.ms_per_million_non_zero_visits, 4)} |`;
}

function optimizationTargetRow(rank, item) {
  const share =
    item.average_stage_share_pct == null ? "n/a" : `${item.average_stage_share_pct.toFixed(1)}%`;
  return `| ${rank} | ${item.matrix} | ${formatProfile(item.profile)} | ${item.stage} | ${item.focus} | ${item.case_count} | ${item.elapsed_ms_total.toFixed(2)} | ${share} | ${formatNullableFixed(item.ms_per_million_non_zero_visits, 4)} | ${item.priority_score.toFixed(2)} |`;
}

function sparseMatvecThroughputRow(item) {
  return `| ${item.matrix} | ${formatProfile(item.profile)} | ${item.case_count} | ${item.non_zero_elapsed_ms_total.toFixed(2)} | ${item.non_zero_visit_count} | ${formatNullableFixed(item.ms_per_million_non_zero_visits, 4)} |`;
}

function preconditionerComparisonRow(item) {
  const iterations =
    item.winner_solver_iterations == null ? "n/a" : String(item.winner_solver_iterations);
  const speedup =
    item.winner_speedup_ratio == null ? "n/a" : `${item.winner_speedup_ratio.toFixed(2)}x`;
  const compared = item.compared
    .map((entry) => {
      const entryIterations =
        entry.solver_iterations == null ? "n/a" : `${entry.solver_iterations} it`;
      return `${entry.solver_preconditioner}: ${entry.median_ms.toFixed(2)} ms / ${entryIterations}`;
    })
    .join("<br>");
  return `| ${item.matrix} | ${formatProfile(item.profile)} | ${item.base_case_id} | ${item.winner_preconditioner} | ${item.winner_median_ms.toFixed(2)} | ${speedup} | ${iterations} | ${compared} |`;
}

function preconditionerEconomicsRow(item) {
  return `| ${item.matrix} | ${formatProfile(item.profile)} | ${item.base_case_id} | ${item.winner_preconditioner} | ${item.slowest_preconditioner} | ${formatNullableFixed(item.elapsed_saved_ms, 2)} | ${formatNullableFixed(item.iterations_saved, 0)} | ${formatNullableFixed(item.winner_pre_ms_per_iteration, 4)} | ${formatNullableFixed(item.slowest_pre_ms_per_iteration, 4)} | ${formatNullableFixed(item.winner_matvec_ms_per_iteration, 4)} | ${formatNullableFixed(item.slowest_matvec_ms_per_iteration, 4)} | ${formatNullableFixed(item.preconditioner_extra_ms, 2)} | ${formatNullableFixed(item.gross_non_preconditioner_saved_ms, 2)} |`;
}

function solverTuningNoteRow(item) {
  return `| ${item.matrix} | ${formatProfile(item.profile)} | ${item.case_id} | ${item.focus} | ${formatNullableFixed(item.winner_pre_ms_per_iteration, 4)} | ${formatNullableFixed(item.winner_matvec_ms_per_iteration, 4)} | ${item.reason} |`;
}

function formatProfile(profile) {
  const profiles = {
    four_hundred_k: "400k",
    hundred_k: "100k",
    ten_k: "10k",
    three_hundred_k: "300k",
    two_hundred_k: "200k",
  };
  return profiles[profile] || profile;
}

function formatNullableFixed(value, digits) {
  return value == null ? "n/a" : value.toFixed(digits);
}
