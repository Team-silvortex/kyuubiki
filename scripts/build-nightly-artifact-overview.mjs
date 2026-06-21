#!/usr/bin/env node

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const DEFAULT_TMP_ROOT = path.resolve("tmp");

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

async function listDirs(root) {
  try {
    const entries = await readdir(root, { withFileTypes: true });
    return entries
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name);
  } catch {
    return [];
  }
}

async function latestRun(root) {
  const names = await listDirs(root);
  const runs = [];

  for (const name of names) {
    const dir = path.join(root, name);
    const info = await stat(dir);
    runs.push({
      slug: name,
      dir,
      generatedAtUnixS: Math.floor(info.mtimeMs / 1000),
    });
  }

  runs.sort((left, right) => right.generatedAtUnixS - left.generatedAtUnixS);
  return runs[0] ?? null;
}

async function readStandardLane(tmpRoot) {
  const indexPath = path.join(tmpRoot, "standard-benchmark", "index.json");
  if (!(await exists(indexPath))) {
    return null;
  }

  const payload = JSON.parse(await readFile(indexPath, "utf8"));
  const run = Array.isArray(payload.retained_runs) ? payload.retained_runs[0] : null;
  if (!run) {
    return {
      id: "standard-benchmark",
      title: "Standard benchmark nightly",
      summary: "No retained standard benchmark runs yet.",
      generatedAtUnixS: payload.generated_at_unix_s ?? 0,
      links: [
        "standard-benchmark/index.html",
        "standard-benchmark/index.json",
        "standard-benchmark/README.md",
      ],
    };
  }

  return {
    id: "standard-benchmark",
    title: "Standard benchmark nightly",
    summary: `Latest retained run \`${run.slug}\` on profile \`${run.profile}\`.`,
    generatedAtUnixS: run.generated_at_unix_s ?? payload.generated_at_unix_s ?? 0,
    links: [
      "standard-benchmark/index.html",
      "standard-benchmark/index.json",
      `standard-benchmark/${run.merged_report}`,
    ],
    detail: run.headline ?? "",
  };
}

async function readDirectMeshLane(tmpRoot) {
  const laneRoot = path.join(tmpRoot, "direct-mesh-benchmark-container");
  const run = await latestRun(laneRoot);
  if (!run) {
    return null;
  }

  return {
    id: "direct-mesh-docker",
    title: "Direct-mesh Docker nightly",
    summary: `Latest run \`${run.slug}\` from the LAN Docker regression harness.`,
    generatedAtUnixS: run.generatedAtUnixS,
    links: [
      `direct-mesh-benchmark-container/${run.slug}/summary.json`,
      `direct-mesh-benchmark-container/${run.slug}/compare.json`,
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

  return {
    id: "workflow-catalog",
    title: "Workflow catalog nightly",
    summary: `Latest run \`${run.slug}\` from the orchestrated composite workflow regression path.`,
    generatedAtUnixS: run.generatedAtUnixS,
    links: [
      `workflow-catalog-benchmark/${run.slug}/summary.json`,
      `workflow-catalog-benchmark/${run.slug}/compare.json`,
      `workflow-catalog-benchmark/${run.slug}/compare.md`,
    ],
  };
}

async function readWorkflowMeshLane(tmpRoot) {
  const indexPath = path.join(tmpRoot, "workflow-mesh-regression", "index.json");
  if (!(await exists(indexPath))) {
    return null;
  }

  const payload = JSON.parse(await readFile(indexPath, "utf8"));
  const run = Array.isArray(payload.retained_runs) ? payload.retained_runs[0] : null;
  if (!run) {
    return {
      id: "workflow-mesh",
      title: "Workflow mesh nightly",
      summary: "No retained workflow mesh runs yet.",
      generatedAtUnixS: payload.generated_at_unix_s ?? 0,
      links: [
        "workflow-mesh-regression/index.html",
        "workflow-mesh-regression/index.json",
        "workflow-mesh-regression/README.md",
      ],
    };
  }

  return {
    id: "workflow-mesh",
    title: "Workflow mesh nightly",
    summary: `Latest run \`${run.slug}\` from the remote distributed workflow mesh regression trio.`,
    generatedAtUnixS: run.generated_at_unix_s ?? payload.generated_at_unix_s ?? 0,
    links: [
      `workflow-mesh-regression/${run.slug}/summary.json`,
      `workflow-mesh-regression/${run.slug}/README.md`,
      `workflow-mesh-regression/${run.slug}/run.log`,
      "workflow-mesh-regression/index.html",
      "workflow-mesh-regression/index.json",
    ],
  };
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function renderReadme(tmpRoot, lanes) {
  const lines = [
    "# Tmp Workspace Map",
    "",
    `- Root: \`${path.relative(process.cwd(), tmpRoot) || "."}\``,
    "- Purpose: disposable local runtime state, benchmark artifacts, and nightly comparison outputs.",
    "- Cross-lane regression catalog: `regression-lane-catalog.json`, `regression-lane-catalog.md`, `regression-lane-catalog.html`.",
    "- Gate report: `regression-gate-report.json`, `regression-gate-report.md`.",
    "",
    "## Nightly lanes",
    "",
  ];

  for (const lane of lanes) {
    lines.push(`- \`${lane.id}\``);
    lines.push(`  ${lane.summary}`);
    lines.push(`  Generated at unix: \`${lane.generatedAtUnixS}\``);
    lines.push(`  Links: ${lane.links.map((item) => `\`${item}\``).join(", ")}`);
  }

  lines.push("");
  return `${lines.join("\n").trimEnd()}\n`;
}

function renderHtml(tmpRoot, lanes) {
  const cards = lanes
    .map(
      (lane) => `<article class="docs-card">
        <div class="docs-kicker">nightly lane</div>
        <h2>${escapeHtml(lane.title)}</h2>
        <p class="docs-copy">${escapeHtml(lane.summary)}</p>
        <div class="docs-meta">
          <span class="docs-chip">Lane: ${escapeHtml(lane.id)}</span>
          <span class="docs-chip">Generated at unix: ${escapeHtml(lane.generatedAtUnixS)}</span>
        </div>
        ${
          lane.detail
            ? `<p class="docs-copy"><strong>Detail:</strong> <code>${escapeHtml(lane.detail)}</code></p>`
            : ""
        }
        <ul class="docs-list">
          ${lane.links
            .map((item) => `<li><code>${escapeHtml(item)}</code></li>`)
            .join("\n")}
        </ul>
      </article>`,
    )
    .join("\n");

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kyuubiki Nightly Artifact Overview</title>
    <link rel="stylesheet" href="../apps/hub-gui/ui/docs/docs.css" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">Tmp Nightly Overview</div>
        <h1>Local nightly artifact map</h1>
        <p class="docs-copy">
          This page indexes the latest local outputs for the current self-hosted nightly regression lanes.
        </p>
        <div class="docs-meta">
          <span class="docs-chip">Root: ${escapeHtml(path.relative(process.cwd(), tmpRoot) || ".")}</span>
          <span class="docs-chip">Lanes indexed: ${escapeHtml(lanes.length)}</span>
        </div>
        <div class="docs-links">
          <a class="docs-link" href="./README.md">Open tmp README</a>
          <a class="docs-link" href="./nightly-overview.json">Open JSON index</a>
          <a class="docs-link" href="./regression-lane-catalog.html">Open regression catalog</a>
          <a class="docs-link" href="./regression-gate-report.json">Open gate report</a>
          <a class="docs-link" href="../docs/testing-and-ci.md">Open testing guide</a>
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
      readWorkflowMeshLane(options.tmpRoot),
      readWorkflowCatalogLane(options.tmpRoot),
      readStandardLane(options.tmpRoot),
    ])
  ).filter(Boolean);

  const payload = {
    schema_version: "kyuubiki.nightly-artifact-overview/v1",
    root: path.relative(process.cwd(), options.tmpRoot) || ".",
    generated_at_unix_s: Math.floor(Date.now() / 1000),
    lanes,
  };

  await writeFile(
    path.join(options.tmpRoot, "nightly-overview.json"),
    `${JSON.stringify(payload, null, 2)}\n`,
  );
  await writeFile(
    path.join(options.tmpRoot, "README.md"),
    renderReadme(options.tmpRoot, lanes),
  );
  await writeFile(
    path.join(options.tmpRoot, "nightly-overview.html"),
    renderHtml(options.tmpRoot, lanes),
  );

  process.stdout.write(`${path.relative(process.cwd(), options.tmpRoot) || "."}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
