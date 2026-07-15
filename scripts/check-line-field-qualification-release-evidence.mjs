#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultInput = "tmp/line-field-qualification-release-evidence.json";
const requiredCommandIds = new Set(["evidence_check", "solver_baseline"]);
const requiredPromotedOperatorIds = new Set([
  "solve.bar_1d",
  "solve.thermal_bar_1d",
  "solve.heat_bar_1d",
  "solve.electrostatic_bar_1d",
]);
const expectedReleaseRecordPath = "releases/qualification-records/1.20.0.json";
const expectedReviewDecisionPath =
  "releases/qualification-review-decisions/2.0.0/line-field-closed-form-review-decision.json";
const requiredTrackedInputs = new Set([
  "evidence/operator-qualification/line-field-closed-form-baseline.json",
  "evidence/operator-qualification/line-field-closed-form-derivation.md",
  "evidence/operator-qualification/line-field-tolerance-policy.json",
  "workers/rust/crates/solver/tests/accuracy_baselines/line_1d.rs",
  "scripts/check-line-field-closed-form-baseline.mjs",
]);

function fail(message) {
  console.error(`line-field qualification release evidence check failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { input: defaultInput };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--in") {
      args.input = argv[index + 1];
      index += 1;
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
  const absoluteInput = path.resolve(repoRoot, inputPath);
  const relativeInput = path.relative(repoRoot, absoluteInput);
  if (relativeInput.startsWith("..") || path.isAbsolute(relativeInput)) {
    fail("--in must stay inside the repository");
  }
  if (!fs.existsSync(absoluteInput)) {
    fail(`input does not exist: ${relativeInput}`);
  }
  return JSON.parse(fs.readFileSync(absoluteInput, "utf8"));
}

function readRepoJson(relativePath, context) {
  const absolutePath = path.resolve(repoRoot, relativePath);
  const repoRelativePath = path.relative(repoRoot, absolutePath);
  if (repoRelativePath.startsWith("..") || path.isAbsolute(repoRelativePath)) {
    fail(`${context}: path must stay inside the repository`);
  }
  if (!fs.existsSync(absolutePath)) {
    fail(`${context}: missing ${relativePath}`);
  }
  return JSON.parse(fs.readFileSync(absolutePath, "utf8"));
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

function validateCommand(command) {
  const context = command?.id ?? "unknown command";
  if (!requiredCommandIds.has(context)) {
    fail(`${context}: unexpected command id`);
  }
  if (command.ok !== true || command.status !== 0) {
    fail(`${context}: command must pass with status 0`);
  }
  if (!Array.isArray(command.argv) || command.argv.length === 0) {
    fail(`${context}: argv must be non-empty`);
  }
  if (typeof command.cwd !== "string" || command.cwd.length === 0) {
    fail(`${context}: cwd must be repo-relative`);
  }
  if (path.isAbsolute(command.cwd) || command.cwd.includes("..")) {
    fail(`${context}: cwd must not escape the repository`);
  }
  if (!Number.isFinite(command.duration_ms) || command.duration_ms < 0) {
    fail(`${context}: duration_ms must be finite and non-negative`);
  }
}

function validateProvenance(provenance) {
  if (provenance?.schema_version !== "kyuubiki.operator-qualification-provenance/v1") {
    fail("provenance: unexpected schema_version");
  }
  if (provenance.candidate_id !== "line-field-closed-form") {
    fail("provenance: candidate_id must be line-field-closed-form");
  }
  if (provenance.retention_policy?.no_local_absolute_paths !== true) {
    fail("provenance: no_local_absolute_paths must be true");
  }
  const tracked = provenance.tracked_inputs ?? [];
  if (!Array.isArray(tracked) || tracked.length !== requiredTrackedInputs.size) {
    fail(`provenance: expected ${requiredTrackedInputs.size} tracked inputs`);
  }
  const seen = new Set();
  for (const input of tracked) {
    if (!requiredTrackedInputs.has(input.path)) {
      fail(`provenance: unexpected tracked input ${input.path}`);
    }
    if (!/^[a-f0-9]{64}$/.test(input.sha256 ?? "")) {
      fail(`provenance: ${input.path} sha256 must be lowercase hex`);
    }
    seen.add(input.path);
  }
  for (const expected of requiredTrackedInputs) {
    if (!seen.has(expected)) {
      fail(`provenance: missing tracked input ${expected}`);
    }
  }
}

function validatePromotionSummary(summary, evidencePath) {
  if (!summary || typeof summary !== "object") {
    fail("promotion_summary: must be present");
  }
  if (summary.candidate_id !== "line-field-closed-form") {
    fail("promotion_summary: candidate_id must be line-field-closed-form");
  }
  if (summary.release_version !== "2.0.0") {
    fail("promotion_summary: release_version must be 2.0.0");
  }
  if (summary.approved_coverage_level !== "qualification") {
    fail("promotion_summary: approved_coverage_level must be qualification");
  }
  if (
    summary.retained_evidence_path !==
    "releases/qualification-evidence/2.0.0/line-field-closed-form-release-evidence.json"
  ) {
    fail("promotion_summary: retained_evidence_path must point to the retained moxi 2.0.0 evidence");
  }
  if (summary.release_record_path !== expectedReleaseRecordPath) {
    fail(`promotion_summary: release_record_path must be ${expectedReleaseRecordPath}`);
  }
  if (summary.review_decision_path !== expectedReviewDecisionPath) {
    fail(`promotion_summary: review_decision_path must be ${expectedReviewDecisionPath}`);
  }
  if (
    !Array.isArray(summary.promoted_operator_ids) ||
    summary.promoted_operator_ids.length !== requiredPromotedOperatorIds.size
  ) {
    fail(`promotion_summary: expected ${requiredPromotedOperatorIds.size} promoted operators`);
  }
  for (const operatorId of summary.promoted_operator_ids) {
    if (!requiredPromotedOperatorIds.has(operatorId)) {
      fail(`promotion_summary: unexpected promoted operator ${operatorId}`);
    }
  }

  const releaseRecords = readRepoJson(summary.release_record_path, "promotion_summary.release_record_path");
  const releaseRecord = releaseRecords.records?.find(
    (record) => record.candidate_id === summary.candidate_id,
  );
  if (!releaseRecord) {
    fail("promotion_summary: release record is missing line-field-closed-form");
  }
  if (releaseRecord.review_status !== "approved") {
    fail("promotion_summary: release record review_status must be approved");
  }
  if (releaseRecord.evidence_path !== summary.retained_evidence_path) {
    fail("promotion_summary: release record evidence_path must match retained evidence");
  }
  if (releaseRecord.review_decision_path !== summary.review_decision_path) {
    fail("promotion_summary: release record review_decision_path mismatch");
  }

  const reviewDecision = readRepoJson(summary.review_decision_path, "promotion_summary.review_decision_path");
  if (reviewDecision.candidate_id !== summary.candidate_id) {
    fail("promotion_summary: review decision candidate_id mismatch");
  }
  if (reviewDecision.release_version !== summary.release_version) {
    fail("promotion_summary: review decision release_version mismatch");
  }
  if (reviewDecision.decision !== "approve_promotion") {
    fail("promotion_summary: review decision must approve promotion");
  }
  if (reviewDecision.evidence_path !== summary.retained_evidence_path) {
    fail("promotion_summary: review decision evidence_path must match retained evidence");
  }
}

function validateEvidence(evidence) {
  if (evidence.schema_version !== "kyuubiki.operator-qualification-release-evidence/v1") {
    fail("unexpected schema_version");
  }
  if (evidence.version_line !== "moxi 2.0.x") {
    fail("version_line must match moxi 2.0.x");
  }
  if (evidence.candidate_id !== "line-field-closed-form") {
    fail("candidate_id must be line-field-closed-form");
  }
  if (evidence.release_retention?.intended_release_artifact !== true) {
    fail("release_retention.intended_release_artifact must be true");
  }
  if (evidence.release_retention?.repo_relative_paths_only !== true) {
    fail("release_retention.repo_relative_paths_only must be true");
  }
  if (evidence.release_retention?.generated_output_should_not_be_committed_directly !== true) {
    fail("release_retention.generated_output_should_not_be_committed_directly must be true");
  }
  if (evidence.summary?.ok !== true || evidence.summary?.failed !== 0) {
    fail("summary must report a passing release evidence run");
  }
  if (!Array.isArray(evidence.commands) || evidence.commands.length !== requiredCommandIds.size) {
    fail(`commands must contain exactly ${requiredCommandIds.size} entries`);
  }
  const seenCommands = new Set();
  for (const command of evidence.commands) {
    validateCommand(command);
    seenCommands.add(command.id);
  }
  for (const expected of requiredCommandIds) {
    if (!seenCommands.has(expected)) {
      fail(`missing command ${expected}`);
    }
  }
  validatePromotionSummary(evidence.promotion_summary, args.input);
  validateProvenance(evidence.provenance);
  assertNoAbsoluteRepoPath(evidence, "evidence");
}

const args = parseArgs(process.argv);
const evidence = readEvidence(args.input);
validateEvidence(evidence);
console.log(`line-field qualification release evidence ok: ${args.input}`);
