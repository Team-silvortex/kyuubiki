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
  const relative = path.relative(repoRoot, path.resolve(value));
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    errors.push(`${context} must stay inside the repository`);
  }
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
  checkDynamicSmoke(readRepoJson(examplePath), examplePath);
  const readme = readRepoText(schemasReadmePath);
  for (const expected of [schemaPath.split("/").at(-1), examplePath.split("/").at(-1)]) {
    if (!readme.includes(expected)) {
      fail(`${schemasReadmePath}: missing ${expected}`);
    }
  }
}

function runSelfTest() {
  const sample = {
    schema_version: schemaVersion,
    generated_at: "2026-07-12T00:00:00Z",
    ok: true,
    template_manifest: path.join(repoRoot, "workers/rust/templates/operator-crate-template/Cargo.toml"),
    package_manifest: path.join(repoRoot, "workers/rust/templates/operator-crate-template/kyuubiki-operator.json"),
    preflight_report: path.join(repoRoot, "tmp/operator-package-dynamic-preflight.json"),
    dynamic_library: path.join(repoRoot, "workers/rust/templates/operator-crate-template/target/debug/libkyuubiki_operator_template.dylib"),
    stages: requiredStages.map((id) => ({ id, status: 0, ok: true })),
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
