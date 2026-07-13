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
const ALLOWED_PLAN_SCOPES = new Set(["local", "integration", "benchmark", "remote", "release"]);
const ALLOWED_SERVICE_SURFACE_KINDS = new Set([
  "control_api",
  "self_host_web",
  "runtime_adapter",
  "storage_api",
]);
const ALLOWED_COMMAND_PREFIXES = [
  "make ",
  "node ",
  "cd apps/frontend && npm run ",
  "cd apps/web && mix ",
  "cd workers/rust && cargo ",
];

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

function checkServiceSurfaces(module, context) {
  if (module.service_surfaces === undefined) return;
  if (!Array.isArray(module.service_surfaces) || module.service_surfaces.length === 0) {
    throw new Error(`${context}: service_surfaces must be a non-empty array when present`);
  }

  const ids = new Set();
  module.service_surfaces.forEach((surface, index) => {
    const surfaceContext = `${context}#service_surfaces/${index}`;
    requireNonEmptyString(surface.id, "id", surfaceContext);
    requireNonEmptyString(surface.kind, "kind", surfaceContext);
    requireNonEmptyString(surface.summary, "summary", surfaceContext);
    if (ids.has(surface.id)) {
      throw new Error(`${surfaceContext}: duplicate service surface id ${surface.id}`);
    }
    if (!ALLOWED_SERVICE_SURFACE_KINDS.has(surface.kind)) {
      throw new Error(`${surfaceContext}: unknown service surface kind ${surface.kind}`);
    }
    ids.add(surface.id);
  });
}

function checkPlanCommand(command, context) {
  if (!ALLOWED_COMMAND_PREFIXES.some((prefix) => command.startsWith(prefix))) {
    throw new Error(`${context}: unsupported command prefix: ${command}`);
  }
  if (command.includes("..") || command.includes(";") || command.includes("&& rm ")) {
    throw new Error(`${context}: command must not contain path traversal or destructive shell chaining`);
  }
}

function checkLaneTestPlan(topology, benchmarkLanes, securityLanes, context) {
  const plan = topology.lane_test_plan;
  if (!plan || typeof plan !== "object") {
    throw new Error(`${context}: lane_test_plan must be defined`);
  }

  for (const [group, lanes] of [
    ["benchmark", benchmarkLanes],
    ["security", securityLanes],
  ]) {
    const groupPlan = plan[group];
    if (!groupPlan || typeof groupPlan !== "object" || Array.isArray(groupPlan)) {
      throw new Error(`${context}: lane_test_plan.${group} must be an object`);
    }

    for (const lane of lanes) {
      const entries = groupPlan[lane];
      if (!Array.isArray(entries) || entries.length === 0) {
        throw new Error(`${context}: lane_test_plan.${group}.${lane} must not be empty`);
      }
      const ids = new Set();
      entries.forEach((entry, index) => {
        const entryContext = `${context}#lane_test_plan/${group}/${lane}/${index}`;
        requireNonEmptyString(entry.id, "id", entryContext);
        requireNonEmptyString(entry.command, "command", entryContext);
        requireNonEmptyString(entry.scope, "scope", entryContext);
        if (ids.has(entry.id)) {
          throw new Error(`${entryContext}: duplicate test plan id ${entry.id}`);
        }
        if (!ALLOWED_PLAN_SCOPES.has(entry.scope)) {
          throw new Error(`${entryContext}: unknown scope ${entry.scope}`);
        }
        ids.add(entry.id);
        checkPlanCommand(entry.command, entryContext);
      });
    }

    for (const lane of Object.keys(groupPlan)) {
      if (!lanes.has(lane)) {
        throw new Error(`${context}: lane_test_plan.${group}.${lane} references unknown lane`);
      }
    }
  }

  requirePlanEntry(
    plan.benchmark?.control_plane,
    "central-db-readiness",
    `${context}: lane_test_plan.benchmark.control_plane`,
  );
  requirePlanEntry(
    plan.security?.data_contract,
    "central-store-contract",
    `${context}: lane_test_plan.security.data_contract`,
  );
  requirePlanEntry(
    plan.security?.data_contract,
    "central-db-readiness",
    `${context}: lane_test_plan.security.data_contract`,
  );
}

function requirePlanEntry(entries, requiredId, context) {
  if (entries === undefined) return;
  if (!Array.isArray(entries) || !entries.some((entry) => entry.id === requiredId)) {
    throw new Error(`${context}: missing required test plan id ${requiredId}`);
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
  checkLaneTestPlan(topology, benchmarkLanes, securityLanes, context);

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
    checkServiceSurfaces(module, moduleContext);

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
    lane_test_plan: {
      benchmark: {
        ui_startup: [{ id: "smoke", command: "make smoke", scope: "local" }],
      },
      security: {
        ui_boundary: [{ id: "audit", command: "node scripts/audit-local-paths.mjs", scope: "local" }],
      },
    },
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
        service_surfaces: [
          {
            id: "central-web-service",
            kind: "self_host_web",
            summary: "self-hosted web service surface",
          },
        ],
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
  sample.modules[0].depends_on = [];
  sample.modules[2].service_surfaces = [{ id: "bad", kind: "module", summary: "wrong" }];
  assert.throws(() => checkTopology(sample, "self-test"), /unknown service surface kind/u);
  console.log("module topology self-test passed");
}
