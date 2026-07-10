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

const reportExamples = [
  {
    name: "python-report",
    command: "python3",
    args: ["sdks/python/examples/run_material_report.py"],
    env: { PYTHONPATH: path.join(repoRoot, "sdks/python") },
  },
  {
    name: "elixir-report",
    command: "mix",
    args: ["run", "examples/run_material_report.exs"],
    cwd: path.join(repoRoot, "sdks/elixir"),
  },
];

const bundleExamples = [
  {
    name: "rust-bundle",
    command: "cargo",
    args: [
      "run",
      "--quiet",
      "--manifest-path",
      "sdks/rust/Cargo.toml",
      "--example",
      "validate_material_research_bundle",
    ],
  },
  {
    name: "python-bundle",
    command: "python3",
    args: ["sdks/python/examples/validate_material_research_bundle.py"],
    env: { PYTHONPATH: path.join(repoRoot, "sdks/python") },
  },
  {
    name: "elixir-bundle",
    command: "mix",
    args: ["run", "examples/validate_material_research_bundle.exs"],
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

function parseKeyValueOutput(name, output) {
  const pairs = Object.fromEntries(
    output
      .split(/\r?\n/u)
      .map((line) => line.trim())
      .filter(Boolean)
      .filter((line) => line.includes("="))
      .map((line) => {
        const separator = line.indexOf("=");
        return [line.slice(0, separator), line.slice(separator + 1)];
      }),
  );
  if (!pairs.study || !pairs.winner || !pairs.reliability) {
    fail(`${name}: expected study, winner, and reliability key-value lines`);
  }
  return pairs;
}

function checkReportExample(name, report) {
  if (report.study !== "material.composite_thermo_electric_panel.v1") {
    fail(`${name}: unexpected study ${report.study}`);
  }
  if (report.winner !== "copper_polyimide_aluminum") {
    fail(`${name}: unexpected winner ${report.winner}`);
  }
  if (!["ready_for_next_round", "blocked_by_quality_gates"].includes(report.reliability)) {
    fail(`${name}: unexpected reliability decision ${report.reliability}`);
  }
}

function checkBundleExample(name, bundle) {
  if (bundle.schema !== "kyuubiki.material-research-bundle/v1") {
    fail(`${name}: unexpected schema ${bundle.schema}`);
  }
  if (bundle.study !== "heat-spreader") {
    fail(`${name}: unexpected study ${bundle.study}`);
  }
  if (bundle.winner !== "pyrolytic_graphite_in_plane") {
    fail(`${name}: unexpected winner ${bundle.winner}`);
  }
  if (bundle.reliability !== "blocked_by_quality_gates") {
    fail(`${name}: unexpected reliability decision ${bundle.reliability}`);
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
  const report = parseKeyValueOutput(
    "self-test-report",
    "study=wrong\nwinner=copper_polyimide_aluminum\nreliability=ready_for_next_round\n",
  );
  failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-report-fail");
  };
  try {
    checkReportExample("self-test-report", report);
  } catch (error) {
    if (error.message !== "self-test-report-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject an invalid report example");
  }
  const bundle = parseKeyValueOutput(
    "self-test-bundle",
    "schema=wrong\nstudy=heat-spreader\nwinner=pyrolytic_graphite_in_plane\nreliability=blocked_by_quality_gates\n",
  );
  failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-bundle-fail");
  };
  try {
    checkBundleExample("self-test-bundle", bundle);
  } catch (error) {
    if (error.message !== "self-test-bundle-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject an invalid bundle example");
  }
  console.log("material study SDK example check self-test passed");
}

function checkExamples() {
  for (const example of examples) {
    const output = runExample(example);
    checkPlan(example.name, parseJsonOutput(example.name, output));
  }
  for (const example of reportExamples) {
    const output = runExample(example);
    checkReportExample(example.name, parseKeyValueOutput(example.name, output));
  }
  for (const example of bundleExamples) {
    const output = runExample(example);
    checkBundleExample(example.name, parseKeyValueOutput(example.name, output));
  }
  console.log("material study SDK example check passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkExamples();
}
