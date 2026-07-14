#!/usr/bin/env node
import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const TENSOR_PATH = "config/architecture/module-function-coverage-tensor.json";
const TOPOLOGY_PATH = "config/architecture/module-topology.json";
const MATRIX_PATH = "config/architecture/module-function-coverage-matrix.json";
const DEFAULT_OUT = "tmp/module-function-coverage-tensor.json";
const SCHEMA_VERSION = "kyuubiki.module-function-coverage-tensor/v1";
const REPORT_SCHEMA_VERSION = "kyuubiki.module-function-coverage-tensor-report/v1";
const ALLOWED_STATUS = new Set(["covered", "partial", "planned", "not_applicable"]);
const GAP_ORDER = new Map([
  ["required_gap", 0],
  ["weak", 1],
  ["weak_evidence", 2],
  ["planned", 3],
  ["watch", 4],
  ["missing", 5],
  ["ok", 6],
  ["not_applicable", 7],
]);

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

function readText(relativePath) {
  return readFileSync(repoPath(relativePath), "utf8");
}

function ensureArray(value, label) {
  if (!Array.isArray(value)) throw new Error(`${label} must be an array`);
  return value;
}

function validateTensorConfig(tensor, topology, matrix) {
  if (tensor.schema_version !== SCHEMA_VERSION) {
    throw new Error(`schema_version must be ${SCHEMA_VERSION}`);
  }
  if (tensor.topology !== TOPOLOGY_PATH) throw new Error(`topology must be ${TOPOLOGY_PATH}`);
  if (tensor.matrix !== MATRIX_PATH) throw new Error(`matrix must be ${MATRIX_PATH}`);
  if (!tensor.depth_axes || Object.keys(tensor.depth_axes).length === 0) {
    throw new Error("depth_axes must not be empty");
  }
  for (const [axis, description] of Object.entries(tensor.depth_axes)) {
    if (!description || typeof description !== "string") throw new Error(`depth axis ${axis} must describe itself`);
  }

  const paradigms = new Set(Object.keys(matrix.paradigms ?? {}));
  const benchmarkLanes = new Set(Object.keys(topology.benchmark_lanes ?? {}));
  const securityLanes = new Set(Object.keys(topology.security_lanes ?? {}));
  for (const paradigm of paradigms) {
    const mapping = tensor.paradigm_lanes?.[paradigm];
    if (!mapping) throw new Error(`missing paradigm lane mapping for ${paradigm}`);
    for (const lane of ensureArray(mapping.benchmark, `${paradigm}.benchmark`)) {
      if (!benchmarkLanes.has(lane)) throw new Error(`${paradigm}: unknown benchmark lane ${lane}`);
    }
    for (const lane of ensureArray(mapping.security, `${paradigm}.security`)) {
      if (!securityLanes.has(lane)) throw new Error(`${paradigm}: unknown security lane ${lane}`);
    }
  }
  for (const paradigm of Object.keys(tensor.paradigm_lanes ?? {})) {
    if (!paradigms.has(paradigm)) throw new Error(`tensor maps unknown paradigm ${paradigm}`);
  }
  for (const [paradigm, entries] of Object.entries(tensor.paradigm_contract_evidence ?? {})) {
    if (!paradigms.has(paradigm)) throw new Error(`tensor evidence maps unknown paradigm ${paradigm}`);
    validateContractEvidenceEntries(entries, paradigm);
  }
}

function validateContractEvidenceEntries(entries, paradigm) {
  const seen = new Set();
  for (const [index, entry] of ensureArray(entries, `${paradigm}.contract_evidence`).entries()) {
    if (!entry.id || typeof entry.id !== "string") {
      throw new Error(`${paradigm}.contract_evidence[${index}] must have id`);
    }
    if (seen.has(entry.id)) throw new Error(`${paradigm}.contract_evidence duplicate id ${entry.id}`);
    seen.add(entry.id);
    let combinedText = "";
    for (const file of ensureArray(entry.files, `${entry.id}.files`)) {
      combinedText += `\n${readText(file)}`;
    }
    for (const requiredText of ensureArray(entry.required_text, `${entry.id}.required_text`)) {
      if (!combinedText.includes(requiredText)) {
        throw new Error(`${entry.id}: evidence bundle missing ${requiredText}`);
      }
    }
  }
}

function getLaneTests(topology, laneKind, lanes) {
  const plan = topology.lane_test_plan?.[laneKind] ?? {};
  return lanes.flatMap((lane) => {
    const tests = plan[lane] ?? [];
    return tests.map((test) => ({
      lane,
      id: test.id,
      command: test.command,
      scope: test.scope,
    }));
  });
}

function intersect(left, right) {
  const rightSet = new Set(right);
  return left.filter((item) => rightSet.has(item));
}

function deriveGap(status, required) {
  if (!ALLOWED_STATUS.has(status)) return required ? "required_gap" : "missing";
  if (status === "covered") return "ok";
  if (status === "partial") return required ? "weak" : "watch";
  if (status === "planned") return required ? "required_gap" : "planned";
  if (status === "not_applicable") return required ? "required_gap" : "not_applicable";
  return "missing";
}

function deriveEvidenceAwareGap(status, required, evidenceDepth) {
  const gap = deriveGap(status, required);
  if (gap !== "ok" || !required) return gap;
  const hasRunnableEvidence = evidenceDepth.test_command_count > 0;
  const hasContractEvidence = evidenceDepth.contract_evidence_count > 0;
  return hasRunnableEvidence || hasContractEvidence ? "ok" : "weak_evidence";
}

function contractEvidenceFor(tensor, paradigm) {
  return (tensor.paradigm_contract_evidence?.[paradigm] ?? []).map((entry) => ({
    id: entry.id,
    files: entry.files,
    required_text: entry.required_text,
  }));
}

function emptyCounts() {
  return {
    ok: 0,
    weak: 0,
    watch: 0,
    planned: 0,
    weak_evidence: 0,
    required_gap: 0,
    missing: 0,
    not_applicable: 0,
  };
}

function increment(counts, key) {
  counts[key] = (counts[key] ?? 0) + 1;
}

function buildTensorReport(tensor, topology, matrix) {
  const paradigms = Object.keys(matrix.paradigms);
  const requiredByModule = matrix.required_by_module ?? {};
  const moduleSummary = {};
  const paradigmSummary = Object.fromEntries(paradigms.map((paradigm) => [paradigm, emptyCounts()]));
  const cells = {};
  const gaps = [];

  for (const module of topology.modules) {
    const moduleCells = {};
    const moduleCounts = emptyCounts();
    const requiredSet = new Set(requiredByModule[module.id] ?? []);
    for (const paradigm of paradigms) {
      const status = matrix.cells?.[module.id]?.[paradigm] ?? "not_applicable";
      const required = requiredSet.has(paradigm);
      const mapping = tensor.paradigm_lanes[paradigm];
      const contractEvidence = contractEvidenceFor(tensor, paradigm);
      const benchmarkLanes = intersect(module.benchmark_lanes ?? [], mapping.benchmark);
      const securityLanes = intersect(module.security_lanes ?? [], mapping.security);
      const evidenceDepth = {
        benchmark_lane_count: benchmarkLanes.length,
        security_lane_count: securityLanes.length,
        contract_evidence_count: contractEvidence.length,
        test_command_count:
          getLaneTests(topology, "benchmark", benchmarkLanes).length +
          getLaneTests(topology, "security", securityLanes).length,
      };
      const gap = deriveEvidenceAwareGap(status, required, evidenceDepth);
      const cell = {
        status,
        required,
        gap,
        benchmark_lanes: benchmarkLanes,
        security_lanes: securityLanes,
        benchmark_tests: getLaneTests(topology, "benchmark", benchmarkLanes),
        security_tests: getLaneTests(topology, "security", securityLanes),
        contract_evidence: contractEvidence,
        evidence_depth: evidenceDepth,
      };
      moduleCells[paradigm] = cell;
      increment(moduleCounts, gap);
      increment(paradigmSummary[paradigm], gap);
      if (!["ok", "not_applicable"].includes(gap)) {
        gaps.push({
          gap,
          module_id: module.id,
          paradigm,
          status,
          required,
          benchmark_lanes: benchmarkLanes,
          security_lanes: securityLanes,
        });
      }
    }
    cells[module.id] = moduleCells;
    moduleSummary[module.id] = {
      layer: module.layer,
      counts: moduleCounts,
    };
  }

  gaps.sort((left, right) => {
    const severity = (GAP_ORDER.get(left.gap) ?? 99) - (GAP_ORDER.get(right.gap) ?? 99);
    if (severity !== 0) return severity;
    return `${left.module_id}/${left.paradigm}`.localeCompare(`${right.module_id}/${right.paradigm}`);
  });

  return {
    schema_version: REPORT_SCHEMA_VERSION,
    source: TENSOR_PATH,
    topology: TOPOLOGY_PATH,
    matrix: MATRIX_PATH,
    ok: !gaps.some((gap) => gap.gap === "required_gap" || gap.gap === "missing"),
    axes: {
      modules: topology.modules.map((module) => module.id),
      paradigms,
      depth: Object.keys(tensor.depth_axes),
    },
    module_summary: moduleSummary,
    paradigm_summary: paradigmSummary,
    paradigm_contract_evidence: tensor.paradigm_contract_evidence ?? {},
    gap_count: gaps.length,
    blocking_gap_count: gaps.filter((gap) => gap.gap === "required_gap" || gap.gap === "missing").length,
    gaps,
    cells,
  };
}

function renderMarkdown(report) {
  const lines = [
    "# Module Function Coverage Tensor",
    "",
    `- Source: \`${TENSOR_PATH}\``,
    `- Topology: \`${TOPOLOGY_PATH}\``,
    `- Matrix: \`${MATRIX_PATH}\``,
    `- Axes: \`module x function_paradigm x evidence_depth\``,
    `- Modules: \`${report.axes.modules.length}\``,
    `- Paradigms: \`${report.axes.paradigms.length}\``,
    `- Depth axes: \`${report.axes.depth.join("`, `")}\``,
    `- Blocking gaps: \`${report.blocking_gap_count}\``,
    "",
    "## Module Summary",
    "",
    "| Module | Layer | OK | Weak | Weak Evidence | Watch | Planned | Required Gap | Missing | N/A |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
  ];
  for (const [moduleId, summary] of Object.entries(report.module_summary)) {
    const counts = summary.counts;
    lines.push(
      `| \`${moduleId}\` | \`${summary.layer}\` | ${counts.ok} | ${counts.weak} | ${counts.weak_evidence} | ${counts.watch} | ${counts.planned} | ${counts.required_gap} | ${counts.missing} | ${counts.not_applicable} |`,
    );
  }

  lines.push("", "## Paradigm Summary", "");
  lines.push("| Paradigm | OK | Weak | Weak Evidence | Watch | Planned | Required Gap | Missing | N/A |");
  lines.push("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
  for (const [paradigm, counts] of Object.entries(report.paradigm_summary)) {
    lines.push(
      `| \`${paradigm}\` | ${counts.ok} | ${counts.weak} | ${counts.weak_evidence} | ${counts.watch} | ${counts.planned} | ${counts.required_gap} | ${counts.missing} | ${counts.not_applicable} |`,
    );
  }

  lines.push("", "## Contract Evidence", "");
  const contractEvidenceEntries = Object.entries(report.paradigm_contract_evidence ?? {});
  if (contractEvidenceEntries.length === 0) {
    lines.push("No contract evidence.");
  } else {
    lines.push("| Paradigm | Evidence | Files | Required Text |");
    lines.push("| --- | --- | --- | --- |");
    for (const [paradigm, entries] of contractEvidenceEntries) {
      for (const entry of entries) {
        lines.push(
          `| \`${paradigm}\` | \`${entry.id}\` | ${entry.files.map((file) => `\`${file}\``).join(", ")} | ${entry.required_text.map((text) => `\`${text}\``).join(", ")} |`,
        );
      }
    }
  }

  lines.push("", "## Gaps", "");
  if (report.gaps.length === 0) {
    lines.push("No gaps.");
  } else {
    lines.push("| Gap | Module | Paradigm | Status | Required | Benchmark Lanes | Security Lanes |");
    lines.push("| --- | --- | --- | --- | --- | --- | --- |");
    for (const gap of report.gaps) {
      lines.push(
        `| \`${gap.gap}\` | \`${gap.module_id}\` | \`${gap.paradigm}\` | \`${gap.status}\` | \`${gap.required}\` | \`${gap.benchmark_lanes.join(", ") || "-"}\` | \`${gap.security_lanes.join(", ") || "-"}\` |`,
      );
    }
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

function writeReport(report, outPath) {
  const absolute = repoPath(outPath);
  mkdirSync(path.dirname(absolute), { recursive: true });
  writeFileSync(absolute, `${JSON.stringify(report, null, 2)}\n`);
  writeFileSync(path.join(path.dirname(absolute), "module-function-coverage-tensor.md"), renderMarkdown(report));
}

function runSelfTest() {
  assert.equal(deriveGap("covered", true), "ok");
  assert.equal(deriveEvidenceAwareGap("covered", true, { test_command_count: 0, contract_evidence_count: 0 }), "weak_evidence");
  assert.equal(deriveEvidenceAwareGap("covered", true, { test_command_count: 1, contract_evidence_count: 0 }), "ok");
  assert.equal(deriveGap("partial", true), "weak");
  assert.equal(deriveGap("partial", false), "watch");
  assert.equal(deriveGap("planned", true), "required_gap");
  const topology = {
    benchmark_lanes: { runtime_solver: "r" },
    security_lanes: { data_contract: "d" },
    lane_test_plan: {
      benchmark: { runtime_solver: [{ id: "rt", command: "make test-rust", scope: "local" }] },
      security: { data_contract: [{ id: "schema", command: "make architecture-check", scope: "release" }] },
    },
    modules: [
      {
        id: "engine",
        layer: "runtime_data_plane",
        benchmark_lanes: ["runtime_solver"],
        security_lanes: ["data_contract"],
      },
    ],
  };
  const matrix = {
    paradigms: { solver_execution: "s" },
    required_by_module: { engine: ["solver_execution"] },
    cells: { engine: { solver_execution: "partial" } },
  };
  const tensor = {
    schema_version: SCHEMA_VERSION,
    topology: TOPOLOGY_PATH,
    matrix: MATRIX_PATH,
    depth_axes: { required: "r", status: "s" },
    paradigm_lanes: {
      solver_execution: { benchmark: ["runtime_solver"], security: ["data_contract"] },
    },
  };
  validateTensorConfig(tensor, topology, matrix);
  const report = buildTensorReport(tensor, topology, matrix);
  assert.equal(report.blocking_gap_count, 0);
  assert.equal(report.module_summary.engine.counts.weak, 1);
  assert.equal(report.cells.engine.solver_execution.evidence_depth.test_command_count, 2);
  console.log("module function coverage tensor self-test passed");
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    runSelfTest();
    process.exit(0);
  }
  const tensor = readJson(TENSOR_PATH);
  const topology = readJson(TOPOLOGY_PATH);
  const matrix = readJson(MATRIX_PATH);
  validateTensorConfig(tensor, topology, matrix);
  const report = buildTensorReport(tensor, topology, matrix);
  writeReport(report, options.out);
  console.log(
    `module function coverage tensor ${report.ok ? "passed" : "has gaps"}: ${report.axes.modules.length} module(s), ${report.axes.paradigms.length} paradigm(s), ${report.gap_count} gap(s)`,
  );
  if (!report.ok) process.exit(1);
} catch (error) {
  console.error(`module function coverage tensor failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
