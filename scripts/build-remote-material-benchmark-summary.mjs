#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function parseArgs(argv) {
  const options = {
    inputDir: path.join(repoRoot, "tmp", "remote-material-research"),
    jsonOut: path.join(repoRoot, "tmp", "remote-material-research", "summary.json"),
    markdownOut: path.join(repoRoot, "tmp", "remote-material-research", "README.md"),
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--help" || arg === "-h") {
      printUsage();
      process.exit(0);
    } else if (arg === "--input-dir" && next) {
      options.inputDir = path.resolve(next);
      index += 1;
    } else if (arg === "--json-out" && next) {
      options.jsonOut = path.resolve(next);
      index += 1;
    } else if (arg === "--markdown-out" && next) {
      options.markdownOut = path.resolve(next);
      index += 1;
    } else {
      fail(`unknown or incomplete argument: ${arg}`);
    }
  }
  return options;
}

function readRuns(inputDir) {
  if (!fs.existsSync(inputDir)) return [];
  const runs = [];
  for (const name of fs.readdirSync(inputDir).sort()) {
    const dir = path.join(inputDir, name);
    const benchmarkPath = path.join(dir, "remote-material-research-benchmark.json");
    const researchPath = path.join(dir, "material-research-example.json");
    if (!fs.statSync(dir).isDirectory() || !fs.existsSync(benchmarkPath)) continue;
    const benchmark = readJson(benchmarkPath);
    const research = fs.existsSync(researchPath) ? readJson(researchPath) : null;
    runs.push({
      benchmark,
      dir,
      name,
      research,
    });
  }
  return runs;
}

function buildSummary(runs) {
  const cases = runs.flatMap((run) =>
    (run.benchmark.cases || []).map((item) => ({
      case_id: item.id,
      dof_count: item.dof_count,
      matrix: run.benchmark.matrix,
      median_ms: item.median_ms,
      ok: item.ok,
      peak_rss_mib: item.peak_rss_kib == null ? null : item.peak_rss_kib / 1024,
      profile: run.benchmark.profile,
      residual_norm: item.solver_residual_norm,
      run: run.name,
      solver_iterations: item.solver_iterations,
    })),
  );
  const sortedByTime = [...cases].sort((left, right) => right.median_ms - left.median_ms);
  const sortedByMemory = [...cases].sort(
    (left, right) => (right.peak_rss_mib || 0) - (left.peak_rss_mib || 0),
  );
  const stageHotspots = runs
    .flatMap((run) =>
      (run.benchmark.cases || []).flatMap((item) =>
        (item.memory_stages || [])
          .filter((stage) => Number.isFinite(stage.elapsed_ms))
          .map((stage) => ({
            case_id: item.id,
            elapsed_ms: stage.elapsed_ms,
            matrix: run.benchmark.matrix,
            profile: run.benchmark.profile,
            run: run.name,
            stage: stage.label,
            stage_rss_mib: stage.rss_kib == null ? null : stage.rss_kib / 1024,
          })),
      ),
    )
    .sort((left, right) => right.elapsed_ms - left.elapsed_ms);
  return {
    schema_version: "kyuubiki.remote-material-benchmark-summary/v1",
    generated_at_utc: new Date().toISOString(),
    run_count: runs.length,
    case_count: cases.length,
    failed_cases: cases.filter((item) => !item.ok),
    hottest_cases: sortedByTime.slice(0, 8),
    memory_heaviest_cases: sortedByMemory.slice(0, 8),
    stage_hotspots: stageHotspots.slice(0, 12),
    runs: runs.map((run) => ({
      matrix: run.benchmark.matrix,
      name: run.name,
      profile: run.benchmark.profile,
      repeat: run.benchmark.repeat,
      winner_candidate_id: run.research?.exploration?.report?.winner_candidate_id || null,
    })),
  };
}

function writeMarkdown(summary, outputPath) {
  const lines = [
    "# Remote Material Benchmark Summary",
    "",
    `Generated: ${summary.generated_at_utc}`,
    "",
    `Runs: ${summary.run_count}`,
    "",
    `Cases: ${summary.case_count}`,
    "",
    `Failed cases: ${summary.failed_cases.length}`,
    "",
    "## Hottest Cases",
    "",
    "| Rank | Matrix | Case | Median ms | RSS MiB | DOF | Iterations | Residual |",
    "| ---: | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ...summary.hottest_cases.map((item, index) => caseRow(index + 1, item)),
    "",
    "## Memory Heaviest Cases",
    "",
    "| Rank | Matrix | Case | Median ms | RSS MiB | DOF | Iterations | Residual |",
    "| ---: | --- | --- | ---: | ---: | ---: | ---: | --- |",
    ...summary.memory_heaviest_cases.map((item, index) => caseRow(index + 1, item)),
    "",
    "## Stage Hotspots",
    "",
    "| Rank | Matrix | Case | Stage | Elapsed ms | Stage RSS MiB |",
    "| ---: | --- | --- | --- | ---: | ---: |",
    ...summary.stage_hotspots.map((item, index) => stageRow(index + 1, item)),
    "",
  ];
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${lines.join("\n")}\n`);
}

function caseRow(rank, item) {
  const rss = item.peak_rss_mib == null ? "n/a" : item.peak_rss_mib.toFixed(1);
  const iterations = item.solver_iterations == null ? "n/a" : String(item.solver_iterations);
  const residual = item.residual_norm == null ? "n/a" : Number(item.residual_norm).toExponential(2);
  return `| ${rank} | ${item.matrix} | ${item.case_id} | ${item.median_ms.toFixed(2)} | ${rss} | ${item.dof_count} | ${iterations} | ${residual} |`;
}

function stageRow(rank, item) {
  const rss = item.stage_rss_mib == null ? "n/a" : item.stage_rss_mib.toFixed(1);
  return `| ${rank} | ${item.matrix} | ${item.case_id} | ${item.stage} | ${item.elapsed_ms.toFixed(2)} | ${rss} |`;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function printUsage() {
  console.log(`Usage:
  node ./scripts/build-remote-material-benchmark-summary.mjs [options]

Options:
  --input-dir <dir>       Default: tmp/remote-material-research
  --json-out <path>       Default: tmp/remote-material-research/summary.json
  --markdown-out <path>   Default: tmp/remote-material-research/README.md`);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

const options = parseArgs(process.argv.slice(2));
const runs = readRuns(options.inputDir);
const summary = buildSummary(runs);
fs.mkdirSync(path.dirname(options.jsonOut), { recursive: true });
fs.writeFileSync(options.jsonOut, `${JSON.stringify(summary, null, 2)}\n`);
writeMarkdown(summary, options.markdownOut);
console.log(`remote material benchmark summary: ${options.markdownOut}`);
