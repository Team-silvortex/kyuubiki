#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

function usage() {
  console.log(`Usage:
  node ./scripts/compare-workflow-catalog-benchmark.mjs [options]

Options:
  --current <path>                    Current workflow catalog benchmark JSON.
  --baseline <path>                   Baseline workflow catalog benchmark JSON.
  --json-out <path>                   Optional compare JSON output path.
  --report-out <path>                 Optional Markdown report output path.
  --fail-on-median-regression-pct <n> Fail when any case median regresses by more than n percent.
  --fail-on-avg-regression-pct <n>    Fail when any case average regresses by more than n percent.
  --help                              Show this message.
`);
}

function parseArgs(argv) {
  const args = {
    current: "",
    baseline: "",
    jsonOut: "",
    reportOut: "",
    failOnMedianRegressionPct: Number.POSITIVE_INFINITY,
    failOnAvgRegressionPct: Number.POSITIVE_INFINITY,
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
      case "--fail-on-median-regression-pct":
        args.failOnMedianRegressionPct = Number(next);
        index += 1;
        break;
      case "--fail-on-avg-regression-pct":
        args.failOnAvgRegressionPct = Number(next);
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

function normalizeSummary(summary) {
  const cases = Array.isArray(summary.cases) ? summary.cases : [];
  return {
    id: summary.id ?? "workflow-catalog-benchmark",
    generated_at: summary.generated_at ?? null,
    repeat: summary.repeat ?? summary.source?.repeat ?? null,
    case_count: cases.length,
    cases,
  };
}

function indexCases(summary) {
  return new Map(summary.cases.map((entry) => [entry.case_id, entry]));
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

function compareCase(baselineCase, currentCase) {
  const baselineSummary = baselineCase.summary ?? {};
  const currentSummary = currentCase.summary ?? {};
  const metrics = [
    compareMetric(
      "median_elapsed_ms",
      "ms",
      Number(baselineSummary.median_elapsed_ms),
      Number(currentSummary.median_elapsed_ms),
    ),
    compareMetric(
      "avg_elapsed_ms",
      "ms",
      Number(baselineSummary.avg_elapsed_ms),
      Number(currentSummary.avg_elapsed_ms),
    ),
    compareMetric(
      "min_elapsed_ms",
      "ms",
      Number(baselineSummary.min_elapsed_ms),
      Number(currentSummary.min_elapsed_ms),
    ),
    compareMetric(
      "max_elapsed_ms",
      "ms",
      Number(baselineSummary.max_elapsed_ms),
      Number(currentSummary.max_elapsed_ms),
    ),
  ];

  const baselineCompletedRange = JSON.stringify(
    baselineSummary.completed_node_count_range ?? [],
  );
  const currentCompletedRange = JSON.stringify(
    currentSummary.completed_node_count_range ?? [],
  );

  return {
    case_id: baselineCase.case_id,
    workflow_id: currentCase.workflow_id ?? baselineCase.workflow_id,
    baseline_repeat: baselineCase.repeat ?? null,
    current_repeat: currentCase.repeat ?? null,
    metrics,
    completed_node_count_range_match:
      baselineCompletedRange === currentCompletedRange,
    baseline_completed_node_count_range:
      baselineSummary.completed_node_count_range ?? [],
    current_completed_node_count_range:
      currentSummary.completed_node_count_range ?? [],
  };
}

function buildComparison(baselineSummary, currentSummary, options) {
  const normalizedBaseline = normalizeSummary(baselineSummary);
  const normalizedCurrent = normalizeSummary(currentSummary);
  const baselineCases = indexCases(normalizedBaseline);
  const currentCases = indexCases(normalizedCurrent);

  const baselineCaseIds = [...baselineCases.keys()].sort();
  const currentCaseIds = [...currentCases.keys()].sort();
  const missingCases = baselineCaseIds.filter((caseId) => !currentCases.has(caseId));
  const addedCases = currentCaseIds.filter((caseId) => !baselineCases.has(caseId));
  const sharedCaseIds = baselineCaseIds.filter((caseId) => currentCases.has(caseId));
  const cases = sharedCaseIds.map((caseId) =>
    compareCase(baselineCases.get(caseId), currentCases.get(caseId)),
  );

  const failures = [];

  if (missingCases.length > 0) {
    failures.push(`missing benchmark cases: ${missingCases.join(", ")}`);
  }

  for (const entry of cases) {
    if (!entry.completed_node_count_range_match) {
      failures.push(
        `${entry.case_id} completed node count range drifted (${entry.baseline_completed_node_count_range.join("..")} vs ${entry.current_completed_node_count_range.join("..")})`,
      );
    }

    const medianMetric = entry.metrics.find((metric) => metric.name === "median_elapsed_ms");
    const avgMetric = entry.metrics.find((metric) => metric.name === "avg_elapsed_ms");

    if (
      medianMetric?.delta_pct != null &&
      medianMetric.delta_pct > options.failOnMedianRegressionPct
    ) {
      failures.push(
        `${entry.case_id} median regression ${medianMetric.delta_pct}% exceeded ${options.failOnMedianRegressionPct}%`,
      );
    }

    if (
      avgMetric?.delta_pct != null &&
      avgMetric.delta_pct > options.failOnAvgRegressionPct
    ) {
      failures.push(
        `${entry.case_id} average regression ${avgMetric.delta_pct}% exceeded ${options.failOnAvgRegressionPct}%`,
      );
    }
  }

  return {
    generated_at: new Date().toISOString(),
    baseline_path: options.baseline,
    current_path: options.current,
    baseline_id: normalizedBaseline.id,
    baseline_generated_at: normalizedBaseline.generated_at,
    current_generated_at: normalizedCurrent.generated_at,
    baseline_repeat: normalizedBaseline.repeat,
    current_repeat: normalizedCurrent.repeat,
    baseline_case_count: normalizedBaseline.case_count,
    current_case_count: normalizedCurrent.case_count,
    missing_cases: missingCases,
    added_cases: addedCases,
    cases,
    failures,
    ok: failures.length === 0,
  };
}

function metricRows(metrics) {
  return metrics
    .map((metric) => {
      const deltaPct = metric.delta_pct == null ? "n/a" : `${metric.delta_pct}%`;
      return `| ${metric.name} | ${metric.baseline} ${metric.unit} | ${metric.current} ${metric.unit} | ${metric.delta} ${metric.unit} | ${deltaPct} |`;
    })
    .join("\n");
}

function renderCaseSection(entry) {
  const completedRangeStatus = entry.completed_node_count_range_match
    ? "match"
    : `drift (${entry.baseline_completed_node_count_range.join("..")} vs ${entry.current_completed_node_count_range.join("..")})`;

  return `### ${entry.case_id}

- Workflow: \`${entry.workflow_id}\`
- Completed node range: ${completedRangeStatus}
- Baseline repeat: \`${entry.baseline_repeat}\`
- Current repeat: \`${entry.current_repeat}\`

| Metric | Baseline | Current | Delta | Delta % |
| --- | ---: | ---: | ---: | ---: |
${metricRows(entry.metrics)}
`;
}

function renderReport(comparison) {
  const statusSection =
    comparison.failures.length === 0
      ? "- Status: pass\n"
      : comparison.failures.map((failure) => `- ${failure}`).join("\n") + "\n";

  const missingSection =
    comparison.missing_cases.length === 0
      ? "- Missing baseline cases: none"
      : `- Missing baseline cases: ${comparison.missing_cases.join(", ")}`;

  const addedSection =
    comparison.added_cases.length === 0
      ? "- Added current cases: none"
      : `- Added current cases: ${comparison.added_cases.join(", ")}`;

  return `# Workflow catalog benchmark comparison

- Baseline: \`${comparison.baseline_path}\`
- Current: \`${comparison.current_path}\`
- Baseline repeat: \`${comparison.baseline_repeat}\`
- Current repeat: \`${comparison.current_repeat}\`
- Baseline cases: \`${comparison.baseline_case_count}\`
- Current cases: \`${comparison.current_case_count}\`

## Status

${statusSection}
## Coverage

${missingSection}
${addedSection}

## Case comparisons

${comparison.cases.map(renderCaseSection).join("\n")}
`;
}

function main() {
  try {
    const options = parseArgs(process.argv.slice(2));
    const baselineSummary = readJson(options.baseline);
    const currentSummary = readJson(options.current);
    const comparison = buildComparison(baselineSummary, currentSummary, options);

    if (options.jsonOut) {
      ensureParent(options.jsonOut);
      fs.writeFileSync(options.jsonOut, JSON.stringify(comparison, null, 2));
    }

    if (options.reportOut) {
      ensureParent(options.reportOut);
      fs.writeFileSync(options.reportOut, renderReport(comparison));
    }

    process.stdout.write(`${JSON.stringify(comparison, null, 2)}\n`);

    if (!comparison.ok) {
      process.exitCode = 1;
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}

main();
