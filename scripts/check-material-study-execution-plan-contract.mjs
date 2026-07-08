#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const schemaPath = "schemas/material-study-execution-plan.schema.json";
const examplePath = "schemas/examples.material-study-execution-plan.json";
const sdkReadmePath = "sdks/README.md";
const sdkExamplePaths = [
  "sdks/rust/examples/plan_material_study.rs",
  "sdks/python/examples/plan_material_study.py",
  "sdks/elixir/examples/plan_material_study.exs",
];
const sdkReadmePaths = [
  "sdks/rust/README.md",
  "sdks/python/README.md",
  "sdks/elixir/README.md",
];

const PLAN_SCHEMA_VERSION = "kyuubiki.material-study-execution-plan/v1";

function fail(message) {
  console.error(`material study execution plan contract check failed: ${message}`);
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
  for (const field of ["study_id", "step_count", "solve_step_count", "candidate_count", "candidate_ids", "actions", "steps"]) {
    if (!schema.required?.includes(field)) {
      fail(`${schemaPath}: missing required field ${field}`);
    }
  }
  if (schema?.$defs?.workflowStep?.properties?.action?.type !== "string") {
    fail(`${schemaPath}: workflowStep.action must be a string`);
  }
}

function checkExample(example) {
  if (example.schema_version !== PLAN_SCHEMA_VERSION) {
    fail(`${examplePath}: schema_version must be ${PLAN_SCHEMA_VERSION}`);
  }
  requireString(example.study_id, "study_id", examplePath);
  if (!Array.isArray(example.steps) || example.steps.length === 0) {
    fail(`${examplePath}: steps must be a non-empty array`);
  }
  if (!Array.isArray(example.actions) || example.actions.length !== example.steps.length) {
    fail(`${examplePath}: actions length must match steps length`);
  }
  if (example.step_count !== example.steps.length) {
    fail(`${examplePath}: step_count must match steps length`);
  }
  const solveSteps = example.steps.filter((step) => step.action?.startsWith("solve_"));
  if (example.solve_step_count !== solveSteps.length) {
    fail(`${examplePath}: solve_step_count must match solve_* steps`);
  }
  if (!Array.isArray(example.candidate_ids) || example.candidate_ids.length === 0) {
    fail(`${examplePath}: candidate_ids must be non-empty`);
  }
  if (example.candidate_count !== example.candidate_ids.length) {
    fail(`${examplePath}: candidate_count must match candidate_ids length`);
  }
  example.steps.forEach((step, index) => {
    const context = `${examplePath}#steps/${index}`;
    requireString(step.action, "action", context);
    if (!step.payload || typeof step.payload !== "object" || Array.isArray(step.payload)) {
      fail(`${context}: payload must be an object`);
    }
  });
  for (const candidateId of example.candidate_ids) {
    if (!solveSteps.some((step) => step.payload?.research?.candidate_id === candidateId)) {
      fail(`${examplePath}: candidate_id ${candidateId} is not present in solve step research`);
    }
  }
}

function checkDocumentation() {
  const readme = readText(sdkReadmePath);
  for (const requiredPath of [schemaPath, examplePath]) {
    if (!readme.includes(requiredPath)) {
      fail(`${sdkReadmePath}: missing link to ${requiredPath}`);
    }
  }
  for (const example of sdkExamplePaths) {
    const linkPath = example.replace("sdks/", "");
    if (!readme.includes(linkPath)) {
      fail(`${sdkReadmePath}: missing SDK example link ${linkPath}`);
    }
    if (!fs.existsSync(path.join(repoRoot, example))) {
      fail(`${example}: SDK example file is missing`);
    }
  }
  for (const readmePath of sdkReadmePaths) {
    const languageReadme = readText(readmePath);
    if (!languageReadme.includes("material_study_execution_plan")) {
      fail(`${readmePath}: missing material study execution-plan helper docs`);
    }
  }
}

function checkContracts() {
  checkSchema(readJson(schemaPath));
  checkExample(readJson(examplePath));
  checkDocumentation();
  console.log("material study execution plan contract check passed");
}

function runSelfTest() {
  const example = readJson(examplePath);
  example.step_count += 1;
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
    fail("self-test did not reject step count mismatch");
  }
  console.log("material study execution plan contract check self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
