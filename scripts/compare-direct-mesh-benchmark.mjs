#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

function usage() {
  console.log(`Usage:
  node ./scripts/compare-direct-mesh-benchmark.mjs [options]

Options:
  --current <path>                    Current benchmark summary JSON.
  --baseline <path>                   Baseline benchmark JSON.
  --json-out <path>                   Optional compare JSON output path.
  --report-out <path>                 Optional Markdown report output path.
  --fail-on-elapsed-regression-pct <n>
                                      Fail when elapsed mean regresses by more than n percent.
  --fail-on-rss-regression-pct <n>    Fail when RSS mean regresses by more than n percent.
  --help                              Show this message.
`);
}

function parseArgs(argv) {
  const args = {
    current: "",
    baseline: "",
    jsonOut: "",
    reportOut: "",
    failOnElapsedRegressionPct: Number.POSITIVE_INFINITY,
    failOnRssRegressionPct: Number.POSITIVE_INFINITY,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    const next = argv[index + 1];
    switch (token) {
      case "--current":
        args.current = next ?? "";
        index += 1;
        break;
      case "--baseline":
        args.baseline = next ?? "";
        index += 1;
        break;
      case "--json-out":
        args.jsonOut = next ?? "";
        index += 1;
        break;
      case "--report-out":
        args.reportOut = next ?? "";
        index += 1;
        break;
      case "--fail-on-elapsed-regression-pct":
        args.failOnElapsedRegressionPct = Number(next);
        index += 1;
        break;
      case "--fail-on-rss-regression-pct":
        args.failOnRssRegressionPct = Number(next);
        index += 1;
        break;
      case "--help":
        usage();
        process.exit(0);
      default:
        throw new Error(`unknown option: ${token}`);
    }
  }

  if (!args.current || !args.baseline) {
    throw new Error("--current and --baseline are required");
  }

  return args;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function ensureParent(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function round(value, digits = 3) {
  return Number(value.toFixed(digits));
}

function safePercentDelta(current, baseline) {
  if (baseline === 0) {
    return current === 0 ? 0 : null;
  }
  return ((current - baseline) / baseline) * 100;
}

function mean(values) {
  if (values.length === 0) {
    return 0;
  }
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function indexSubtestMeans(summary) {
  const map = new Map();
  for (const run of summary.runs ?? []) {
    for (const subtest of run.subtests ?? []) {
      const durations = map.get(subtest.name) ?? [];
      durations.push(Number(subtest.duration_ms));
      map.set(subtest.name, durations);
    }
  }

  return new Map(
    [...map.entries()].map(([name, durations]) => [
      name,
      {
        name,
        mean_duration_ms: mean(durations),
        min_duration_ms: Math.min(...durations),
        max_duration_ms: Math.max(...durations),
      },
    ]),
  );
}

function compareMetric(name, unit, baseline, current) {
  const delta = current - baseline;
  const deltaPct = safePercentDelta(current, baseline);
  return {
    name,
    unit,
    baseline: round(baseline),
    current: round(current),
    delta: round(delta),
    delta_pct: deltaPct == null ? null : round(deltaPct),
    direction: delta > 0 ? "up" : delta < 0 ? "down" : "flat",
  };
}

function buildComparison(baselineSummary, currentSummary, options) {
  const baselineSubtests = indexSubtestMeans(baselineSummary);
  const currentSubtests = indexSubtestMeans(currentSummary);
  const subtests = [];

  for (const [name, baselineStats] of baselineSubtests.entries()) {
    const currentStats = currentSubtests.get(name);
    if (!currentStats) {
      continue;
    }
    subtests.push(
      compareMetric(
        name,
        "ms",
        baselineStats.mean_duration_ms,
        currentStats.mean_duration_ms,
      ),
    );
  }

  const metrics = [
    compareMetric(
      "elapsed_mean",
      "s",
      Number(baselineSummary.aggregate.elapsed_s.mean),
      Number(currentSummary.aggregate.elapsed_s.mean),
    ),
    compareMetric(
      "rss_mean",
      "KiB",
      Number(baselineSummary.aggregate.max_rss_kib.mean),
      Number(currentSummary.aggregate.max_rss_kib.mean),
    ),
    compareMetric(
      "user_cpu_mean",
      "s",
      Number(baselineSummary.aggregate.user_s.mean),
      Number(currentSummary.aggregate.user_s.mean),
    ),
    compareMetric(
      "sys_cpu_mean",
      "s",
      Number(baselineSummary.aggregate.sys_s.mean),
      Number(currentSummary.aggregate.sys_s.mean),
    ),
  ];

  const elapsed = metrics.find((metric) => metric.name === "elapsed_mean");
  const rss = metrics.find((metric) => metric.name === "rss_mean");
  const failures = [];

  if (
    elapsed?.delta_pct != null &&
    elapsed.delta_pct > options.failOnElapsedRegressionPct
  ) {
    failures.push(
      `elapsed mean regression ${elapsed.delta_pct}% exceeded ${options.failOnElapsedRegressionPct}%`,
    );
  }

  if (rss?.delta_pct != null && rss.delta_pct > options.failOnRssRegressionPct) {
    failures.push(
      `rss mean regression ${rss.delta_pct}% exceeded ${options.failOnRssRegressionPct}%`,
    );
  }

  return {
    generated_at: new Date().toISOString(),
    baseline_path: options.baseline,
    current_path: options.current,
    baseline_id: baselineSummary.id ?? "direct-mesh-docker-baseline",
    current_repeat: currentSummary.repeat,
    baseline_repeat: baselineSummary.repeat,
    metrics,
    subtests,
    failures,
    ok: failures.length === 0,
  };
}

function markdownTableRows(metrics) {
  return metrics
    .map((metric) => {
      const deltaPct = metric.delta_pct == null ? "n/a" : `${metric.delta_pct}%`;
      return `| ${metric.name} | ${metric.baseline} ${metric.unit} | ${metric.current} ${metric.unit} | ${metric.delta} ${metric.unit} | ${deltaPct} |`;
    })
    .join("\n");
}

function renderReport(comparison) {
  const failureSection =
    comparison.failures.length === 0
      ? "- Status: pass\n"
      : comparison.failures.map((failure) => `- ${failure}`).join("\n") + "\n";

  const subtestRows = comparison.subtests
    .map((metric) => {
      const deltaPct = metric.delta_pct == null ? "n/a" : `${metric.delta_pct}%`;
      return `| ${metric.name} | ${metric.baseline} ms | ${metric.current} ms | ${metric.delta} ms | ${deltaPct} |`;
    })
    .join("\n");

  return `# Direct-mesh Docker benchmark comparison

- Baseline: \`${comparison.baseline_path}\`
- Current: \`${comparison.current_path}\`
- Baseline repeat: \`${comparison.baseline_repeat}\`
- Current repeat: \`${comparison.current_repeat}\`

## Status

${failureSection}
## Aggregate metrics

| Metric | Baseline | Current | Delta | Delta % |
| --- | ---: | ---: | ---: | ---: |
${markdownTableRows(comparison.metrics)}

## Subtest means

| Subtest | Baseline | Current | Delta | Delta % |
| --- | ---: | ---: | ---: | ---: |
${subtestRows}
`;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const baselineSummary = readJson(options.baseline);
  const currentSummary = readJson(options.current);
  const comparison = buildComparison(baselineSummary, currentSummary, options);

  if (options.jsonOut) {
    ensureParent(options.jsonOut);
    fs.writeFileSync(options.jsonOut, `${JSON.stringify(comparison, null, 2)}\n`);
  }

  if (options.reportOut) {
    ensureParent(options.reportOut);
    fs.writeFileSync(options.reportOut, renderReport(comparison));
  }

  if (!options.jsonOut && !options.reportOut) {
    process.stdout.write(`${JSON.stringify(comparison, null, 2)}\n`);
  } else {
    process.stdout.write(`${comparison.ok ? "comparison ok" : "comparison failed"}\n`);
  }

  if (!comparison.ok) {
    process.exit(1);
  }
}

main();
