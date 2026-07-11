#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const DEFAULT_TOPOLOGY = "config/architecture/module-topology.json";
const DEFAULT_OUT_DIR = "tmp/module-topology";

function parseArgs(argv) {
  const options = {
    topology: path.join(ROOT, DEFAULT_TOPOLOGY),
    outDir: path.join(ROOT, DEFAULT_OUT_DIR),
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--topology" && next) {
      options.topology = path.resolve(next);
      index += 1;
    } else if (arg === "--out-dir" && next) {
      options.outDir = path.resolve(next);
      index += 1;
    }
  }

  return options;
}

function readTopology(topologyPath) {
  const topology = JSON.parse(readFileSync(topologyPath, "utf8"));
  if (topology.schema_version !== "kyuubiki.module-topology/v1") {
    throw new Error(`unsupported module topology schema: ${topology.schema_version}`);
  }
  return topology;
}

function laneIndex(topology, field, descriptions) {
  return Object.fromEntries(
    Object.entries(descriptions).map(([laneId, description]) => {
      const modules = topology.modules
        .filter((module) => module[field].includes(laneId))
        .map((module) => ({
          id: module.id,
          layer: module.layer,
          owned_paths: module.owned_paths,
          risk_tags: module.risk_tags,
        }));
      return [laneId, { description, modules }];
    }),
  );
}

function buildReport(topology) {
  const modules = topology.modules.map((module) => ({
    id: module.id,
    layer: module.layer,
    summary: module.summary,
    owned_paths: module.owned_paths,
    depends_on: module.depends_on,
    benchmark_lanes: module.benchmark_lanes,
    security_lanes: module.security_lanes,
    risk_tags: module.risk_tags,
  }));

  return {
    schema_version: "kyuubiki.module-topology-report/v1",
    generated_from: DEFAULT_TOPOLOGY,
    version_line: topology.version_line,
    module_count: modules.length,
    benchmark_lanes: laneIndex(topology, "benchmark_lanes", topology.benchmark_lanes),
    security_lanes: laneIndex(topology, "security_lanes", topology.security_lanes),
    modules,
  };
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function renderLaneMarkdown(title, lanes) {
  const lines = [`## ${title}`, ""];
  for (const [laneId, lane] of Object.entries(lanes)) {
    lines.push(`### \`${laneId}\``);
    lines.push("");
    lines.push(lane.description);
    lines.push("");
    lines.push("| Module | Layer | Risks |");
    lines.push("| --- | --- | --- |");
    for (const module of lane.modules) {
      lines.push(
        `| \`${module.id}\` | \`${module.layer}\` | ${module.risk_tags.map((tag) => `\`${tag}\``).join(", ")} |`,
      );
    }
    lines.push("");
  }
  return lines;
}

function renderMarkdown(report) {
  const lines = [
    "# Module Topology Report",
    "",
    `- Schema: \`${report.schema_version}\``,
    `- Source: \`${report.generated_from}\``,
    `- Version line: \`${report.version_line}\``,
    `- Modules: \`${report.module_count}\``,
    "",
    "## Modules",
    "",
    "| Module | Layer | Depends on | Benchmark lanes | Security lanes |",
    "| --- | --- | --- | --- | --- |",
  ];

  for (const module of report.modules) {
    lines.push(
      `| \`${module.id}\` | \`${module.layer}\` | ${module.depends_on.map((id) => `\`${id}\``).join(", ") || "-"} | ${module.benchmark_lanes.map((id) => `\`${id}\``).join(", ")} | ${module.security_lanes.map((id) => `\`${id}\``).join(", ")} |`,
    );
  }
  lines.push("");
  lines.push(...renderLaneMarkdown("Benchmark Lanes", report.benchmark_lanes));
  lines.push(...renderLaneMarkdown("Security Lanes", report.security_lanes));
  return `${lines.join("\n").trimEnd()}\n`;
}

function renderHtml(report, markdown) {
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Kyuubiki Module Topology</title>
  <style>
    body { margin: 0; background: #11161d; color: #e7edf6; font: 15px/1.6 ui-sans-serif, system-ui; }
    main { max-width: 1120px; margin: 0 auto; padding: 48px 24px; }
    pre { white-space: pre-wrap; background: #18212c; border: 1px solid #2d3b4c; border-radius: 16px; padding: 24px; }
    code { color: #9bdcff; }
  </style>
</head>
<body>
  <main>
    <h1>Kyuubiki Module Topology</h1>
    <p>Generated from <code>${escapeHtml(report.generated_from)}</code>.</p>
    <pre>${escapeHtml(markdown)}</pre>
  </main>
</body>
</html>
`;
}

function writeReport(options) {
  const topology = readTopology(options.topology);
  const report = buildReport(topology);
  const markdown = renderMarkdown(report);
  mkdirSync(options.outDir, { recursive: true });
  writeFileSync(path.join(options.outDir, "index.json"), `${JSON.stringify(report, null, 2)}\n`);
  writeFileSync(path.join(options.outDir, "README.md"), markdown);
  writeFileSync(path.join(options.outDir, "index.html"), renderHtml(report, markdown));
  return report;
}

try {
  const options = parseArgs(process.argv.slice(2));
  const report = writeReport(options);
  console.log(
    `module topology report written: ${path.relative(ROOT, options.outDir)} (${report.module_count} modules)`,
  );
} catch (error) {
  console.error(`module topology report failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
}
