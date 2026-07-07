#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  optimizationTargets,
  preconditionerEconomics,
  solverTuningNotes,
  sparseMatvecThroughput,
  summarizeStageRows,
} from "./remote-material-benchmark-analysis.mjs";
import { writeRemoteMaterialBenchmarkMarkdown } from "./remote-material-benchmark-markdown.mjs";
import { runRemoteMaterialBenchmarkSummarySelfTest } from "./remote-material-benchmark-summary-self-test.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

function parseArgs(argv) {
  const options = {
    inputDir: path.join(repoRoot, "tmp", "remote-material-research"),
    jsonOut: path.join(repoRoot, "tmp", "remote-material-research", "summary.json"),
    markdownOut: path.join(repoRoot, "tmp", "remote-material-research", "README.md"),
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--help" || arg === "-h") {
      printUsage();
      process.exit(0);
    } else if (arg === "--self-test") {
      options.selfTest = true;
    } else if (arg === "--input-dir" && next) {
      options.inputDir = path.resolve(next);
      index += 1;
    } else if (arg === "--json-out" && next) {
      options.jsonOut = path.resolve(next);
      index += 1;
    } else if (arg === "--markdown-out" && next) {
      options.markdownOut = path.resolve(next);
      index += 1;
    } else {
      fail(`unknown or incomplete argument: ${arg}`);
    }
  }
  return options;
}

function readRuns(inputDir) {
  if (!fs.existsSync(inputDir)) return [];
  const runs = [];
  for (const name of fs.readdirSync(inputDir).sort()) {
    const dir = path.join(inputDir, name);
    const benchmarkPath = path.join(dir, "remote-material-research-benchmark.json");
    const researchPath = path.join(dir, "material-research-example.json");
    if (!fs.statSync(dir).isDirectory() || !fs.existsSync(benchmarkPath)) continue;
    const benchmark = readJson(benchmarkPath);
    const research = fs.existsSync(researchPath) ? readJson(researchPath) : null;
    runs.push({
      benchmark,
      dir,
      name,
      research,
    });
  }
  return runs;
}

export function buildSummary(runs) {
  const sortedRuns = [...runs].sort((left, right) => left.name.localeCompare(right.name));
  const cases = runs.flatMap((run) =>
    (run.benchmark.cases || []).map((item) => ({
      case_id: item.id,
      dof_count: item.dof_count,
      matrix: run.benchmark.matrix,
      median_ms: item.median_ms,
      ok: item.ok,
      peak_rss_mib: item.peak_rss_kib == null ? null : item.peak_rss_kib / 1024,
      profile: run.benchmark.profile,
      residual_norm: item.solver_residual_norm,
      run: run.name,
      solver_iterations: item.solver_iterations,
      solver_matrix_non_zero_count: item.solver_matrix_non_zero_count,
      solver_preconditioner: item.solver_preconditioner || preconditionerFromCaseId(item.id),
    })),
  );
  const sortedByTime = [...cases].sort((left, right) => right.median_ms - left.median_ms);
  const sortedByMemory = [...cases].sort(
    (left, right) => (right.peak_rss_mib || 0) - (left.peak_rss_mib || 0),
  );
  const latestCases = latestCaseResults(sortedRuns);
  const bestCases = bestCaseResults(cases);
  const stageHotspots = stageRowsForRuns(runs);
  const latestStageHotspots = latestStageRows(sortedRuns, latestCases);
  const latestStageSummary = summarizeStageRows(latestStageHotspots);
  const stageSummary = summarizeStageRows(stageHotspots);
  const latestPreconditionerComparisons = latestPreconditionerComparisonsForRuns(
    sortedRuns,
    latestCases,
  );
  const latestPreconditionerEconomics = preconditionerEconomics(
    latestPreconditionerComparisons,
    latestStageHotspots,
  );
  return {
    schema_version: "kyuubiki.remote-material-benchmark-summary/v1",
    generated_at_utc: new Date().toISOString(),
    run_count: runs.length,
    case_count: cases.length,
    best_cases: bestCases.slice(0, 12),
    failed_cases: cases.filter((item) => !item.ok),
    hottest_cases: sortedByTime.slice(0, 8),
    latest_cases: latestCases,
    latest_optimization_targets: optimizationTargets(latestStageSummary).slice(0, 8),
    latest_preconditioner_economics: latestPreconditionerEconomics,
    latest_preconditioner_comparisons: latestPreconditionerComparisons,
    latest_solver_tuning_notes: solverTuningNotes(latestPreconditionerEconomics),
    latest_sparse_matvec_throughput: sparseMatvecThroughput(latestStageSummary),
    latest_stage_summary: latestStageSummary,
    latest_stage_hotspots: latestStageHotspots.slice(0, 12),
    memory_heaviest_cases: sortedByMemory.slice(0, 8),
    stage_summary: stageSummary,
    stage_hotspots: stageHotspots.slice(0, 12),
    runs: runs.map((run) => ({
      matrix: run.benchmark.matrix,
      name: run.name,
      profile: run.benchmark.profile,
      repeat: run.benchmark.repeat,
      winner_candidate_id: run.research?.exploration?.report?.winner_candidate_id || null,
    })),
  };
}

function latestCaseResults(sortedRuns) {
  const latestByCase = new Map();
  for (const run of sortedRuns) {
    for (const row of caseRowsForRun(run)) {
      latestByCase.set(caseKey(row), row);
    }
  }
  return [...latestByCase.values()].sort(compareCaseRows);
}

function caseRowsForRun(run) {
  return (run.benchmark.cases || []).map((item) => ({
    case_id: item.id,
    dof_count: item.dof_count,
    matrix: run.benchmark.matrix,
    median_ms: item.median_ms,
    ok: item.ok,
    peak_rss_mib: item.peak_rss_kib == null ? null : item.peak_rss_kib / 1024,
    profile: run.benchmark.profile,
    residual_norm: item.solver_residual_norm,
    run: run.name,
    solver_iterations: item.solver_iterations,
    solver_matrix_non_zero_count: item.solver_matrix_non_zero_count,
    solver_preconditioner: item.solver_preconditioner || preconditionerFromCaseId(item.id),
  }));
}

function latestStageRows(sortedRuns, latestCases) {
  const latestCaseKeys = new Set(
    latestCases.map((item) => `${item.run}/${item.matrix}/${item.profile}/${item.case_id}`),
  );
  return stageRowsForRuns(sortedRuns).filter((item) =>
    latestCaseKeys.has(`${item.run}/${item.matrix}/${item.profile}/${item.case_id}`),
  );
}

function caseKey(item) {
  return `${item.matrix}/${item.profile}/${item.case_id}`;
}

function compareCaseRows(left, right) {
  return (
    left.matrix.localeCompare(right.matrix) ||
    left.profile.localeCompare(right.profile) ||
    left.case_id.localeCompare(right.case_id)
  );
}

function stageRowsForRuns(runs) {
  return runs
    .flatMap((run) =>
      (run.benchmark.cases || []).flatMap((item) =>
        (item.memory_stages || [])
          .filter((stage) => Number.isFinite(stage.elapsed_ms) && isHotspotStage(stage.label))
          .map((stage) => ({
            case_id: item.id,
            elapsed_ms: stage.elapsed_ms,
            matrix: run.benchmark.matrix,
            profile: run.benchmark.profile,
            run: run.name,
            solver_iterations: item.solver_iterations,
            solver_matrix_non_zero_count: item.solver_matrix_non_zero_count,
            stage: stage.label,
            stage_share_pct:
              Number.isFinite(item.median_ms) && item.median_ms > 0
                ? (stage.elapsed_ms / item.median_ms) * 100.0
                : null,
            stage_rss_mib: stage.rss_kib == null ? null : stage.rss_kib / 1024,
          })),
      ),
    )
    .sort((left, right) => right.elapsed_ms - left.elapsed_ms);
}

function isHotspotStage(label) {
  return !["solve_system", "solve_spd_system"].includes(label);
}

function bestCaseResults(cases) {
  const best = new Map();
  for (const item of cases.filter((candidate) => candidate.ok)) {
    const key = `${item.matrix}/${item.profile}/${item.case_id}`;
    const current = best.get(key);
    if (!current || item.median_ms < current.median_ms) best.set(key, item);
  }
  return [...best.values()].sort((left, right) => right.median_ms - left.median_ms);
}

function preconditionerComparisons(cases) {
  const groups = new Map();
  for (const item of cases.filter((candidate) => candidate.ok && candidate.solver_preconditioner)) {
    const key = `${item.matrix}/${item.profile}/${baseCaseId(item.case_id)}`;
    const group = groups.get(key) || [];
    group.push(item);
    groups.set(key, group);
  }
  return [...groups.values()]
    .map((items) => dedupeByPreconditioner(items))
    .filter((items) => items.length > 1)
    .map((items) => {
      const sorted = [...items].sort((left, right) => left.median_ms - right.median_ms);
      return {
        base_case_id: baseCaseId(sorted[0].case_id),
        compared: sorted.map((item) => ({
          median_ms: item.median_ms,
          solver_iterations: item.solver_iterations,
          solver_preconditioner: item.solver_preconditioner,
        })),
        matrix: sorted[0].matrix,
        profile: sorted[0].profile,
        winner_iteration_reduction_pct: iterationReductionPct(
          sorted[0].solver_iterations,
          sorted.at(-1)?.solver_iterations,
        ),
        winner_median_ms: sorted[0].median_ms,
        winner_preconditioner: sorted[0].solver_preconditioner,
        winner_solver_iterations: sorted[0].solver_iterations,
        winner_speedup_ratio: (sorted.at(-1)?.median_ms || sorted[0].median_ms) / sorted[0].median_ms,
      };
    })
    .sort((left, right) => right.winner_median_ms - left.winner_median_ms);
}

function latestPreconditionerComparisonsForRuns(sortedRuns, latestCases) {
  const latestByCase = new Map(latestCases.map((item) => [caseKey(item), item]));
  const comparisons = new Map(
    preconditionerComparisons(latestCases).map((item) => [comparisonKey(item), item]),
  );
  for (const run of sortedRuns) {
    for (const comparison of run.benchmark.preconditioner_comparisons || []) {
      if (!reportComparisonMatchesLatest(run, comparison, latestByCase)) continue;
      const item = {
        base_case_id: comparison.base_case_id,
        compared: comparison.compared || [],
        matrix: run.benchmark.matrix,
        profile: run.benchmark.profile,
        winner_iteration_reduction_pct: comparison.winner_iteration_reduction_pct,
        winner_median_ms: comparison.winner_median_ms,
        winner_preconditioner: comparison.winner_preconditioner,
        winner_solver_iterations: comparison.winner_solver_iterations,
        winner_speedup_ratio: comparison.winner_speedup_ratio,
      };
      comparisons.set(comparisonKey(item), item);
    }
  }
  return [...comparisons.values()].sort(
    (left, right) => right.winner_median_ms - left.winner_median_ms,
  );
}

function comparisonKey(item) {
  return `${item.matrix}/${item.profile}/${item.base_case_id}`;
}

function iterationReductionPct(winnerIterations, slowestIterations) {
  if (!Number.isFinite(winnerIterations) || !Number.isFinite(slowestIterations)) return null;
  if (slowestIterations <= 0) return null;
  return ((slowestIterations - winnerIterations) / slowestIterations) * 100.0;
}

function reportComparisonMatchesLatest(run, comparison, latestByCase) {
  return (comparison.compared || []).every((item) => {
    const caseId = `${comparison.base_case_id}#${item.solver_preconditioner}`;
    const latest = latestByCase.get(`${run.benchmark.matrix}/${run.benchmark.profile}/${caseId}`);
    return latest?.run === run.name;
  });
}

function dedupeByPreconditioner(items) {
  const hasExplicitVariants = items.some((item) => item.case_id.includes("#"));
  const bestByPreconditioner = new Map();
  for (const item of items.filter((candidate) => !hasExplicitVariants || candidate.case_id.includes("#"))) {
    const current = bestByPreconditioner.get(item.solver_preconditioner);
    if (!current || item.median_ms < current.median_ms) {
      bestByPreconditioner.set(item.solver_preconditioner, item);
    }
  }
  return [...bestByPreconditioner.values()];
}

function baseCaseId(caseId) {
  return caseId.split("#")[0];
}

function preconditionerFromCaseId(caseId) {
  return caseId.includes("#") ? caseId.split("#").at(-1) : null;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function printUsage() {
  console.log(`Usage:
  node ./scripts/build-remote-material-benchmark-summary.mjs [options]

Options:
  --input-dir <dir>       Default: tmp/remote-material-research
  --json-out <path>       Default: tmp/remote-material-research/summary.json
  --markdown-out <path>   Default: tmp/remote-material-research/README.md`);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

if (isMain) {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    runRemoteMaterialBenchmarkSummarySelfTest(buildSummary);
    process.exit(0);
  }
  const runs = readRuns(options.inputDir);
  const summary = buildSummary(runs);
  fs.mkdirSync(path.dirname(options.jsonOut), { recursive: true });
  fs.writeFileSync(options.jsonOut, `${JSON.stringify(summary, null, 2)}\n`);
  writeRemoteMaterialBenchmarkMarkdown(summary, options.markdownOut);
  console.log(`remote material benchmark summary: ${options.markdownOut}`);
}
