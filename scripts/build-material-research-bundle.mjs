#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultOut = "tmp/material-research-bundle.json";
const supportedStudies = new Map([
  [
    "heat-spreader",
    {
      bundleId: "material.heat_spreader_screening.reproducible_bundle.v1",
      workSlug: "heat-spreader",
    },
  ],
  [
    "composite-thermo-electric-panel",
    {
      bundleId: "material.composite_thermo_electric_panel.reproducible_bundle.v1",
      workSlug: "composite-thermo-electric-panel",
    },
  ],
]);

function fail(message) {
  console.error(`material research bundle failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { out: defaultOut, study: "heat-spreader", rounds: 2 };
  for (let index = 2; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === "--out") {
      args.out = argv[++index];
    } else if (flag === "--study") {
      args.study = argv[++index];
    } else if (flag === "--rounds") {
      args.rounds = Number.parseInt(argv[++index], 10);
    } else {
      fail(`unknown argument ${flag}`);
    }
  }
  if (!supportedStudies.has(args.study)) {
    fail(`unsupported retained research bundle study: ${args.study}`);
  }
  if (!Number.isInteger(args.rounds) || args.rounds < 1) {
    fail("--rounds must be a positive integer");
  }
  return args;
}

function repoPath(relativePath, flag) {
  const absolute = path.resolve(repoRoot, relativePath);
  const relative = path.relative(repoRoot, absolute);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    fail(`${flag} must stay inside the repository`);
  }
  return { absolute, relative };
}

function runMaterialExplore(args) {
  const argv = [
    "cargo",
    "run",
    "-q",
    "-p",
    "kyuubiki-cli",
    "--bin",
    "kyuubiki-material-explore",
    "--",
    ...args,
    "--json",
  ];
  const startedAt = Date.now();
  const result = spawnSync(argv[0], argv.slice(1), {
    cwd: path.join(repoRoot, "workers/rust"),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    fail(result.stderr.trim() || `command failed: ${argv.join(" ")}`);
  }
  return {
    command: { cwd: "workers/rust", argv },
    duration_ms: Date.now() - startedAt,
    payload: parseJson(result.stdout, argv.join(" ")),
  };
}

function parseJson(text, context) {
  try {
    return JSON.parse(text);
  } catch (error) {
    fail(`${context} did not emit JSON: ${error.message}`);
  }
}

function writeJson(relativePath, payload) {
  const target = repoPath(relativePath, "work path");
  fs.mkdirSync(path.dirname(target.absolute), { recursive: true });
  fs.writeFileSync(target.absolute, `${JSON.stringify(payload, null, 2)}\n`);
  return target.relative;
}

function sha256(payload) {
  return crypto.createHash("sha256").update(`${JSON.stringify(payload)}\n`).digest("hex");
}

function buildBundle(options) {
  const profile = supportedStudies.get(options.study);
  const workRoot = `tmp/material-research-bundle-work/${profile.workSlug}/${process.pid}`;
  const initial = runMaterialExplore([options.study]);
  const initialPath = writeJson(`${workRoot}/initial-exploration.json`, initial.payload);
  const initialInput = path.relative(path.join(repoRoot, "workers/rust"), path.join(repoRoot, initialPath));
  const plan = runMaterialExplore(["--plan-next", initialInput]);
  const next = runMaterialExplore(["--run-next", initialInput]);
  const nextPath = writeJson(`${workRoot}/next-exploration.json`, next.payload);
  const chain = runMaterialExplore(["--chain-next", initialInput, "--rounds", String(options.rounds)]);
  return {
    schema_version: "kyuubiki.material-research-bundle/v1",
    bundle_id: profile.bundleId,
    generated_at_utc: new Date().toISOString(),
    posture: "screening_research_bundle",
    study: options.study,
    artifact_checksums: {
      initial_exploration_sha256: sha256(initial.payload),
      next_round_execution_plan_sha256: sha256(plan.payload),
      next_exploration_sha256: sha256(next.payload),
      chain_sha256: sha256(chain.payload),
    },
    reproducibility: {
      workspace: "workers/rust",
      initial_command: initial.command.argv,
      plan_next_command_template: materialExploreTemplate(["--plan-next", "<initial-exploration.json>"]),
      run_next_command_template: materialExploreTemplate(["--run-next", "<initial-exploration.json>"]),
      chain_next_command_template: materialExploreTemplate([
        "--chain-next",
        "<initial-exploration.json>",
        "--rounds",
        String(options.rounds),
      ]),
      transient_work_files: [initialPath, nextPath],
    },
    execution_trace: {
      initial_duration_ms: initial.duration_ms,
      plan_next_duration_ms: plan.duration_ms,
      run_next_duration_ms: next.duration_ms,
      chain_next_duration_ms: chain.duration_ms,
    },
    summary: bundleSummary(initial.payload, plan.payload, next.payload, chain.payload),
    initial_exploration: initial.payload,
    next_round_execution_plan: plan.payload,
    next_exploration: next.payload,
    chain: chain.payload,
  };
}

function materialExploreTemplate(args) {
  return [
    "cargo",
    "run",
    "-q",
    "-p",
    "kyuubiki-cli",
    "--bin",
    "kyuubiki-material-explore",
    "--",
    ...args,
    "--json",
  ];
}

function bundleSummary(initial, plan, next, chain) {
  return {
    winner_candidate_id: initial.report?.winner_candidate_id,
    reliability_decision: initial.report?.reliability?.summary?.decision,
    next_round_decision: plan.decision,
    runnable_next_step_count: plan.runnable_step_count,
    next_iteration: next.iteration,
    chain_stop_reason: chain.stop_reason,
    chain_convergence_state: chain.convergence_assessment?.state,
    chain_round_count: chain.round_count,
  };
}

function writeBundle(outPath, payload) {
  const target = repoPath(outPath, "--out");
  fs.mkdirSync(path.dirname(target.absolute), { recursive: true });
  fs.writeFileSync(target.absolute, `${JSON.stringify(payload, null, 2)}\n`);
  console.log(`material research bundle wrote ${target.relative}`);
}

const options = parseArgs(process.argv);
writeBundle(options.out, buildBundle(options));
