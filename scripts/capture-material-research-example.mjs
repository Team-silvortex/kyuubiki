#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultOut = "tmp/material-research-example.json";

function fail(message) {
  console.error(`material research example capture failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { out: defaultOut, study: "heat-spreader" };
  for (let index = 2; index < argv.length; index += 1) {
    if (argv[index] === "--out") {
      args.out = argv[++index];
    } else if (argv[index] === "--study") {
      args.study = argv[++index];
    } else {
      fail(`unknown argument ${argv[index]}`);
    }
  }
  if (!args.out) {
    fail("--out requires a repo-local path");
  }
  if (args.study !== "heat-spreader") {
    fail("the first automated research example is intentionally fixed to heat-spreader");
  }
  return args;
}

function repoRelativePath(outPath) {
  const absolute = path.resolve(repoRoot, outPath);
  const relative = path.relative(repoRoot, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    fail("--out must stay inside the repository");
  }
  return { absolute, relative };
}

function runExploration(study) {
  const startedAt = Date.now();
  const command = {
    id: "material_explore_heat_spreader",
    cwd: "workers/rust",
    argv: [
      "cargo",
      "run",
      "-q",
      "-p",
      "kyuubiki-cli",
      "--bin",
      "kyuubiki-material-explore",
      "--",
      study,
      "--json",
    ],
  };
  const result = spawnSync(command.argv[0], command.argv.slice(1), {
    cwd: path.join(repoRoot, command.cwd),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    ...command,
    status: result.status,
    signal: result.signal,
    duration_ms: Date.now() - startedAt,
    stdout: result.stdout,
    stderr: result.stderr,
    ok: result.status === 0,
  };
}

function parseExploration(command) {
  if (!command.ok) {
    fail(command.stderr.trim() || "material exploration command failed");
  }
  try {
    return JSON.parse(command.stdout);
  } catch (error) {
    fail(`material exploration did not emit JSON: ${error.message}`);
  }
}

function sha256Json(value) {
  return crypto
    .createHash("sha256")
    .update(`${JSON.stringify(value)}\n`)
    .digest("hex");
}

function candidateSummary(report) {
  return (report.candidates ?? []).map((candidate) => ({
    rank: candidate.rank,
    candidate_id: candidate.candidate_id,
    score: candidate.score,
    peak_temperature_c: candidate.peak_temperature_c,
    areal_mass_kg_m2: candidate.areal_mass_kg_m2,
    conductivity_density_ratio: candidate.conductivity_density_ratio,
    material_card_confidence: candidate.material_card_confidence,
  }));
}

function buildEvidence(study) {
  const command = runExploration(study);
  const exploration = parseExploration(command);
  const report = exploration.report ?? {};
  delete command.stdout;
  delete command.stderr;
  return {
    schema_version: "kyuubiki.automated-material-research-example/v1",
    example_id: "material.heat_spreader_screening.automated_research.v1",
    generated_at_utc: new Date().toISOString(),
    posture: "screening_research_example",
    study,
    command,
    exploration_sha256: sha256Json(exploration),
    summary: {
      exploration_schema_version: exploration.schema_version,
      report_schema_version: report.schema_version,
      template_id: exploration.template_id,
      mode: exploration.mode,
      iteration: exploration.iteration,
      next_round_iteration: exploration.next_round?.iteration,
      next_round_decision: exploration.next_round?.decision,
      candidate_count: exploration.candidate_count,
      result_payload_count: (exploration.result_payloads ?? []).length,
      winner_candidate_id: report.winner_candidate_id,
      reliability_posture: report.reliability?.posture,
      optimization_id: report.optimization?.id,
      candidates: candidateSummary(report),
      quality_gates: report.reliability?.quality_gates ?? [],
      limitations: report.reliability?.limitations ?? [],
    },
    exploration,
  };
}

function writeEvidence(outPath, payload) {
  const { absolute, relative } = repoRelativePath(outPath);
  fs.mkdirSync(path.dirname(absolute), { recursive: true });
  fs.writeFileSync(absolute, `${JSON.stringify(payload, null, 2)}\n`);
  console.log(`material research example wrote ${relative}`);
}

const args = parseArgs(process.argv);
writeEvidence(args.out, buildEvidence(args.study));
