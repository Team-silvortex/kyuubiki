import { readFile, stat } from "node:fs/promises";
import path from "node:path";

async function exists(filePath) {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

function makeGate(status, reasons) {
  return { status, reasons };
}

function round(value, digits = 3) {
  return Number(Number(value).toFixed(digits));
}

export async function readBenchmarkProfileLane(tmpRoot) {
  const indexPath = path.join(tmpRoot, "benchmark-profile", "index.json");
  if (!(await exists(indexPath))) {
    return null;
  }

  const indexPayload = await readJson(indexPath);
  const latest = Array.isArray(indexPayload.retained_runs)
    ? indexPayload.retained_runs[0]
    : null;
  const matrixSummary = Array.isArray(indexPayload.matrix_summaries)
    ? indexPayload.matrix_summaries[0]
    : null;
  const coverageSummary = Array.isArray(indexPayload.coverage_summaries)
    ? indexPayload.coverage_summaries[0]
    : null;
  const gate = indexPayload.gate ?? makeGate("warn", ["missing profile index gate"]);

  return {
    id: "benchmark-profile",
    title: "Benchmark profile exploration",
    category: "evidence",
    gate_scope: "advisory",
    generated_at_unix_s: latest?.generated_at_unix_s ?? indexPayload.generated_at_unix_s ?? 0,
    status: gate.status === "pass" ? "observed" : "needs-review",
    gate,
    summary: summaryText(latest, matrixSummary, coverageSummary),
    metrics: profileMetrics(indexPayload, latest, matrixSummary, coverageSummary),
    links: profileLinks(latest),
  };
}

function summaryText(latest, matrixSummary, coverageSummary) {
  const coverage = coverageSummary
    ? ` Coverage ${coverageSummary.covered_case_count}/${coverageSummary.expected_case_count}.`
    : "";
  if (matrixSummary) {
    return `${matrixSummary.run_count ?? 0} run(s), ${matrixSummary.case_count ?? 0} case(s), leading matrix \`${matrixSummary.matrix ?? "unknown"}\`.${coverage}`;
  }
  if (latest) {
    return `${latest.case_count ?? 0} case(s), matrix \`${latest.matrix ?? "unknown"}\`, profile \`${latest.profile ?? "unknown"}\`.${coverage}`;
  }
  return "No retained exploratory benchmark profile runs.";
}

function profileMetrics(indexPayload, latest, matrixSummary, coverageSummary) {
  return [
    {
      name: "retained_runs",
      unit: "run",
      value: indexPayload.retained_runs?.length ?? 0,
    },
    {
      name: "skipped_runs",
      unit: "run",
      value: indexPayload.skipped_runs?.length ?? 0,
    },
    {
      name: "leading_matrix_total_median",
      unit: "ms",
      value: round(matrixSummary?.total_median_ms ?? latest?.total_median_ms ?? 0),
    },
    {
      name: "leading_matrix_peak_rss",
      unit: "MiB",
      value: round(matrixSummary?.peak_rss_mib ?? latest?.peak_rss_mib ?? 0),
    },
    {
      name: "coverage",
      unit: "case",
      value: `${coverageSummary?.covered_case_count ?? 0}/${coverageSummary?.expected_case_count ?? 0}`,
    },
  ];
}

function profileLinks(latest) {
  if (!latest) {
    return ["benchmark-profile/index.json", "benchmark-profile/README.md"];
  }
  return [
    "benchmark-profile/index.json",
    "benchmark-profile/README.md",
    `benchmark-profile/${latest.files?.summary_json ?? `${latest.slug}/summary.json`}`,
    `benchmark-profile/${latest.files?.readme_md ?? `${latest.slug}/README.md`}`,
  ];
}
