#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultInput = "tmp/material-research-example.json";
const manifestPath = "docs/automated-material-research-example.manifest.json";

function fail(message) {
  console.error(`material research example check failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { input: defaultInput };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--in") {
      args.input = argv[++index];
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
  }
  if (!args.input) {
    fail("--in requires a repo-local path");
  }
  return args;
}

function readEvidence(inputPath) {
  const absolute = path.resolve(repoRoot, inputPath);
  const relative = path.relative(repoRoot, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    fail("--in must stay inside the repository");
  }
  if (!fs.existsSync(absolute)) {
    fail(`input does not exist: ${relative}`);
  }
  return JSON.parse(fs.readFileSync(absolute, "utf8"));
}

function readRepoJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function requireMarkdownContains(markdown, value, context) {
  if (!markdown.includes(value)) {
    fail(`documentation missing ${context}: ${value}`);
  }
}

function assertNoAbsoluteRepoPath(value, context) {
  if (typeof value === "string" && value.includes(repoRoot)) {
    fail(`${context}: contains local absolute repository path`);
  }
  if (Array.isArray(value)) {
    value.forEach((entry, index) => assertNoAbsoluteRepoPath(entry, `${context}[${index}]`));
    return;
  }
  if (value && typeof value === "object") {
    for (const [key, nested] of Object.entries(value)) {
      assertNoAbsoluteRepoPath(nested, `${context}.${key}`);
    }
  }
}

function assertFiniteNumber(value, context) {
  if (!Number.isFinite(value)) {
    fail(`${context}: expected finite number`);
  }
}

function assertEquals(actual, expected, context) {
  if (actual !== expected) {
    fail(`${context}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

function validateCommand(command, manifest) {
  assertEquals(command?.id, manifest.command.id, "command.id");
  assertEquals(command?.cwd, manifest.command.cwd, "command.cwd");
  if (!Array.isArray(command.argv) || command.argv.length < 8) {
    fail("command.argv must include the cargo material exploration command");
  }
  for (const expected of manifest.command.argv_contains) {
    if (!command.argv.includes(expected)) {
      fail(`command.argv missing ${expected}`);
    }
  }
  if (command.ok !== true || command.status !== 0) {
    fail("command must pass with status 0");
  }
  assertFiniteNumber(command.duration_ms, "command.duration_ms");
}

function validateNextRoundCommand(inputPath, manifest) {
  const absoluteInputPath = path.resolve(repoRoot, inputPath);
  const command = materialExploreCommand(
    manifest.next_round_command,
    "--plan-next",
    absoluteInputPath,
  );
  for (const expected of manifest.next_round_command.argv_contains) {
    if (!command.argv.includes(expected)) {
      fail(`next_round_command.argv missing ${expected}`);
    }
  }
  const result = spawnSync(command.argv[0], command.argv.slice(1), {
    cwd: path.join(repoRoot, command.cwd),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    fail(result.stderr.trim() || "next round execution plan command failed");
  }
  let plan;
  try {
    plan = JSON.parse(result.stdout);
  } catch (error) {
    fail(`next round execution plan did not emit JSON: ${error.message}`);
  }
  assertEquals(
    plan.schema_version,
    manifest.expected.next_round_execution_schema_version,
    "next round execution schema",
  );
  if (!Array.isArray(plan.steps) || plan.steps.length === 0) {
    fail("next round execution plan must include runnable steps");
  }
  assertFiniteNumber(plan.runnable_step_count, "next_round_execution.runnable_step_count");
  assertEquals(plan.runnable_step_count, plan.steps.length, "next round runnable step count");
}

function validateRunNextCommand(inputPath, manifest) {
  const absoluteInputPath = path.resolve(repoRoot, inputPath);
  const command = materialExploreCommand(
    manifest.run_next_command,
    "--run-next",
    absoluteInputPath,
  );
  for (const expected of manifest.run_next_command.argv_contains) {
    if (!command.argv.includes(expected)) {
      fail(`run_next_command.argv missing ${expected}`);
    }
  }
  const result = spawnSync(command.argv[0], command.argv.slice(1), {
    cwd: path.join(repoRoot, command.cwd),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    fail(result.stderr.trim() || "run-next command failed");
  }
  let exploration;
  try {
    exploration = JSON.parse(result.stdout);
  } catch (error) {
    fail(`run-next did not emit JSON: ${error.message}`);
  }
  assertEquals(
    exploration.schema_version,
    manifest.expected.exploration_schema_version,
    "run-next exploration schema",
  );
  assertEquals(exploration.mode, "local_solver_next_round", "run-next mode");
  assertEquals(
    exploration.candidate_count,
    manifest.expected.candidate_count,
    "run-next candidate count",
  );
  validateNextRound(exploration.next_round, manifest);
}

function materialExploreCommand(commandManifest, mode, inputPath) {
  return {
    ...commandManifest,
    argv: [
      "cargo",
      "run",
      "-q",
      "-p",
      "kyuubiki-cli",
      "--bin",
      "kyuubiki-material-explore",
      "--",
      mode,
      inputPath,
      "--json",
    ],
  };
}

function validateReport(report, manifest) {
  const expected = manifest.expected;
  assertEquals(report?.schema_version, expected.report_schema_version, "report schema");
  assertEquals(
    report.winner_candidate_id,
    expected.winner_candidate_id,
    "winner_candidate_id",
  );
  assertEquals(
    report.optimization?.id,
    expected.optimization_id,
    "optimization.id",
  );
  assertEquals(report.reliability?.posture, expected.reliability_posture, "reliability.posture");
  for (const section of manifest.required_report_sections) {
    if (report[section] == null) {
      fail(`report missing section ${section}`);
    }
  }
  for (const section of manifest.required_reliability_sections) {
    if (report.reliability?.[section] == null) {
      fail(`reliability missing section ${section}`);
    }
  }
  if (!Array.isArray(report.candidates) || report.candidates.length !== expected.candidate_count) {
    fail("report.candidates must contain exactly three candidates");
  }
  for (const [index, candidate] of report.candidates.entries()) {
    assertEquals(candidate.rank, index + 1, `candidate[${index}].rank`);
    for (const field of manifest.required_candidate_fields) {
      if (candidate[field] == null) {
        fail(`candidate[${index}] missing ${field}`);
      }
    }
    assertFiniteNumber(candidate.score, `candidate[${index}].score`);
    assertFiniteNumber(candidate.peak_temperature_c, `candidate[${index}].peak_temperature_c`);
    assertFiniteNumber(candidate.areal_mass_kg_m2, `candidate[${index}].areal_mass_kg_m2`);
    if (!Array.isArray(candidate.optimization_terms) || candidate.optimization_terms.length < 3) {
      fail(`candidate[${index}] must expose optimization terms`);
    }
  }
  if (
    !Array.isArray(report.reliability?.quality_gates) ||
    report.reliability.quality_gates.length < 3
  ) {
    fail("reliability quality gates must be present");
  }
  if (!Array.isArray(report.reliability?.limitations) || report.reliability.limitations.length < 3) {
    fail("reliability limitations must stay visible");
  }
}

function validateExploration(exploration, manifest) {
  const expected = manifest.expected;
  assertEquals(
    exploration?.schema_version,
    expected.exploration_schema_version,
    "exploration schema",
  );
  assertEquals(exploration.study, expected.exploration_study, "exploration.study");
  assertEquals(exploration.mode, expected.mode, "exploration.mode");
  assertEquals(exploration.candidate_count, expected.candidate_count, "exploration.candidate_count");
  if (
    !Array.isArray(exploration.result_payloads) ||
    exploration.result_payloads.length !== expected.result_payload_count
  ) {
    fail("exploration.result_payloads must contain exactly three solver outputs");
  }
  for (const [index, result] of exploration.result_payloads.entries()) {
    assertFiniteNumber(result.max_temperature, `result_payloads[${index}].max_temperature`);
    assertFiniteNumber(result.max_heat_flux, `result_payloads[${index}].max_heat_flux`);
    if (!Array.isArray(result.nodes) || result.nodes.length !== 4) {
      fail(`result_payloads[${index}] must expose four thermal nodes`);
    }
  }
  validateReport(exploration.report, manifest);
  validateNextRound(exploration.next_round, manifest);
}

function validateNextRound(nextRound, manifest) {
  if (nextRound == null || typeof nextRound !== "object") {
    fail("exploration.next_round must be present");
  }
  const expected = manifest.expected;
  assertEquals(
    nextRound?.schema_version,
    expected.next_round_schema_version,
    "next_round schema",
  );
  for (const section of manifest.required_next_round_sections) {
    if (nextRound[section] == null) {
      fail(`next_round missing section ${section}`);
    }
  }
  if (!manifest.allowed_next_round_decisions.includes(nextRound.decision)) {
    fail(`next_round.decision is not allowed: ${nextRound.decision}`);
  }
  if (!Number.isInteger(nextRound.iteration) || nextRound.iteration < 2) {
    fail("next_round.iteration must point at a future iteration");
  }
  if (!Array.isArray(nextRound.focus_candidate_ids) || nextRound.focus_candidate_ids.length === 0) {
    fail("next_round.focus_candidate_ids must name at least one candidate");
  }
  if (!Array.isArray(nextRound.actions) || nextRound.actions.length === 0) {
    fail("next_round.actions must include at least one action");
  }
  if (!Array.isArray(nextRound.rationale) || nextRound.rationale.length === 0) {
    fail("next_round.rationale must explain the decision");
  }
}

function validateDocumentation(manifest) {
  const markdown = fs.readFileSync(path.join(repoRoot, manifest.documentation), "utf8");
  requireMarkdownContains(markdown, manifest.output_path, "output path");
  requireMarkdownContains(markdown, manifest.verify_target, "verify target");
  requireMarkdownContains(markdown, manifest.expected.winner_candidate_id, "expected winner");
  requireMarkdownContains(markdown, manifest.expected.optimization_id, "optimization id");
  requireMarkdownContains(markdown, manifest.expected.reliability_posture, "reliability posture");
  requireMarkdownContains(markdown, manifest.expected.next_round_schema_version, "next round schema");
}

function validateEvidence(evidence, manifest) {
  const expected = manifest.expected;
  assertEquals(
    evidence.schema_version,
    expected.evidence_schema_version,
    "evidence schema",
  );
  assertEquals(evidence.example_id, manifest.example_id, "example_id");
  assertEquals(evidence.posture, "screening_research_example", "posture");
  assertEquals(evidence.summary?.winner_candidate_id, expected.winner_candidate_id, "summary winner");
  assertEquals(evidence.summary?.result_payload_count, expected.result_payload_count, "summary result count");
  validateCommand(evidence.command, manifest);
  validateExploration(evidence.exploration, manifest);
  assertNoAbsoluteRepoPath(evidence, "evidence");
}

const args = parseArgs(process.argv);
const manifest = readRepoJson(manifestPath);
validateDocumentation(manifest);
validateEvidence(readEvidence(args.input), manifest);
validateNextRoundCommand(args.input, manifest);
validateRunNextCommand(args.input, manifest);
console.log(`material research example ok: ${args.input}`);
