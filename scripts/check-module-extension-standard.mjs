#!/usr/bin/env node
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const STANDARD_PATH = "config/architecture/module-extension-standard.json";
const SCHEMA_PATH = "schemas/module-extension-standard.schema.json";
const SCHEMA_VERSION = "kyuubiki.module-extension-standard/v1";
const REQUIRED_TYPES = new Set(["module", "function_paradigm", "service_surface", "evidence_lane", "contract_family"]);

function repoPath(relativePath) {
  const absolute = path.resolve(ROOT, relativePath);
  const relative = path.relative(ROOT, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`path must stay inside repository: ${relativePath}`);
  }
  return absolute;
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(repoPath(relativePath), "utf8"));
}

function assertRepoFile(relativePath) {
  if (!relativePath || relativePath.startsWith("/") || relativePath.includes("..")) {
    throw new Error(`invalid repository-relative path: ${relativePath}`);
  }
  if (!existsSync(repoPath(relativePath))) throw new Error(`missing required path: ${relativePath}`);
}

function listMakeTargets() {
  const targets = new Set();
  for (const file of ["Makefile", "make/checks.mk", "make/help.mk", "make/tests.mk", "make/benchmarks.mk"]) {
    if (!existsSync(repoPath(file))) continue;
    const text = readFileSync(repoPath(file), "utf8");
    for (const match of text.matchAll(/^([a-zA-Z0-9_.-]+):/gm)) targets.add(match[1]);
  }
  return targets;
}

function validateStandard(standard) {
  if (standard.$schema !== "../../schemas/module-extension-standard.schema.json") {
    throw new Error("standard must point at module-extension-standard schema");
  }
  if (standard.schema_version !== SCHEMA_VERSION) throw new Error(`schema_version must be ${SCHEMA_VERSION}`);
  assertRepoFile(SCHEMA_PATH);
  for (const file of Object.values(standard.source_of_truth ?? {})) assertRepoFile(file);
  if (standard.evidence_rules?.required_cell_without_evidence !== "weak_evidence") {
    throw new Error("extension standard must preserve weak_evidence for empty required coverage");
  }

  const seenTypes = new Set();
  for (const entry of standard.extension_types ?? []) {
    if (!REQUIRED_TYPES.has(entry.id)) throw new Error(`unknown extension type: ${entry.id}`);
    if (seenTypes.has(entry.id)) throw new Error(`duplicate extension type: ${entry.id}`);
    seenTypes.add(entry.id);
    for (const file of entry.required_files ?? []) assertRepoFile(file);
    if (!Array.isArray(entry.steps) || entry.steps.length < 3) {
      throw new Error(`${entry.id} must have at least three onboarding steps`);
    }
  }
  for (const type of REQUIRED_TYPES) {
    if (!seenTypes.has(type)) throw new Error(`missing extension type: ${type}`);
  }

  const makeTargets = listMakeTargets();
  for (const gate of standard.gates ?? []) {
    const target = gate.command?.match(/^make ([a-zA-Z0-9_.-]+)$/)?.[1];
    if (!target) throw new Error(`gate ${gate.id} must use a make target command`);
    if (!makeTargets.has(target)) throw new Error(`gate ${gate.id} points at unknown make target ${target}`);
  }

  const docsText = readFileSync(repoPath(standard.source_of_truth.docs), "utf8");
  for (const anchor of ["Adding A Module", "Adding A Function Paradigm", "Adding A Service Surface", "weak_evidence"]) {
    if (!docsText.includes(anchor)) throw new Error(`extension standard docs missing ${anchor}`);
  }
}

function runSelfTest() {
  const fixture = {
    $schema: "../../schemas/module-extension-standard.schema.json",
    schema_version: SCHEMA_VERSION,
    source_of_truth: {
      topology: "config/architecture/module-topology.json",
      matrix: "config/architecture/module-function-coverage-matrix.json",
      tensor: "config/architecture/module-function-coverage-tensor.json",
      docs: "docs/architecture-extension-standard.md",
    },
    evidence_rules: { required_cell_without_evidence: "weak_evidence" },
    extension_types: [...REQUIRED_TYPES].map((id) => ({
      id,
      required_files: ["config/architecture/module-topology.json"],
      steps: ["first required step", "second required step", "third required step"],
    })),
    gates: [{ id: "topology", command: "make check-module-topology" }],
  };
  validateStandard(fixture);
  assert.throws(() => validateStandard({ ...fixture, evidence_rules: { required_cell_without_evidence: "ok" } }));
  console.log("module extension standard self-test passed");
}

try {
  if (process.argv.includes("--self-test")) {
    runSelfTest();
    process.exit(0);
  }
  validateStandard(readJson(STANDARD_PATH));
  console.log("module extension standard passed");
} catch (error) {
  console.error(`module extension standard failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
