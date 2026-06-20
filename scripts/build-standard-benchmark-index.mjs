#!/usr/bin/env node

import { readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const DEFAULT_ROOT = path.resolve("tmp/standard-benchmark");
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

async function listRunDirectories(root) {
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
    const reportFiles = [
      `standard-10k-compare.md`,
      `standard-15k-compare.md`,
      `standard-20k-compare.md`,
      `standard-100k-compare.md`,
    ];

    let mergedReport = null;
    for (const file of reportFiles) {
      const candidate = path.join(runDir, file);
      try {
        await stat(candidate);
        mergedReport = candidate;
        break;
      } catch {
        continue;
      }
    }

    if (!mergedReport) {
      continue;
    }

    const reportStat = await stat(mergedReport);
    const profile = path
      .basename(mergedReport)
      .replace(/^standard-/, "")
      .replace(/-compare\.md$/, "");
    const matrixReports = [
      `mechanical-core-${profile}-compare.md`,
      `thermal-core-${profile}-compare.md`,
      `compound-core-${profile}-compare.md`,
    ];

    runs.push({
      slug: entry.name,
      dir: runDir,
      profile,
      merged_report: path.relative(root, mergedReport),
      matrix_reports: matrixReports,
      generated_at_unix_s: Math.floor(reportStat.mtimeMs / 1000),
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

async function readHeadline(reportPath) {
  try {
    const content = await readFile(reportPath, "utf8");
    const lines = content.split("\n");
    const firstTableRow = lines.find(
      (line) => line.startsWith("| `") && !line.includes("| --- |"),
    );
    return firstTableRow ?? "";
  } catch {
    return "";
  }
}

function renderReadme(root, retainedRuns, removedRuns, retain) {
  const lines = [
    "# Standard Benchmark Runs",
    "",
    `- Root: \`${path.relative(process.cwd(), root) || "."}\``,
    `- Retention window: newest \`${retain}\` run directories`,
    `- Retained runs: \`${retainedRuns.length}\``,
    `- Pruned this refresh: \`${removedRuns.length}\``,
    "",
  ];

  if (retainedRuns.length === 0) {
    lines.push("No retained standard benchmark runs were found.");
    lines.push("");
    return `${lines.join("\n").trimEnd()}\n`;
  }

  lines.push("| Slug | Profile | Generated (unix) | Merged report |");
  lines.push("| --- | --- | ---: | --- |");
  for (const run of retainedRuns) {
    lines.push(
      `| \`${run.slug}\` | \`${run.profile}\` | \`${run.generated_at_unix_s}\` | \`${run.merged_report}\` |`,
    );
  }
  lines.push("");

  return `${lines.join("\n").trimEnd()}\n`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function renderHtml(root, retainedRuns, removedRuns, retain) {
  const rootLabel = path.relative(process.cwd(), root) || ".";
  const runCards =
    retainedRuns.length === 0
      ? `<article class="docs-card">
          <h2>No retained runs</h2>
          <p class="docs-copy">Run the standard benchmark regression wrapper to populate this index.</p>
        </article>`
      : retainedRuns
          .map(
            (run) => `<article class="docs-card">
          <div class="docs-kicker">standard run</div>
          <h2>${escapeHtml(run.slug)}</h2>
          <p class="docs-copy">Profile <code>${escapeHtml(run.profile)}</code> · generated at unix <code>${escapeHtml(run.generated_at_unix_s)}</code></p>
          <div class="docs-meta">
            <span class="docs-chip">Merged report: <code>${escapeHtml(run.merged_report)}</code></span>
          </div>
          ${
            run.headline
              ? `<p class="docs-copy"><strong>First reported row:</strong> <code>${escapeHtml(run.headline)}</code></p>`
              : ""
          }
          <h3>Per-matrix reports</h3>
          <ul class="docs-list">
            ${run.matrix_reports
              .map((report) => `<li><code>${escapeHtml(report)}</code></li>`)
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
    <title>Kyuubiki Standard Benchmark Runs</title>
    <link rel="stylesheet" href="../../apps/hub-gui/ui/docs/docs.css" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">Standard Benchmark Index</div>
        <h1>Retained nightly benchmark runs</h1>
        <p class="docs-copy">
          This page is generated from the local standard benchmark artifact directory and mirrors the same retained run window used by the remote regression wrapper.
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
        ${runCards}
      </section>
    </main>
  </body>
</html>
`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const discovered = await listRunDirectories(options.root);
  const { kept, removed } = await pruneRuns(discovered, options.retain);
  const runs = [];

  for (const run of kept) {
    const mergedReportPath = path.join(options.root, run.merged_report);
    runs.push({
      ...run,
      headline: await readHeadline(mergedReportPath),
    });
  }

  const indexPayload = {
    schema_version: "kyuubiki.standard-benchmark-index/v1",
    root: path.relative(process.cwd(), options.root) || ".",
    generated_at_unix_s: Math.floor(Date.now() / 1000),
    retain: options.retain,
    retained_runs: runs,
    pruned_slugs: removed.map((run) => run.slug),
  };

  await writeFile(
    path.join(options.root, "index.json"),
    `${JSON.stringify(indexPayload, null, 2)}\n`,
  );
  await writeFile(
    path.join(options.root, "README.md"),
    renderReadme(options.root, runs, removed, options.retain),
  );
  await writeFile(
    path.join(options.root, "index.html"),
    renderHtml(options.root, runs, removed, options.retain),
  );

  process.stdout.write(`${path.relative(process.cwd(), options.root) || "."}\n`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
