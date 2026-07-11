#!/usr/bin/env node
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const TOPOLOGY_PATH = "config/architecture/module-topology.json";
const SCHEMA_VERSION = "kyuubiki.module-topology/v1";
const REQUIRED_LAYERS = new Set([
  "product_shell",
  "control_plane",
  "runtime_data_plane",
  "sdk",
  "contract",
  "verification",
]);

if (process.argv.includes("--self-test")) {
  runSelfTest();
  process.exit(0);
}

checkTopology(readTopology(), TOPOLOGY_PATH);
console.log("module topology check passed");

function fail(message) {
  console.error(`module topology check failed: ${message}`);
  process.exit(1);
}

function readTopology() {
  return JSON.parse(readFileSync(path.join(ROOT, TOPOLOGY_PATH), "utf8"));
}

function requireNonEmptyString(value, field, context) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${context}: ${field} must be a non-empty string`);
  }
}

function requireStringArray(value, field, context, minLength = 1) {
  if (!Array.isArray(value) || value.length < minLength) {
    throw new Error(`${context}: ${field} must contain at least ${minLength} item(s)`);
  }
  for (const [index, entry] of value.entries()) {
    if (typeof entry !== "string" || entry.trim().length === 0) {
      throw new Error(`${context}: ${field}[${index}] must be a non-empty string`);
    }
  }
}

function checkPathExists(relativePath, context) {
  if (path.isAbsolute(relativePath) || relativePath.includes("..")) {
    throw new Error(`${context}: owned path must be repository-relative: ${relativePath}`);
  }
  if (!existsSync(path.join(ROOT, relativePath))) {
    throw new Error(`${context}: owned path does not exist: ${relativePath}`);
  }
}

function checkLaneReferences(module, lanes, field, context) {
  requireStringArray(module[field], field, context);
  for (const lane of module[field]) {
    if (!lanes.has(lane)) {
      throw new Error(`${context}: unknown ${field} entry: ${lane}`);
    }
  }
}

function checkAcyclicDependencyGraph(modulesById) {
  const visiting = new Set();
  const visited = new Set();

  function visit(moduleId, chain) {
    if (visited.has(moduleId)) return;
    if (visiting.has(moduleId)) {
      throw new Error(`dependency cycle detected: ${[...chain, moduleId].join(" -> ")}`);
    }

    visiting.add(moduleId);
    const module = modulesById.get(moduleId);
    for (const dependencyId of module.depends_on) {
      visit(dependencyId, [...chain, moduleId]);
    }
    visiting.delete(moduleId);
    visited.add(moduleId);
  }

  for (const moduleId of modulesById.keys()) {
    visit(moduleId, []);
  }
}

function checkTopology(topology, context) {
  assert.equal(topology.schema_version, SCHEMA_VERSION);
  requireNonEmptyString(topology.version_line, "version_line", context);

  const benchmarkLanes = new Set(Object.keys(topology.benchmark_lanes ?? {}));
  const securityLanes = new Set(Object.keys(topology.security_lanes ?? {}));
  if (benchmarkLanes.size === 0) throw new Error(`${context}: benchmark_lanes must not be empty`);
  if (securityLanes.size === 0) throw new Error(`${context}: security_lanes must not be empty`);
  if (!Array.isArray(topology.modules) || topology.modules.length === 0) {
    throw new Error(`${context}: modules must not be empty`);
  }

  const modulesById = new Map();
  const seenPaths = new Map();
  const seenLayers = new Set();

  for (const [index, module] of topology.modules.entries()) {
    const moduleContext = `${context}#modules/${index}`;
    requireNonEmptyString(module.id, "id", moduleContext);
    requireNonEmptyString(module.layer, "layer", moduleContext);
    requireNonEmptyString(module.summary, "summary", moduleContext);
    requireStringArray(module.owned_paths, "owned_paths", moduleContext);
    requireStringArray(module.risk_tags, "risk_tags", moduleContext);
    requireStringArray(module.depends_on, "depends_on", moduleContext, 0);

    if (modulesById.has(module.id)) {
      throw new Error(`${moduleContext}: duplicate module id ${module.id}`);
    }
    if (!REQUIRED_LAYERS.has(module.layer)) {
      throw new Error(`${moduleContext}: unknown layer ${module.layer}`);
    }

    seenLayers.add(module.layer);
    modulesById.set(module.id, module);
    checkLaneReferences(module, benchmarkLanes, "benchmark_lanes", moduleContext);
    checkLaneReferences(module, securityLanes, "security_lanes", moduleContext);

    for (const ownedPath of module.owned_paths) {
      checkPathExists(ownedPath, moduleContext);
      if (seenPaths.has(ownedPath)) {
        throw new Error(`${moduleContext}: owned path ${ownedPath} also belongs to ${seenPaths.get(ownedPath)}`);
      }
      seenPaths.set(ownedPath, module.id);
    }
  }

  for (const layer of REQUIRED_LAYERS) {
    if (!seenLayers.has(layer)) {
      throw new Error(`${context}: missing required layer ${layer}`);
    }
  }

  for (const module of modulesById.values()) {
    for (const dependencyId of module.depends_on) {
      if (dependencyId === module.id) {
        throw new Error(`${module.id}: module must not depend on itself`);
      }
      if (!modulesById.has(dependencyId)) {
        throw new Error(`${module.id}: dependency does not exist: ${dependencyId}`);
      }
    }
  }

  checkAcyclicDependencyGraph(modulesById);
}

function runSelfTest() {
  const sample = {
    schema_version: SCHEMA_VERSION,
    version_line: "tamamono test",
    benchmark_lanes: { ui_startup: "test lane" },
    security_lanes: { ui_boundary: "test lane" },
    modules: [
      {
        id: "contracts",
        layer: "contract",
        summary: "contract",
        owned_paths: ["docs"],
        depends_on: [],
        benchmark_lanes: ["ui_startup"],
        security_lanes: ["ui_boundary"],
        risk_tags: ["drift"],
      },
      {
        id: "hub",
        layer: "product_shell",
        summary: "hub",
        owned_paths: ["apps"],
        depends_on: ["contracts"],
        benchmark_lanes: ["ui_startup"],
        security_lanes: ["ui_boundary"],
        risk_tags: ["coupling"],
      },
      {
        id: "api",
        layer: "control_plane",
        summary: "api",
        owned_paths: ["config"],
        depends_on: ["contracts"],
        benchmark_lanes: ["ui_startup"],
        security_lanes: ["ui_boundary"],
        risk_tags: ["auth"],
      },
      {
        id: "runtime",
        layer: "runtime_data_plane",
        summary: "runtime",
        owned_paths: ["workers"],
        depends_on: ["contracts"],
        benchmark_lanes: ["ui_startup"],
        security_lanes: ["ui_boundary"],
        risk_tags: ["sandbox"],
      },
      {
        id: "sdk",
        layer: "sdk",
        summary: "sdk",
        owned_paths: ["sdks"],
        depends_on: ["contracts"],
        benchmark_lanes: ["ui_startup"],
        security_lanes: ["ui_boundary"],
        risk_tags: ["parity"],
      },
      {
        id: "verification",
        layer: "verification",
        summary: "verification",
        owned_paths: ["scripts"],
        depends_on: ["contracts"],
        benchmark_lanes: ["ui_startup"],
        security_lanes: ["ui_boundary"],
        risk_tags: ["coverage"],
      },
    ],
  };

  assert.doesNotThrow(() => checkTopology(sample, "self-test"));
  sample.modules[1].depends_on = ["missing"];
  assert.throws(() => checkTopology(sample, "self-test"), /dependency does not exist/u);
  sample.modules[1].depends_on = ["contracts"];
  sample.modules[0].depends_on = ["hub"];
  assert.throws(() => checkTopology(sample, "self-test"), /dependency cycle/u);
  console.log("module topology self-test passed");
}
