#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

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
  ["blocked", 4],
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
  if (report.summary?.collecting !== collecting) errors.push(`${relativeInput}: summary.collecting is stale`);
  if (report.summary?.planned !== planned) errors.push(`${relativeInput}: summary.planned is stale`);
  if (report.summary?.with_entries !== withEntries) errors.push(`${relativeInput}: summary.with_entries is stale`);
  if (report.summary?.broken !== broken) errors.push(`${relativeInput}: summary.broken is stale`);
  errors.push(...countMapErrors(report.summary?.target_levels, countBy(report.candidates, "target_level", targetLevels), "target_levels", relativeInput));
  errors.push(...countMapErrors(report.summary?.evidence_phases, countBy(report.candidates, "evidence_phase", evidencePhases), "evidence_phases", relativeInput));
  errors.push(...countMapErrors(report.summary?.release_gate_impacts, countBy(report.candidates, "release_gate_impact", releaseGateImpacts), "release_gate_impacts", relativeInput));
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
    version_line: "tamamono 1.20.x",
    generated_at_utc: "2026-01-01T00:00:00.000Z",
    summary: {
      candidates: 1,
      collecting: 0,
      planned: 1,
      with_entries: 0,
      not_started: 1,
      broken: 0,
      next_action_count: 2,
      target_levels: { baseline: 0, review: 0, qualification: 1 },
      evidence_phases: { planned: 1, collecting: 0, ready_for_review: 0, blocked: 0 },
      release_gate_impacts: { release_blocker: 1, release_watch: 0, experimental_only: 0 },
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
        path: null,
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
