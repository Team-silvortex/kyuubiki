#!/usr/bin/env node
import { readFile } from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

async function readText(relativePath) {
  return await readFile(path.join(ROOT, relativePath), "utf8");
}

async function readJson(relativePath) {
  return JSON.parse(await readText(relativePath));
}

function parseVersion(text, pattern) {
  const match = text.match(pattern);
  return match ? match[1] : null;
}

function versionParts(version) {
  return String(version || "")
    .split(/[.-]/u)
    .map((part) => Number.parseInt(part, 10))
    .filter((part) => Number.isFinite(part));
}

function compareVersions(actual, expected) {
  const left = versionParts(actual);
  const right = versionParts(expected);
  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const a = left[index] || 0;
    const b = right[index] || 0;
    if (a !== b) {
      return a > b ? 1 : -1;
    }
  }
  return 0;
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: ROOT,
    encoding: "utf8",
  });
  return {
    ok: result.status === 0,
    status: result.status,
    stdout: result.stdout || "",
    stderr: result.stderr || "",
    error: result.error?.message || null,
  };
}

function pushVersionIssue(issues, label, actual, expected) {
  if (!actual) {
    issues.push(`${label}: unable to detect version`);
  } else if (compareVersions(actual, expected) < 0) {
    issues.push(`${label}: ${actual} is below required ${expected}`);
  }
}

async function main() {
  const jsonMode = process.argv.includes("--json");
  const staticOnly = process.argv.includes("--static-only");
  const contract = await readJson("config/toolchains.json");
  const issues = [];
  const report = {
    schema_version: "kyuubiki.elixir-self-host-preflight/v1",
    mode: staticOnly ? "static" : "runtime",
    contract: {
      elixir_constraint: contract.elixir.constraint,
      elixir_minimum: contract.elixir.minimum,
      otp_minimum: contract.elixir.otp_minimum,
      container_base: contract.elixir.container_base,
      required_env: contract.elixir.self_host_required_env,
      optional_env: contract.elixir.self_host_optional_env,
    },
    detected: {},
    env: {},
    checks: [],
  };

  const mix = await readText("apps/web/mix.exs");
  if (!mix.includes(`elixir: "${contract.elixir.constraint}"`)) {
    issues.push(`apps/web/mix.exs does not declare ${contract.elixir.constraint}`);
  }

  const config = await readText("apps/web/config/config.exs");
  for (const key of contract.elixir.self_host_required_env) {
    const present = config.includes(key);
    report.env[key] = {
      referenced_by_config: present,
      value_present: Boolean(process.env[key]),
    };
    if (!present) {
      issues.push(`apps/web/config/config.exs does not reference ${key}`);
    }
  }

  for (const key of contract.elixir.self_host_optional_env || []) {
    report.env[key] = {
      referenced_by_config: config.includes(key),
      value_present: Boolean(process.env[key]),
    };
  }

  if (!staticOnly) {
    const elixir = run("elixir", ["--version"]);
    const mixVersion = run("mix", ["--version"]);
    if (!elixir.ok) {
      issues.push(`elixir --version failed: ${elixir.error || elixir.stderr.trim() || elixir.status}`);
    }
    if (!mixVersion.ok) {
      issues.push(`mix --version failed: ${mixVersion.error || mixVersion.stderr.trim() || mixVersion.status}`);
    }
    const combined = `${elixir.stdout}\n${mixVersion.stdout}`;
    report.detected.elixir = parseVersion(combined, /Elixir\s+([0-9]+\.[0-9]+\.[0-9]+)/u);
    report.detected.mix = parseVersion(combined, /Mix\s+([0-9]+\.[0-9]+\.[0-9]+)/u);
    report.detected.otp = parseVersion(combined, /Erlang\/OTP\s+([0-9]+(?:\.[0-9]+)?)/u);
    pushVersionIssue(issues, "Elixir", report.detected.elixir, contract.elixir.minimum);
    pushVersionIssue(issues, "Mix", report.detected.mix, contract.elixir.minimum);
    pushVersionIssue(issues, "OTP", report.detected.otp, contract.elixir.otp_minimum);
  }

  report.status = issues.length === 0 ? "ok" : "fail";
  report.issues = issues;

  if (jsonMode) {
    console.log(JSON.stringify(report, null, 2));
  } else if (issues.length === 0) {
    console.log("elixir self-host preflight ok");
    if (!staticOnly) {
      console.log(`Elixir ${report.detected.elixir}, Mix ${report.detected.mix}, OTP ${report.detected.otp}`);
    }
  } else {
    console.error("elixir self-host preflight failed:");
    for (const issue of issues) {
      console.error(`- ${issue}`);
    }
  }

  process.exit(issues.length === 0 ? 0 : 1);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
