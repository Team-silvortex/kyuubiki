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
  validateOptimizationObjectives(plan.optimization_objectives, manifest, "next_round_execution");
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
    exploration.iteration,
    manifest.expected.next_run_iteration,
    "run-next iteration",
  );
  assertEquals(
    exploration.candidate_count,
    manifest.expected.candidate_count,
    "run-next candidate count",
  );
  validateNextRound(exploration.next_round, manifest);
  assertEquals(
    exploration.next_round.iteration,
    manifest.expected.next_run_next_round_iteration,
    "run-next next_round iteration",
  );
  validateNextRoundLineage(exploration.lineage, manifest);
}

function validateChainNextCommand(inputPath, manifest) {
  const absoluteInputPath = path.resolve(repoRoot, inputPath);
  const command = materialExploreCommand(
    manifest.chain_next_command,
    "--chain-next",
    absoluteInputPath,
  );
  command.argv.push("--rounds", String(manifest.chain_next_command.rounds));
  for (const expected of manifest.chain_next_command.argv_contains) {
    if (!command.argv.includes(expected)) {
      fail(`chain_next_command.argv missing ${expected}`);
    }
  }
  const result = spawnSync(command.argv[0], command.argv.slice(1), {
    cwd: path.join(repoRoot, command.cwd),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    fail(result.stderr.trim() || "chain-next command failed");
  }
  let chain;
  try {
    chain = JSON.parse(result.stdout);
  } catch (error) {
    fail(`chain-next did not emit JSON: ${error.message}`);
  }
  assertEquals(
    chain.schema_version,
    manifest.expected.chain_schema_version,
    "chain-next schema",
  );
  assertEquals(chain.round_count, manifest.expected.chain_round_count, "chain round count");
  assertEquals(
    chain.final_iteration,
    manifest.expected.chain_final_iteration,
    "chain final iteration",
  );
  assertEquals(
    chain.stop_reason,
    manifest.expected.chain_stop_reason,
    "chain stop reason",
  );
  assertEquals(
    chain.all_winners_stable,
    manifest.expected.chain_all_winners_stable,
    "chain winner stability",
  );
  validateConvergenceAssessment(chain.convergence_assessment, manifest);
  if (chain.decision_counts == null || typeof chain.decision_counts !== "object") {
    fail("chain-next must expose decision_counts");
  }
  if (
    !Array.isArray(chain.optimization_trace) ||
    chain.optimization_trace.length !== manifest.expected.chain_round_count
  ) {
    fail("chain-next must expose one optimization_trace entry per requested round");
  }
  for (const [index, trace] of chain.optimization_trace.entries()) {
    if (!Array.isArray(trace.primary_metric_ids) || trace.primary_metric_ids.length === 0) {
      fail(`optimization_trace[${index}] must expose primary_metric_ids`);
    }
  }
  if (chain.repair_summary == null || typeof chain.repair_summary !== "object") {
    fail("chain-next must expose repair_summary");
  }
  if (chain.repair_plan == null || typeof chain.repair_plan !== "object") {
    fail("chain-next must expose repair_plan");
  }
  if (chain.stop_reason === "repair_required") {
    assertEquals(chain.repair_summary.required, true, "chain repair_summary.required");
    assertEquals(chain.repair_plan.required, true, "chain repair_plan.required");
    assertEquals(chain.repair_plan.priority, "before_expansion", "chain repair_plan.priority");
    if (!Array.isArray(chain.repair_plan.actions) || chain.repair_plan.actions.length < 3) {
      fail("repair_plan must expose concrete repair actions");
    }
    if (
      !Array.isArray(chain.repair_summary.violated_gate_ids) ||
      chain.repair_summary.violated_gate_ids.length === 0
    ) {
      fail("repair_summary must expose violated quality gates");
    }
    if (
      !Array.isArray(chain.repair_summary.focus_candidate_ids) ||
      chain.repair_summary.focus_candidate_ids.length === 0
    ) {
      fail("repair_summary must expose focus candidates");
    }
  }
  if (!Array.isArray(chain.runs) || chain.runs.length !== manifest.expected.chain_round_count) {
    fail("chain-next must expose one exploration artifact per requested round");
  }
  if (
    !Array.isArray(chain.summaries) ||
    chain.summaries.length !== manifest.expected.chain_round_count
  ) {
    fail("chain-next must expose one summary per requested round");
  }
  assertEquals(
    chain.summaries.at(-1)?.iteration,
    manifest.expected.chain_final_iteration,
    "chain final summary iteration",
  );
  for (const [index, summary] of chain.summaries.entries()) {
    validateOptimizationObjectives(
      summary.optimization_objectives,
      manifest,
      `chain.summaries[${index}]`,
    );
    assertFiniteNumber(summary.winner_score, `chain.summaries[${index}].winner_score`);
  }
}

function validateOptimizationObjectives(objectives, manifest, context) {
  assertEquals(
    objectives?.schema_version,
    manifest.expected.next_round_optimization_objectives_schema_version,
    `${context}.optimization_objectives schema`,
  );
  if (!Array.isArray(objectives.primary_metric_ids) || objectives.primary_metric_ids.length === 0) {
    fail(`${context}.optimization_objectives must expose primary_metric_ids`);
  }
  if (!Array.isArray(objectives.metric_objectives) || objectives.metric_objectives.length === 0) {
    fail(`${context}.optimization_objectives must expose metric_objectives`);
  }
}

function validateNextRoundLineage(lineage, manifest) {
  assertEquals(
    lineage?.schema_version,
    manifest.expected.next_round_lineage_schema_version,
    "run-next lineage schema",
  );
  assertEquals(lineage.source_iteration, manifest.expected.initial_iteration, "lineage source_iteration");
  assertEquals(lineage.planned_iteration, manifest.expected.next_run_iteration, "lineage planned_iteration");
  validateOptimizationObjectives(lineage.optimization_objectives, manifest, "run-next.lineage");
}

function validateConvergenceAssessment(assessment, manifest) {
  assertEquals(
    assessment?.schema_version,
    manifest.expected.chain_convergence_schema_version,
    "chain convergence schema",
  );
  assertEquals(
    assessment.state,
    manifest.expected.chain_convergence_state,
    "chain convergence state",
  );
  assertEquals(assessment.winner_stable, true, "chain convergence winner_stable");
  assertFiniteNumber(assessment.winner_score_delta, "chain convergence winner_score_delta");
  if (typeof assessment.recommendation !== "string" || assessment.recommendation.length === 0) {
    fail("chain convergence assessment must expose recommendation");
  }
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
  assertEquals(exploration.iteration, expected.initial_iteration, "exploration.iteration");
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
  requireMarkdownContains(markdown, manifest.expected.chain_schema_version, "chain schema");
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
  assertEquals(evidence.summary?.iteration, expected.initial_iteration, "summary iteration");
  assertEquals(
    evidence.summary?.next_round_iteration,
    expected.next_run_iteration,
    "summary next_round_iteration",
  );
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
validateChainNextCommand(args.input, manifest);
console.log(`material research example ok: ${args.input}`);
