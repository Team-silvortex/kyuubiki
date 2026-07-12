#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import {
  operatorReliabilityPaths,
  operatorReliabilitySchemaVersions,
} from "./operator-reliability-contracts.mjs";
import {
  allowedLevels,
  isBelowMinimumCoverageLevel,
  qualificationEvidenceErrors,
  qualificationEvidenceKitErrors,
  qualificationRoadmapErrors,
} from "./operator-reliability-rules.mjs";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const catalogPath = path.join(repoRoot, "workers/rust/crates/benchmark/src/catalog_defaults.rs");
const workflowPayloadPath = path.join(repoRoot, "workers/rust/crates/benchmark/src/workflow_payloads.rs");

function fail(message) {
  console.error(`operator reliability check failed: ${message}`);
  process.exit(1);
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function extractArrayBlock(source, marker) {
  const markerIndex = source.indexOf(marker);
  if (markerIndex < 0) {
    fail(`missing marker ${marker}`);
  }
  const start = source.indexOf("&[", markerIndex);
  if (start < 0) {
    fail(`missing string array after ${marker}`);
  }
  let depth = 0;
  for (let index = start; index < source.length; index += 1) {
    const char = source[index];
    if (char === "[") {
      depth += 1;
    } else if (char === "]") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(start, index + 1);
      }
    }
  }
  fail(`unterminated array after ${marker}`);
}

function extractQuotedStrings(source) {
  return [...source.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

function physicsCoverageTemplates() {
  const source = fs.readFileSync(catalogPath, "utf8");
  return new Set(extractQuotedStrings(extractArrayBlock(source, '"physics-coverage"')));
}

function workflowOperatorIds() {
  const source = fs.readFileSync(workflowPayloadPath, "utf8");
  return new Set([...source.matchAll(/payload\("([^"]+)"/g)].map((match) => match[1]));
}

function ensureFileContains(relativePath, needle, context) {
  const absolutePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(absolutePath)) {
    fail(`${context}: evidence file does not exist: ${relativePath}`);
  }
  if (needle && !fs.readFileSync(absolutePath, "utf8").includes(needle)) {
    fail(`${context}: evidence file ${relativePath} does not contain ${needle}`);
  }
}

function evidenceReferencePath(reference) {
  return reference.split("#")[0];
}

function validateEvidenceReferences(references, requiredNeedles, context, fieldName) {
  if (!Array.isArray(references) || references.length === 0) {
    fail(`${context}: evidence.review.${fieldName} must be non-empty`);
  }
  for (const reference of references) {
    const relativePath = evidenceReferencePath(reference);
    for (const needle of requiredNeedles) {
      ensureFileContains(relativePath, needle, context);
    }
  }
}

function makeTargetSources() {
  const sources = [fs.readFileSync(path.join(repoRoot, "Makefile"), "utf8")];
  const makeDir = path.join(repoRoot, "make");
  if (fs.existsSync(makeDir)) {
    for (const entry of fs.readdirSync(makeDir).sort()) {
      if (entry.endsWith(".mk")) {
        sources.push(fs.readFileSync(path.join(makeDir, entry), "utf8"));
      }
    }
  }
  return sources.join("\n");
}

function validateReviewEvidence(entry, context) {
  const review = entry.evidence.review;
  if (!review) {
    fail(`${context}: review-level operators must declare evidence.review`);
  }
  for (const field of ["assumptions", "boundary_checks", "diagnostics", "tests"]) {
    if (!Array.isArray(review[field]) || review[field].length === 0) {
      fail(`${context}: evidence.review.${field} must be non-empty`);
    }
  }
  for (const testPath of review.tests) {
    ensureFileContains(testPath, null, context);
  }
}

function validateStokesReviewEvidence(entry, context) {
  const stokesOperators = new Set([
    "solve.stokes_flow_quad_2d",
    "solve.stokes_flow_triangle_2d",
  ]);
  if (!stokesOperators.has(entry.operator_id)) {
    return;
  }
  const review = entry.evidence.review;
  validateEvidenceReferences(
    review.scope_notes,
    ["CFD Stokes Screening Scope", "Stokes-only", "screening", "Navier-Stokes"],
    context,
    "scope_notes"
  );
  validateEvidenceReferences(
    review.tolerance_notes.filter((reference) => reference.endsWith(".md#cfd-stokes-divergence-tolerance")),
    ["CFD Stokes Divergence Tolerance", "1e-10", "divergence"],
    context,
    "tolerance_notes"
  );
  validateEvidenceReferences(
    review.tolerance_notes.filter((reference) => reference.endsWith(".json")),
    ["stokes_screening_divergence", "1e-10", "Navier-Stokes", "mesh-convergence"],
    context,
    "tolerance_notes"
  );
  for (const requiredLimit of ["stokes_only", "screening_only", "screening_divergence_tolerance_1e-10"]) {
    if (!entry.limits.includes(requiredLimit)) {
      fail(`${context}: limits must include ${requiredLimit}`);
    }
  }
}

function validateElectromagneticPlaneReviewEvidence(entry, context) {
  const planeOperators = new Set([
    "solve.electrostatic_plane_triangle_2d",
    "solve.electrostatic_plane_quad_2d",
    "solve.magnetostatic_plane_triangle_2d",
    "solve.magnetostatic_plane_quad_2d",
  ]);
  if (!planeOperators.has(entry.operator_id)) {
    return;
  }
  const review = entry.evidence.review;
  validateEvidenceReferences(
    review.scope_notes,
    ["Electromagnetic Plane Review Scope", "single-patch", "orientation", "qualification"],
    context,
    "scope_notes"
  );
  validateEvidenceReferences(
    review.material_notes,
    ["Electromagnetic Plane Material", "linear material", "permittivity", "permeability", "stored energy"],
    context,
    "material_notes"
  );
  for (const requiredLimit of ["linear_material", "two_dimensional"]) {
    if (!entry.limits.includes(requiredLimit)) {
      fail(`${context}: limits must include ${requiredLimit}`);
    }
  }
}

function validateThermalPlaneReviewEvidence(entry, context) {
  const planeOperators = new Set([
    "solve.heat_plane_triangle_2d",
    "solve.heat_plane_quad_2d",
    "solve.thermal_plane_triangle_2d",
    "solve.thermal_plane_quad_2d",
  ]);
  if (!planeOperators.has(entry.operator_id)) {
    return;
  }
  const review = entry.evidence.review;
  validateEvidenceReferences(
    review.scope_notes,
    ["Thermal Plane Review Scope", "mesh convergence", "boundary coverage", "qualification"],
    context,
    "scope_notes"
  );
  validateEvidenceReferences(
    review.material_notes,
    ["Thermal Plane Material", "linear", "conductivity", "thermal expansion", "material-card"],
    context,
    "material_notes"
  );
  const requiredLimits =
    entry.domain === "thermal" ? ["steady_state", "linear_conductivity"] : ["linear_plane_stress"];
  for (const requiredLimit of requiredLimits) {
    if (!entry.limits.includes(requiredLimit)) {
      fail(`${context}: limits must include ${requiredLimit}`);
    }
  }
}

function validateQualificationEvidence(entry, context, options = {}) {
  const errors = qualificationEvidenceErrors(entry);
  if (errors.length > 0) {
    fail(`${context}: ${errors[0]}`);
  }
  const qualification = entry.evidence.qualification;
  if (options.checkFiles !== false) {
    for (const testPath of qualification.tests) {
      ensureFileContains(testPath, null, context);
    }
  }
}

function loadManifest() {
  const manifest = readJson(operatorReliabilityPaths.manifest);
  if (Array.isArray(manifest.operators)) {
    return manifest;
  }
  if (!Array.isArray(manifest.shards) || manifest.shards.length === 0) {
    fail("manifest must declare operators or non-empty shards");
  }

  const operators = [];
  const seenShards = new Set();
  for (const shardPath of manifest.shards) {
    if (seenShards.has(shardPath)) {
      fail(`duplicate shard path ${shardPath}`);
    }
    seenShards.add(shardPath);

    const shard = readJson(shardPath);
    if (shard.schema_version !== operatorReliabilitySchemaVersions.shard) {
      fail(`${shardPath}: unexpected shard schema_version`);
    }
    if (!shard.domain) {
      fail(`${shardPath}: missing domain`);
    }
    if (!Array.isArray(shard.operators) || shard.operators.length === 0) {
      fail(`${shardPath}: operators must be non-empty`);
    }
    for (const entry of shard.operators) {
      if (entry.domain !== shard.domain) {
        fail(`${shardPath}: ${entry.operator_id} domain must match shard domain`);
      }
      operators.push(entry);
    }
  }

  return { ...manifest, operators };
}

function validateQualificationEvidenceKits(manifest, roadmap) {
  const kitsPath = operatorReliabilityPaths.evidenceKits;
  const kits = readJson(kitsPath);
  const errors = qualificationEvidenceKitErrors(kits, roadmap, manifest);
  if (errors.length > 0) {
    fail(`qualification evidence kits: ${errors[0]}`);
  }
  const makefile = makeTargetSources();
  for (const kit of kits.kits) {
    for (const requirement of kit.artifact_requirements ?? []) {
      if (requirement.artifact_path) {
        ensureFileContains(
          requirement.artifact_path,
          null,
          `qualification evidence kit ${kit.candidate_id}:${requirement.artifact_id}`
        );
      }
      if (requirement.artifact_command && !makefile.includes(requirement.artifact_command)) {
        fail(
          `qualification evidence kit ${kit.candidate_id}:${requirement.artifact_id}: ` +
            `artifact_command is not discoverable in Make target sources`
        );
      }
    }
  }
}

function validateQualificationRoadmap(manifest, seenOperators, operatorLevels) {
  const roadmapPath = operatorReliabilityPaths.roadmap;
  const roadmap = readJson(roadmapPath);
  const errors = qualificationRoadmapErrors(roadmap, manifest, seenOperators, operatorLevels);
  if (errors.length > 0) {
    fail(`qualification roadmap: ${errors[0]}`);
  }
  validateQualificationEvidenceKits(manifest, roadmap);
}

function validate() {
  const manifest = loadManifest();
  if (manifest.schema_version !== operatorReliabilitySchemaVersions.manifest) {
    fail("unexpected schema_version");
  }
  if (manifest.coverage_matrix !== "physics-coverage") {
    fail("coverage_matrix must be physics-coverage");
  }
  for (const level of manifest.levels ?? []) {
    if (!allowedLevels.has(level)) {
      fail(`unknown level ${level}`);
    }
  }
  if (!allowedLevels.has(manifest.minimum_coverage_level)) {
    fail(`unknown minimum_coverage_level ${manifest.minimum_coverage_level}`);
  }

  const expectedTemplates = physicsCoverageTemplates();
  const operatorIds = workflowOperatorIds();
  const seenTemplates = new Set();
  const seenOperators = new Set();
  const operatorLevels = new Map();
  const levelCounts = new Map();

  for (const entry of manifest.operators ?? []) {
    const context = entry.operator_id ?? entry.benchmark_template ?? "unknown operator";
    if (!allowedLevels.has(entry.coverage_level)) {
      fail(`${context}: unknown coverage_level ${entry.coverage_level}`);
    }
    if (isBelowMinimumCoverageLevel(entry.coverage_level, manifest.minimum_coverage_level)) {
      fail(
        `${context}: coverage_level ${entry.coverage_level} is below manifest minimum ` +
          `${manifest.minimum_coverage_level}`
      );
    }
    if (entry.evidence?.benchmark_matrix !== manifest.coverage_matrix) {
      fail(`${context}: benchmark_matrix must match manifest coverage_matrix`);
    }
    if (entry.evidence?.headless_workflow !== true) {
      fail(`${context}: headless_workflow must be true for physics coverage`);
    }
    if (!expectedTemplates.has(entry.benchmark_template)) {
      fail(`${context}: benchmark_template is not in physics-coverage`);
    }
    if (!operatorIds.has(entry.operator_id)) {
      fail(`${context}: operator_id is not exported by workflow_payloads`);
    }
    if (seenTemplates.has(entry.benchmark_template)) {
      fail(`${context}: duplicate benchmark_template ${entry.benchmark_template}`);
    }
    if (seenOperators.has(entry.operator_id)) {
      fail(`${context}: duplicate operator_id`);
    }
    seenTemplates.add(entry.benchmark_template);
    seenOperators.add(entry.operator_id);
    operatorLevels.set(entry.operator_id, entry.coverage_level);
    levelCounts.set(entry.coverage_level, (levelCounts.get(entry.coverage_level) ?? 0) + 1);

    for (const evidencePath of entry.evidence.tests ?? []) {
      ensureFileContains(evidencePath, null, context);
    }
    if (entry.evidence.accuracy_baseline) {
      ensureFileContains(entry.evidence.tests[0], entry.evidence.accuracy_baseline, context);
    }
    if (entry.evidence.reliability_suite) {
      ensureFileContains(entry.evidence.tests[0], null, context);
    }
    if (entry.coverage_level === "review" || entry.coverage_level === "qualification") {
      validateReviewEvidence(entry, context);
      validateStokesReviewEvidence(entry, context);
      validateElectromagneticPlaneReviewEvidence(entry, context);
      validateThermalPlaneReviewEvidence(entry, context);
    }
    if (entry.coverage_level === "qualification") {
      validateQualificationEvidence(entry, context);
    }
    if (!Array.isArray(entry.limits) || entry.limits.length === 0) {
      fail(`${context}: limits must be non-empty`);
    }
  }

  const missing = [...expectedTemplates].filter((template) => !seenTemplates.has(template));
  if (missing.length > 0) {
    fail(`physics-coverage templates missing from manifest: ${missing.join(", ")}`);
  }
  validateQualificationRoadmap(manifest, seenOperators, operatorLevels);

  console.log(
    `operator reliability manifest ok: ${manifest.operators.length} operators, ` +
      `${[...levelCounts.entries()].map(([level, count]) => `${level}=${count}`).join(", ")}`
  );
}

validate();
