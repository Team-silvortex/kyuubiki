import Link from "next/link";
import type { CSSProperties } from "react";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Workflow Docs | tamamono 1.11.5",
  description: "Architecture and contract references for the tamamono workflow system.",
};

const DOCS = [
  {
    href: "/docs/workflow-architecture",
    title: "Workflow Architecture",
    summary:
      "System-wide overview of operators, graph contracts, runtime records, artifacts, audit surfaces, and the newer peak diagnostics flow.",
  },
  {
    href: "/docs/workflow-bridge-contracts",
    title: "Workflow Bridge Contracts",
    summary:
      "Cross-operator bridge contract shape for electrostatic-to-heat and heat-to-thermo transforms.",
  },
  {
    href: "/docs/workflow-summary-contracts",
    title: "Workflow Summary Contracts",
    summary:
      "Shared summary artifact contract used by extract, transform, export, benchmark, and headless flows.",
  },
  {
    href: "/docs/workbench-window-protection",
    title: "Workbench Window Protection",
    summary:
      "Responsive shell, panel, tab, and content-priority rules that protect the built-in UI on narrow and fullscreen layouts.",
  },
];

export default function DocsPage() {
  return (
    <main style={mainStyle}>
      <div style={{ maxWidth: 980, margin: "0 auto" }}>
        <p style={eyebrowStyle}>tamamono 1.11.5</p>
        <h1 style={{ fontSize: "clamp(2rem, 4vw, 3.4rem)", margin: "0 0 12px" }}>
          Workflow Docs
        </h1>
        <p style={copyStyle}>
          This documentation hub collects the architectural and contract-level references for the
          workflow system so the builder, headless SDK, and future runtime targets can align on one
          shared model. It also tracks newer workflow families such as the dedicated peak
          diagnostics review path.
        </p>

        <section style={{ marginTop: 32, display: "grid", gap: 16 }}>
          {DOCS.map((entry) => (
            <Link href={entry.href} key={entry.href} style={cardStyle}>
              <strong style={{ fontSize: "1.05rem", color: "#d9e3f0" }}>{entry.title}</strong>
              <span style={{ ...copyStyle, maxWidth: "none" }}>{entry.summary}</span>
              <span style={linkStyle}>Open reference</span>
            </Link>
          ))}
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
  maxWidth: 760,
};

const cardStyle: CSSProperties = {
  display: "grid",
  gap: 8,
  padding: 20,
  borderRadius: 16,
  border: "1px solid rgba(127, 179, 255, 0.18)",
  background: "rgba(9, 13, 18, 0.92)",
  textDecoration: "none",
};

const linkStyle: CSSProperties = {
  color: "#7fb3ff",
};
