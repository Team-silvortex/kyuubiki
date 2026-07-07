#!/usr/bin/env node
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

function parseArgs(argv) {
  const options = {
    input: "tmp/remote-material-research/summary.json",
    maxStageSharePct: 105,
  };
  for (let index = 2; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--self-test") {
      options.selfTest = true;
    } else if (arg === "--in" && next) {
      options.input = next;
      index += 1;
    } else if (arg === "--max-stage-share-pct" && next) {
      options.maxStageSharePct = Number(next);
      index += 1;
    } else {
      fail(`unknown or incomplete argument: ${arg}`);
    }
  }
  validateOptions(options);
  return options;
}

function validateOptions(options) {
  const issues = optionIssues(options);
  if (issues.length > 0) fail(issues[0]);
}

export function optionIssues(options) {
  const issues = [];
  if (!Number.isFinite(options.maxStageSharePct) || options.maxStageSharePct <= 0) {
    issues.push("--max-stage-share-pct must be a finite positive number");
  }
  return issues;
}

export function stageHealthIssues(summary, options) {
  const hotspots = summary.latest_stage_hotspots || [];
  if (hotspots.length === 0) {
    return ["summary has no latest_stage_hotspots"];
  }
  const issues = [];
  issues.push(...optimizationTargetIssues(summary.latest_optimization_targets || []));
  issues.push(...preconditionerEconomicsIssues(summary.latest_preconditioner_economics || []));
  issues.push(...solverTuningNoteIssues(summary.latest_solver_tuning_notes || []));
  issues.push(...sparseMatvecThroughputIssues(summary.latest_sparse_matvec_throughput || []));
  issues.push(...stageSummaryIssues(summary.latest_stage_summary || []));
  for (const item of hotspots) {
    const context = `${item.matrix}/${item.profile}/${item.case_id}/${item.stage}`;
    if (typeof item.stage !== "string" || item.stage.length === 0) {
      issues.push(`${context}: missing non-empty stage`);
    }
    if (!Number.isFinite(item.elapsed_ms) || item.elapsed_ms < 0) {
      issues.push(`${context}: missing finite non-negative elapsed_ms`);
    }
    if (!Number.isFinite(item.stage_share_pct)) {
      issues.push(`${context}: missing finite stage_share_pct`);
    } else if (item.stage_share_pct > options.maxStageSharePct) {
      issues.push(
        `${context}: stage share ${item.stage_share_pct.toFixed(2)}% > ${options.maxStageSharePct}%`,
      );
    }
  }
  return issues;
}

function solverTuningNoteIssues(rows) {
  if (rows.length === 0) return ["summary has no latest_solver_tuning_notes"];
  const issues = [];
  for (const item of rows) {
    const context = `${item.matrix}/${item.profile}/${item.case_id}`;
    if (typeof item.focus !== "string" || item.focus.length === 0) {
      issues.push(`${context}: missing tuning focus`);
    }
    if (typeof item.reason !== "string" || item.reason.length === 0) {
      issues.push(`${context}: missing tuning reason`);
    }
    for (const field of ["winner_pre_ms_per_iteration", "winner_matvec_ms_per_iteration"]) {
      if (!Number.isFinite(item[field])) {
        issues.push(`${context}: missing finite ${field}`);
      }
    }
  }
  return issues;
}

function preconditionerEconomicsIssues(rows) {
  if (rows.length === 0) return ["summary has no latest_preconditioner_economics"];
  const issues = [];
  for (const item of rows) {
    const context = `${item.matrix}/${item.profile}/${item.base_case_id}`;
    if (typeof item.winner_preconditioner !== "string" || item.winner_preconditioner.length === 0) {
      issues.push(`${context}: missing winner_preconditioner`);
    }
    if (typeof item.slowest_preconditioner !== "string" || item.slowest_preconditioner.length === 0) {
      issues.push(`${context}: missing slowest_preconditioner`);
    }
    for (const field of ["elapsed_saved_ms", "iterations_saved", "winner_speedup_ratio"]) {
      if (!Number.isFinite(item[field])) {
        issues.push(`${context}: missing finite ${field}`);
      }
    }
    for (const field of [
      "extra_pre_ms_per_iteration_saved",
      "gross_non_preconditioner_saved_ms",
      "ms_saved_per_iteration_saved",
      "preconditioner_extra_ms",
      "winner_preconditioner_ms",
      "slowest_preconditioner_ms",
      "winner_pre_ms_per_iteration",
      "slowest_pre_ms_per_iteration",
      "winner_matvec_ms_per_iteration",
      "slowest_matvec_ms_per_iteration",
    ]) {
      if (item[field] != null && !Number.isFinite(item[field])) {
        issues.push(`${context}: invalid ${field}`);
      }
    }
  }
  return issues;
}

function sparseMatvecThroughputIssues(rows) {
  const issues = [];
  for (const item of rows) {
    const context = `${item.matrix}/${item.profile}/${item.stage}`;
    if (item.stage !== "solve_spd_matvec") {
      issues.push(`${context}: sparse matvec throughput row has non-matvec stage`);
    }
    if (!Number.isFinite(item.non_zero_visit_count) || item.non_zero_visit_count <= 0) {
      issues.push(`${context}: missing positive non_zero_visit_count`);
    }
    if (
      !Number.isFinite(item.ms_per_million_non_zero_visits) ||
      item.ms_per_million_non_zero_visits < 0
    ) {
      issues.push(`${context}: missing finite non-negative ms_per_million_non_zero_visits`);
    }
  }
  return issues;
}

function optimizationTargetIssues(targets) {
  if (targets.length === 0) return ["summary has no latest_optimization_targets"];
  const issues = [];
  for (const item of targets) {
    const context = `${item.matrix}/${item.profile}/${item.stage}`;
    if (typeof item.stage !== "string" || item.stage.length === 0) {
      issues.push(`${context}: missing non-empty target stage`);
    }
    if (typeof item.focus !== "string" || item.focus.length === 0) {
      issues.push(`${context}: missing non-empty target focus`);
    }
    if (!Number.isFinite(item.case_count) || item.case_count <= 0) {
      issues.push(`${context}: missing positive target case_count`);
    }
    if (!Number.isFinite(item.elapsed_ms_total) || item.elapsed_ms_total < 0) {
      issues.push(`${context}: missing finite non-negative target elapsed_ms_total`);
    }
    if (!Number.isFinite(item.priority_score) || item.priority_score < 0) {
      issues.push(`${context}: missing finite non-negative priority_score`);
    }
    if (
      item.ms_per_million_non_zero_visits != null &&
      (!Number.isFinite(item.ms_per_million_non_zero_visits) ||
        item.ms_per_million_non_zero_visits < 0)
    ) {
      issues.push(`${context}: invalid target ms_per_million_non_zero_visits`);
    }
  }
  return issues;
}

function stageSummaryIssues(summaryRows) {
  if (summaryRows.length === 0) return ["summary has no latest_stage_summary"];
  const issues = [];
  for (const item of summaryRows) {
    const context = `${item.matrix}/${item.profile}/${item.stage}`;
    if (typeof item.stage !== "string" || item.stage.length === 0) {
      issues.push(`${context}: missing non-empty summary stage`);
    }
    if (!Number.isFinite(item.case_count) || item.case_count <= 0) {
      issues.push(`${context}: missing positive summary case_count`);
    }
    if (!Number.isFinite(item.elapsed_ms_total) || item.elapsed_ms_total < 0) {
      issues.push(`${context}: missing finite non-negative elapsed_ms_total`);
    }
    if (!Number.isFinite(item.max_elapsed_ms) || item.max_elapsed_ms < 0) {
      issues.push(`${context}: missing finite non-negative max_elapsed_ms`);
    }
    if (
      item.ms_per_million_non_zero_visits != null &&
      (!Number.isFinite(item.ms_per_million_non_zero_visits) ||
        item.ms_per_million_non_zero_visits < 0)
    ) {
      issues.push(`${context}: invalid summary ms_per_million_non_zero_visits`);
    }
  }
  return issues;
}

function checkSummary(summary, options) {
  const issues = stageHealthIssues(summary, options);
  if (issues.length > 0) {
    console.error("remote material stage health failed:");
    for (const issue of issues) console.error(`- ${issue}`);
    process.exit(1);
  }
  console.log(
    `remote material stage health ok: ${summary.latest_stage_hotspots.length} hotspots, max share ${options.maxStageSharePct}%`,
  );
}

function readRepoJson(relativePath) {
  const absolute = path.resolve(repoRoot, relativePath);
  const relative = path.relative(repoRoot, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    fail("--in must stay inside the repository");
  }
  if (!fs.existsSync(absolute)) {
    fail(`input does not exist: ${relative}`);
  }
  try {
    return JSON.parse(fs.readFileSync(absolute, "utf8"));
  } catch (error) {
    fail(`failed to parse ${relative}: ${error.message}`);
  }
}

function runSelfTest() {
  const options = { maxStageSharePct: 105 };
  assert.deepEqual(stageHealthIssues(fakeSummary(99, 10), options), []);
  assert.match(stageHealthIssues(fakeSummary(106, 10), options)[0], /stage share/);
  assert.match(stageHealthIssues(fakeSummary(Number.NaN, 10), options)[0], /stage_share_pct/);
  assert.match(stageHealthIssues(fakeSummary(99, -1), options)[0], /elapsed_ms/);
  assert.match(stageHealthIssues(fakeSummary(99, 10, ""), options)[0], /stage/);
  assert.match(
    stageHealthIssues({ ...fakeSummary(99, 10), latest_stage_summary: [] }, options)[0],
    /latest_stage_summary/,
  );
  assert.match(
    stageHealthIssues({ ...fakeSummary(99, 10), latest_optimization_targets: [] }, options)[0],
    /latest_optimization_targets/,
  );
  assert.match(
    stageHealthIssues({ ...fakeSummary(99, 10), latest_preconditioner_economics: [] }, options)[0],
    /latest_preconditioner_economics/,
  );
  assert.match(
    stageHealthIssues({ ...fakeSummary(99, 10), latest_solver_tuning_notes: [] }, options)[0],
    /latest_solver_tuning_notes/,
  );
  assert.match(
    stageHealthIssues(
      {
        ...fakeSummary(99, 10),
        latest_sparse_matvec_throughput: [
          { ...fakeSummary(99, 10).latest_sparse_matvec_throughput[0], stage: "solve_spd_dot" },
        ],
      },
      options,
    )[0],
    /non-matvec/,
  );
  assert.match(stageHealthIssues({ latest_stage_hotspots: [] }, options)[0], /no latest/);
  assert.match(optionIssues({ maxStageSharePct: 0 })[0], /positive/);
  console.log("remote material stage health self-test passed");
}

function fakeSummary(stageSharePct, elapsedMs, stage = "solve_spd_matvec") {
  return {
    latest_stage_hotspots: [
      {
        case_id: "panel-10k",
        elapsed_ms: elapsedMs,
        matrix: "mechanical-core",
        profile: "ten_k",
        stage,
        stage_share_pct: stageSharePct,
      },
    ],
    latest_optimization_targets: [
      {
        case_count: 1,
        elapsed_ms_total: elapsedMs,
        focus: "sparse-matvec-throughput",
        matrix: "mechanical-core",
        priority_score: elapsedMs,
        profile: "ten_k",
        stage,
      },
    ],
    latest_preconditioner_economics: [
      {
        base_case_id: "panel-10k",
        elapsed_saved_ms: 1,
        extra_pre_ms_per_iteration_saved: 1,
        gross_non_preconditioner_saved_ms: 2,
        iterations_saved: 1,
        matrix: "mechanical-core",
        ms_saved_per_iteration_saved: 1,
        preconditioner_extra_ms: 1,
        profile: "ten_k",
        slowest_preconditioner: "jacobi",
        slowest_matvec_ms_per_iteration: 1,
        slowest_preconditioner_ms: 1,
        slowest_pre_ms_per_iteration: 1,
        winner_matvec_ms_per_iteration: 1,
        winner_preconditioner: "symmetric-gauss-seidel",
        winner_preconditioner_ms: 2,
        winner_pre_ms_per_iteration: 2,
        winner_speedup_ratio: 1.2,
      },
    ],
    latest_solver_tuning_notes: [
      {
        case_id: "panel-10k",
        focus: "sgs-sweep-cost",
        matrix: "mechanical-core",
        profile: "ten_k",
        reason: "winner preconditioner cost per iteration is above matvec cost per iteration",
        winner_matvec_ms_per_iteration: 1,
        winner_pre_ms_per_iteration: 2,
      },
    ],
    latest_sparse_matvec_throughput: [
      {
        case_count: 1,
        elapsed_ms_total: elapsedMs,
        matrix: "mechanical-core",
        ms_per_million_non_zero_visits: 1,
        non_zero_elapsed_ms_total: elapsedMs,
        non_zero_visit_count: 1_000_000,
        profile: "ten_k",
        stage,
      },
    ],
    latest_stage_summary: [
      {
        case_count: 1,
        elapsed_ms_total: elapsedMs,
        matrix: "mechanical-core",
        max_elapsed_ms: elapsedMs,
        profile: "ten_k",
        stage,
      },
    ],
  };
}

function fail(message) {
  console.error(`remote material stage health failed: ${message}`);
  process.exit(1);
}

if (isMain) {
  const options = parseArgs(process.argv);
  if (options.selfTest) {
    runSelfTest();
  } else {
    checkSummary(readRepoJson(options.input), options);
  }
}
