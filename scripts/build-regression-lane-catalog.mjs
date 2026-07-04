#!/usr/bin/env node

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { readBenchmarkProfileLane } from "./build-regression-lane-catalog-profile.mjs";

const DEFAULT_TMP_ROOT = path.resolve("tmp");
const GATE_POLICY = {
  directMeshDocker: {
    elapsedWarnPct: 8,
    elapsedFailPct: 15,
    rssWarnPct: 10,
    rssFailPct: 20,
  },
  workflowCatalog: {
    caseMedianWarnPct: 20,
    caseAvgWarnPct: 30,
  },
  workflowMesh: {
    totalDurationWarnMs: 22000,
    totalDurationFailMs: 30000,
    slowestTestWarnMs: 8000,
    slowestTestFailMs: 12000,
  },
};

function parseArgs(argv) {
  const options = {
    tmpRoot: DEFAULT_TMP_ROOT,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];

    if (arg === "--tmp-root" && next) {
      options.tmpRoot = path.resolve(next);
      index += 1;
    }
  }

  return options;
}

async function exists(filePath) {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

async function latestRun(root) {
  let entries = [];
  try {
    entries = await readdir(root, { withFileTypes: true });
  } catch {
    return null;
  }

  const runs = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    const dir = path.join(root, entry.name);
    const info = await stat(dir);
    runs.push({
      slug: entry.name,
      dir,
      generatedAtUnixS: Math.floor(info.mtimeMs / 1000),
    });
  }

  runs.sort((left, right) => right.generatedAtUnixS - left.generatedAtUnixS);
  return runs[0] ?? null;
}

function round(value, digits = 3) {
  return Number(Number(value).toFixed(digits));
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function renderMetric(metric) {
  const baselinePart =
    metric.baseline == null
      ? ""
      : ` · baseline <code>${escapeHtml(metric.baseline)}</code>`;
  const deltaPart =
    metric.delta_pct == null
      ? ""
      : ` · delta <code>${escapeHtml(metric.delta_pct)}%</code>`;
  return `<li><code>${escapeHtml(metric.name)}</code>: <code>${escapeHtml(metric.value)}</code> ${escapeHtml(metric.unit ?? "")}${baselinePart}${deltaPart}</li>`;
}

function makeGate(status, reasons) {
  return {
    status,
    reasons,
  };
}

function worstGateStatus(statuses) {
  if (statuses.includes("fail")) {
    return "fail";
  }
  if (statuses.includes("warn")) {
    return "warn";
  }
  return "pass";
}

function enforceableGateStatuses(lanes) {
  return lanes
    .filter((lane) => lane.gate_scope !== "advisory")
    .map((lane) => lane.gate?.status ?? lane.status ?? "pass");
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

async function readDirectMeshLane(tmpRoot) {
  const laneRoot = path.join(tmpRoot, "direct-mesh-benchmark-container");
  const run = await latestRun(laneRoot);
  if (!run) {
    return null;
  }

  const summaryPath = path.join(run.dir, "summary.json");
  if (!(await exists(summaryPath))) {
    return null;
  }

  const summary = await readJson(summaryPath);
  const comparePath = path.join(run.dir, "compare.json");
  const compare = (await exists(comparePath)) ? await readJson(comparePath) : null;
  const elapsed = compare?.metrics?.find((item) => item.name === "elapsed_mean");
  const rss = compare?.metrics?.find((item) => item.name === "rss_mean");
  const gateReasons = [];

  if (Array.isArray(compare?.failures) && compare.failures.length > 0) {
    gateReasons.push(...compare.failures);
  }

  if (typeof elapsed?.delta_pct === "number") {
    if (elapsed.delta_pct > GATE_POLICY.directMeshDocker.elapsedFailPct) {
      gateReasons.push(
        `elapsed mean regression ${elapsed.delta_pct}% exceeded fail threshold ${GATE_POLICY.directMeshDocker.elapsedFailPct}%`,
      );
    } else if (elapsed.delta_pct > GATE_POLICY.directMeshDocker.elapsedWarnPct) {
      gateReasons.push(
        `elapsed mean regression ${elapsed.delta_pct}% exceeded warn threshold ${GATE_POLICY.directMeshDocker.elapsedWarnPct}%`,
      );
    }
  }

  if (typeof rss?.delta_pct === "number") {
    if (rss.delta_pct > GATE_POLICY.directMeshDocker.rssFailPct) {
      gateReasons.push(
        `rss mean regression ${rss.delta_pct}% exceeded fail threshold ${GATE_POLICY.directMeshDocker.rssFailPct}%`,
      );
    } else if (rss.delta_pct > GATE_POLICY.directMeshDocker.rssWarnPct) {
      gateReasons.push(
        `rss mean regression ${rss.delta_pct}% exceeded warn threshold ${GATE_POLICY.directMeshDocker.rssWarnPct}%`,
      );
    }
  }

  const gateStatus = gateReasons.some((reason) => reason.includes("fail threshold"))
    || (compare && compare.ok === false)
    ? "fail"
    : gateReasons.length > 0
      ? "warn"
      : "pass";

  return {
    id: "direct-mesh-docker",
    title: "Direct-mesh Docker",
    category: "benchmark",
    generated_at_unix_s: compare
      ? Math.floor(Date.parse(compare.generated_at) / 1000)
      : run.generatedAtUnixS,
    status: compare ? (compare.ok ? "pass" : "fail") : "observed",
    gate: makeGate(gateStatus, gateReasons),
    summary: `Repeat ${summary.repeat ?? 0}, ${summary.runs?.length ?? 0} run(s), ${summary.runs?.[0]?.subtests?.length ?? 0} subtest(s).`,
    metrics: [
      {
        name: "elapsed_mean",
        unit: "s",
        value: round(summary.aggregate?.elapsed_s?.mean ?? 0),
        baseline: elapsed?.baseline ?? null,
        delta_pct: elapsed?.delta_pct ?? null,
      },
      {
        name: "rss_mean",
        unit: "KiB",
        value: round(summary.aggregate?.max_rss_kib?.mean ?? 0),
        baseline: rss?.baseline ?? null,
        delta_pct: rss?.delta_pct ?? null,
      },
    ],
    links: [
      path.relative(tmpRoot, summaryPath),
      path.relative(tmpRoot, comparePath),
      `direct-mesh-benchmark-container/${run.slug}/compare.md`,
    ],
  };
}

async function readWorkflowCatalogLane(tmpRoot) {
  const laneRoot = path.join(tmpRoot, "workflow-catalog-benchmark");
  const run = await latestRun(laneRoot);
  if (!run) {
    return null;
  }

  const summaryPath = path.join(run.dir, "summary.json");
  if (!(await exists(summaryPath))) {
    return null;
  }

  const summary = await readJson(summaryPath);
  const comparePath = path.join(run.dir, "compare.json");
  const compare = (await exists(comparePath)) ? await readJson(comparePath) : null;
  const range = summary.summary?.median_elapsed_ms_range ?? [];
  const gateReasons = [];

  if (Array.isArray(compare?.failures) && compare.failures.length > 0) {
    gateReasons.push(...compare.failures);
  }

  const comparedCases = Array.isArray(compare?.cases) ? compare.cases : [];
  for (const entry of comparedCases) {
    const medianMetric = entry.metrics?.find((metric) => metric.name === "median_elapsed_ms");
    const avgMetric = entry.metrics?.find((metric) => metric.name === "avg_elapsed_ms");

    if (
      typeof medianMetric?.delta_pct === "number" &&
      medianMetric.delta_pct > GATE_POLICY.workflowCatalog.caseMedianWarnPct
    ) {
      gateReasons.push(
        `${entry.case_id} median regression ${medianMetric.delta_pct}% exceeded warn threshold ${GATE_POLICY.workflowCatalog.caseMedianWarnPct}%`,
      );
    }

    if (
      typeof avgMetric?.delta_pct === "number" &&
      avgMetric.delta_pct > GATE_POLICY.workflowCatalog.caseAvgWarnPct
    ) {
      gateReasons.push(
        `${entry.case_id} average regression ${avgMetric.delta_pct}% exceeded warn threshold ${GATE_POLICY.workflowCatalog.caseAvgWarnPct}%`,
      );
    }
  }

  const gateStatus = compare && compare.ok === false
    ? "fail"
    : gateReasons.length > 0
      ? "warn"
      : "pass";

  return {
    id: "workflow-catalog",
    title: "Workflow catalog",
    category: "workflow-benchmark",
    generated_at_unix_s: Math.floor(Date.parse(summary.generated_at ?? 0) / 1000) || run.generatedAtUnixS,
    status: compare ? (compare.ok ? "pass" : "fail") : "observed",
    gate: makeGate(gateStatus, gateReasons),
    summary: `${summary.summary?.case_count ?? summary.cases?.length ?? 0} case(s), fastest \`${summary.summary?.fastest_case_id ?? "n/a"}\`, slowest \`${summary.summary?.slowest_case_id ?? "n/a"}\`.`,
    metrics: [
      {
        name: "case_count",
        unit: "case",
        value: summary.summary?.case_count ?? summary.cases?.length ?? 0,
      },
      {
        name: "median_elapsed_min",
        unit: "ms",
        value: range[0] ?? 0,
      },
      {
        name: "median_elapsed_max",
        unit: "ms",
        value: range[1] ?? 0,
      },
      {
        name: "regression_failures",
        unit: "count",
        value: compare?.failures?.length ?? 0,
      },
    ],
    links: [
      path.relative(tmpRoot, summaryPath),
      path.relative(tmpRoot, comparePath),
      `workflow-catalog-benchmark/${run.slug}/compare.md`,
    ],
  };
}

async function readWorkflowMeshLane(tmpRoot) {
  const indexPath = path.join(tmpRoot, "workflow-mesh-regression", "index.json");
  if (!(await exists(indexPath))) {
    return null;
  }

  const indexPayload = await readJson(indexPath);
  const latest = Array.isArray(indexPayload.retained_runs) ? indexPayload.retained_runs[0] : null;
  if (!latest) {
    return null;
  }

  const maxDuration = latest.tests.reduce(
    (max, test) => Math.max(max, Number(test.duration_ms ?? 0)),
    0,
  );
  const gateReasons = [];

  if (latest.status !== "passed") {
    gateReasons.push("latest workflow mesh regression run did not pass");
  }

  if (Number(latest.total_duration_ms ?? 0) > GATE_POLICY.workflowMesh.totalDurationFailMs) {
    gateReasons.push(
      `total duration ${round(latest.total_duration_ms ?? 0)}ms exceeded fail threshold ${GATE_POLICY.workflowMesh.totalDurationFailMs}ms`,
    );
  } else if (Number(latest.total_duration_ms ?? 0) > GATE_POLICY.workflowMesh.totalDurationWarnMs) {
    gateReasons.push(
      `total duration ${round(latest.total_duration_ms ?? 0)}ms exceeded warn threshold ${GATE_POLICY.workflowMesh.totalDurationWarnMs}ms`,
    );
  }

  if (maxDuration > GATE_POLICY.workflowMesh.slowestTestFailMs) {
    gateReasons.push(
      `slowest test duration ${round(maxDuration)}ms exceeded fail threshold ${GATE_POLICY.workflowMesh.slowestTestFailMs}ms`,
    );
  } else if (maxDuration > GATE_POLICY.workflowMesh.slowestTestWarnMs) {
    gateReasons.push(
      `slowest test duration ${round(maxDuration)}ms exceeded warn threshold ${GATE_POLICY.workflowMesh.slowestTestWarnMs}ms`,
    );
  }

  const gateStatus = latest.status !== "passed"
    || gateReasons.some((reason) => reason.includes("fail threshold"))
    ? "fail"
    : gateReasons.length > 0
      ? "warn"
      : "pass";

  return {
    id: "workflow-mesh",
    title: "Workflow mesh",
    category: "workflow-regression",
    generated_at_unix_s: latest.generated_at_unix_s ?? indexPayload.generated_at_unix_s ?? 0,
    status: latest.status === "passed" ? "pass" : "fail",
    gate: makeGate(gateStatus, gateReasons),
    summary: `${latest.total_tests ?? 0} test(s), pass ${latest.total_pass ?? 0}, fail ${latest.total_fail ?? 0}.`,
    metrics: [
      {
        name: "total_duration",
        unit: "ms",
        value: round(latest.total_duration_ms ?? 0),
      },
      {
        name: "slowest_test_duration",
        unit: "ms",
        value: round(maxDuration),
      },
      {
        name: "retained_runs",
        unit: "run",
        value: indexPayload.retained_runs?.length ?? 0,
      },
    ],
    links: [
      "workflow-mesh-regression/index.json",
      "workflow-mesh-regression/index.html",
      latest.files?.summary_json
        ? `workflow-mesh-regression/${latest.files.summary_json}`
        : `workflow-mesh-regression/${latest.slug}/summary.json`,
    ],
  };
}

function renderReadme(tmpRoot, lanes) {
  const overallStatus = worstGateStatus(enforceableGateStatuses(lanes));
  const lines = [
    "# Regression Lane Catalog",
    "",
    `- Root: \`${path.relative(process.cwd(), tmpRoot) || "."}\``,
    `- Lanes indexed: \`${lanes.length}\``,
    `- Overall gate status: \`${overallStatus}\``,
    "",
    "| Lane | Category | Status | Gate | Generated (unix) | Summary |",
    "| --- | --- | --- | --- | ---: | --- |",
  ];

  for (const lane of lanes) {
    lines.push(
      `| \`${lane.id}\` | \`${lane.category}\` | \`${lane.status}\` | \`${lane.gate?.status ?? "n/a"}\` | \`${lane.generated_at_unix_s}\` | ${lane.summary} |`,
    );
  }

  lines.push("");
  return `${lines.join("\n").trimEnd()}\n`;
}

function renderHtml(tmpRoot, lanes) {
  const overallStatus = worstGateStatus(enforceableGateStatuses(lanes));
  const cards = lanes
    .map(
      (lane) => `<article class="docs-card">
        <div class="docs-kicker">${escapeHtml(lane.category)}</div>
        <h2>${escapeHtml(lane.title)}</h2>
        <p class="docs-copy">${escapeHtml(lane.summary)}</p>
        <div class="docs-meta">
          <span class="docs-chip">Lane: ${escapeHtml(lane.id)}</span>
          <span class="docs-chip">Status: ${escapeHtml(lane.status)}</span>
          <span class="docs-chip">Gate: ${escapeHtml(lane.gate?.status ?? "n/a")}</span>
          <span class="docs-chip">Scope: ${escapeHtml(lane.gate_scope ?? "enforced")}</span>
          <span class="docs-chip">Generated at unix: ${escapeHtml(lane.generated_at_unix_s)}</span>
        </div>
        ${
          lane.gate?.reasons?.length
            ? `<p class="docs-copy"><strong>Gate reasons:</strong> ${escapeHtml(lane.gate.reasons.join(" | "))}</p>`
            : ""
        }
        <h3>Metrics</h3>
        <ul class="docs-list">
          ${lane.metrics.map(renderMetric).join("\n")}
        </ul>
        <h3>Artifacts</h3>
        <ul class="docs-list">
          ${lane.links.map((item) => `<li><code>${escapeHtml(item)}</code></li>`).join("\n")}
        </ul>
      </article>`,
    )
    .join("\n");

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kyuubiki Regression Lane Catalog</title>
    <link rel="stylesheet" href="../apps/hub-gui/ui/docs/docs.css" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">Regression Catalog</div>
        <h1>Unified regression lane view</h1>
        <p class="docs-copy">
          This page normalizes the latest retained outputs across direct-mesh, workflow-catalog, and workflow-mesh lanes into a shared read model.
        </p>
        <div class="docs-meta">
          <span class="docs-chip">Root: ${escapeHtml(path.relative(process.cwd(), tmpRoot) || ".")}</span>
          <span class="docs-chip">Lanes indexed: ${escapeHtml(lanes.length)}</span>
          <span class="docs-chip">Overall gate: ${escapeHtml(overallStatus)}</span>
        </div>
        <div class="docs-links">
          <a class="docs-link" href="./regression-lane-catalog.json">Open JSON catalog</a>
          <a class="docs-link" href="./regression-lane-catalog.md">Open Markdown catalog</a>
          <a class="docs-link" href="./nightly-overview.html">Open nightly overview</a>
        </div>
      </section>
      <section class="docs-grid">
        ${cards}
      </section>
    </main>
  </body>
</html>
`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const lanes = (
    await Promise.all([
      readDirectMeshLane(options.tmpRoot),
      readWorkflowCatalogLane(options.tmpRoot),
      readWorkflowMeshLane(options.tmpRoot),
      readBenchmarkProfileLane(options.tmpRoot),
    ])
  )
    .filter(Boolean)
    .sort((left, right) => right.generated_at_unix_s - left.generated_at_unix_s);

  const payload = {
    schema_version: "kyuubiki.regression-lane-catalog/v1",
    root: path.relative(process.cwd(), options.tmpRoot) || ".",
    generated_at_unix_s: Math.floor(Date.now() / 1000),
    policy: GATE_POLICY,
    overall_gate_status: worstGateStatus(enforceableGateStatuses(lanes)),
    lanes,
  };

  await writeFile(
    path.join(options.tmpRoot, "regression-lane-catalog.json"),
    `${JSON.stringify(payload, null, 2)}\n`,
  );
  await writeFile(
    path.join(options.tmpRoot, "regression-lane-catalog.md"),
    renderReadme(options.tmpRoot, lanes),
  );
  await writeFile(
    path.join(options.tmpRoot, "regression-lane-catalog.html"),
    renderHtml(options.tmpRoot, lanes),
  );

  process.stdout.write(`${path.relative(process.cwd(), options.tmpRoot) || "."}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
