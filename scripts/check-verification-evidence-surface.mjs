#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const SURFACE_PATH = "config/architecture/verification-evidence-surface.json";
const SCHEMA_VERSION = "kyuubiki.verification-evidence-surface/v1";
const COVERED_PARADIGMS = [
  "runtime_api",
  "solver_execution",
  "workflow_composition",
  "deployment_update",
  "sdk_headless",
];

function repoPath(relativePath) {
  const absolute = path.resolve(ROOT, relativePath);
  const relative = path.relative(ROOT, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`path escapes repository: ${relativePath}`);
  }
  return absolute;
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(repoPath(relativePath), "utf8"));
}

function assertPathExists(relativePath, label) {
  if (!existsSync(repoPath(relativePath))) {
    throw new Error(`${label} path does not exist: ${relativePath}`);
  }
}

function assertCommand(command) {
  const [binary, script] = command.split(/\s+/u);
  if (binary !== "node") throw new Error(`runtime command must use node: ${command}`);
  assertPathExists(script, "runtime command script");
}

function assertEvidenceSource(source) {
  assertPathExists(source, "evidence source");
}

function assertSurface(surface) {
  if (surface.schema_version !== SCHEMA_VERSION) {
    throw new Error(`schema_version must be ${SCHEMA_VERSION}`);
  }
  if (surface.module_id !== "verification-evidence") {
    throw new Error("module_id must be verification-evidence");
  }
  assertPathExists(surface.matrix, "matrix");
  assertPathExists(surface.tensor, "tensor");
  for (const command of surface.runtime_api?.stable_commands ?? []) {
    assertCommand(command);
  }
  for (const artifact of surface.runtime_api?.generated_artifacts ?? []) {
    if (!artifact.startsWith("tmp/")) {
      throw new Error(`generated artifact must live under tmp/: ${artifact}`);
    }
  }
  for (const paradigm of COVERED_PARADIGMS) {
    const block = surface[paradigm];
    if (!block) throw new Error(`missing ${paradigm} evidence block`);
    if (paradigm === "runtime_api") {
      continue;
    }
    const sources = block.evidence_sources ?? [];
    if (sources.length === 0) {
      throw new Error(`${paradigm} must list evidence_sources`);
    }
    for (const source of sources) {
      assertEvidenceSource(source);
    }
  }
}

function assertMatrixAlignment(surface) {
  const matrix = readJson(surface.matrix);
  const row = matrix.cells?.[surface.module_id];
  if (!row) throw new Error(`missing matrix row for ${surface.module_id}`);
  for (const paradigm of COVERED_PARADIGMS) {
    if (row[paradigm] !== "covered") {
      throw new Error(`${surface.module_id}/${paradigm} must be covered`);
    }
  }
}

try {
  const surface = readJson(SURFACE_PATH);
  assertSurface(surface);
  assertMatrixAlignment(surface);
  console.log(
    `verification evidence surface passed: ${COVERED_PARADIGMS.length} covered evidence paradigm(s)`,
  );
} catch (error) {
  console.error(
    `verification evidence surface failed: ${error instanceof Error ? error.message : String(error)}`,
  );
  process.exit(1);
}
