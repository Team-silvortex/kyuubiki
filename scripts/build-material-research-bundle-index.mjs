#!/usr/bin/env node
import childProcess from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL("..", import.meta.url).pathname);
const defaultOutputDir = "tmp/material-research-bundles";
const bundleProfiles = [
  { study: "heat-spreader", file: "heat-spreader.json" },
  { study: "composite-thermo-electric-panel", file: "composite-thermo-electric-panel.json" },
];

function fail(message) {
  console.error(`material research bundle index failed: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { outDir: defaultOutputDir, ensureBundles: false, selfTest: false };
  for (let index = 2; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === "--out-dir") {
      args.outDir = argv[++index];
    } else if (flag === "--ensure-bundles") {
      args.ensureBundles = true;
    } else if (flag === "--self-test") {
      args.selfTest = true;
    } else {
      fail(`unknown argument ${flag}`);
    }
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

function run(command, args) {
  const result = childProcess.spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    fail(result.stderr.trim() || `command failed: ${command} ${args.join(" ")}`);
  }
  return result.stdout;
}

function ensureBundle(profile, outDir) {
  const bundlePath = path.join(outDir.relative, profile.file);
  run("node", [
    "scripts/build-material-research-bundle.mjs",
    "--study",
    profile.study,
    "--out",
    bundlePath,
  ]);
  run("node", ["scripts/check-material-research-bundle.mjs", "--in", bundlePath]);
}

function readBundle(relativePath) {
  const target = repoPath(relativePath, "bundle path");
  if (!fs.existsSync(target.absolute)) {
    fail(`bundle does not exist: ${target.relative}; pass --ensure-bundles to build it`);
  }
  return JSON.parse(fs.readFileSync(target.absolute, "utf8"));
}

export function buildIndex(entries) {
  const bundles = entries.map(({ profile, path: relativePath, bundle }) => {
    const initialWinner = bundle.summary?.winner_candidate_id ?? null;
    const finalWinner = bundle.research_evidence?.final_winner_candidate_id ?? null;
    return {
      study: bundle.study,
      bundle_id: bundle.bundle_id,
      path: relativePath,
      posture: bundle.posture,
      winner_candidate_id: initialWinner,
      final_winner_candidate_id: finalWinner,
      winner_changed_in_chain: Boolean(initialWinner && finalWinner && initialWinner !== finalWinner),
      reliability_decision: bundle.summary?.reliability_decision ?? null,
      next_round_decision: bundle.summary?.next_round_decision ?? null,
      runnable_next_step_count: bundle.summary?.runnable_next_step_count ?? null,
      next_iteration: bundle.summary?.next_iteration ?? null,
      chain_stop_reason: bundle.summary?.chain_stop_reason ?? null,
      chain_convergence_state: bundle.summary?.chain_convergence_state ?? null,
      chain_round_count: bundle.summary?.chain_round_count ?? null,
      chain_trace_round_count: bundle.research_evidence?.chain_trace_round_count ?? null,
      research_candidate_count: bundle.research_evidence?.candidate_count ?? null,
      primary_metric_ids: bundle.research_evidence?.primary_metric_ids ?? null,
      metric_objective_count: bundle.research_evidence?.metric_objective_count ?? null,
      violated_quality_gate_ids: bundle.research_evidence?.violated_quality_gate_ids ?? null,
      focus_candidate_ids: bundle.research_evidence?.focus_candidate_ids ?? null,
      profile_study: profile.study,
    };
  });
  return {
    schema_version: "kyuubiki.material-research-bundle-index/v1",
    generated_at_utc: new Date().toISOString(),
    bundle_count: bundles.length,
    studies: bundles.map((bundle) => bundle.study),
    winner_changed_in_chain_count: bundles.filter((bundle) => bundle.winner_changed_in_chain).length,
    reliability_decision_counts: countsBy(bundles, "reliability_decision"),
    next_round_decision_counts: countsBy(bundles, "next_round_decision"),
    bundles,
  };
}

function countsBy(items, key) {
  return Object.fromEntries(
    [...items.reduce((counts, item) => {
      const value = item[key] ?? "unknown";
      counts.set(value, (counts.get(value) ?? 0) + 1);
      return counts;
    }, new Map())].sort(([left], [right]) => left.localeCompare(right)),
  );
}

function writeReadme(index, outputPath) {
  const lines = [
    "# Material Research Bundles",
    "",
    `Generated: ${index.generated_at_utc}`,
    "",
    `Bundles: ${index.bundle_count}`,
    "",
    "| Study | Winner | Final winner | Metrics | Gates | Next round | Chain |",
    "| --- | --- | --- | --- | --- | --- | --- |",
    ...index.bundles.map(
      (bundle) =>
        `| \`${bundle.study}\` | \`${bundle.winner_candidate_id}\` | \`${bundle.final_winner_candidate_id}\` | \`${bundle.primary_metric_ids?.length ?? 0}\` | \`${bundle.violated_quality_gate_ids?.length ?? 0}\` | \`${bundle.next_round_decision}@${bundle.next_iteration}\` steps=\`${bundle.runnable_next_step_count}\` | \`${bundle.chain_stop_reason}/${bundle.chain_convergence_state}\` rounds=\`${bundle.chain_round_count}\` trace=\`${bundle.chain_trace_round_count}\` |`,
    ),
    "",
  ];
  fs.writeFileSync(outputPath, `${lines.join("\n")}\n`);
}

function runSelfTest() {
  const index = buildIndex([
    {
      profile: { study: "heat-spreader" },
      path: "tmp/a.json",
      bundle: {
        study: "heat-spreader",
        bundle_id: "bundle.a",
        posture: "screening_research_bundle",
        summary: {
          winner_candidate_id: "candidate-a",
          reliability_decision: "blocked_by_quality_gates",
          next_round_decision: "mitigate_design_risk",
          runnable_next_step_count: 3,
          next_iteration: 2,
          chain_stop_reason: "risk_mitigation_required",
          chain_convergence_state: "blocked_by_quality_gates",
          chain_round_count: 2,
        },
        research_evidence: {
          candidate_count: 2,
          ranked_candidate_ids: ["candidate-a", "candidate-b"],
          winner_candidate_id: "candidate-a",
          primary_metric_ids: ["peak_temperature_c"],
          metric_objective_count: 1,
          violated_quality_gate_ids: ["gate.temperature"],
          focus_candidate_ids: ["candidate-a"],
          quality_gate_decision: "blocked_by_quality_gates",
          plan_decision: "mitigate_design_risk",
          plan_step_count: 3,
          chain_round_count: 2,
          chain_trace_round_count: 2,
          final_winner_candidate_id: "candidate-b",
        },
      },
    },
  ]);
  if (index.bundle_count !== 1 || index.reliability_decision_counts.blocked_by_quality_gates !== 1) {
    fail("self-test did not build expected index counts");
  }
  if (index.bundles[0].runnable_next_step_count !== 3 || index.bundles[0].next_iteration !== 2) {
    fail("self-test did not retain next-round execution summary");
  }
  if (
    index.bundles[0].final_winner_candidate_id !== "candidate-b" ||
    index.bundles[0].winner_changed_in_chain !== true ||
    index.winner_changed_in_chain_count !== 1
  ) {
    fail("self-test did not retain compact research evidence");
  }
  console.log("material research bundle index self-test passed");
}

function main() {
  const args = parseArgs(process.argv);
  if (args.selfTest) {
    runSelfTest();
    return;
  }
  const outDir = repoPath(args.outDir, "--out-dir");
  fs.mkdirSync(outDir.absolute, { recursive: true });
  if (args.ensureBundles) {
    for (const profile of bundleProfiles) {
      ensureBundle(profile, outDir);
    }
  }
  const entries = bundleProfiles.map((profile) => {
    const relativePath = path.join(outDir.relative, profile.file);
    return { profile, path: relativePath, bundle: readBundle(relativePath) };
  });
  const index = buildIndex(entries);
  fs.writeFileSync(path.join(outDir.absolute, "index.json"), `${JSON.stringify(index, null, 2)}\n`);
  writeReadme(index, path.join(outDir.absolute, "README.md"));
  console.log(`material research bundle index wrote ${path.join(outDir.relative, "index.json")}`);
}

main();
