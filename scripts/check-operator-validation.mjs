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
const ALLOWED_KINDS = new Set(["analytic", "contract", "cross_check", "invariant"]);

function parseArgs(argv) {
  const options = { config: DEFAULT_CONFIG, out: DEFAULT_OUT, execute: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--config" && next) {
      options.config = next;
      index += 1;
    } else if (arg === "--out" && next) {
      options.out = next;
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
  requireString(profile.trust_goal, "trust_goal", context);
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
  const profiles = config.profiles.map((profile) => {
    const commands = profile.commands.map((command) => ({
      id: command.id,
      kind: command.kind,
      command: command.command,
      ...(options.execute ? { result: runCommand(command.command) } : { result: { ok: null, status: "not_run" } }),
    }));
    return {
      profile_id: profile.profile_id,
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
    source: DEFAULT_CONFIG,
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
  const sample = {
    schema_version: SCHEMA_VERSION,
    version_line: "tamamono test",
    profiles: [
      {
        profile_id: "sample",
        trust_goal: "review",
        operators: ["solve.sample"],
        validation_methods: ["analytic"],
        formal_invariants: ["finite"],
        evidence_paths: ["docs/operator-reliability.md"],
        commands: [{ id: "smoke", kind: "contract", command: "make check-make-modules" }],
      },
    ],
  };
  assert.doesNotThrow(() => validateConfig(sample));
  sample.profiles[0].commands[0].command = "rm -rf tmp";
  assert.throws(() => validateConfig(sample), /unsupported command prefix/u);
  console.log("operator validation self-test passed");
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    runSelfTest();
    process.exit(0);
  }
  const config = readJson(options.config);
  validateConfig(config);
  const report = buildReport(config, options);
  writeReport(report, options.out);
  console.log(`operator validation ${report.ok ? "passed" : "failed"}: ${report.profile_count} profile(s), executed=${report.executed}`);
  if (!report.ok) process.exit(1);
} catch (error) {
  console.error(`operator validation failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
