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
    minIterationReductionPct: 10,
    minSpeedupRatio: 1.05,
    winner: "symmetric-gauss-seidel",
  };
  for (let index = 2; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--self-test") {
      options.selfTest = true;
    } else if (arg === "--in" && next) {
      options.input = next;
      index += 1;
    } else if (arg === "--min-speedup-ratio" && next) {
      options.minSpeedupRatio = Number(next);
      index += 1;
    } else if (arg === "--min-iteration-reduction-pct" && next) {
      options.minIterationReductionPct = Number(next);
      index += 1;
    } else if (arg === "--winner" && next) {
      options.winner = next;
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
  if (issues.length > 0) {
    fail(issues[0]);
  }
}

export function optionIssues(options) {
  const issues = [];
  if (!Number.isFinite(options.minSpeedupRatio) || options.minSpeedupRatio < 0) {
    issues.push("--min-speedup-ratio must be a finite non-negative number");
  }
  if (
    !Number.isFinite(options.minIterationReductionPct) ||
    options.minIterationReductionPct < 0
  ) {
    issues.push("--min-iteration-reduction-pct must be a finite non-negative number");
  }
  if (typeof options.winner !== "string" || options.winner.trim() === "") {
    issues.push("--winner must be non-empty");
  }
  return issues;
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
    return parseJsonText(fs.readFileSync(absolute, "utf8"), relative);
  } catch (error) {
    fail(error.message);
  }
}

function parseJsonText(text, context) {
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(`failed to parse ${context}: ${error.message}`);
  }
}

function checkSummary(summary, options) {
  const issues = healthIssues(summary, options);
  if (issues.length > 0) {
    console.error("remote material preconditioner health failed:");
    for (const issue of issues) console.error(`- ${issue}`);
    process.exit(1);
  }
  const comparisons = summary.latest_preconditioner_comparisons || [];
  console.log(
    `remote material preconditioner health ok: ${comparisons.length} comparisons, min speedup ${options.minSpeedupRatio}x`,
  );
}

export function healthIssues(summary, options) {
  const comparisons = summary.latest_preconditioner_comparisons || [];
  if (comparisons.length === 0) {
    return ["summary has no latest_preconditioner_comparisons"];
  }
  const issues = [];
  for (const item of comparisons) {
    const context = `${item.matrix}/${item.profile}/${item.base_case_id}`;
    if (item.winner_preconditioner !== options.winner) {
      issues.push(`${context}: winner ${item.winner_preconditioner} != ${options.winner}`);
    }
    if (!Number.isFinite(item.winner_speedup_ratio)) {
      issues.push(`${context}: missing finite winner_speedup_ratio`);
    } else if (item.winner_speedup_ratio < options.minSpeedupRatio) {
      issues.push(
        `${context}: speedup ${item.winner_speedup_ratio.toFixed(3)} < ${options.minSpeedupRatio}`,
      );
    }
    if (!Number.isFinite(item.winner_iteration_reduction_pct)) {
      issues.push(`${context}: missing finite winner_iteration_reduction_pct`);
    } else if (item.winner_iteration_reduction_pct < options.minIterationReductionPct) {
      issues.push(
        `${context}: iteration reduction ${item.winner_iteration_reduction_pct.toFixed(2)}% < ${options.minIterationReductionPct}%`,
      );
    }
  }
  return issues;
}

function runSelfTest() {
  const options = {
    minIterationReductionPct: 10,
    minSpeedupRatio: 1.05,
    winner: "symmetric-gauss-seidel",
  };
  assert.deepEqual(healthIssues(fakeSummary(1.2, 25, "symmetric-gauss-seidel"), options), []);
  assert.match(
    healthIssues(fakeSummary(1.01, 25, "symmetric-gauss-seidel"), options)[0],
    /speedup/,
  );
  assert.match(
    healthIssues(fakeSummary(1.2, 5, "symmetric-gauss-seidel"), options)[0],
    /iteration reduction/,
  );
  assert.match(healthIssues(fakeSummary(1.2, 25, "jacobi"), options)[0], /winner/);
  assert.match(healthIssues({ latest_preconditioner_comparisons: [] }, options)[0], /no latest/);
  assert.match(
    optionIssues({
      minIterationReductionPct: 10,
      minSpeedupRatio: Number.NaN,
      winner: "symmetric-gauss-seidel",
    })[0],
    /finite/,
  );
  assert.throws(() => parseJsonText("{", "broken.json"), /failed to parse broken\.json/);
  console.log("remote material preconditioner health self-test passed");
}

function fakeSummary(speedup, iterationReductionPct, winner) {
  return {
    latest_preconditioner_comparisons: [
      {
        base_case_id: "panel-10k",
        matrix: "mechanical-core",
        profile: "ten_k",
        winner_iteration_reduction_pct: iterationReductionPct,
        winner_preconditioner: winner,
        winner_speedup_ratio: speedup,
      },
    ],
  };
}

function fail(message) {
  console.error(`remote material preconditioner health failed: ${message}`);
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
