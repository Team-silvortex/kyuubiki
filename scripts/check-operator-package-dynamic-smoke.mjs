#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultInput = "tmp/operator-package-dynamic-smoke.json";
const schemaPath = "schemas/operator-package-dynamic-smoke.schema.json";
const examplePath = "schemas/examples.operator-package-dynamic-smoke.json";
const schemasReadmePath = "schemas/README.md";
const schemaVersion = "kyuubiki.operator-package-dynamic-smoke/v1";
const requiredStages = [
  "template_tests",
  "strict_preflight",
  "template_cdylib_build",
  "engine_dynamic_host_load",
];

function fail(message) {
  console.error(`operator package dynamic smoke check failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { input: defaultInput, selfTest: false };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--self-test") {
      args.selfTest = true;
    } else if (argv[index] === "--in") {
      args.input = argv[++index];
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
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

function readRepoJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function readRepoText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function requireString(value, context, errors) {
  if (typeof value !== "string" || value.length === 0) {
    errors.push(`${context} must be a non-empty string`);
  }
}

function requireRepoPath(value, context, errors) {
  requireString(value, context, errors);
  if (typeof value !== "string") {
    return;
  }
  if (path.isAbsolute(value) || value.includes(repoRoot)) {
    errors.push(`${context} must be a repo-relative path`);
    return;
  }
  const relative = path.relative(repoRoot, path.resolve(repoRoot, value));
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    errors.push(`${context} must stay inside the repository`);
  }
}

function requireStringArray(value, context, errors) {
  if (!Array.isArray(value) || value.length === 0) {
    errors.push(`${context} must be a non-empty string array`);
    return;
  }
  value.forEach((item, index) => {
    requireString(item, `${context}[${index}]`, errors);
  });
}

function requirePortableCommand(value, context, errors) {
  requireStringArray(value, context, errors);
  if (!Array.isArray(value)) {
    return;
  }
  value.forEach((item, index) => {
    if (typeof item !== "string") {
      return;
    }
    if (path.isAbsolute(item) || item.includes(repoRoot)) {
      errors.push(`${context}[${index}] must not contain local absolute paths`);
    }
  });
}

function dynamicSmokeErrors(report, context) {
  const errors = [];
  if (report.schema_version !== schemaVersion) {
    errors.push(`${context}: unexpected schema_version`);
  }
  if (report.ok !== true) {
    errors.push(`${context}: ok must be true`);
  }
  requireString(report.generated_at, `${context}.generated_at`, errors);
  requireString(report.package_id, `${context}.package_id`, errors);
  requireString(report.host_version, `${context}.host_version`, errors);
  requireString(report.sdk_api_version, `${context}.sdk_api_version`, errors);
  if (!Array.isArray(report.operator_ids) || report.operator_ids.length === 0) {
    errors.push(`${context}.operator_ids must be a non-empty array`);
  } else {
    report.operator_ids.forEach((operatorId, index) => {
      requireString(operatorId, `${context}.operator_ids[${index}]`, errors);
    });
  }
  requireRepoPath(report.template_manifest, `${context}.template_manifest`, errors);
  requireRepoPath(report.package_manifest, `${context}.package_manifest`, errors);
  requireRepoPath(report.preflight_report, `${context}.preflight_report`, errors);
  requireRepoPath(report.dynamic_library, `${context}.dynamic_library`, errors);
  if (!Array.isArray(report.stages)) {
    errors.push(`${context}.stages must be an array`);
    return errors;
  }
  const actualStages = report.stages.map((stage) => stage?.id);
  if (actualStages.join("\n") !== requiredStages.join("\n")) {
    errors.push(`${context}.stages must match the canonical stage order`);
  }
  for (const [index, stage] of report.stages.entries()) {
    requireString(stage?.description, `${context}.stages[${index}].description`, errors);
    requireRepoPath(stage?.cwd, `${context}.stages[${index}].cwd`, errors);
    requirePortableCommand(stage?.command, `${context}.stages[${index}].command`, errors);
    if (stage?.ok !== true || stage?.status !== 0) {
      errors.push(`${context}.stages[${index}] must pass with status 0`);
    }
  }
  return errors;
}

function checkDynamicSmoke(report, context) {
  const errors = dynamicSmokeErrors(report, context);
  if (errors.length > 0) {
    fail(errors[0]);
  }
}

function checkSchemaAndExample() {
  const schema = readRepoJson(schemaPath);
  if (schema.properties?.schema_version?.const !== schemaVersion) {
    fail(`${schemaPath}: schema_version const must match ${schemaVersion}`);
  }
  const stageIds = schema.properties?.stages?.prefixItems?.map((item) => {
    const defName = item?.$ref?.replace("#/$defs/", "");
    return schema.$defs?.[defName]?.allOf?.[1]?.properties?.id?.const;
  });
  if (stageIds?.join("\n") !== requiredStages.join("\n")) {
    fail(`${schemaPath}: stage prefixItems must match canonical stage order`);
  }
  for (const requiredProperty of ["description", "cwd", "command"]) {
    if (!schema.$defs?.passingStage?.required?.includes(requiredProperty)) {
      fail(`${schemaPath}: passingStage must require ${requiredProperty}`);
    }
  }
  checkDynamicSmoke(readRepoJson(examplePath), examplePath);
  const readme = readRepoText(schemasReadmePath);
  for (const expected of [schemaPath.split("/").at(-1), examplePath.split("/").at(-1)]) {
    if (!readme.includes(expected)) {
      fail(`${schemasReadmePath}: missing ${expected}`);
    }
  }
}

function runSelfTest() {
  const stageFixture = (id) => ({
    id,
    description: `${id} diagnostic stage`,
    cwd: ".",
    command: ["echo", id],
    status: 0,
    ok: true,
  });
  const sample = {
    schema_version: schemaVersion,
    generated_at: "2026-07-12T00:00:00Z",
    ok: true,
    package_id: "operator.template.summary",
    operator_ids: ["extract.template_summary"],
    host_version: "1.19.0",
    sdk_api_version: "kyuubiki.operator-sdk/v1",
    template_manifest: "workers/rust/templates/operator-crate-template/Cargo.toml",
    package_manifest: "workers/rust/templates/operator-crate-template/kyuubiki-operator.json",
    preflight_report: "tmp/operator-package-dynamic-preflight.json",
    dynamic_library: "workers/rust/templates/operator-crate-template/target/debug/libkyuubiki_operator_template.dylib",
    stages: requiredStages.map(stageFixture),
  };
  checkDynamicSmoke(sample, "self-test");
  checkSchemaAndExample();
  const broken = { ...sample, stages: [...sample.stages].reverse() };
  if (!dynamicSmokeErrors(broken, "self-test").some((error) => error.includes("stage order"))) {
    fail("self-test expected reversed stages to fail");
  }
  const failedStage = {
    ...sample,
    stages: sample.stages.map((stage, index) =>
      index === 1 ? { ...stage, status: 1, ok: false } : stage,
    ),
  };
  if (!dynamicSmokeErrors(failedStage, "self-test").some((error) => error.includes("status 0"))) {
    fail("self-test expected failed stage to fail");
  }
  const missingCommand = {
    ...sample,
    stages: sample.stages.map((stage, index) =>
      index === 2 ? { ...stage, command: [] } : stage,
    ),
  };
  if (
    !dynamicSmokeErrors(missingCommand, "self-test").some((error) =>
      error.includes("command"),
    )
  ) {
    fail("self-test expected missing command to fail");
  }
  const absoluteCommand = {
    ...sample,
    stages: sample.stages.map((stage, index) =>
      index === 0 ? { ...stage, command: ["cargo", path.join(repoRoot, "Cargo.toml")] } : stage,
    ),
  };
  if (
    !dynamicSmokeErrors(absoluteCommand, "self-test").some((error) =>
      error.includes("absolute paths"),
    )
  ) {
    fail("self-test expected absolute command path to fail");
  }
  const absoluteCwd = {
    ...sample,
    stages: sample.stages.map((stage, index) =>
      index === 0 ? { ...stage, cwd: repoRoot } : stage,
    ),
  };
  if (
    !dynamicSmokeErrors(absoluteCwd, "self-test").some((error) =>
      error.includes("repo-relative path"),
    )
  ) {
    fail("self-test expected absolute cwd to fail");
  }
  console.log("operator package dynamic smoke check self-test passed");
}

const args = parseArgs(process.argv);
if (args.selfTest) {
  runSelfTest();
  process.exit(0);
}

const { absoluteInput, relativeInput } = repoLocalInput(args.input);
if (!fs.existsSync(absoluteInput)) {
  fail(`input does not exist: ${relativeInput}`);
}
checkSchemaAndExample();
checkDynamicSmoke(JSON.parse(fs.readFileSync(absoluteInput, "utf8")), relativeInput);
console.log(`operator package dynamic smoke check passed: ${relativeInput}`);
