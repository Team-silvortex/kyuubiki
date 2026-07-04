import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import test from "node:test";

const execFileAsync = promisify(execFile);
const scriptPath = path.resolve("scripts/build-benchmark-profile-index.mjs");

async function runIndex(root) {
  await execFileAsync(process.execPath, [scriptPath, "--root", root], {
    cwd: path.resolve("."),
  });
  return JSON.parse(await readFile(path.join(root, "index.json"), "utf8"));
}

async function runIndexWithCoverage(root, coverageTargets) {
  await execFileAsync(
    process.execPath,
    [scriptPath, "--root", root, "--coverage-targets", coverageTargets],
    { cwd: path.resolve(".") },
  );
  return JSON.parse(await readFile(path.join(root, "index.json"), "utf8"));
}

async function runIndexExpectFailure(root, coverageTargets) {
  await assert.rejects(
    execFileAsync(
      process.execPath,
      [scriptPath, "--root", root, "--coverage-targets", coverageTargets],
      { cwd: path.resolve(".") },
    ),
  );
}

test("benchmark profile index reports pass for a valid retained summary", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-"));
  const runDir = path.join(root, "thermal-structural-400k-auto-full");
  await mkdir(runDir, { recursive: true });
  await writeFile(path.join(runDir, "README.md"), "# Run\n");
  await writeFile(
    path.join(runDir, "summary.json"),
    `${JSON.stringify({
      profile: "four_hundred_k",
      matrix: "thermal-structural",
      case_count: 9,
      total_median_ms: 121496.497537,
      peak_rss_mib: 1625.7890625,
      slowest_case: "thermal-plane-triangle-400k",
    })}\n`,
  );

  const index = await runIndex(root);

  assert.equal(index.gate.status, "pass");
  assert.deepEqual(index.gate.reasons, []);
  assert.equal(index.retained_runs.length, 1);
  assert.equal(index.retained_runs[0].slug, "thermal-structural-400k-auto-full");
});

test("benchmark profile index warns when no retained summaries exist", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-empty-"));

  const index = await runIndex(root);
  const readme = await readFile(path.join(root, "README.md"), "utf8");

  assert.equal(index.gate.status, "warn");
  assert.deepEqual(index.gate.reasons, ["no retained benchmark profile runs"]);
  assert.equal(index.retained_runs.length, 0);
  assert.match(readme, /Gate status: `warn`/);
});

test("benchmark profile index warns when latest summary metrics are invalid", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-invalid-"));
  const runDir = path.join(root, "broken-400k-profile");
  await mkdir(runDir, { recursive: true });
  await writeFile(
    path.join(runDir, "summary.json"),
    `${JSON.stringify({
      profile: "four_hundred_k",
      matrix: "thermal-structural",
      case_count: 0,
      total_median_ms: -1,
      peak_rss_mib: 0,
      slowest_case: "--",
    })}\n`,
  );

  const index = await runIndex(root);

  assert.equal(index.gate.status, "warn");
  assert.deepEqual(index.gate.reasons, [
    "latest run broken-400k-profile has no benchmark cases",
    "latest run broken-400k-profile has invalid total median time",
    "latest run broken-400k-profile has invalid peak RSS",
  ]);
  assert.equal(index.retained_runs[0].slug, "broken-400k-profile");
});

test("benchmark profile index skips malformed summaries without failing the run", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-malformed-"));
  const validDir = path.join(root, "valid-profile");
  const brokenDir = path.join(root, "broken-profile");
  await mkdir(validDir, { recursive: true });
  await mkdir(brokenDir, { recursive: true });
  await writeFile(
    path.join(validDir, "summary.json"),
    `${JSON.stringify({
      profile: "four_hundred_k",
      matrix: "thermal-structural",
      case_count: 1,
      total_median_ms: 12.5,
      peak_rss_mib: 256,
      slowest_case: "thermal-bar-400k",
    })}\n`,
  );
  await writeFile(path.join(brokenDir, "summary.json"), "{not-json\n");

  const index = await runIndex(root);
  const readme = await readFile(path.join(root, "README.md"), "utf8");

  assert.equal(index.gate.status, "warn");
  assert.equal(index.retained_runs.length, 1);
  assert.equal(index.skipped_runs.length, 1);
  assert.equal(index.skipped_runs[0].slug, "broken-profile");
  assert.match(index.gate.reasons[0], /skipped run broken-profile/);
  assert.match(readme, /Skipped runs/);
});

test("benchmark profile index groups retained runs by matrix", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-matrix-"));
  const summaries = [
    ["mechanical-a", "mechanical-core", 1, 10.0, 100.0, "axial-bar-400k"],
    ["mechanical-b", "mechanical-core", 2, 25.5, 250.0, "truss-roof-400k"],
    ["thermal-a", "thermal-structural", 1, 15.0, 180.0, "thermal-plane-quad-400k"],
  ];
  for (const [slug, matrix, caseCount, totalMedian, peakRss, slowestCase] of summaries) {
    const runDir = path.join(root, slug);
    await mkdir(runDir, { recursive: true });
    await writeFile(
      path.join(runDir, "summary.json"),
      `${JSON.stringify({
        profile: "four_hundred_k",
        matrix,
        case_count: caseCount,
        total_median_ms: totalMedian,
        peak_rss_mib: peakRss,
        slowest_case: slowestCase,
      })}\n`,
    );
  }

  const index = await runIndex(root);
  const mechanical = index.matrix_summaries.find((entry) => entry.matrix === "mechanical-core");

  assert.equal(mechanical.run_count, 2);
  assert.equal(mechanical.case_count, 3);
  assert.equal(mechanical.total_median_ms, 35.5);
  assert.equal(mechanical.peak_rss_mib, 250);
  assert.equal(mechanical.slowest_case, "truss-roof-400k");
});

test("benchmark profile index reports mechanical 400k coverage", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-coverage-"));
  const cases = [
    "axial-bar-400k",
    "truss-roof-400k#jacobi",
    "space-frame-400k",
    "plane-panel-400k",
    "plane-quad-panel-400k",
  ];
  for (const caseId of cases) {
    const runDir = path.join(root, caseId.replaceAll("#", "-"));
    await mkdir(runDir, { recursive: true });
    await writeFile(
      path.join(runDir, "summary.json"),
      `${JSON.stringify({
        profile: "four_hundred_k",
        matrix: "mechanical-core",
        case_count: 1,
        total_median_ms: 10,
        peak_rss_mib: 100,
        slowest_case: caseId,
      })}\n`,
    );
  }

  const index = await runIndex(root);
  const coverage = index.coverage_summaries.find(
    (entry) => entry.matrix === "mechanical-core",
  );

  assert.equal(coverage.covered_case_count, 5);
  assert.equal(coverage.missing_case_count, 0);
  assert.deepEqual(coverage.missing_cases, []);
});

test("benchmark profile index warns when mechanical 400k coverage is partial", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-partial-"));
  const runDir = path.join(root, "axial-only");
  await mkdir(runDir, { recursive: true });
  await writeFile(
    path.join(runDir, "summary.json"),
    `${JSON.stringify({
      profile: "four_hundred_k",
      matrix: "mechanical-core",
      case_count: 1,
      total_median_ms: 10,
      peak_rss_mib: 100,
      slowest_case: "axial-bar-400k",
    })}\n`,
  );

  const index = await runIndex(root);
  const coverage = index.coverage_summaries.find(
    (entry) => entry.matrix === "mechanical-core",
  );

  assert.equal(index.gate.status, "warn");
  assert.match(
    index.gate.reasons.join("\n"),
    /coverage mechanical-core\/four_hundred_k missing 4 case\(s\)/,
  );
  assert.equal(coverage.covered_case_count, 1);
  assert.equal(coverage.missing_case_count, 4);
  assert.deepEqual(coverage.covered_cases, ["axial-bar-400k"]);
  assert.ok(coverage.missing_cases.includes("truss-roof-400k"));
});

test("benchmark profile index counts all case ids from a combined summary", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-combined-"));
  const runDir = path.join(root, "mechanical-combined");
  await mkdir(runDir, { recursive: true });
  await writeFile(
    path.join(runDir, "summary.json"),
    `${JSON.stringify({
      profile: "four_hundred_k",
      matrix: "mechanical-core",
      case_count: 5,
      case_ids: [
        "axial-bar-400k",
        "truss-roof-400k#jacobi",
        "space-frame-400k",
        "plane-panel-400k",
        "plane-quad-panel-400k",
      ],
      total_median_ms: 1000,
      peak_rss_mib: 2000,
      slowest_case: "plane-quad-panel-400k",
    })}\n`,
  );

  const index = await runIndex(root);
  const coverage = index.coverage_summaries.find(
    (entry) => entry.matrix === "mechanical-core",
  );

  assert.equal(index.gate.status, "pass");
  assert.equal(coverage.covered_case_count, 5);
  assert.equal(coverage.missing_case_count, 0);
});

test("benchmark profile index accepts a custom coverage manifest", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-custom-"));
  const manifestPath = path.join(root, "coverage.json");
  const runDir = path.join(root, "fluid-smoke");
  await mkdir(runDir, { recursive: true });
  await writeFile(
    manifestPath,
    `${JSON.stringify({
      schema_version: "kyuubiki.benchmark-profile-coverage/v1",
      targets: [
        {
          matrix: "fluid-core",
          profile: "four_hundred_k",
          expected_cases: ["cavity-flow-400k", "pipe-flow-400k"],
        },
      ],
    })}\n`,
  );
  await writeFile(
    path.join(runDir, "summary.json"),
    `${JSON.stringify({
      profile: "four_hundred_k",
      matrix: "fluid-core",
      case_count: 1,
      case_ids: ["cavity-flow-400k"],
      total_median_ms: 50,
      peak_rss_mib: 300,
      slowest_case: "cavity-flow-400k",
    })}\n`,
  );

  const index = await runIndexWithCoverage(root, manifestPath);

  assert.equal(index.coverage_targets_manifest, path.relative(path.resolve("."), manifestPath));
  assert.equal(index.coverage_summaries[0].matrix, "fluid-core");
  assert.equal(index.coverage_summaries[0].covered_case_count, 1);
  assert.deepEqual(index.coverage_summaries[0].missing_cases, ["pipe-flow-400k"]);
  assert.match(index.gate.reasons.join("\n"), /coverage fluid-core\/four_hundred_k missing 1 case/);
});

test("benchmark profile index rejects an empty coverage manifest", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-empty-targets-"));
  const manifestPath = path.join(root, "coverage.json");
  await writeFile(
    manifestPath,
    `${JSON.stringify({
      schema_version: "kyuubiki.benchmark-profile-coverage/v1",
      targets: [],
    })}\n`,
  );

  await runIndexExpectFailure(root, manifestPath);
});

test("benchmark profile index rejects malformed coverage targets", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "kyuubiki-profile-index-bad-target-"));
  const manifestPath = path.join(root, "coverage.json");
  await writeFile(
    manifestPath,
    `${JSON.stringify({
      schema_version: "kyuubiki.benchmark-profile-coverage/v1",
      targets: [
        {
          matrix: "mechanical-core",
          profile: "four_hundred_k",
          expected_cases: ["axial-bar-400k", ""],
        },
      ],
    })}\n`,
  );

  await runIndexExpectFailure(root, manifestPath);
});
