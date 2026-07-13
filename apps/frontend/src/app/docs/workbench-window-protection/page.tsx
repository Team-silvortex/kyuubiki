import Link from "next/link";
import type { CSSProperties } from "react";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Workbench Window Protection | tamamono 1.19.0",
  description: "Responsive protection rules for the built-in tamamono workbench shell.",
};

const LAYERS = [
  {
    title: "1. Shell Layer",
    body:
      "The root workbench shell detects window width and fullscreen state, then exposes stable data attributes that the built-in UI can respond to without changing the automation contract.",
    rules: [
      'Use `data-workbench-window-mode="standard|compact|narrow|ultranarrow"` as the top-level responsive switch.',
      'Use `data-workbench-fullscreen="true|false"` to tighten spacing and usable height without introducing a second shell implementation.',
      "Keep the shell built-in and non-extensible so wasm Python and future automation can rely on a fixed frame.",
    ],
    references: [
      "apps/frontend/src/components/workbench/workbench-shell-frame.tsx",
      "apps/frontend/src/app/workbench-shell-window-protection.css",
    ],
  },
  {
    title: "2. Panel Layer",
    body:
      "When the shell narrows, major panels should rearrange before they truncate. The inspector can drop below the main workspace, the rail can reflow, and split panes can collapse into one column.",
    rules: [
      "Prefer panel reflow before hiding information.",
      "Collapse split canvas and dense side-by-side compositions once the window reaches ultranarrow mode.",
      "Reduce padding, tab height, and chrome spacing before touching primary controls.",
    ],
    references: [
      "apps/frontend/src/app/workbench-shell-window-protection.css",
      "apps/frontend/src/components/workbench/workbench-sidebar-panel.tsx",
      "apps/frontend/src/components/workbench/workbench-inspector.tsx",
    ],
  },
  {
    title: "3. Tab Layer",
    body:
      "Once panel reflow is exhausted, each built-in surface can degrade by active tab. Workflow and inspector surfaces now expose tab-level state so narrow layouts can preserve the currently useful content and discard supporting chrome first.",
    rules: [
      "Expose active tab and page state through data attributes rather than deriving it from DOM text.",
      "Let workflow overview, catalog, builder, and runs protect different content blocks.",
      "Let inspector status, result, and actions pages reduce support chrome independently.",
    ],
    references: [
      "apps/frontend/src/components/workbench/workflow/workbench-workflow-sidebar.tsx",
      "apps/frontend/src/components/workbench/workbench-inspector.tsx",
      "apps/frontend/src/app/workbench-shell-window-protection.css",
    ],
  },
  {
    title: "4. Content Priority Layer",
    body:
      "The final step is not more shrinking. It is deliberate loss. On the smallest widths, the UI should preserve title, current status, primary value, and the next action. Secondary hints, long descriptions, auxiliary counts, and duplicate status summaries should be the first to leave.",
    rules: [
      "Protect title, active status, current result, and primary action first.",
      "Hide long card copy and duplicate overview statistics before hiding main controls.",
      "Shorten meta bars so the front half of the line survives while tail metrics yield.",
      "Treat documentation pages the same way: long file paths and contract references must wrap rather than force horizontal scrolling.",
    ],
    references: [
      "apps/frontend/src/app/workbench-shell-window-protection.css",
      "apps/frontend/src/app/docs/workflow-architecture/page.tsx",
      "apps/frontend/scripts/check-ui-layout.mjs",
    ],
  },
];

const PRINCIPLES = [
  "Responsive protection must preserve the fixed built-in UI contract. The UI adapts, but it does not become user-extensible.",
  "Structural reflow is always preferred over hiding information.",
  "Visibility loss should follow a stable priority order, not ad hoc one-off fixes.",
  "Every layout protection rule should be verifiable through automated viewport checks.",
];

const CURRENT_BEHAVIOR = [
  "Compact mode reduces shell column widths and trims chrome spacing.",
  "Narrow mode pulls the inspector below the main workspace and simplifies split layouts.",
  "Ultranarrow mode aggressively reduces rail branding, tab density, meta-bar chatter, and low-priority copy.",
  "Workflow and inspector surfaces expose active tab state so tab-specific degradation can stay intentional.",
  "The UI audit now checks desktop, tablet, and phone widths instead of assuming laptop layouts are sufficient.",
];

const NEXT_STEPS = [
  "Keep adding page-specific responsive fixes only after checking whether an existing shell or panel-level rule should handle the case.",
  "Document any new built-in panel with its own priority order: what must survive first, what may collapse second, and what can disappear last.",
  "Extend the layout audit when a new surface introduces a different density pattern, especially if it uses tabs, summary cards, or long contract strings.",
];

export default function WorkbenchWindowProtectionPage() {
  return (
    <main style={mainStyle}>
      <div style={{ maxWidth: 980, margin: "0 auto" }}>
        <p style={eyebrowStyle}>tamamono 1.19.0</p>
        <h1 style={{ fontSize: "clamp(2rem, 4vw, 3.4rem)", margin: "0 0 12px" }}>
          Workbench Window Protection
        </h1>
        <p style={copyStyle}>
          The tamamono workbench now treats narrow layouts as a first-class architectural concern.
          The goal is not merely to avoid overlap. The goal is to preserve the built-in automation-safe
          UI while the window shrinks, panels reflow, and low-priority information yields in a
          predictable order.
        </p>

        <section style={{ marginTop: 32 }}>
          <h2>Why This Exists</h2>
          <p style={copyStyle}>
            A fixed UI contract is one of the project&apos;s stability requirements. wasm Python,
            headless tooling, and future agent-facing control surfaces all benefit when the shell
            remains stable. That means responsiveness cannot be an afterthought or a loose visual
            tweak. It has to be a protected behavior layer.
          </p>
        </section>

        <section style={{ marginTop: 32, display: "grid", gap: 18 }}>
          {LAYERS.map((layer) => (
            <div key={layer.title} style={cardStyle}>
              <h2 style={{ margin: 0 }}>{layer.title}</h2>
              <p style={copyStyle}>{layer.body}</p>
              <h3 style={subheadStyle}>Rules</h3>
              <ul style={listStyle}>
                {layer.rules.map((entry) => (
                  <li key={entry}>{entry}</li>
                ))}
              </ul>
              <h3 style={subheadStyle}>References</h3>
              <ul style={listStyle}>
                {layer.references.map((entry) => (
                  <li key={entry}>{entry}</li>
                ))}
              </ul>
            </div>
          ))}
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Protection Principles</h2>
          <ul style={listStyle}>
            {PRINCIPLES.map((entry) => (
              <li key={entry}>{entry}</li>
            ))}
          </ul>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Current Behavior</h2>
          <ul style={listStyle}>
            {CURRENT_BEHAVIOR.map((entry) => (
              <li key={entry}>{entry}</li>
            ))}
          </ul>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Next Steps</h2>
          <ul style={listStyle}>
            {NEXT_STEPS.map((entry) => (
              <li key={entry}>{entry}</li>
            ))}
          </ul>
        </section>

        <section style={{ marginTop: 32, display: "grid", gap: 12 }}>
          <Link href="/docs/workflow-architecture" style={linkCardStyle}>
            Workflow architecture reference
          </Link>
          <Link href="/docs" style={linkCardStyle}>
            Back to docs hub
          </Link>
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
  maxWidth: 840,
};

const subheadStyle: CSSProperties = {
  margin: "2px 0 0",
  color: "#d9e3f0",
  fontSize: "0.95rem",
};

const cardStyle: CSSProperties = {
  display: "grid",
  gap: 10,
  padding: 20,
  borderRadius: 16,
  border: "1px solid rgba(127, 179, 255, 0.18)",
  background: "rgba(9, 13, 18, 0.92)",
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
