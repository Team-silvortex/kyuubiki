#!/usr/bin/env node
import childProcess from "node:child_process";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const PLAN_SCHEMA_VERSION = "kyuubiki.material-study-execution-plan/v1";

const examples = [
  {
    name: "rust",
    command: "cargo",
    args: [
      "run",
      "--quiet",
      "--manifest-path",
      "sdks/rust/Cargo.toml",
      "--example",
      "plan_material_study",
    ],
  },
  {
    name: "python",
    command: "python3",
    args: ["sdks/python/examples/plan_material_study.py"],
    env: { PYTHONPATH: path.join(repoRoot, "sdks/python") },
  },
  {
    name: "elixir",
    command: "mix",
    args: ["run", "examples/plan_material_study.exs"],
    cwd: path.join(repoRoot, "sdks/elixir"),
  },
];

function fail(message) {
  console.error(`material study SDK example check failed: ${message}`);
  process.exit(1);
}

function runExample(example) {
  return childProcess.execFileSync(example.command, example.args, {
    cwd: example.cwd || repoRoot,
    env: { ...process.env, ...(example.env || {}) },
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function parseJsonOutput(name, output) {
  const start = output.indexOf("{");
  const end = output.lastIndexOf("}");
  if (start < 0 || end < start) {
    fail(`${name}: output did not contain a JSON object`);
  }
  try {
    return JSON.parse(output.slice(start, end + 1));
  } catch (error) {
    fail(`${name}: output JSON could not be parsed: ${error.message}`);
  }
}

function checkPlan(name, plan) {
  if (plan.schema_version !== PLAN_SCHEMA_VERSION) {
    fail(`${name}: schema_version must be ${PLAN_SCHEMA_VERSION}`);
  }
  if (plan.study_id !== "material_heat_spreader_screening") {
    fail(`${name}: unexpected study_id ${plan.study_id}`);
  }
  if (!Array.isArray(plan.steps) || plan.step_count !== plan.steps.length) {
    fail(`${name}: step_count must match steps length`);
  }
  if (!Array.isArray(plan.candidate_ids) || plan.candidate_count !== plan.candidate_ids.length) {
    fail(`${name}: candidate_count must match candidate_ids length`);
  }
  if (!plan.candidate_ids.includes("copper_c110")) {
    fail(`${name}: expected copper_c110 candidate`);
  }
  if (!String(plan.recommended_command || "").includes("heat-spreader")) {
    fail(`${name}: recommended_command must expose the heat-spreader alias`);
  }
}

function runSelfTest() {
  const badOutput = "compiled\n{\"schema_version\":\"wrong\",\"steps\":[],\"step_count\":0}";
  const plan = parseJsonOutput("self-test", badOutput);
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    checkPlan("self-test", plan);
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject an invalid schema version");
  }
  console.log("material study SDK example check self-test passed");
}

function checkExamples() {
  for (const example of examples) {
    const output = runExample(example);
    checkPlan(example.name, parseJsonOutput(example.name, output));
  }
  console.log("material study SDK example check passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkExamples();
}
