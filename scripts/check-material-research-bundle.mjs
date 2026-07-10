#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultInput = "tmp/material-research-bundle.json";

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
  assertEquals(bundle.study, "heat-spreader", "study");
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
    bundle.chain?.schema_version,
    "kyuubiki.material-exploration-chain/v1",
    "chain schema",
  );
  assertNonEmptyString(bundle.summary?.winner_candidate_id, "summary.winner_candidate_id");
  assertNonEmptyString(bundle.summary?.reliability_decision, "summary.reliability_decision");
  assertNonEmptyString(bundle.summary?.next_round_decision, "summary.next_round_decision");
  assertNonEmptyString(bundle.summary?.chain_stop_reason, "summary.chain_stop_reason");
  if (!Array.isArray(bundle.reproducibility?.initial_command)) {
    fail("reproducibility.initial_command must be an argv array");
  }
}

function runSelfTest() {
  const badBundle = {
    schema_version: "kyuubiki.material-research-bundle/v1",
    posture: "screening_research_bundle",
    study: "heat-spreader",
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
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    validateBundle(badBundle);
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject a bad checksum");
  }
  console.log("material research bundle check self-test passed");
}

const args = parseArgs(process.argv);
if (args.selfTest) {
  runSelfTest();
} else {
  validateBundle(readBundle(args.input));
  console.log(`material research bundle ok: ${args.input}`);
}
