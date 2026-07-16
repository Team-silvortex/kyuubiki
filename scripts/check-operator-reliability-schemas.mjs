#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { operatorReliabilityPaths } from "./operator-reliability-contracts.mjs";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);

const schemaContracts = [
  {
    config: operatorReliabilityPaths.manifest,
    schema: operatorReliabilityPaths.manifestSchema,
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
  {
    config: operatorReliabilityPaths.roadmap,
    schema: operatorReliabilityPaths.roadmapSchema,
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
  {
    config: operatorReliabilityPaths.evidenceKits,
    schema: operatorReliabilityPaths.evidenceKitsSchema,
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
  {
    config: operatorReliabilityPaths.releaseRecords,
    schema: operatorReliabilityPaths.releaseRecordsSchema,
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
  {
    config: operatorReliabilityPaths.releaseEvidenceExample,
    schema: operatorReliabilityPaths.releaseEvidenceSchema,
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
  {
    config: operatorReliabilityPaths.reviewDecisionExample,
    schema: operatorReliabilityPaths.reviewDecisionSchema,
    schemaVersionPath: ["properties", "schema_version", "const"],
  },
];

function fail(message) {
  console.error(`operator reliability schema check failed: ${message}`);
  process.exit(1);
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(repoRoot, relativePath), "utf8"));
}

function repoPath(relativePath) {
  const absolute = path.resolve(repoRoot, relativePath);
  const relative = path.relative(repoRoot, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    fail(`path escapes repository: ${relativePath}`);
  }
  return absolute;
}

function schemaValueAt(schema, valuePath) {
  return valuePath.reduce((value, key) => value?.[key], schema);
}

function requiredFieldErrors(value, schema, context) {
  const errors = [];
  if (!schema || typeof schema !== "object") {
    return errors;
  }

  if (Array.isArray(schema.required) && value && typeof value === "object") {
    for (const field of schema.required) {
      if (!(field in value)) {
        errors.push(`${context}: missing required field ${field}`);
      }
    }
  }

  if (schema.properties && value && typeof value === "object" && !Array.isArray(value)) {
    for (const [field, fieldSchema] of Object.entries(schema.properties)) {
      if (field in value) {
        errors.push(...requiredFieldErrors(value[field], fieldSchema, `${context}.${field}`));
      }
    }
  }

  if (schema.items && Array.isArray(value)) {
    value.forEach((item, index) => {
      errors.push(...requiredFieldErrors(item, schema.items, `${context}[${index}]`));
    });
  }

  return errors;
}

function checkRequiredFields(value, schema, context) {
  const errors = requiredFieldErrors(value, schema, context);
  if (errors.length > 0) {
    fail(errors[0]);
  }
}

function checkSchemaContract({ config, schema, schemaVersionPath }) {
  const configJson = readJson(config);
  const schemaJson = readJson(schema);
  checkRequiredFields(configJson, schemaJson, config);
  const expectedSchemaVersion = schemaValueAt(schemaJson, schemaVersionPath);
  if (!expectedSchemaVersion) {
    fail(`${schema}: missing schema_version const`);
  }
  if (configJson.schema_version !== expectedSchemaVersion) {
    fail(`${config}: schema_version must match ${schema}`);
  }
}

function checkReliabilityShards() {
  const manifest = readJson(operatorReliabilityPaths.manifest);
  const shardSchema = readJson(operatorReliabilityPaths.shardSchema);
  const expectedSchemaVersion = schemaValueAt(shardSchema, [
    "properties",
    "schema_version",
    "const",
  ]);
  if (!expectedSchemaVersion) {
    fail(`${operatorReliabilityPaths.shardSchema}: missing schema_version const`);
  }
  if (!Array.isArray(manifest.shards) || manifest.shards.length === 0) {
    fail(`${operatorReliabilityPaths.manifest}: shards must be non-empty`);
  }
  for (const shardPath of manifest.shards) {
    const shard = readJson(shardPath);
    checkRequiredFields(shard, shardSchema, shardPath);
    if (shard.schema_version !== expectedSchemaVersion) {
      fail(`${shardPath}: schema_version must match reliability shard schema`);
    }
  }
}

function listMakeTargets() {
  const targets = new Set();
  for (const file of ["Makefile", "make/checks.mk", "make/benchmarks.mk", "make/tests.mk", "make/help.mk"]) {
    const absolute = path.join(repoRoot, file);
    if (!fs.existsSync(absolute)) continue;
    const text = fs.readFileSync(absolute, "utf8");
    for (const match of text.matchAll(/^([a-zA-Z0-9_.-]+):/gm)) targets.add(match[1]);
  }
  return targets;
}

function sameItems(left, right) {
  return left.length === right.length && left.every((item, index) => item === right[index]);
}

function sortedStrings(values) {
  return [...values].sort((left, right) => left.localeCompare(right));
}

function makeTarget(command) {
  return command?.match(/^make ([a-zA-Z0-9_.-]+)$/)?.[1] ?? null;
}

function checkQualificationRoadmapClosure() {
  const roadmap = readJson(operatorReliabilityPaths.roadmap);
  const evidenceKits = loadQualificationEvidenceKits();
  if (roadmap.version_line !== evidenceKits.version_line) {
    fail(`${operatorReliabilityPaths.roadmap}: version_line must match evidence kits`);
  }

  const candidates = new Map();
  for (const candidate of roadmap.candidates ?? []) {
    if (candidates.has(candidate.candidate_id)) {
      fail(`${operatorReliabilityPaths.roadmap}: duplicate candidate ${candidate.candidate_id}`);
    }
    candidates.set(candidate.candidate_id, candidate);
  }

  const kits = new Map();
  for (const kit of evidenceKits.kits ?? []) {
    if (kits.has(kit.candidate_id)) {
      fail(`${operatorReliabilityPaths.evidenceKits}: duplicate kit ${kit.candidate_id}`);
    }
    kits.set(kit.candidate_id, kit);
  }

  for (const [candidateId, candidate] of candidates) {
    const kit = kits.get(candidateId);
    if (!kit) fail(`${candidateId}: missing evidence kit`);
    if (!sameItems(sortedStrings(candidate.operator_ids), sortedStrings(kit.operator_ids))) {
      fail(`${candidateId}: roadmap operator_ids must match evidence kit operator_ids`);
    }
    if (kit.artifact_requirements.length < candidate.required_artifacts.length) {
      fail(`${candidateId}: evidence kit must not have fewer artifact requirements than roadmap required_artifacts`);
    }
  }
  for (const candidateId of kits.keys()) {
    if (!candidates.has(candidateId)) fail(`${candidateId}: evidence kit has no roadmap candidate`);
  }

  const makeTargets = listMakeTargets();
  for (const candidate of candidates.values()) {
    const target = makeTarget(candidate.preferred_validation_lane);
    if (!target) {
      fail(`${candidate.candidate_id}: preferred_validation_lane must be a make target`);
    }
    if (!makeTargets.has(target)) {
      fail(`${candidate.candidate_id}: unknown preferred_validation_lane target ${target}`);
    }
  }
  for (const kit of kits.values()) {
    for (const artifact of kit.artifact_requirements) {
      if (artifact.artifact_path) {
        if (artifact.artifact_path.startsWith("/") || artifact.artifact_path.includes("..")) {
          fail(`${kit.candidate_id}/${artifact.artifact_id}: artifact_path must be repository-relative`);
        }
        if (["collecting", "ready_for_review"].includes(kit.status) && !fs.existsSync(repoPath(artifact.artifact_path))) {
          fail(`${kit.candidate_id}/${artifact.artifact_id}: collecting artifact_path does not exist`);
        }
      }
      if (artifact.artifact_command) {
        const target = makeTarget(artifact.artifact_command);
        if (!target) fail(`${kit.candidate_id}/${artifact.artifact_id}: artifact_command must be a make target`);
        if (!makeTargets.has(target)) fail(`${kit.candidate_id}/${artifact.artifact_id}: unknown make target ${target}`);
      }
      if (artifact.artifact_check_command) {
        const target = makeTarget(artifact.artifact_check_command);
        if (!target) fail(`${kit.candidate_id}/${artifact.artifact_id}: artifact_check_command must be a make target`);
        if (!makeTargets.has(target)) fail(`${kit.candidate_id}/${artifact.artifact_id}: unknown check make target ${target}`);
      }
    }
  }
}

function loadQualificationEvidenceKits() {
  const source = readJson(operatorReliabilityPaths.evidenceKits);
  const kits = [...(source.kits ?? [])];
  for (const shardPath of source.kit_shards ?? []) {
    const shard = readJson(shardPath);
    if (shard.schema_version !== source.schema_version) {
      fail(`${shardPath}: schema_version must match ${operatorReliabilityPaths.evidenceKits}`);
    }
    if (shard.version_line !== source.version_line) {
      fail(`${shardPath}: version_line must match ${operatorReliabilityPaths.evidenceKits}`);
    }
    if ((shard.kit_shards ?? []).length > 0) {
      fail(`${shardPath}: nested kit_shards are not supported`);
    }
    kits.push(...(shard.kits ?? []));
  }
  return { ...source, kits };
}

function runSelfTest() {
  const schema = {
    type: "object",
    required: ["schema_version", "items"],
    properties: {
      schema_version: { const: "self-test/v1" },
      items: {
        type: "array",
        items: {
          type: "object",
          required: ["id", "nested"],
          properties: {
            id: { type: "string" },
            nested: {
              type: "object",
              required: ["value"],
              properties: {
                value: { type: "string" },
              },
            },
          },
        },
      },
    },
  };
  const errors = requiredFieldErrors({ items: [{ id: "ok", nested: {} }] }, schema, "self");
  for (const expected of [
    "self: missing required field schema_version",
    "self.items[0].nested: missing required field value",
  ]) {
    if (!errors.includes(expected)) {
      fail(`self-test did not report expected error: ${expected}`);
    }
  }
  if (requiredFieldErrors({ schema_version: "self-test/v1", items: [] }, schema, "self").length > 0) {
    fail("self-test valid sample should not report required-field errors");
  }
  if (!sameItems(sortedStrings(["b", "a"]), ["a", "b"])) {
    fail("self-test sorted string comparison failed");
  }
  if (makeTarget("make check-sample") !== "check-sample" || makeTarget("make check-sample EXTRA=1") !== null || makeTarget("cargo test") !== null) {
    fail("self-test make target parser failed");
  }
  console.log("operator reliability schema smoke self-test passed");
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
  process.exit(0);
}

for (const contract of schemaContracts) {
  checkSchemaContract(contract);
}
checkReliabilityShards();
checkQualificationRoadmapClosure();

console.log("operator reliability schema smoke passed");
