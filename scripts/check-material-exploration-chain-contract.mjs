#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const schemaPath = "schemas/material-exploration-chain.schema.json";
const examplePath = "schemas/examples.material-exploration-chain.json";
const schemasReadmePath = "schemas/README.md";
const sdkReadmePath = "sdks/README.md";

const CHAIN_SCHEMA_VERSION = "kyuubiki.material-exploration-chain/v1";
const CONVERGENCE_SCHEMA_VERSION = "kyuubiki.material-chain-convergence-assessment/v1";
const OBJECTIVES_SCHEMA_VERSION = "kyuubiki.material-next-round-optimization-objectives/v1";

function fail(message) {
  console.error(`material exploration chain contract check failed: ${message}`);
  process.exit(1);
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
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
  if (schema?.properties?.schema_version?.const !== CHAIN_SCHEMA_VERSION) {
    fail(`${schemaPath}: schema_version const must be ${CHAIN_SCHEMA_VERSION}`);
  }
  const convergence = schema?.$defs?.convergenceAssessment;
  if (convergence?.properties?.schema_version?.const !== CONVERGENCE_SCHEMA_VERSION) {
    fail(`${schemaPath}: convergence schema_version const must be ${CONVERGENCE_SCHEMA_VERSION}`);
  }
  const objectives = schema?.$defs?.optimizationObjectives;
  if (objectives?.properties?.schema_version?.const !== OBJECTIVES_SCHEMA_VERSION) {
    fail(`${schemaPath}: objectives schema_version const must be ${OBJECTIVES_SCHEMA_VERSION}`);
  }
  for (const field of ["convergence_assessment", "optimization_trace", "summaries", "runs"]) {
    if (!schema.required?.includes(field)) {
      fail(`${schemaPath}: missing required field ${field}`);
    }
  }
}

function checkExample(example) {
  if (example.schema_version !== CHAIN_SCHEMA_VERSION) {
    fail(`${examplePath}: schema_version must be ${CHAIN_SCHEMA_VERSION}`);
  }
  if (example.convergence_assessment?.schema_version !== CONVERGENCE_SCHEMA_VERSION) {
    fail(`${examplePath}: convergence schema_version must be ${CONVERGENCE_SCHEMA_VERSION}`);
  }
  requireArray(example.optimization_trace, "optimization_trace", examplePath);
  requireArray(example.summaries, "summaries", examplePath);
  requireArray(example.runs, "runs", examplePath);
  if (example.round_count !== example.runs.length) {
    fail(`${examplePath}: round_count must match runs length`);
  }
  if (example.optimization_trace.length !== example.summaries.length) {
    fail(`${examplePath}: optimization_trace length must match summaries length`);
  }
  if (example.summaries.length !== example.runs.length) {
    fail(`${examplePath}: summaries length must match runs length`);
  }
  example.optimization_trace.forEach((entry, index) => {
    const context = `${examplePath}#optimization_trace/${index}`;
    requireString(entry.decision, "decision", context);
    requireString(entry.mode, "mode", context);
    requireString(entry.winner_candidate_id, "winner_candidate_id", context);
    requireArray(entry.primary_metric_ids, "primary_metric_ids", context);
  });
  example.summaries.forEach((summary, index) => {
    const context = `${examplePath}#summaries/${index}`;
    requireString(summary.winner_candidate_id, "winner_candidate_id", context);
    if (!Number.isFinite(summary.winner_score)) {
      fail(`${context}: winner_score must be finite`);
    }
    if (summary.optimization_objectives?.schema_version !== OBJECTIVES_SCHEMA_VERSION) {
      fail(`${context}: optimization_objectives schema_version must be ${OBJECTIVES_SCHEMA_VERSION}`);
    }
    requireArray(summary.optimization_objectives.primary_metric_ids, "primary_metric_ids", context);
  });
}

function checkDocumentation() {
  const schemasReadme = readText(schemasReadmePath);
  for (const requiredPath of [schemaPath, examplePath]) {
    const fileName = path.basename(requiredPath);
    if (!schemasReadme.includes(fileName)) {
      fail(`${schemasReadmePath}: missing entry for ${fileName}`);
    }
  }
  const sdkReadme = readText(sdkReadmePath);
  for (const requiredPath of [schemaPath, examplePath]) {
    if (!sdkReadme.includes(requiredPath)) {
      fail(`${sdkReadmePath}: missing link to ${requiredPath}`);
    }
  }
}

function checkContracts() {
  checkSchema(readJson(schemaPath));
  checkExample(readJson(examplePath));
  checkDocumentation();
  console.log("material exploration chain contract check passed");
}

function runSelfTest() {
  const example = readJson(examplePath);
  example.round_count += 1;
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
    fail("self-test did not reject round count mismatch");
  }
  console.log("material exploration chain contract check self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
