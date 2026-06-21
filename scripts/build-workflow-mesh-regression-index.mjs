#!/usr/bin/env node

import { readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const DEFAULT_ROOT = path.resolve("tmp/workflow-mesh-regression");
const DEFAULT_RETAIN = 12;

function parseArgs(argv) {
  const options = {
    root: DEFAULT_ROOT,
    retain: DEFAULT_RETAIN,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];

    if (arg === "--root" && next) {
      options.root = path.resolve(next);
      index += 1;
      continue;
    }

    if (arg === "--retain" && next) {
      const parsed = Number.parseInt(next, 10);
      if (Number.isFinite(parsed) && parsed >= 0) {
        options.retain = parsed;
      }
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

async function discoverRuns(root) {
  let entries = [];
  try {
    entries = await readdir(root, { withFileTypes: true });
  } catch {
    return [];
  }

  const runs = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }

    const runDir = path.join(root, entry.name);
    const summaryPath = path.join(runDir, "summary.json");
    if (!(await exists(summaryPath))) {
      continue;
    }

    const payload = JSON.parse(await readFile(summaryPath, "utf8"));
    const info = await stat(summaryPath);
    runs.push({
      slug: entry.name,
      generated_at_unix_s:
        payload.generated_at_unix_s ?? Math.floor(info.mtimeMs / 1000),
      status: payload.status ?? "unknown",
      total_tests: payload.total_tests ?? 0,
      total_pass: payload.total_pass ?? 0,
      total_fail: payload.total_fail ?? 0,
      total_duration_ms: payload.total_duration_ms ?? 0,
      tests: Array.isArray(payload.tests) ? payload.tests : [],
      files: {
        summary_json: path.relative(root, summaryPath),
        readme_md: path.relative(root, path.join(runDir, "README.md")),
        run_log: path.relative(root, path.join(runDir, "run.log")),
      },
    });
  }

  runs.sort((left, right) => right.generated_at_unix_s - left.generated_at_unix_s);
  return runs;
}

async function pruneRuns(runs, retain) {
  if (retain < 0 || runs.length <= retain) {
    return { kept: runs, removed: [] };
  }

  const kept = runs.slice(0, retain);
  const removed = runs.slice(retain);

  for (const run of removed) {
    await rm(run.dir, { recursive: true, force: true });
  }

  return { kept, removed };
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function renderReadme(root, retainedRuns, removedRuns, retain) {
  const lines = [
    "# Workflow Mesh Regression Runs",
    "",
    `- Root: \`${path.relative(process.cwd(), root) || "."}\``,
    `- Retention window: newest \`${retain}\` run directories`,
    `- Retained runs: \`${retainedRuns.length}\``,
    `- Pruned this refresh: \`${removedRuns.length}\``,
    "",
  ];

  if (retainedRuns.length === 0) {
    lines.push("No retained workflow mesh runs were found.");
    lines.push("");
    return `${lines.join("\n").trimEnd()}\n`;
  }

  lines.push("| Slug | Status | Tests | Pass | Fail | Duration ms |");
  lines.push("| --- | --- | ---: | ---: | ---: | ---: |");
  for (const run of retainedRuns) {
    lines.push(
      `| \`${run.slug}\` | \`${run.status}\` | \`${run.total_tests}\` | \`${run.total_pass}\` | \`${run.total_fail}\` | \`${Number(run.total_duration_ms).toFixed(3)}\` |`,
    );
  }
  lines.push("");
  return `${lines.join("\n").trimEnd()}\n`;
}

function renderHtml(root, retainedRuns, removedRuns, retain) {
  const rootLabel = path.relative(process.cwd(), root) || ".";
  const cards =
    retainedRuns.length === 0
      ? `<article class="docs-card">
          <h2>No retained runs</h2>
          <p class="docs-copy">Run the workflow mesh regression wrapper to populate this index.</p>
        </article>`
      : retainedRuns
          .map(
            (run) => `<article class="docs-card">
        <div class="docs-kicker">workflow mesh run</div>
        <h2>${escapeHtml(run.slug)}</h2>
        <p class="docs-copy">Status <code>${escapeHtml(run.status)}</code> · total tests <code>${escapeHtml(run.total_tests)}</code> · duration ms <code>${escapeHtml(Number(run.total_duration_ms).toFixed(3))}</code></p>
        <div class="docs-meta">
          <span class="docs-chip">Pass: ${escapeHtml(run.total_pass)}</span>
          <span class="docs-chip">Fail: ${escapeHtml(run.total_fail)}</span>
          <span class="docs-chip">Generated at unix: ${escapeHtml(run.generated_at_unix_s)}</span>
        </div>
        <ul class="docs-list">
          <li><code>${escapeHtml(run.files.summary_json)}</code></li>
          <li><code>${escapeHtml(run.files.readme_md)}</code></li>
          <li><code>${escapeHtml(run.files.run_log)}</code></li>
        </ul>
      </article>`,
          )
          .join("\n");

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kyuubiki Workflow Mesh Regression Runs</title>
    <link rel="stylesheet" href="../../apps/hub-gui/ui/docs/docs.css" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">Workflow Mesh Index</div>
        <h1>Retained workflow mesh regression runs</h1>
        <p class="docs-copy">
          This page tracks the retained local and remote workflow mesh regression outputs using the same artifact layout.
        </p>
        <div class="docs-meta">
          <span class="docs-chip">Root: ${escapeHtml(rootLabel)}</span>
          <span class="docs-chip">Retention window: ${escapeHtml(retain)}</span>
          <span class="docs-chip">Retained runs: ${escapeHtml(retainedRuns.length)}</span>
          <span class="docs-chip">Pruned this refresh: ${escapeHtml(removedRuns.length)}</span>
        </div>
        <div class="docs-links">
          <a class="docs-link" href="./README.md">Open README</a>
          <a class="docs-link" href="./index.json">Open JSON index</a>
          <a class="docs-link" href="../../docs/testing-and-ci.md">Open testing guide</a>
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
  const discovered = await discoverRuns(options.root);
  const { kept, removed } = await pruneRuns(discovered, options.retain);
  const payload = {
    schema_version: "kyuubiki.workflow-mesh-regression-index/v1",
    root: path.relative(process.cwd(), options.root) || ".",
    generated_at_unix_s: Math.floor(Date.now() / 1000),
    retain: options.retain,
    retained_runs: kept,
    pruned_slugs: removed.map((run) => run.slug),
  };

  await writeFile(path.join(options.root, "index.json"), `${JSON.stringify(payload, null, 2)}\n`);
  await writeFile(path.join(options.root, "README.md"), renderReadme(options.root, kept, removed, options.retain));
  await writeFile(path.join(options.root, "index.html"), renderHtml(options.root, kept, removed, options.retain));

  process.stdout.write(`${path.relative(process.cwd(), options.root) || "."}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
