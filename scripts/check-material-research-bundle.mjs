#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultInput = "tmp/material-research-bundle.json";
const supportedStudies = new Set(["heat-spreader", "composite-thermo-electric-panel"]);

function fail(message) {
  console.error(`material research bundle check failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { input: defaultInput, selfTest: false };
  for (let index = 2; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === "--in") {
      args.input = argv[++index];
    } else if (flag === "--self-test") {
      args.selfTest = true;
    } else {
      fail(`unknown argument ${flag}`);
    }
  }
  return args;
}

function readBundle(inputPath) {
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

function sha256(payload) {
  return crypto.createHash("sha256").update(`${JSON.stringify(payload)}\n`).digest("hex");
}

function assertEquals(actual, expected, context) {
  if (actual !== expected) {
    fail(`${context}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
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

function assertNonEmptyString(value, context) {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${context}: expected non-empty string`);
  }
}

function assertChecksum(bundle, key, artifactKey) {
  assertEquals(bundle.artifact_checksums?.[key], sha256(bundle[artifactKey]), `checksum ${key}`);
}

function validateBundle(bundle) {
  assertEquals(
    bundle.schema_version,
    "kyuubiki.material-research-bundle/v1",
    "schema_version",
  );
  assertEquals(bundle.posture, "screening_research_bundle", "posture");
  if (!supportedStudies.has(bundle.study)) {
    fail(`study: unsupported retained bundle study ${JSON.stringify(bundle.study)}`);
  }
  assertNoAbsoluteRepoPath(bundle, "bundle");
  assertChecksum(bundle, "initial_exploration_sha256", "initial_exploration");
  assertChecksum(bundle, "next_round_execution_plan_sha256", "next_round_execution_plan");
  assertChecksum(bundle, "next_exploration_sha256", "next_exploration");
  assertChecksum(bundle, "chain_sha256", "chain");
  assertEquals(
    bundle.initial_exploration?.schema_version,
    "kyuubiki.material-exploration-run/v1",
    "initial exploration schema",
  );
  assertEquals(
    bundle.next_round_execution_plan?.schema_version,
    "kyuubiki.material-exploration-next-round-execution/v1",
    "next round execution schema",
  );
  assertEquals(
    bundle.next_exploration?.schema_version,
    "kyuubiki.material-exploration-run/v1",
    "next exploration schema",
  );
  assertEquals(
    bundle.chain?.schema_version,
    "kyuubiki.material-exploration-chain/v1",
    "chain schema",
  );
  assertNonEmptyString(bundle.summary?.winner_candidate_id, "summary.winner_candidate_id");
  assertNonEmptyString(bundle.summary?.reliability_decision, "summary.reliability_decision");
  assertNonEmptyString(bundle.summary?.next_round_decision, "summary.next_round_decision");
  assertNonEmptyString(bundle.summary?.chain_stop_reason, "summary.chain_stop_reason");
  assertEquals(
    bundle.next_round_execution_plan?.decision,
    bundle.summary.next_round_decision,
    "next_round_execution_plan.decision",
  );
  assertEquals(
    bundle.next_round_execution_plan?.runnable_step_count,
    bundle.summary.runnable_next_step_count,
    "next_round_execution_plan.runnable_step_count",
  );
  assertEquals(
    bundle.next_round_execution_plan?.iteration,
    bundle.summary.next_iteration,
    "next_round_execution_plan.iteration",
  );
  assertEquals(
    bundle.next_exploration?.iteration,
    bundle.summary.next_iteration,
    "next_exploration.iteration",
  );
  assertEquals(bundle.chain?.stop_reason, bundle.summary.chain_stop_reason, "chain.stop_reason");
  if (!Array.isArray(bundle.reproducibility?.initial_command)) {
    fail("reproducibility.initial_command must be an argv array");
  }
}

function runSelfTest() {
  const badBundle = {
    schema_version: "kyuubiki.material-research-bundle/v1",
    posture: "screening_research_bundle",
    study: "unsupported-study",
    artifact_checksums: {
      initial_exploration_sha256: "bad",
    },
    initial_exploration: {},
    next_round_execution_plan: {},
    next_exploration: {},
    chain: {},
    summary: {},
    reproducibility: { initial_command: [] },
  };
  expectValidateBundleFailure(badBundle, "bad checksum");

  const artifact = { schema_version: "kyuubiki.material-exploration-run/v1", iteration: 2 };
  const plan = {
    schema_version: "kyuubiki.material-exploration-next-round-execution/v1",
    decision: "repair_validation",
    iteration: 2,
    runnable_step_count: 1,
  };
  const chain = {
    schema_version: "kyuubiki.material-exploration-chain/v1",
    stop_reason: "validation_repair_required",
  };
  const mismatchBundle = {
    schema_version: "kyuubiki.material-research-bundle/v1",
    posture: "screening_research_bundle",
    study: "heat-spreader",
    artifact_checksums: {
      initial_exploration_sha256: sha256(artifact),
      next_round_execution_plan_sha256: sha256(plan),
      next_exploration_sha256: sha256(artifact),
      chain_sha256: sha256(chain),
    },
    initial_exploration: artifact,
    next_round_execution_plan: plan,
    next_exploration: artifact,
    chain,
    summary: {
      winner_candidate_id: "candidate-a",
      reliability_decision: "blocked_by_quality_gates",
      next_round_decision: "mitigate_design_risk",
      runnable_next_step_count: 1,
      next_iteration: 2,
      chain_stop_reason: "validation_repair_required",
    },
    reproducibility: { initial_command: ["kyuubiki-material-explore"] },
  };
  expectValidateBundleFailure(mismatchBundle, "summary/plan decision mismatch");

  console.log("material research bundle check self-test passed");
}

function expectValidateBundleFailure(bundle, label) {
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    validateBundle(bundle);
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail(`self-test did not reject ${label}`);
  }
}

const args = parseArgs(process.argv);
if (args.selfTest) {
  runSelfTest();
} else {
  validateBundle(readBundle(args.input));
  console.log(`material research bundle ok: ${args.input}`);
}
