#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const schemaPath = "schemas/operator-task-ir.schema.json";
const examplePaths = [
  "schemas/examples.operator-task-ir.json",
  "schemas/examples.operator-task-batch.json",
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
  for (const field of ["operator", "execution_program", "runtime_hints"]) {
    if (!fields.includes(field)) {
      fail(`${context}: task digest must cover ${field}`);
    }
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
  for (const examplePath of examplePaths) {
    const tasks = collectTasks(readJson(examplePath));
    if (tasks.length === 0) {
      fail(`${examplePath}: no TaskIR examples found`);
    }
    tasks.forEach((task, index) => {
      const context = `${examplePath}#task-${index + 1}`;
      validateMirrorConstraints(task, constraints, context);
      validateDigestFieldCoverage(task, context);
      taskCount += 1;
    });
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
  console.log("operator task IR contract check self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
} else {
  checkContracts();
}
