#!/usr/bin/env node

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const DEFAULT_ROOT = path.resolve("tmp/benchmark-profile");
const DEFAULT_COVERAGE_TARGETS = path.resolve("config/benchmark-profile-coverage.json");

function parseArgs(argv) {
  const options = { root: DEFAULT_ROOT, coverageTargets: DEFAULT_COVERAGE_TARGETS };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--root" && next) {
      options.root = path.resolve(next);
      index += 1;
    } else if (arg === "--coverage-targets" && next) {
      options.coverageTargets = path.resolve(next);
      index += 1;
    }
  }

  return options;
}

async function exists(filePath) {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

async function readCoverageTargets(manifestPath) {
  let payload;
  try {
    payload = JSON.parse(await readFile(manifestPath, "utf8"));
  } catch (error) {
    throw new Error(
      `failed to read coverage targets ${manifestPath}: ${error instanceof Error ? error.message : String(error)}`,
    );
  }

  if (!Array.isArray(payload.targets) || payload.targets.length === 0) {
    throw new Error(`coverage targets ${manifestPath} must define a non-empty targets array`);
  }

  return payload.targets.map((target, index) => validateCoverageTarget(manifestPath, target, index));
}

function validateCoverageTarget(manifestPath, target, index) {
  const prefix = `coverage target ${index} in ${manifestPath}`;
  if (!target || typeof target !== "object") {
    throw new Error(`${prefix} must be an object`);
  }
  if (typeof target.matrix !== "string" || target.matrix.trim() === "") {
    throw new Error(`${prefix} must define a non-empty matrix`);
  }
  if (typeof target.profile !== "string" || target.profile.trim() === "") {
    throw new Error(`${prefix} must define a non-empty profile`);
  }
  if (!Array.isArray(target.expected_cases) || target.expected_cases.length === 0) {
    throw new Error(`${prefix} must define a non-empty expected_cases array`);
  }

  const expectedCases = target.expected_cases.map((id, caseIndex) => {
    if (typeof id !== "string" || id.trim() === "") {
      throw new Error(`${prefix} expected_cases[${caseIndex}] must be a non-empty string`);
    }
    return id;
  });
  const uniqueCases = new Set(expectedCases);
  if (uniqueCases.size !== expectedCases.length) {
    throw new Error(`${prefix} has duplicate expected_cases entries`);
  }

  return {
    matrix: target.matrix,
    profile: target.profile,
    expected_cases: expectedCases,
  };
}

async function discoverRuns(root) {
  let entries = [];
  try {
    entries = await readdir(root, { withFileTypes: true });
  } catch {
    return [];
  }

  const runs = [];
  const skipped = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }

    const runDir = path.join(root, entry.name);
    const summaryPath = path.join(runDir, "summary.json");
    if (!(await exists(summaryPath))) {
      continue;
    }

    let summary;
    try {
      summary = JSON.parse(await readFile(summaryPath, "utf8"));
    } catch (error) {
      skipped.push({
        slug: entry.name,
        reason: `failed to parse summary.json: ${error instanceof Error ? error.message : String(error)}`,
      });
      continue;
    }
    const info = await stat(summaryPath);
    runs.push({
      slug: entry.name,
      generated_at_unix_s: Math.floor(info.mtimeMs / 1000),
      profile: summary.profile ?? "unknown",
      matrix: summary.matrix ?? "unknown",
      case_count: summary.case_count ?? 0,
      case_ids: Array.isArray(summary.case_ids)
        ? summary.case_ids.filter((id) => typeof id === "string")
        : [],
      total_median_ms: summary.total_median_ms ?? 0,
      peak_rss_mib: summary.peak_rss_mib ?? 0,
      slowest_case: summary.slowest_case ?? "--",
      files: {
        summary_json: path.relative(root, summaryPath),
        readme_md: path.relative(root, path.join(runDir, "README.md")),
      },
    });
  }

  runs.sort((left, right) => right.generated_at_unix_s - left.generated_at_unix_s);
  skipped.sort((left, right) => left.slug.localeCompare(right.slug));
  return { runs, skipped };
}

function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value);
}

function evaluateGate(runs, skipped = [], coverageTargets = []) {
  const reasons = [];
  for (const entry of skipped) {
    reasons.push(`skipped run ${entry.slug}: ${entry.reason}`);
  }
  for (const entry of coverageSummaries(runs, coverageTargets)) {
    if (entry.missing_case_count > 0 && entry.covered_case_count > 0) {
      reasons.push(
        `coverage ${entry.matrix}/${entry.profile} missing ${entry.missing_case_count} case(s): ${entry.missing_cases.join(", ")}`,
      );
    }
  }
  if (runs.length === 0) {
    reasons.push("no retained benchmark profile runs");
    return { status: "warn", reasons };
  }

  const latest = runs[0];
  if (!Number.isInteger(latest.case_count) || latest.case_count <= 0) {
    reasons.push(`latest run ${latest.slug} has no benchmark cases`);
  }
  if (!finiteNumber(latest.total_median_ms) || latest.total_median_ms <= 0) {
    reasons.push(`latest run ${latest.slug} has invalid total median time`);
  }
  if (!finiteNumber(latest.peak_rss_mib) || latest.peak_rss_mib <= 0) {
    reasons.push(`latest run ${latest.slug} has invalid peak RSS`);
  }

  return {
    status: reasons.length > 0 ? "warn" : "pass",
    reasons,
  };
}

function matrixSummaries(runs) {
  const byMatrix = new Map();
  for (const run of runs) {
    const entry = byMatrix.get(run.matrix) ?? {
      matrix: run.matrix,
      run_count: 0,
      case_count: 0,
      total_median_ms: 0,
      peak_rss_mib: 0,
      slowest_case: "--",
      slowest_case_median_ms: 0,
    };
    entry.run_count += 1;
    entry.case_count += Number(run.case_count ?? 0);
    entry.total_median_ms += Number(run.total_median_ms ?? 0);
    entry.peak_rss_mib = Math.max(entry.peak_rss_mib, Number(run.peak_rss_mib ?? 0));
    if (Number(run.total_median_ms ?? 0) > entry.slowest_case_median_ms) {
      entry.slowest_case = run.slowest_case ?? "--";
      entry.slowest_case_median_ms = Number(run.total_median_ms ?? 0);
    }
    byMatrix.set(run.matrix, entry);
  }

  return [...byMatrix.values()].sort((left, right) => left.matrix.localeCompare(right.matrix));
}

function normalizeCaseId(id) {
  return String(id).split("#")[0];
}

function observedCaseIds(run) {
  if (Array.isArray(run.case_ids) && run.case_ids.length > 0) {
    return run.case_ids.map(normalizeCaseId);
  }
  return [normalizeCaseId(run.slowest_case)];
}

function coverageSummaries(runs, coverageTargets) {
  return coverageTargets.map((target) => {
    const observed = new Set(
      runs
        .filter((run) => run.matrix === target.matrix && run.profile === target.profile)
        .flatMap((run) => observedCaseIds(run)),
    );
    const covered_cases = target.expected_cases.filter((id) => observed.has(id));
    const missing_cases = target.expected_cases.filter((id) => !observed.has(id));
    return {
      matrix: target.matrix,
      profile: target.profile,
      expected_case_count: target.expected_cases.length,
      covered_case_count: covered_cases.length,
      missing_case_count: missing_cases.length,
      covered_cases,
      missing_cases,
    };
  });
}

function renderReadme(root, coverageManifestPath, coverageTargets, runs, skipped) {
  const gate = evaluateGate(runs, skipped, coverageTargets);
  const matrices = matrixSummaries(runs);
  const coverage = coverageSummaries(runs, coverageTargets);
  const lines = [
    "# Benchmark Profile Runs",
    "",
    `- Root: \`${path.relative(process.cwd(), root) || "."}\``,
    `- Coverage targets: \`${path.relative(process.cwd(), coverageManifestPath) || "."}\``,
    `- Indexed runs: \`${runs.length}\``,
    `- Skipped runs: \`${skipped.length}\``,
    `- Gate status: \`${gate.status}\``,
    "",
  ];

  for (const reason of gate.reasons) {
    lines.push(`- Gate reason: ${reason}`);
  }
  if (gate.reasons.length > 0) {
    lines.push("");
  }

  if (runs.length === 0) {
    lines.push("No benchmark profile runs were found.");
    lines.push("");
    return `${lines.join("\n").trimEnd()}\n`;
  }

  lines.push("## Matrix summaries");
  lines.push("");
  lines.push("| Matrix | Runs | Cases | Total median ms | Peak RSS MiB | Slowest case |");
  lines.push("| --- | ---: | ---: | ---: | ---: | --- |");
  for (const entry of matrices) {
    lines.push(
      `| \`${entry.matrix}\` | \`${entry.run_count}\` | \`${entry.case_count}\` | \`${entry.total_median_ms.toFixed(3)}\` | \`${entry.peak_rss_mib.toFixed(1)}\` | \`${entry.slowest_case}\` |`,
    );
  }
  lines.push("");
  lines.push("## Coverage summaries");
  lines.push("");
  lines.push("| Matrix | Profile | Covered | Missing | Missing cases |");
  lines.push("| --- | --- | ---: | ---: | --- |");
  for (const entry of coverage) {
    lines.push(
      `| \`${entry.matrix}\` | \`${entry.profile}\` | \`${entry.covered_case_count}/${entry.expected_case_count}\` | \`${entry.missing_case_count}\` | ${entry.missing_cases.map((id) => `\`${id}\``).join(", ") || "--"} |`,
    );
  }
  lines.push("");
  lines.push("## Runs");
  lines.push("");
  lines.push("| Slug | Profile | Matrix | Cases | Total median ms | Peak RSS MiB | Slowest case |");
  lines.push("| --- | --- | --- | ---: | ---: | ---: | --- |");
  for (const run of runs) {
    lines.push(
      `| \`${run.slug}\` | \`${run.profile}\` | \`${run.matrix}\` | \`${run.case_count}\` | \`${Number(run.total_median_ms).toFixed(3)}\` | \`${Number(run.peak_rss_mib).toFixed(1)}\` | \`${run.slowest_case}\` |`,
    );
  }
  lines.push("");
  if (skipped.length > 0) {
    lines.push("## Skipped runs");
    lines.push("");
    lines.push("| Slug | Reason |");
    lines.push("| --- | --- |");
    for (const entry of skipped) {
      lines.push(`| \`${entry.slug}\` | ${entry.reason} |`);
    }
    lines.push("");
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const coverageTargets = await readCoverageTargets(options.coverageTargets);
  const { runs, skipped } = await discoverRuns(options.root);
  const gate = evaluateGate(runs, skipped, coverageTargets);
  const payload = {
    schema_version: "kyuubiki.benchmark-profile-index/v1",
    root: path.relative(process.cwd(), options.root) || ".",
    coverage_targets_manifest: path.relative(process.cwd(), options.coverageTargets) || ".",
    generated_at_unix_s: Math.floor(Date.now() / 1000),
    gate,
    coverage_summaries: coverageSummaries(runs, coverageTargets),
    matrix_summaries: matrixSummaries(runs),
    skipped_runs: skipped,
    retained_runs: runs,
  };

  await writeFile(path.join(options.root, "index.json"), `${JSON.stringify(payload, null, 2)}\n`);
  await writeFile(
    path.join(options.root, "README.md"),
    renderReadme(options.root, options.coverageTargets, coverageTargets, runs, skipped),
  );
  process.stdout.write(`${path.relative(process.cwd(), options.root) || "."}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
