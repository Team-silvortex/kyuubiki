import type { CSSProperties } from "react";
import type { Metadata } from "next";
import { WORKFLOW_SUMMARY_ARTIFACT_CONTRACT } from "@/components/workbench/workflow/workbench-workflow-summary-contract";

export const metadata: Metadata = {
  title: "Workflow Summary Contracts | tamamono 1.6.0",
  description: "Summary artifact contract reference for cross-operator workflow extracts and transforms.",
};

function pretty(value: unknown) {
  return JSON.stringify(value, null, 2);
}

const EXAMPLE = {
  contract_version: `${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.schema}@${WORKFLOW_SUMMARY_ARTIFACT_CONTRACT.version}`,
  summary_kind: "field_statistics",
  source_operator_id: "extract.field_statistics",
  source_artifact_type: "result/thermal_plane_triangle_2d",
  field_namespace: "thermo",
  fields: {
    max_temperature: 412.5,
    p90_temperature: 398.2,
    mean_temperature: 356.8,
    hotspot_count: 4,
  },
  metadata: {
    source: "elements",
    field: "temperature",
    percentile: 90,
    sample_limit: 4,
  },
};

export default function WorkflowSummaryContractsPage() {
  return (
    <main style={mainStyle}>
      <div style={{ maxWidth: 980, margin: "0 auto" }}>
        <p style={eyebrowStyle}>tamamono 1.6.0</p>
        <h1 style={{ fontSize: "clamp(2rem, 4vw, 3.4rem)", margin: "0 0 12px" }}>
          Workflow Summary Contracts
        </h1>
        <p style={copyStyle}>
          This page documents the shared result-summary artifact contract used by extract and
          transform operators so benchmark, merge, compare, export, and headless SDK flows can
          agree on one payload shape.
        </p>

        <section style={{ marginTop: 32 }}>
          <h2>Contract Shape</h2>
          <pre style={preStyle}>{pretty(EXAMPLE)}</pre>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Rules</h2>
          <ul style={listStyle}>
            <li>`contract_version` pins the shared summary-artifact family.</li>
            <li>`summary_kind` distinguishes summary, statistics, hotspot, compare, and aggregate outputs.</li>
            <li>`fields` is the cross-operator metric map that downstream transforms consume.</li>
            <li>`field_namespace` lets multi-stage chains keep source metrics distinct before merge or normalize.</li>
            <li>`metadata` carries operator-specific hints without breaking the shared contract core.</li>
          </ul>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Schema</h2>
          <pre style={preStyle}>{pretty(WORKFLOW_SUMMARY_ARTIFACT_CONTRACT)}</pre>
        </section>
      </div>
    </main>
  );
}

const mainStyle: CSSProperties = {
  minHeight: "100vh",
  background:
    "radial-gradient(circle at top, rgba(103, 132, 166, 0.18), transparent 36%), #10151b",
  color: "#d9e3f0",
  padding: "48px 20px 80px",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
};

const eyebrowStyle: CSSProperties = {
  color: "#7fb3ff",
  letterSpacing: "0.12em",
  textTransform: "uppercase",
};

const preStyle: CSSProperties = {
  background: "rgba(9, 13, 18, 0.92)",
  border: "1px solid rgba(127, 179, 255, 0.18)",
  borderRadius: 16,
  padding: 20,
  overflowX: "auto",
  lineHeight: 1.6,
  fontSize: 13,
};

const copyStyle: CSSProperties = {
  color: "#9eb1c8",
  lineHeight: 1.7,
  maxWidth: 760,
};

const listStyle: CSSProperties = {
  color: "#b9c7d8",
  lineHeight: 1.8,
  paddingLeft: 20,
};
