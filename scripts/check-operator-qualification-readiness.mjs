#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { operatorReliabilityPaths } from "./operator-reliability-contracts.mjs";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultInput = "tmp/operator-qualification-readiness.json";
const schemaPath = "schemas/operator-qualification-readiness.schema.json";
const priorityOrder = new Map([
  ["p0", 0],
  ["p1", 1],
  ["p2", 2],
  ["p3", 3],
]);
const readinessOrder = new Map([
  ["broken", 0],
  ["planned", 1],
  ["partially_collecting", 2],
  ["collecting_with_entries", 3],
  ["ready_for_review", 4],
  ["blocked", 5],
]);
const allowedActionKinds = new Set([
  "collect_artifact",
  "restore_or_generate_artifact",
  "run_command",
  "review",
]);
const targetLevels = ["baseline", "review", "qualification"];
const evidencePhases = ["planned", "collecting", "ready_for_review", "blocked"];
const releaseGateImpacts = ["release_blocker", "release_watch", "experimental_only"];
const releaseReviewStatuses = ["missing", "pending_signoff", "approved", "blocked_scope", "rejected"];
const operatorTrustLevels = ["smoke", "baseline", "review", "qualification"];

function fail(message) {
  console.error(`operator qualification readiness check failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { input: defaultInput, selfTest: false };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--self-test") {
      args.selfTest = true;
    } else if (argv[index] === "--in") {
      args.input = argv[index + 1];
      index += 1;
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
  }
  if (!args.selfTest && !args.input) {
    fail("--in requires a repo-local path");
  }
  return args;
}

function repoLocalInput(inputPath) {
  const absoluteInput = path.resolve(repoRoot, inputPath);
  const relativeInput = path.relative(repoRoot, absoluteInput);
  if (relativeInput.startsWith("..") || path.isAbsolute(relativeInput)) {
    fail("--in must stay inside the repository");
  }
  return { absoluteInput, relativeInput };
}

function rank(map, value) {
  return map.get(value) ?? 99;
}

function compareActions(left, right) {
  return (
    rank(priorityOrder, left.priority) - rank(priorityOrder, right.priority)
    || rank(readinessOrder, left.readiness) - rank(readinessOrder, right.readiness)
    || String(left.candidate_id).localeCompare(String(right.candidate_id))
  );
}

function requireStringErrors(value, field, context) {
  if (typeof value !== "string" || value.length === 0) {
    return [`${context}: ${field} must be a non-empty string`];
  }
  return [];
}

function requireIntegerErrors(value, field, context) {
  if (!Number.isInteger(value) || value < 0) {
    return [`${context}: ${field} must be a non-negative integer`];
  }
  return [];
}

function actionErrors(action, index) {
  const errors = [];
  const context = `next_actions[${index}]`;
  errors.push(...requireStringErrors(action.candidate_id, "candidate_id", context));
  errors.push(...requireStringErrors(action.priority, "priority", context));
  errors.push(...requireStringErrors(action.target_level, "target_level", context));
  errors.push(...requireStringErrors(action.evidence_phase, "evidence_phase", context));
  errors.push(...requireStringErrors(action.readiness, "readiness", context));
  errors.push(...requireStringErrors(action.action_kind, "action_kind", context));
  if (!allowedActionKinds.has(action.action_kind)) {
    errors.push(`${context}: unsupported action_kind ${action.action_kind}`);
  }
  if (action.action_kind === "run_command") {
    errors.push(...requireStringErrors(action.command, "command", context));
  }
  if (action.action_kind === "restore_or_generate_artifact") {
    errors.push(...requireStringErrors(action.path, "path", context));
  }
  if (action.action_kind === "review") {
    errors.push(...requireStringErrors(action.review_reason, "review_reason", context));
  }
  errors.push(...requireIntegerErrors(action.validation_profile_count, "validation_profile_count", context));
  errors.push(...requireIntegerErrors(action.release_candidate_profile_count, "release_candidate_profile_count", context));
  errors.push(...requireStringErrors(action.gate, "gate", context));
  errors.push(...requireStringErrors(action.preferred_validation_lane, "preferred_validation_lane", context));
  errors.push(...requireStringErrors(action.release_gate_impact, "release_gate_impact", context));
  return errors;
}

function countBy(candidates, field, values) {
  return Object.fromEntries(values.map((value) => [
    value,
    candidates.filter((candidate) => candidate[field] === value).length,
  ]));
}

function countReleaseReviewStatuses(candidates) {
  const releaseArtifacts = candidates.flatMap((candidate) =>
    (candidate.artifacts ?? []).filter((artifact) => artifact.kind === "release_retained_regression_output")
  );
  return Object.fromEntries(releaseReviewStatuses.map((status) => [
    status,
    releaseArtifacts.filter((artifact) => (artifact.release_review_status ?? "missing") === status).length,
  ]));
}

function countReleaseReviewDecisions(candidates) {
  const releaseArtifacts = candidates.flatMap((candidate) =>
    (candidate.artifacts ?? []).filter((artifact) => artifact.kind === "release_retained_regression_output")
  );
  const withDecisionPath = releaseArtifacts.filter((artifact) => artifact.release_review_decision_path);
  const retained = withDecisionPath.filter((artifact) =>
    fs.existsSync(path.join(repoRoot, artifact.release_review_decision_path))
  );
  return {
    required: releaseArtifacts.length,
    declared: withDecisionPath.length,
    retained: retained.length,
    missing: releaseArtifacts.length - retained.length,
  };
}

function sameStrings(left, right) {
  const sortedLeft = [...left].sort((a, b) => a.localeCompare(b));
  const sortedRight = [...right].sort((a, b) => a.localeCompare(b));
  return sortedLeft.length === sortedRight.length
    && sortedLeft.every((item, index) => item === sortedRight[index]);
}

function countReleasePromotionSummaries(candidates) {
  const releaseVersion = readJson(operatorReliabilityPaths.releaseRecords).release_version;
  const approvedArtifacts = candidates.flatMap((candidate) =>
    (candidate.artifacts ?? [])
      .filter((artifact) =>
        artifact.kind === "release_retained_regression_output"
        && artifact.release_review_status === "approved"
      )
      .map((artifact) => ({ candidate, artifact }))
  );
  const retained = approvedArtifacts.filter(({ artifact }) =>
    artifact.release_record_path && fs.existsSync(path.join(repoRoot, artifact.release_record_path))
  );
  const withSummary = retained
    .map(({ candidate, artifact }) => ({
      candidate,
      artifact,
      evidence: readJson(artifact.release_record_path),
    }))
    .filter(({ evidence }) => evidence.promotion_summary);
  const matched = withSummary.filter(({ candidate, artifact, evidence }) => {
    const summary = evidence.promotion_summary;
    return summary.candidate_id === candidate.candidate_id
      && summary.release_version === releaseVersion
      && summary.approved_coverage_level === "qualification"
      && summary.retained_evidence_path === artifact.release_record_path
      && summary.release_record_path === operatorReliabilityPaths.releaseRecords
      && summary.review_decision_path === artifact.release_review_decision_path
      && sameStrings(summary.promoted_operator_ids ?? [], candidate.operator_ids ?? []);
  });
  return {
    required: approvedArtifacts.length,
    retained: retained.length,
    declared: withSummary.length,
    matched: matched.length,
    missing: approvedArtifacts.length - matched.length,
  };
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function operatorTrustLevelCounts() {
  const manifest = readJson(operatorReliabilityPaths.manifest);
  const operators = (manifest.shards ?? []).flatMap((shardPath) => readJson(shardPath).operators ?? []);
  return Object.fromEntries(operatorTrustLevels.map((level) => [
    level,
    operators.filter((operator) => operator.coverage_level === level).length,
  ]));
}

function countMapErrors(actual, expected, field, context) {
  const errors = [];
  if (!actual || typeof actual !== "object" || Array.isArray(actual)) {
    return [`${context}: summary.${field} must be an object`];
  }
  for (const [key, value] of Object.entries(expected)) {
    if (actual[key] !== value) {
      errors.push(`${context}: summary.${field}.${key} is stale`);
    }
  }
  return errors;
}

function readinessErrors(report, relativeInput) {
  const errors = [];
  const schema = JSON.parse(fs.readFileSync(path.join(repoRoot, schemaPath), "utf8"));
  if (schema.properties?.schema_version?.const !== "kyuubiki.operator-qualification-readiness/v1") {
    errors.push(`${schemaPath}: schema_version const is wrong`);
  }
  if (report.schema_version !== "kyuubiki.operator-qualification-readiness/v1") {
    errors.push(`${relativeInput}: unexpected schema_version`);
  }
  if (typeof report.version_line !== "string" || report.version_line.length === 0) {
    errors.push(`${relativeInput}: version_line must be non-empty`);
  }
  if (!Array.isArray(report.candidates) || report.candidates.length === 0) {
    errors.push(`${relativeInput}: candidates must be non-empty`);
  }
  if (!Array.isArray(report.next_actions)) {
    errors.push(`${relativeInput}: next_actions must be an array`);
    return errors;
  }
  if (report.summary?.next_action_count !== report.next_actions.length) {
    errors.push(`${relativeInput}: summary.next_action_count must match next_actions length`);
  }
  if (report.summary?.candidates !== report.candidates?.length) {
    errors.push(`${relativeInput}: summary.candidates must match candidates length`);
  }
  const collecting = report.candidates.filter((candidate) => candidate.status === "collecting").length;
  const planned = report.candidates.filter((candidate) => candidate.status === "planned").length;
  const withEntries = report.candidates.filter((candidate) =>
    candidate.artifact_counts?.present > 0 || candidate.artifact_counts?.command_available > 0
  ).length;
  const broken = report.candidates.filter((candidate) => candidate.readiness === "broken").length;
  const validationProfileCount = report.candidates.reduce((count, candidate) => count + (candidate.validation_profiles?.length ?? 0), 0);
  const releaseCandidateProfiles = report.candidates.reduce((count, candidate) =>
    count + (candidate.validation_profiles ?? []).filter((profile) => profile.profile_role === "release_candidate").length, 0);
  const missingReleaseProfiles = report.candidates.filter((candidate) =>
    !(candidate.validation_profiles ?? []).some((profile) => profile.profile_role === "release_candidate")
  ).length;
  if (report.summary?.collecting !== collecting) errors.push(`${relativeInput}: summary.collecting is stale`);
  if (report.summary?.planned !== planned) errors.push(`${relativeInput}: summary.planned is stale`);
  if (report.summary?.with_entries !== withEntries) errors.push(`${relativeInput}: summary.with_entries is stale`);
  if (report.summary?.broken !== broken) errors.push(`${relativeInput}: summary.broken is stale`);
  if (report.summary?.validation_profile_count !== validationProfileCount) errors.push(`${relativeInput}: summary.validation_profile_count is stale`);
  if (report.summary?.release_candidate_profiles !== releaseCandidateProfiles) errors.push(`${relativeInput}: summary.release_candidate_profiles is stale`);
  if (report.summary?.component_profiles !== validationProfileCount - releaseCandidateProfiles) errors.push(`${relativeInput}: summary.component_profiles is stale`);
  if (report.summary?.candidates_missing_release_profile !== missingReleaseProfiles) errors.push(`${relativeInput}: summary.candidates_missing_release_profile is stale`);
  errors.push(...countMapErrors(report.summary?.target_levels, countBy(report.candidates, "target_level", targetLevels), "target_levels", relativeInput));
  errors.push(...countMapErrors(report.summary?.operator_trust_levels, operatorTrustLevelCounts(), "operator_trust_levels", relativeInput));
  errors.push(...countMapErrors(report.summary?.evidence_phases, countBy(report.candidates, "evidence_phase", evidencePhases), "evidence_phases", relativeInput));
  errors.push(...countMapErrors(report.summary?.release_gate_impacts, countBy(report.candidates, "release_gate_impact", releaseGateImpacts), "release_gate_impacts", relativeInput));
  errors.push(...countMapErrors(report.summary?.release_review_statuses, countReleaseReviewStatuses(report.candidates), "release_review_statuses", relativeInput));
  errors.push(...countMapErrors(report.summary?.release_review_decisions, countReleaseReviewDecisions(report.candidates), "release_review_decisions", relativeInput));
  errors.push(...countMapErrors(report.summary?.release_promotion_summaries, countReleasePromotionSummaries(report.candidates), "release_promotion_summaries", relativeInput));
  report.next_actions.forEach((action, index) => {
    errors.push(...actionErrors(action, index));
  });
  for (let index = 1; index < report.next_actions.length; index += 1) {
    if (compareActions(report.next_actions[index - 1], report.next_actions[index]) > 0) {
      errors.push(`${relativeInput}: next_actions must stay priority/readiness sorted`);
    }
  }
  return errors;
}

function checkReadiness(report, relativeInput) {
  const errors = readinessErrors(report, relativeInput);
  if (errors.length > 0) {
    fail(errors[0]);
  }
}

const args = parseArgs(process.argv);
if (args.selfTest) {
  const sample = {
    schema_version: "kyuubiki.operator-qualification-readiness/v1",
    version_line: "moxi 2.0.x",
    generated_at_utc: "2026-01-01T00:00:00.000Z",
    summary: {
      candidates: 1,
      collecting: 0,
      planned: 1,
      with_entries: 0,
      not_started: 1,
      broken: 0,
      validation_profile_count: 1,
      release_candidate_profiles: 1,
      component_profiles: 0,
      candidates_missing_release_profile: 0,
      next_action_count: 2,
      target_levels: { baseline: 0, review: 0, qualification: 1 },
      operator_trust_levels: operatorTrustLevelCounts(),
      evidence_phases: { planned: 1, collecting: 0, ready_for_review: 0, blocked: 0 },
      release_gate_impacts: { release_blocker: 1, release_watch: 0, experimental_only: 0 },
      release_review_statuses: { missing: 0, pending_signoff: 0, approved: 0, blocked_scope: 0, rejected: 0 },
      release_review_decisions: { required: 0, declared: 0, retained: 0, missing: 0 },
      release_promotion_summaries: { required: 0, retained: 0, declared: 0, matched: 0, missing: 0 },
    },
    candidates: [{
      candidate_id: "sample",
      priority: "p0",
      domain: "sample",
      target_level: "qualification",
      evidence_phase: "planned",
      status: "planned",
      readiness: "planned",
      operator_ids: ["solve.sample"],
      artifact_counts: { total: 1, present: 0, command_available: 0, missing: 0, not_started: 1 },
      artifacts: [],
      validation_profiles: [{
        profile_id: "sample",
        profile_role: "release_candidate",
        trust_goal: "review",
        operator_count: 1,
        command_count: 1,
      }],
      primary_blocker: "sample blocker",
      evidence_gaps: ["sample"],
      graduation_gate: "sample gate",
      preferred_validation_lane: "make sample-validation",
      release_gate_impact: "release_blocker",
    }],
    next_actions: [
      {
        candidate_id: "candidate_a",
        priority: "p0",
        target_level: "qualification",
        evidence_phase: "planned",
        readiness: "planned",
        action_kind: "collect_artifact",
        artifact_id: "note",
        artifact_state: "not_started",
        artifact_kind: "reference_note",
        command: null,
        check_command: null,
        path: null,
        review_reason: null,
        validation_profile_count: 1,
        release_candidate_profile_count: 1,
        gate: "collect canonical reference note",
        preferred_validation_lane: "make sample-validation",
        release_gate_impact: "release_blocker",
      },
      {
        candidate_id: "candidate_b",
        priority: "p1",
        target_level: "review",
        evidence_phase: "collecting",
        readiness: "collecting_with_entries",
        action_kind: "run_command",
        artifact_id: "release-output",
        artifact_state: "command_available",
        artifact_kind: "release_output",
        command: "make sample-release-evidence",
        check_command: "make check-sample-release-evidence",
        path: null,
        review_reason: null,
        validation_profile_count: 1,
        release_candidate_profile_count: 1,
        gate: "retain release evidence",
        preferred_validation_lane: "make sample-release-evidence",
        release_gate_impact: "release_watch",
      },
    ],
  };
  checkReadiness(sample, "self-test");
  if (compareActions(sample.next_actions[1], sample.next_actions[0]) <= 0) {
    fail("self-test expected p0 action to sort before p1 action");
  }
  const missingCommand = structuredClone(sample);
  delete missingCommand.next_actions[1].command;
  if (!readinessErrors(missingCommand, "self-test").some((error) => error.includes("command"))) {
    fail("self-test expected missing run_command command to fail");
  }
  const unsorted = {
    ...sample,
    next_actions: [sample.next_actions[1], sample.next_actions[0]],
  };
  if (!readinessErrors(unsorted, "self-test").some((error) => error.includes("sorted"))) {
    fail("self-test expected unsorted next_actions to fail");
  }
  console.log("operator qualification readiness check self-test passed");
  process.exit(0);
}
const { absoluteInput, relativeInput } = repoLocalInput(args.input);
const report = JSON.parse(fs.readFileSync(absoluteInput, "utf8"));
checkReadiness(report, relativeInput);
console.log(`operator qualification readiness check passed: ${relativeInput}`);
