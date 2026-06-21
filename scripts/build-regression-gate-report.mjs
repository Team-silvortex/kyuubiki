#!/usr/bin/env node

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const DEFAULT_TMP_ROOT = path.resolve("tmp");

function usage() {
  console.log(`Usage:
  node ./scripts/build-regression-gate-report.mjs [options]

Options:
  --tmp-root <path>       Tmp root containing regression-lane-catalog.json.
  --catalog <path>        Optional explicit regression lane catalog path.
  --fail-on-warn          Exit non-zero when overall gate is warn.
  --help                  Show this message.
`);
}

function parseArgs(argv) {
  const options = {
    tmpRoot: DEFAULT_TMP_ROOT,
    catalogPath: "",
    failOnWarn: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];

    if (arg === "--tmp-root" && next) {
      options.tmpRoot = path.resolve(next);
      index += 1;
      continue;
    }

    if (arg === "--catalog" && next) {
      options.catalogPath = path.resolve(next);
      index += 1;
      continue;
    }

    if (arg === "--fail-on-warn") {
      options.failOnWarn = true;
      continue;
    }

    if (arg === "--help") {
      usage();
      process.exit(0);
    }

    throw new Error(`unknown option: ${arg}`);
  }

  return options;
}

function renderMarkdown(report) {
  const lines = [
    "# Regression Gate Report",
    "",
    `- Catalog: \`${report.catalog_path}\``,
    `- Overall gate status: \`${report.overall_gate_status}\``,
    `- Generated at unix: \`${report.generated_at_unix_s}\``,
    `- Failing lane count: \`${report.failing_lane_count}\``,
    `- Warning lane count: \`${report.warning_lane_count}\``,
    "",
    "| Lane | Gate | Status | Reason count |",
    "| --- | --- | --- | ---: |",
  ];

  for (const lane of report.lanes) {
    lines.push(
      `| \`${lane.id}\` | \`${lane.gate_status}\` | \`${lane.status}\` | \`${lane.gate_reasons.length}\` |`,
    );
  }

  const lanesWithReasons = report.lanes.filter((lane) => lane.gate_reasons.length > 0);
  if (lanesWithReasons.length > 0) {
    lines.push("");
    lines.push("## Reasons");
    lines.push("");

    for (const lane of lanesWithReasons) {
      lines.push(`- \`${lane.id}\``);
      for (const reason of lane.gate_reasons) {
        lines.push(`  ${reason}`);
      }
    }
  }

  lines.push("");
  return `${lines.join("\n").trimEnd()}\n`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const catalogPath =
    options.catalogPath || path.join(options.tmpRoot, "regression-lane-catalog.json");
  const catalog = JSON.parse(await readFile(catalogPath, "utf8"));
  const lanes = Array.isArray(catalog.lanes) ? catalog.lanes : [];

  const normalizedLanes = lanes.map((lane) => ({
    id: lane.id,
    title: lane.title,
    category: lane.category,
    status: lane.status ?? "unknown",
    gate_status: lane.gate?.status ?? lane.status ?? "unknown",
    gate_reasons: Array.isArray(lane.gate?.reasons) ? lane.gate.reasons : [],
    generated_at_unix_s: lane.generated_at_unix_s ?? 0,
    links: Array.isArray(lane.links) ? lane.links : [],
  }));

  const failingLaneCount = normalizedLanes.filter((lane) => lane.gate_status === "fail").length;
  const warningLaneCount = normalizedLanes.filter((lane) => lane.gate_status === "warn").length;

  const report = {
    schema_version: "kyuubiki.regression-gate-report/v1",
    generated_at_unix_s: Math.floor(Date.now() / 1000),
    catalog_path: path.relative(process.cwd(), catalogPath) || path.basename(catalogPath),
    overall_gate_status: catalog.overall_gate_status ?? "unknown",
    failing_lane_count: failingLaneCount,
    warning_lane_count: warningLaneCount,
    lanes: normalizedLanes,
  };

  await writeFile(
    path.join(options.tmpRoot, "regression-gate-report.json"),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  await writeFile(
    path.join(options.tmpRoot, "regression-gate-report.md"),
    renderMarkdown(report),
  );

  process.stdout.write(`${report.overall_gate_status}\n`);

  if (report.overall_gate_status === "fail") {
    process.exit(2);
  }

  if (options.failOnWarn && report.overall_gate_status === "warn") {
    process.exit(1);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
