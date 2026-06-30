import Link from "next/link";
import type { CSSProperties } from "react";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Workflow Architecture | tamamono 1.13.0",
  description: "Architecture overview for the tamamono workflow operator system.",
};

const LAYERS = [
  {
    title: "1. Operator Layer",
    body:
      "Operators are described by stable descriptors with ports, schema refs, config examples, validation state, capability tags, and runtime origin. This layer defines what one operator is and what it can legally consume or produce.",
    references: [
      "apps/frontend/src/lib/api/workflow-types.ts",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-node-templates.ts",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-runtime-support.ts",
    ],
  },
  {
    title: "2. Workflow Graph Layer",
    body:
      "A workflow is represented as a graph with nodes, edges, entry artifacts, output artifacts, and a dataset contract. This is the closest part of the system to an ONNX-like philosophy because the graph is described as a contract-driven operator network, not just a visual layout.",
    references: [
      "apps/frontend/src/lib/api/workflow-types.ts",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-package.ts",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-package-adapter.ts",
    ],
  },
  {
    title: "3. Contract And Artifact Layer",
    body:
      "Bridge contracts define how one solver domain feeds the next. Summary contracts define a shared artifact payload family for extracts, transforms, exports, benchmarks, and headless flows. This layer is the real cross-operator glue of the system.",
    references: [
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-bridge-contract.ts",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-summary-contract.ts",
      "apps/frontend/src/app/docs/workflow-bridge-contracts/page.tsx",
      "apps/frontend/src/app/docs/workflow-summary-contracts/page.tsx",
    ],
  },
  {
    title: "4. Orchestration And Runtime Layer",
    body:
      "The controller fetches workflow catalogs, submits graph jobs, polls runtime state, records node runs, tracks lineage, and stores result envelopes in run records. This is where a graph becomes a run with inspectable artifacts.",
    references: [
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-controller.ts",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-types.ts",
      "apps/frontend/src/lib/api/runtime-client.ts",
    ],
  },
  {
    title: "5. Inspection And Audit Layer",
    body:
      "The sidebar, trace card, audit report, integrity checks, snapshot storage, and runtime diff views turn workflow execution into something users can inspect rather than merely trust. This is the usability layer that makes composite operators practical.",
    references: [
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-sidebar.tsx",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-run-trace-card.tsx",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-run-trace-report.ts",
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-snapshot-storage.ts",
    ],
  },
];

const STRATEGY = [
  "The architectural center is not the UI. It is the combination of operator descriptors, graph contracts, runtime records, and artifact families.",
  "UI remains fixed and automation-safe, while workflows and operators remain extensible through contract-visible graph composition.",
  "Bridge and summary contracts are the first stable cross-operator families. More artifact families can be added later without changing the philosophy.",
  "Headless SDK, wasm Python, and future agent-network runtimes should consume the same workflow graph and contract model rather than inventing parallel execution formats.",
];

const GAPS = [
  "The frontend now understands standardized summary artifacts, but not every external execution target is guaranteed to emit them yet.",
  "Bridge and summary artifact families are clearer than some future mesh, report, and benchmark artifact families, which still need the same level of formalization.",
  "Distributed agent and direct-mesh control remain important to the project vision, but they are not yet as structurally integrated as the workflow workbench itself.",
];

const PEAK_DIAGNOSTICS_FLOW = [
  "Peak diagnostics flows are not a separate execution architecture. They are a specialized reporting family built on the same operator descriptors, workflow graph model, dataset contracts, and runtime records as standard diagnostics flows.",
  "The dedicated peak extracts narrow each domain down to a few contract-stable extrema: electrostatic field peak, thermal flux peak, and thermo-mechanical displacement or stress peaks. That makes them better suited for limit review, release checks, and concise operator comparisons.",
  "The bundle, guard, report, and markdown export stages stay shared. What changes is the meaning of the upstream summaries and the ordering of the resulting focus metrics and highlights.",
  "In practice this gives tamamono two complementary inspection lanes: full diagnostics for broader field review, and peak diagnostics for fast threshold-oriented review.",
];

const PEAK_REFERENCES = [
  "workers/rust/crates/engine/src/operator_sdk_workflow_extensions.rs",
  "apps/web/lib/kyuubiki_web/workflow_peak_runtime.ex",
  "apps/web/lib/kyuubiki_web/workflow_template_diagnostics_entries.ex",
  "apps/frontend/src/components/workbench/workflow/workbench-workflow-diagnostics-presentation.ts",
];

const MERMAID_OVERVIEW = `flowchart TD
  A["Operator Descriptors"]
  B["Workflow Graph Definition"]
  C["Dataset Contract"]
  D["Bridge Contracts"]
  E["Summary Artifact Contracts"]
  F["Workflow Controller"]
  G["Runtime Job Result"]
  H["Run Record / Trace Summary"]
  I["Trace Card / Sidebar"]
  J["Audit Report / Snapshots"]
  K["Headless SDK / WASM Python / Future Agents"]

  A --> B
  C --> B
  D --> B
  B --> F
  E --> F
  F --> G
  G --> H
  E --> H
  H --> I
  H --> J
  B --> K
  E --> K`;

export default function WorkflowArchitecturePage() {
  return (
    <main style={mainStyle}>
      <div style={{ maxWidth: 980, margin: "0 auto" }}>
        <p style={eyebrowStyle}>tamamono 1.13.0</p>
        <h1 style={{ fontSize: "clamp(2rem, 4vw, 3.4rem)", margin: "0 0 12px" }}>
          Workflow Architecture
        </h1>
        <p style={copyStyle}>
          tamamono is no longer just a FEM frontend. It is evolving into a contract-driven operator
          platform where graphs, artifacts, runtime traces, and inspection surfaces align around a
          shared composition model.
        </p>

        <section style={{ marginTop: 32 }}>
          <h2>Core Idea</h2>
          <p style={copyStyle}>
            The system can be summarized as operator descriptors flowing into dataset contracts,
            then into workflow graphs, then into runtime traces, and finally into inspectable
            artifacts. The UI is the control surface, not the architectural center.
          </p>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Architecture Diagram</h2>
          <p style={copyStyle}>
            The following Mermaid source captures the current architectural flow from operator
            definition through runtime execution and inspection.
          </p>
          <pre style={preStyle}>{MERMAID_OVERVIEW}</pre>
        </section>

        <section style={{ marginTop: 32, display: "grid", gap: 18 }}>
          {LAYERS.map((layer) => (
            <div key={layer.title} style={cardStyle}>
              <h2 style={{ margin: 0 }}>{layer.title}</h2>
              <p style={copyStyle}>{layer.body}</p>
              <ul style={listStyle}>
                {layer.references.map((reference) => (
                  <li key={reference}>{reference}</li>
                ))}
              </ul>
            </div>
          ))}
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Strategic Direction</h2>
          <ul style={listStyle}>
            {STRATEGY.map((entry) => (
              <li key={entry}>{entry}</li>
            ))}
          </ul>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Current Gaps</h2>
          <ul style={listStyle}>
            {GAPS.map((entry) => (
              <li key={entry}>{entry}</li>
            ))}
          </ul>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Peak Diagnostics Flow</h2>
          <p style={copyStyle}>
            Peak diagnostics is now a first-class workflow pattern inside tamamono. It exists for
            cases where users care more about extrema and guard thresholds than about carrying the
            entire diagnostic field narrative through every surface.
          </p>
          <ul style={listStyle}>
            {PEAK_DIAGNOSTICS_FLOW.map((entry) => (
              <li key={entry}>{entry}</li>
            ))}
          </ul>
          <div style={{ ...cardStyle, marginTop: 18 }}>
            <h3 style={{ margin: 0 }}>Peak Flow References</h3>
            <ul style={listStyle}>
              {PEAK_REFERENCES.map((reference) => (
                <li key={reference}>{reference}</li>
              ))}
            </ul>
          </div>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Related References</h2>
          <div style={{ display: "grid", gap: 12 }}>
            <Link href="/docs/workflow-bridge-contracts" style={linkCardStyle}>
              Bridge contract reference
            </Link>
            <Link href="/docs/workflow-summary-contracts" style={linkCardStyle}>
              Summary artifact contract reference
            </Link>
            <Link href="/docs" style={linkCardStyle}>
              Back to docs hub
            </Link>
          </div>
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

const copyStyle: CSSProperties = {
  color: "#9eb1c8",
  lineHeight: 1.7,
  maxWidth: 820,
};

const cardStyle: CSSProperties = {
  display: "grid",
  gap: 10,
  padding: 20,
  borderRadius: 16,
  border: "1px solid rgba(127, 179, 255, 0.18)",
  background: "rgba(9, 13, 18, 0.92)",
};

const preStyle: CSSProperties = {
  background: "rgba(9, 13, 18, 0.92)",
  border: "1px solid rgba(127, 179, 255, 0.18)",
  borderRadius: 16,
  padding: 20,
  overflowX: "auto",
  maxWidth: "100%",
  lineHeight: 1.6,
  fontSize: 13,
  color: "#d9e3f0",
};

const listStyle: CSSProperties = {
  color: "#b9c7d8",
  lineHeight: 1.8,
  paddingLeft: 20,
  margin: 0,
  overflowWrap: "anywhere",
  wordBreak: "break-word",
};

const linkCardStyle: CSSProperties = {
  display: "block",
  padding: 16,
  borderRadius: 14,
  border: "1px solid rgba(127, 179, 255, 0.18)",
  background: "rgba(9, 13, 18, 0.92)",
  color: "#7fb3ff",
  textDecoration: "none",
};
