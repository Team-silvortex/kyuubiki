#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const schemaPath = "schemas/material-research-bundle.schema.json";
const examplePath = "schemas/examples.material-research-bundle.json";
const schemasReadmePath = "schemas/README.md";
const scriptsReadmePath = "scripts/README.md";
const docsPath = "docs/automated-material-research-example.md";

const BUNDLE_SCHEMA_VERSION = "kyuubiki.material-research-bundle/v1";
const POSTURE = "screening_research_bundle";
const EXPLORATION_SCHEMA_VERSION = "kyuubiki.material-exploration-run/v1";
const EXECUTION_SCHEMA_VERSION = "kyuubiki.material-exploration-next-round-execution/v1";
const CHAIN_SCHEMA_VERSION = "kyuubiki.material-exploration-chain/v1";

function fail(message) {
  console.error(`material research bundle contract check failed: ${message}`);
  process.exit(1);
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function sha256(payload) {
  return crypto.createHash("sha256").update(`${JSON.stringify(payload)}\n`).digest("hex");
}

function requireString(value, field, context) {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${context}: ${field} must be a non-empty string`);
  }
}

function requireArray(value, field, context) {
  if (!Array.isArray(value) || value.length === 0) {
    fail(`${context}: ${field} must be a non-empty array`);
  }
}

function checkSchema(schema) {
  if (schema?.properties?.schema_version?.const !== BUNDLE_SCHEMA_VERSION) {
    fail(`${schemaPath}: schema_version const must be ${BUNDLE_SCHEMA_VERSION}`);
  }
  if (schema?.properties?.posture?.const !== POSTURE) {
    fail(`${schemaPath}: posture const must be ${POSTURE}`);
  }
  const required = schema?.required ?? [];
  for (const field of ["artifact_checksums", "reproducibility", "summary", "chain"]) {
    if (!required.includes(field)) {
      fail(`${schemaPath}: missing required field ${field}`);
    }
  }
  const checksumRequired = schema?.$defs?.artifactChecksums?.required ?? [];
  for (const field of [
    "initial_exploration_sha256",
    "next_round_execution_plan_sha256",
    "next_exploration_sha256",
    "chain_sha256",
  ]) {
    if (!checksumRequired.includes(field)) {
      fail(`${schemaPath}: missing checksum field ${field}`);
    }
  }
}

function checkExample(example) {
  if (example.schema_version !== BUNDLE_SCHEMA_VERSION) {
    fail(`${examplePath}: schema_version must be ${BUNDLE_SCHEMA_VERSION}`);
  }
  if (example.posture !== POSTURE) {
    fail(`${examplePath}: posture must be ${POSTURE}`);
  }
  requireString(example.bundle_id, "bundle_id", examplePath);
  requireString(example.study, "study", examplePath);
  requireArray(example.reproducibility?.initial_command, "initial_command", examplePath);
  requireArray(example.reproducibility?.plan_next_command_template, "plan_next_command_template", examplePath);
  requireArray(example.reproducibility?.run_next_command_template, "run_next_command_template", examplePath);
  requireArray(example.reproducibility?.chain_next_command_template, "chain_next_command_template", examplePath);
  requireString(example.summary?.winner_candidate_id, "summary.winner_candidate_id", examplePath);
  requireString(example.summary?.reliability_decision, "summary.reliability_decision", examplePath);
  requireString(example.summary?.next_round_decision, "summary.next_round_decision", examplePath);
  requireString(example.summary?.chain_stop_reason, "summary.chain_stop_reason", examplePath);
  assertChecksum(example, "initial_exploration_sha256", "initial_exploration");
  assertChecksum(example, "next_round_execution_plan_sha256", "next_round_execution_plan");
  assertChecksum(example, "next_exploration_sha256", "next_exploration");
  assertChecksum(example, "chain_sha256", "chain");
  if (example.initial_exploration?.schema_version !== EXPLORATION_SCHEMA_VERSION) {
    fail(`${examplePath}: initial_exploration schema must be ${EXPLORATION_SCHEMA_VERSION}`);
  }
  if (example.next_round_execution_plan?.schema_version !== EXECUTION_SCHEMA_VERSION) {
    fail(`${examplePath}: next_round_execution_plan schema must be ${EXECUTION_SCHEMA_VERSION}`);
  }
  if (example.next_exploration?.schema_version !== EXPLORATION_SCHEMA_VERSION) {
    fail(`${examplePath}: next_exploration schema must be ${EXPLORATION_SCHEMA_VERSION}`);
  }
  if (example.chain?.schema_version !== CHAIN_SCHEMA_VERSION) {
    fail(`${examplePath}: chain schema must be ${CHAIN_SCHEMA_VERSION}`);
  }
}

function assertChecksum(example, checksumKey, artifactKey) {
  const actual = example.artifact_checksums?.[checksumKey];
  const expected = sha256(example[artifactKey]);
  if (actual !== expected) {
    fail(`${examplePath}: ${checksumKey} must match ${artifactKey}`);
  }
}

function checkDocumentation() {
  const schemasReadme = readText(schemasReadmePath);
  for (const requiredPath of [schemaPath, examplePath]) {
    if (!schemasReadme.includes(path.basename(requiredPath))) {
      fail(`${schemasReadmePath}: missing entry for ${path.basename(requiredPath)}`);
    }
  }
  for (const docPath of [scriptsReadmePath, docsPath]) {
    const text = readText(docPath);
    for (const requiredPath of [schemaPath, examplePath]) {
      if (!text.includes(requiredPath)) {
        fail(`${docPath}: missing reference to ${requiredPath}`);
      }
    }
  }
}

function checkContracts() {
  checkSchema(readJson(schemaPath));
  checkExample(readJson(examplePath));
  checkDocumentation();
  console.log("material research bundle contract check passed");
}

function runSelfTest() {
  const example = readJson(examplePath);
  example.artifact_checksums.chain_sha256 = "0".repeat(64);
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    checkExample(example);
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject a bad retained artifact checksum");
  }
  console.log("material research bundle contract check self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
