#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultOut = "tmp/line-field-qualification-release-evidence.json";
const promotionSummary = {
  candidate_id: "line-field-closed-form",
  release_version: "2.0.0",
  approved_coverage_level: "qualification",
  retained_evidence_path:
    "releases/qualification-evidence/2.0.0/line-field-closed-form-release-evidence.json",
  release_record_path: "releases/qualification-records/1.20.0.json",
  review_decision_path:
    "releases/qualification-review-decisions/2.0.0/line-field-closed-form-review-decision.json",
  promoted_operator_ids: [
    "solve.bar_1d",
    "solve.thermal_bar_1d",
    "solve.heat_bar_1d",
    "solve.electrostatic_bar_1d",
  ],
};
const evidenceCommands = [
  {
    id: "evidence_check",
    cwd: ".",
    command: "node",
    args: ["./scripts/check-line-field-closed-form-baseline.mjs"],
  },
  {
    id: "solver_baseline",
    cwd: "workers/rust",
    command: "cargo",
    args: ["test", "-p", "kyuubiki-solver", "--test", "accuracy_baselines", "line_1d"],
  },
];

function fail(message) {
  console.error(`line-field qualification release evidence capture failed: ${message}`);
  process.exit(1);
}

function relativeOutputPath(outPath) {
  const absoluteOut = path.resolve(repoRoot, outPath);
  const relativeOut = path.relative(repoRoot, absoluteOut);
  if (relativeOut.startsWith("..") || path.isAbsolute(relativeOut)) {
    fail("--out must stay inside the repository");
  }
  return { absoluteOut, relativeOut };
}

function parseArgs(argv) {
  const args = { out: defaultOut, allowFailure: false };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--out") {
      args.out = argv[index + 1];
      index += 1;
    } else if (argv[index] === "--allow-failure") {
      args.allowFailure = true;
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
  }
  if (!args.out) {
    fail("--out requires a repo-local path");
  }
  return args;
}

function run(commandSpec) {
  const startedAt = Date.now();
  const result = spawnSync(commandSpec.command, commandSpec.args, {
    cwd: path.join(repoRoot, commandSpec.cwd),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    id: commandSpec.id,
    cwd: commandSpec.cwd,
    argv: [commandSpec.command, ...commandSpec.args],
    status: result.status,
    signal: result.signal,
    duration_ms: Date.now() - startedAt,
    stdout: result.stdout,
    stderr: result.stderr,
    ok: result.status === 0,
  };
}

function captureProvenance() {
  const result = spawnSync("node", ["./scripts/capture-line-field-qualification-provenance.mjs"], {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    fail(result.stderr.trim() || "provenance capture command failed");
  }
  return JSON.parse(result.stdout);
}

function buildEvidence() {
  const commandResults = evidenceCommands.map(run);
  return {
    schema_version: "kyuubiki.operator-qualification-release-evidence/v1",
    version_line: "moxi 2.0.x",
    candidate_id: "line-field-closed-form",
    generated_at_utc: new Date().toISOString(),
    release_retention: {
      intended_release_artifact: true,
      repo_relative_paths_only: true,
      generated_output_should_not_be_committed_directly: true,
    },
    promotion_summary: promotionSummary,
    provenance: captureProvenance(),
    commands: commandResults,
    summary: {
      command_count: commandResults.length,
      passed: commandResults.filter((result) => result.ok).length,
      failed: commandResults.filter((result) => !result.ok).length,
      ok: commandResults.every((result) => result.ok),
    },
  };
}

function writeEvidence(outPath, payload) {
  const { absoluteOut, relativeOut } = relativeOutputPath(outPath);
  fs.mkdirSync(path.dirname(absoluteOut), { recursive: true });
  fs.writeFileSync(absoluteOut, `${JSON.stringify(payload, null, 2)}\n`);
  console.log(`line-field qualification release evidence wrote ${relativeOut}`);
}

const args = parseArgs(process.argv);
const payload = buildEvidence();
writeEvidence(args.out, payload);
if (!args.allowFailure && !payload.summary.ok) {
  process.exit(1);
}
