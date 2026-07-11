#!/usr/bin/env node
import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const MATRIX_PATH = "config/architecture/module-function-coverage-matrix.json";
const TOPOLOGY_PATH = "config/architecture/module-topology.json";
const DEFAULT_OUT = "tmp/module-function-matrix-report.json";
const SCHEMA_VERSION = "kyuubiki.module-function-coverage-matrix/v1";
const ALLOWED_STATUS = new Set(["covered", "partial", "planned", "not_applicable"]);
const BLOCKING_STATUS = new Set(["planned", "not_applicable"]);

function parseArgs(argv) {
  const options = { out: DEFAULT_OUT };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--out" && next) {
      options.out = next;
      index += 1;
    } else if (arg === "--self-test") {
      options.selfTest = true;
    } else {
      throw new Error(`unknown argument ${arg}`);
    }
  }
  return options;
}

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

function validateMatrix(matrix, topology) {
  if (matrix.schema_version !== SCHEMA_VERSION) {
    throw new Error(`schema_version must be ${SCHEMA_VERSION}`);
  }
  if (matrix.topology !== TOPOLOGY_PATH) {
    throw new Error(`topology must be ${TOPOLOGY_PATH}`);
  }
  const paradigms = new Set(Object.keys(matrix.paradigms ?? {}));
  if (paradigms.size === 0) throw new Error("paradigms must not be empty");
  const moduleIds = new Set(topology.modules.map((module) => module.id));
  const matrixModuleIds = new Set(Object.keys(matrix.cells ?? {}));
  for (const moduleId of moduleIds) {
    if (!matrixModuleIds.has(moduleId)) throw new Error(`missing matrix row for module ${moduleId}`);
  }
  for (const moduleId of matrixModuleIds) {
    if (!moduleIds.has(moduleId)) throw new Error(`matrix row references unknown module ${moduleId}`);
  }

  const findings = [];
  for (const [moduleId, requiredParadigms] of Object.entries(matrix.required_by_module ?? {})) {
    if (!moduleIds.has(moduleId)) throw new Error(`required_by_module references unknown module ${moduleId}`);
    for (const paradigm of requiredParadigms) {
      if (!paradigms.has(paradigm)) throw new Error(`${moduleId}: unknown required paradigm ${paradigm}`);
      const status = matrix.cells[moduleId]?.[paradigm];
      if (!ALLOWED_STATUS.has(status)) {
        findings.push({ severity: "fail", module_id: moduleId, paradigm, status: status ?? "missing" });
      } else if (BLOCKING_STATUS.has(status)) {
        findings.push({ severity: "warn", module_id: moduleId, paradigm, status });
      }
    }
  }

  for (const [moduleId, cells] of Object.entries(matrix.cells)) {
    for (const [paradigm, status] of Object.entries(cells)) {
      if (!paradigms.has(paradigm)) throw new Error(`${moduleId}: unknown paradigm ${paradigm}`);
      if (!ALLOWED_STATUS.has(status)) throw new Error(`${moduleId}/${paradigm}: unknown status ${status}`);
    }
  }
  return findings;
}

function buildReport(matrix, topology, findings) {
  const paradigms = Object.keys(matrix.paradigms);
  const rows = topology.modules.map((module) => {
    const cells = Object.fromEntries(
      paradigms.map((paradigm) => [paradigm, matrix.cells[module.id]?.[paradigm] ?? "not_applicable"]),
    );
    const covered = Object.values(cells).filter((status) => status === "covered").length;
    const partial = Object.values(cells).filter((status) => status === "partial").length;
    const planned = Object.values(cells).filter((status) => status === "planned").length;
    return { module_id: module.id, layer: module.layer, cells, covered, partial, planned };
  });
  return {
    schema_version: "kyuubiki.module-function-matrix-report/v1",
    source: MATRIX_PATH,
    topology: TOPOLOGY_PATH,
    ok: !findings.some((finding) => finding.severity === "fail"),
    paradigm_count: paradigms.length,
    module_count: rows.length,
    findings,
    rows,
  };
}

function renderMarkdown(report, matrix) {
  const paradigms = Object.keys(matrix.paradigms);
  const lines = [
    "# Module Function Coverage Matrix",
    "",
    `- Source: \`${MATRIX_PATH}\``,
    `- Modules: \`${report.module_count}\``,
    `- Paradigms: \`${report.paradigm_count}\``,
    `- Status: \`${report.ok ? "pass" : "fail"}\``,
    "",
    "| Module | Layer | Covered | Partial | Planned |",
    "| --- | --- | ---: | ---: | ---: |",
  ];
  for (const row of report.rows) {
    lines.push(`| \`${row.module_id}\` | \`${row.layer}\` | \`${row.covered}\` | \`${row.partial}\` | \`${row.planned}\` |`);
  }
  lines.push("", "## Matrix", "");
  lines.push(`| Module | ${paradigms.map((item) => `\`${item}\``).join(" | ")} |`);
  lines.push(`| --- | ${paradigms.map(() => "---").join(" | ")} |`);
  for (const row of report.rows) {
    lines.push(`| \`${row.module_id}\` | ${paradigms.map((item) => `\`${row.cells[item]}\``).join(" | ")} |`);
  }
  if (report.findings.length > 0) {
    lines.push("", "## Findings", "");
    for (const finding of report.findings) {
      lines.push(`- \`${finding.severity}\`: \`${finding.module_id}\` / \`${finding.paradigm}\` is \`${finding.status}\``);
    }
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

function writeReport(report, matrix, outPath) {
  const absolute = repoPath(outPath);
  mkdirSync(path.dirname(absolute), { recursive: true });
  writeFileSync(absolute, `${JSON.stringify(report, null, 2)}\n`);
  writeFileSync(path.join(path.dirname(absolute), "module-function-matrix.md"), renderMarkdown(report, matrix));
}

function runSelfTest() {
  const topology = { modules: [{ id: "a", layer: "contract" }] };
  const matrix = {
    schema_version: SCHEMA_VERSION,
    topology: TOPOLOGY_PATH,
    paradigms: { validation: "v" },
    required_by_module: { a: ["validation"] },
    cells: { a: { validation: "covered" } },
  };
  assert.deepEqual(validateMatrix(matrix, topology), []);
  matrix.cells.a.validation = "planned";
  assert.equal(validateMatrix(matrix, topology)[0].severity, "warn");
  matrix.cells.a.validation = "missing";
  assert.throws(() => validateMatrix(matrix, topology), /unknown status/u);
  console.log("module function matrix self-test passed");
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    runSelfTest();
    process.exit(0);
  }
  const matrix = readJson(MATRIX_PATH);
  const topology = readJson(TOPOLOGY_PATH);
  const findings = validateMatrix(matrix, topology);
  const report = buildReport(matrix, topology, findings);
  writeReport(report, matrix, options.out);
  console.log(`module function matrix ${report.ok ? "passed" : "failed"}: ${report.module_count} module(s), ${report.paradigm_count} paradigm(s)`);
  if (!report.ok) process.exit(1);
} catch (error) {
  console.error(`module function matrix failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
