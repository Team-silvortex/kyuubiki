#!/usr/bin/env node
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const DEFAULT_CONFIG = "config/operator-validation-profiles.json";
const DEFAULT_OUT = "tmp/operator-validation-report.json";
const SCHEMA_VERSION = "kyuubiki.operator-validation-profiles/v1";
const REPORT_SCHEMA_VERSION = "kyuubiki.operator-validation-report/v1";
const ALLOWED_COMMAND_PREFIXES = ["make ", "cd workers/rust && cargo "];
const ALLOWED_KINDS = new Set(["analytic", "boundary_regression", "contract", "cross_check", "invariant"]);
const ALLOWED_PROFILE_ROLES = new Set(["release_candidate", "component_profile"]);
const PROFILE_SCHEMA = "schemas/operator-validation-profiles.schema.json";
const REPORT_SCHEMA = "schemas/operator-validation-report.schema.json";

function parseArgs(argv) {
  const options = { config: DEFAULT_CONFIG, out: DEFAULT_OUT, execute: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--config" && next) {
      options.config = next;
      index += 1;
    } else if (arg === "--in" && next) {
      options.inputReport = next;
      index += 1;
    } else if (arg === "--out" && next) {
      options.out = next;
      index += 1;
    } else if (arg === "--profile" && next) {
      options.profile = next;
      index += 1;
    } else if (arg === "--execute") {
      options.execute = true;
    } else if (arg === "--self-test") {
      options.selfTest = true;
    } else {
      throw new Error(`unknown argument ${arg}`);
    }
  }
  return options;
}

function repoPath(relativePath) {
  const resolved = path.resolve(ROOT, relativePath);
  const relative = path.relative(ROOT, resolved);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`path must stay inside repository: ${relativePath}`);
  }
  return resolved;
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(repoPath(relativePath), "utf8"));
}

function requireString(value, field, context) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${context}: ${field} must be a non-empty string`);
  }
}

function requireStringArray(value, field, context) {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`${context}: ${field} must be a non-empty array`);
  }
  value.forEach((entry, index) => requireString(entry, `${field}[${index}]`, context));
}

function requireStringList(value, field, context) {
  if (!Array.isArray(value)) {
    throw new Error(`${context}: ${field} must be an array`);
  }
  value.forEach((entry, index) => requireString(entry, `${field}[${index}]`, context));
}

function requireBoolean(value, field, context) {
  if (typeof value !== "boolean") {
    throw new Error(`${context}: ${field} must be a boolean`);
  }
}

function requireNumber(value, field, context) {
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(`${context}: ${field} must be a non-negative integer`);
  }
}

function validateCommand(command, context) {
  requireString(command.id, "command.id", context);
  requireString(command.kind, "command.kind", context);
  requireString(command.command, "command.command", context);
  if (!ALLOWED_KINDS.has(command.kind)) {
    throw new Error(`${context}: unsupported command kind ${command.kind}`);
  }
  if (!ALLOWED_COMMAND_PREFIXES.some((prefix) => command.command.startsWith(prefix))) {
    throw new Error(`${context}: unsupported command prefix ${command.command}`);
  }
  if (command.command.includes("..") || command.command.includes(";") || command.command.includes("&& rm ")) {
    throw new Error(`${context}: command contains unsafe shell structure`);
  }
}

function validateProfile(profile, context) {
  requireString(profile.profile_id, "profile_id", context);
  requireString(profile.profile_role, "profile_role", context);
  requireString(profile.qualification_candidate_id, "qualification_candidate_id", context);
  requireString(profile.trust_goal, "trust_goal", context);
  if (!ALLOWED_PROFILE_ROLES.has(profile.profile_role)) {
    throw new Error(`${context}: unsupported profile_role ${profile.profile_role}`);
  }
  requireStringArray(profile.operators, "operators", context);
  requireStringArray(profile.validation_methods, "validation_methods", context);
  requireStringArray(profile.formal_invariants, "formal_invariants", context);
  requireStringArray(profile.evidence_paths, "evidence_paths", context);
  if (!Array.isArray(profile.commands) || profile.commands.length === 0) {
    throw new Error(`${context}: commands must be non-empty`);
  }
  for (const evidencePath of profile.evidence_paths) {
    if (!existsSync(repoPath(evidencePath))) {
      throw new Error(`${context}: evidence path does not exist: ${evidencePath}`);
    }
  }
  profile.commands.forEach((command, index) => validateCommand(command, `${context}#commands/${index}`));
}

function validateConfig(config) {
  if (config.schema_version !== SCHEMA_VERSION) {
    throw new Error(`schema_version must be ${SCHEMA_VERSION}`);
  }
  requireString(config.version_line, "version_line", "config");
  if (!Array.isArray(config.profiles) || config.profiles.length === 0) {
    throw new Error("profiles must be non-empty");
  }
  const seenProfiles = new Set();
  config.profiles.forEach((profile, index) => {
    validateProfile(profile, `profiles/${index}`);
    if (seenProfiles.has(profile.profile_id)) {
      throw new Error(`duplicate profile_id ${profile.profile_id}`);
    }
    seenProfiles.add(profile.profile_id);
  });
}

function loadConfig(relativePath) {
  const config = readJson(relativePath);
  const profiles = [...(config.profiles ?? [])];
  for (const shardPath of config.profile_shards ?? []) {
    requireString(shardPath, "profile_shards[]", "config");
    const shard = readJson(shardPath);
    if (shard.schema_version !== config.schema_version) {
      throw new Error(`${shardPath}: schema_version must match ${relativePath}`);
    }
    if (shard.version_line !== config.version_line) {
      throw new Error(`${shardPath}: version_line must match ${relativePath}`);
    }
    profiles.push(...(shard.profiles ?? []));
  }
  return { ...config, profiles };
}

function requireSchemaCommandKinds(relativePath) {
  const schema = readJson(relativePath);
  const enumValues = schema?.$defs?.commandKind?.enum;
  if (!Array.isArray(enumValues) || enumValues.length === 0) {
    throw new Error(`${relativePath}: missing $defs.commandKind.enum`);
  }
  return new Set(enumValues);
}

function assertSetEquals(actual, expected, context) {
  assert.deepEqual([...actual].sort(), [...expected].sort(), context);
}

function validateReportCommand(command, executed, context) {
  requireString(command.id, "id", context);
  requireString(command.kind, "kind", context);
  requireString(command.command, "command", context);
  if (!ALLOWED_KINDS.has(command.kind)) {
    throw new Error(`${context}: unsupported command kind ${command.kind}`);
  }
  const result = command.result;
  if (!result || typeof result !== "object") {
    throw new Error(`${context}: result must be an object`);
  }
  if (executed) {
    requireBoolean(result.ok, "result.ok", context);
    if (!(Number.isInteger(result.status) || result.status === null)) {
      throw new Error(`${context}: result.status must be an integer or null`);
    }
    requireNumber(result.duration_ms, "result.duration_ms", context);
    requireStringList(result.stdout_tail, "result.stdout_tail", context);
    requireStringList(result.stderr_tail, "result.stderr_tail", context);
  } else if (result.ok !== null || result.status !== "not_run") {
    throw new Error(`${context}: skipped command result must be ok=null,status=not_run`);
  }
}

function validateReport(report, options) {
  if (report.schema_version !== REPORT_SCHEMA_VERSION) {
    throw new Error(`report schema_version must be ${REPORT_SCHEMA_VERSION}`);
  }
  if (report.source !== options.config) {
    throw new Error(`report source must be ${options.config}`);
  }
  requireBoolean(report.executed, "executed", "report");
  requireNumber(report.profile_count, "profile_count", "report");
  requireBoolean(report.ok, "ok", "report");
  if (!Array.isArray(report.profiles)) {
    throw new Error("report profiles must be an array");
  }
  if (report.profile_count !== report.profiles.length) {
    throw new Error("report profile_count must match profiles length");
  }
  const expectedOk = report.profiles.every((profile, profileIndex) => {
    const context = `report.profiles/${profileIndex}`;
    requireString(profile.profile_id, "profile_id", context);
    requireString(profile.trust_goal, "trust_goal", context);
    requireStringArray(profile.operators, "operators", context);
    requireStringArray(profile.validation_methods, "validation_methods", context);
    requireStringArray(profile.formal_invariants, "formal_invariants", context);
    requireStringArray(profile.evidence_paths, "evidence_paths", context);
    requireBoolean(profile.ok, "ok", context);
    if (!Array.isArray(profile.commands) || profile.commands.length === 0) {
      throw new Error(`${context}: commands must be non-empty`);
    }
    profile.commands.forEach((command, commandIndex) =>
      validateReportCommand(command, report.executed, `${context}.commands/${commandIndex}`),
    );
    return profile.ok;
  });
  if (report.ok !== expectedOk) {
    throw new Error("report ok must equal the profile status rollup");
  }
}

function validateInputReport(report, options) {
  if (report.executed !== true) {
    throw new Error("input report must be executed=true");
  }
  if (report.ok !== true) {
    throw new Error("input report must be ok=true");
  }
  if (options.profile) {
    if (report.profiles.length !== 1 || report.profiles[0].profile_id !== options.profile) {
      throw new Error(`input report must contain only profile ${options.profile}`);
    }
  }
}

function runCommand(command) {
  const startedAt = Date.now();
  let result;
  if (command.startsWith("make ")) {
    result = spawnSync("make", command.slice("make ".length).split(/\s+/), {
      cwd: ROOT,
      encoding: "utf8",
    });
  } else if (command.startsWith("cd workers/rust && cargo ")) {
    result = spawnSync("cargo", command.slice("cd workers/rust && cargo ".length).split(/\s+/), {
      cwd: path.join(ROOT, "workers/rust"),
      encoding: "utf8",
    });
  } else {
    throw new Error(`unsupported command ${command}`);
  }
  return {
    ok: result.status === 0,
    status: result.status,
    duration_ms: Date.now() - startedAt,
    stdout_tail: tail(result.stdout),
    stderr_tail: tail(result.stderr),
  };
}

function tail(text) {
  return String(text ?? "").split("\n").filter(Boolean).slice(-8);
}

function buildReport(config, options) {
  const selectedProfiles = config.profiles.filter((profile) => !options.profile || profile.profile_id === options.profile);
  if (selectedProfiles.length === 0) {
    throw new Error(`no operator validation profiles matched ${options.profile ?? "<all>"}`);
  }
  const profiles = selectedProfiles.map((profile) => {
    const commands = profile.commands.map((command) => ({
      id: command.id,
      kind: command.kind,
      command: command.command,
      ...(options.execute ? { result: runCommand(command.command) } : { result: { ok: null, status: "not_run" } }),
    }));
    return {
      profile_id: profile.profile_id,
      profile_role: profile.profile_role,
      qualification_candidate_id: profile.qualification_candidate_id,
      trust_goal: profile.trust_goal,
      operators: profile.operators,
      validation_methods: profile.validation_methods,
      formal_invariants: profile.formal_invariants,
      evidence_paths: profile.evidence_paths,
      commands,
      ok: commands.every((command) => command.result.ok !== false),
    };
  });
  return {
    schema_version: REPORT_SCHEMA_VERSION,
    source: options.config,
    executed: options.execute,
    profile_count: profiles.length,
    ok: profiles.every((profile) => profile.ok),
    profiles,
  };
}

function writeReport(report, outPath) {
  const absolute = repoPath(outPath);
  mkdirSync(path.dirname(absolute), { recursive: true });
  writeFileSync(absolute, `${JSON.stringify(report, null, 2)}\n`);
}

function runSelfTest() {
  assertSetEquals(requireSchemaCommandKinds(PROFILE_SCHEMA), ALLOWED_KINDS, "profile schema command kinds");
  assertSetEquals(requireSchemaCommandKinds(REPORT_SCHEMA), ALLOWED_KINDS, "report schema command kinds");
  const sample = {
    schema_version: SCHEMA_VERSION,
    version_line: "tamamono test",
    profiles: [
      {
        profile_id: "sample",
        profile_role: "release_candidate",
        qualification_candidate_id: "sample",
        trust_goal: "review",
        operators: ["solve.sample"],
        validation_methods: ["analytic"],
        formal_invariants: ["finite"],
        evidence_paths: ["docs/operator-reliability.md"],
        commands: [
          { id: "smoke", kind: "contract", command: "make check-make-modules" },
          {
            id: "boundary",
            kind: "boundary_regression",
            command: "cd workers/rust && cargo test -p kyuubiki-solver --test stokes_flow_triangle_reliability",
          },
        ],
      },
    ],
  };
  assert.doesNotThrow(() => validateConfig(sample));
  sample.profiles[0].commands[0].command = "python -c 'print(1)'";
  assert.throws(() => validateConfig(sample), /unsupported command prefix/u);
  sample.profiles[0].commands[0].command = "make check-make-modules";
  sample.profiles[0].commands[1].kind = "ad_hoc";
  assert.throws(() => validateConfig(sample), /unsupported command kind/u);
  sample.profiles[0].commands[1].kind = "boundary_regression";
  const report = buildReport(sample, { config: "config/sample.json", execute: false });
  assert.doesNotThrow(() => validateReport(report, { config: "config/sample.json" }));
  const executedReport = buildReport(sample, { config: "config/sample.json", execute: false, profile: "sample" });
  executedReport.executed = true;
  executedReport.profiles[0].commands.forEach((command) => {
    command.result = { ok: true, status: 0, duration_ms: 1, stdout_tail: [], stderr_tail: [] };
  });
  assert.doesNotThrow(() => validateReport(executedReport, { config: "config/sample.json", profile: "sample" }));
  assert.doesNotThrow(() => validateInputReport(executedReport, { profile: "sample" }));
  assert.throws(() => validateInputReport(report, { profile: "sample" }), /executed=true/u);
  assert.throws(
    () => buildReport(sample, { config: "config/sample.json", execute: false, profile: "missing" }),
    /no operator validation profiles matched/u,
  );
  report.profile_count = 2;
  assert.throws(() => validateReport(report, { config: "config/sample.json" }), /profile_count/u);
  console.log("operator validation self-test passed");
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    runSelfTest();
    process.exit(0);
  }
  if (options.inputReport) {
    const report = readJson(options.inputReport);
    validateReport(report, options);
    validateInputReport(report, options);
    console.log(`operator validation report ok: ${options.inputReport} (${report.profile_count} profile(s))`);
    process.exit(0);
  }
  const config = loadConfig(options.config);
  validateConfig(config);
  const report = buildReport(config, options);
  validateReport(report, options);
  writeReport(report, options.out);
  console.log(`operator validation ${report.ok ? "passed" : "failed"}: ${report.profile_count} profile(s), executed=${report.executed}`);
  if (!report.ok) process.exit(1);
} catch (error) {
  console.error(`operator validation failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
