#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const schemaPath = "schemas/operator-task-ir.schema.json";
const examplePaths = [
  "schemas/examples.operator-task-ir.json",
  "schemas/examples.operator-task-ir-float.json",
  "schemas/examples.operator-task-ir-elixir.json",
  "schemas/examples.operator-task-batch.json",
];
const requiredAuthoringModes = ["rust_native", "elixir_control_plane"];
const requiredDigestFields = [
  "schema_version",
  "task_id",
  "operator",
  "descriptor_authoring",
  "node",
  "input_artifact",
  "config",
  "execution_program",
  "dataset_contract",
  "orchestration_context",
  "runtime_hints",
];

function fail(message) {
  console.error(`operator task IR contract check failed: ${message}`);
  process.exit(1);
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function pointerGet(value, pointer) {
  if (pointer === "") {
    return value;
  }
  return pointer
    .split("/")
    .slice(1)
    .reduce((current, segment) => {
      const key = segment.replace(/~1/g, "/").replace(/~0/g, "~");
      return current == null ? undefined : current[key];
    }, value);
}

function collectTasks(value) {
  const tasks = [];
  if (value?.schema_version === "kyuubiki.operator-task-ir/v1") {
    tasks.push(value);
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      tasks.push(...collectTasks(item));
    }
    return tasks;
  }
  if (value && typeof value === "object") {
    for (const child of Object.values(value)) {
      tasks.push(...collectTasks(child));
    }
  }
  return tasks;
}

function validateMirrorConstraints(task, constraints, context) {
  for (const constraint of constraints) {
    const source = pointerGet(task, constraint.source);
    const mirror = pointerGet(task, constraint.mirror);
    if (source == null || mirror == null) {
      continue;
    }
    if (source !== mirror) {
      fail(
        `${context}: ${constraint.mirror} must mirror ${constraint.source} (${constraint.reason})`,
      );
    }
  }
}

function validateDigestFieldCoverage(task, context) {
  const fields = task.integrity?.task_digest_fields;
  if (!Array.isArray(fields)) {
    fail(`${context}: integrity.task_digest_fields must be an array`);
  }
  if (fields.join("\n") !== requiredDigestFields.join("\n")) {
    fail(`${context}: integrity.task_digest_fields must match the canonical field order`);
  }
}

function canonicalJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  if (typeof value === "number") {
    return canonicalNumber(value);
  }
  return JSON.stringify(value);
}

function canonicalNumber(value) {
  if (!Number.isFinite(value)) {
    fail("canonical JSON cannot encode non-finite numbers");
  }
  if (Number.isInteger(value)) {
    return `${value}`;
  }
  let encoded = value.toFixed(15);
  while (encoded.endsWith("0")) {
    encoded = encoded.slice(0, -1);
  }
  if (encoded.endsWith(".")) {
    encoded += "0";
  }
  return encoded;
}

function computeTaskDigest(task) {
  const digestPayload = {};
  for (const field of requiredDigestFields) {
    if (Object.hasOwn(task, field)) {
      digestPayload[field] = task[field];
    }
  }
  return sha256Canonical(digestPayload);
}

function computeDescriptorDigest(task) {
  if (!task.operator || typeof task.operator !== "object" || Array.isArray(task.operator)) {
    fail("operator task descriptor digest requires an operator object");
  }
  return sha256Canonical(task.operator);
}

function sha256Canonical(value) {
  return crypto.createHash("sha256").update(canonicalJson(value), "utf8").digest("hex");
}

function validateDescriptorDigest(task, context) {
  const expected = task.integrity?.descriptor_digest;
  if (typeof expected !== "string" || expected.length === 0) {
    fail(`${context}: integrity.descriptor_digest must be a non-empty string`);
  }
  const actual = computeDescriptorDigest(task);
  if (expected !== actual) {
    fail(
      `${context}: integrity.descriptor_digest mismatch; expected ${expected}, computed ${actual}`,
    );
  }
}

function validateTaskDigest(task, context) {
  const expected = task.integrity?.task_digest;
  if (typeof expected !== "string" || expected.length === 0) {
    fail(`${context}: integrity.task_digest must be a non-empty string`);
  }
  const actual = computeTaskDigest(task);
  if (expected !== actual) {
    fail(`${context}: integrity.task_digest mismatch; expected ${expected}, computed ${actual}`);
  }
}

function checkContracts() {
  const schema = readJson(schemaPath);
  const constraints = schema["x-kyuubiki-mirror_constraints"];
  if (!Array.isArray(constraints) || constraints.length === 0) {
    fail(`${schemaPath}: missing x-kyuubiki-mirror_constraints`);
  }
  for (const [index, constraint] of constraints.entries()) {
    for (const field of ["source", "mirror", "reason"]) {
      if (typeof constraint[field] !== "string" || constraint[field].length === 0) {
        fail(`${schemaPath}: mirror constraint ${index} missing ${field}`);
      }
    }
  }

  let taskCount = 0;
  const authoringModes = new Set();
  for (const examplePath of examplePaths) {
    const tasks = collectTasks(readJson(examplePath));
    if (tasks.length === 0) {
      fail(`${examplePath}: no TaskIR examples found`);
    }
    tasks.forEach((task, index) => {
      const context = `${examplePath}#task-${index + 1}`;
      validateMirrorConstraints(task, constraints, context);
      validateDigestFieldCoverage(task, context);
      validateDescriptorDigest(task, context);
      validateTaskDigest(task, context);
      if (typeof task.descriptor_authoring?.mode === "string") {
        authoringModes.add(task.descriptor_authoring.mode);
      }
      taskCount += 1;
    });
  }
  for (const mode of requiredAuthoringModes) {
    if (!authoringModes.has(mode)) {
      fail(`TaskIR examples must include descriptor_authoring.mode=${mode}`);
    }
  }

  console.log(`Validated ${taskCount} operator task IR example contracts.`);
}

function runSelfTest() {
  const task = {
    operator: { kind: "transform" },
    execution_program: { program_kind: "solver" },
  };
  const constraint = {
    source: "/operator/kind",
    mirror: "/execution_program/program_kind",
    reason: "self-test",
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
    validateMirrorConstraints(task, [constraint], "self");
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject a mirror mismatch");
  }
  runDigestSelfTest();
  console.log("operator task IR contract check self-test passed");
}

function runDigestSelfTest() {
  const task = readJson(examplePaths[0]);
  task.integrity.task_digest = "0".repeat(64);
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    validateTaskDigest(task, "self-digest");
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject a digest mismatch");
  }
  runDescriptorDigestSelfTest();
}

function runDescriptorDigestSelfTest() {
  const task = readJson(examplePaths[0]);
  task.integrity.descriptor_digest = "0".repeat(64);
  const originalExit = process.exit;
  const originalError = console.error;
  let failed = false;
  console.error = () => {};
  process.exit = () => {
    failed = true;
    throw new Error("self-test-fail");
  };
  try {
    validateDescriptorDigest(task, "self-descriptor-digest");
  } catch (error) {
    if (error.message !== "self-test-fail") {
      throw error;
    }
  } finally {
    process.exit = originalExit;
    console.error = originalError;
  }
  if (!failed) {
    fail("self-test did not reject a descriptor digest mismatch");
  }
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
