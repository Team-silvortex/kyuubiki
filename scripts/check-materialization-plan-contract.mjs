#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const schemaPath = "schemas/material-candidate-materialization-plan.schema.json";
const examplePath = "schemas/examples.material-candidate-materialization-plan.json";
const sdkReadmePath = "sdks/README.md";

const PLAN_SCHEMA_VERSION = "kyuubiki.material-candidate-materialization-plan/v1";
const SPEC_SCHEMA_VERSION = "kyuubiki.materialized-candidate-spec/v1";
const PLAN_STATUS = "ready_for_solver_rerun";
const SPEC_STATUS = "requires_solver_rerun";

function fail(message) {
  console.error(`materialization plan contract check failed: ${message}`);
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

function checkSchema(schema) {
  const schemaVersion = schema?.properties?.schema_version?.const;
  if (schemaVersion !== PLAN_SCHEMA_VERSION) {
    fail(`${schemaPath}: schema_version const must be ${PLAN_SCHEMA_VERSION}`);
  }
  const status = schema?.properties?.status?.const;
  if (status !== PLAN_STATUS) {
    fail(`${schemaPath}: status const must be ${PLAN_STATUS}`);
  }
  const candidateSchema = schema?.$defs?.materializedCandidate;
  if (candidateSchema?.properties?.schema_version?.const !== SPEC_SCHEMA_VERSION) {
    fail(`${schemaPath}: candidate schema_version const must be ${SPEC_SCHEMA_VERSION}`);
  }
  if (candidateSchema?.properties?.status?.const !== SPEC_STATUS) {
    fail(`${schemaPath}: candidate status const must be ${SPEC_STATUS}`);
  }
}

function checkExample(example) {
  if (example.schema_version !== PLAN_SCHEMA_VERSION) {
    fail(`${examplePath}: schema_version must be ${PLAN_SCHEMA_VERSION}`);
  }
  if (example.status !== PLAN_STATUS) {
    fail(`${examplePath}: status must be ${PLAN_STATUS}`);
  }
  if (!Array.isArray(example.materialized_candidates)) {
    fail(`${examplePath}: materialized_candidates must be an array`);
  }
  if (example.materialized_candidates.length === 0) {
    fail(`${examplePath}: materialized_candidates must not be empty`);
  }
  if (example.materialized_candidate_count !== example.materialized_candidates.length) {
    fail(`${examplePath}: materialized_candidate_count must match candidate length`);
  }
  requireString(example.source_request_schema_version, "source_request_schema_version", examplePath);
  requireString(example.required_result_schema, "required_result_schema", examplePath);
  example.materialized_candidates.forEach((candidate, index) => {
    const context = `${examplePath}#materialized_candidates/${index}`;
    if (candidate.schema_version !== SPEC_SCHEMA_VERSION) {
      fail(`${context}: schema_version must be ${SPEC_SCHEMA_VERSION}`);
    }
    if (candidate.status !== SPEC_STATUS) {
      fail(`${context}: status must be ${SPEC_STATUS}`);
    }
    for (const field of [
      "candidate_id",
      "source_draft_id",
      "source_candidate_id",
      "strategy",
      "study",
      "required_result_schema",
    ]) {
      requireString(candidate[field], field, context);
    }
  });
}

function checkDocumentation() {
  const readme = readText(sdkReadmePath);
  for (const requiredPath of [schemaPath, examplePath]) {
    if (!readme.includes(requiredPath)) {
      fail(`${sdkReadmePath}: missing link to ${requiredPath}`);
    }
  }
}

function checkContracts() {
  checkSchema(readJson(schemaPath));
  checkExample(readJson(examplePath));
  checkDocumentation();
  console.log("materialization plan contract check passed");
}

function runSelfTest() {
  const example = readJson(examplePath);
  example.materialized_candidate_count += 1;
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
    fail("self-test did not reject candidate count mismatch");
  }
  console.log("materialization plan contract check self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
