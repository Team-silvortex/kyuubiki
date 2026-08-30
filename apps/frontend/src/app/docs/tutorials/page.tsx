import Link from "next/link";
import type { CSSProperties } from "react";
import type { Metadata } from "next";
import { KYUUBIKI_PRODUCT_VERSION_LABEL } from "@/lib/product-version";

export const metadata: Metadata = {
  title: `Tutorial Paths | ${KYUUBIKI_PRODUCT_VERSION_LABEL}`,
  description: "Task-first onboarding for Workbench, PWDT, Rust Headless, and the Rust Operator SDK.",
};

const firstLoop = [
  ["01", "Own the project", "Create or deliberately select one project before changing a model."],
  ["02", "Build a bounded model", "Use a small 2D truss and save it with a stable name."],
  ["03", "Run once", "Wait for one terminal job state instead of submitting duplicates."],
  ["04", "Inspect evidence", "Open the result and retain project, model, job, version, unit, and metric identity."],
  ["05", "Prove persistence", "Leave the project, reopen it, and recover the same model and result."],
] as const;

const paths = [
  {
    code: "GUI",
    title: "First Workbench loop",
    summary: "Learn project, model, study, result, and persistence ownership before automation.",
    proof: "Reopenable project and result",
    href: null,
  },
  {
    code: "FLOW",
    title: "Composite operator workflow",
    summary: "Start from Catalog, open Builder, validate graph and dataset contracts, then inspect Runs.",
    proof: "Terminal node trace and exports",
    href: "/docs/workflow-architecture",
  },
  {
    code: "PWDT",
    title: "Frontend automation",
    summary: "Use registered actions, recipes, observable state waits, and the product UI contract.",
    proof: "Action timeline and terminal UI state",
    href: null,
  },
  {
    code: "RUST",
    title: "Headless research",
    summary: "Discover a template, validate, retain a plan, mock the orchestration, then use a live service.",
    proof: "Plan, run report, and result evidence",
    href: null,
  },
  {
    code: "OP",
    title: "Operator authoring",
    summary: "Use the Rust-only Operator SDK for executable physics, transforms, bridges, and exports.",
    proof: "Readiness, baselines, and package preflight",
    href: null,
  },
] as const;

const boundaries = [
  ["PWDT", "Automates the fixed Workbench UI", "It does not implement solvers or replace Headless control."],
  ["Headless SDK", "Controls existing capabilities without a frontend", "It does not add an executable operator."],
  ["Operator SDK", "Adds executable agent-engine capability in Rust", "It does not automate GUI interactions."],
] as const;

export default function TutorialPathsPage() {
  return (
    <main style={mainStyle}>
      <div style={shellStyle}>
        <header style={heroStyle}>
          <div style={heroGridStyle}>
            <div>
              <p style={eyebrowStyle}>{KYUUBIKI_PRODUCT_VERSION_LABEL} / tutorial routes</p>
              <h1 style={titleStyle}>Finish one loop before adding scale.</h1>
              <p style={heroCopyStyle}>
                These paths begin with an observable result and end with retained evidence. They
                are intentionally smaller than the architecture reference so the next action is
                always clear.
              </p>
            </div>
            <div style={statusStyle}>
              <span style={statusLabelStyle}>FIRST GATE</span>
              <strong style={{ fontSize: 28 }}>Project to result</strong>
              <span style={mutedStyle}>Target: 15 minutes</span>
              <span style={mutedStyle}>Retry policy: one bounded stage</span>
            </div>
          </div>
          <div style={navStyle}>
            <Link href="/docs" style={buttonStyle}>Back to references</Link>
            <Link href="/docs/workflow-architecture" style={quietButtonStyle}>Workflow contract</Link>
          </div>
        </header>

        <section style={sectionStyle}>
          <div style={sectionHeadingStyle}>
            <span style={sectionNumberStyle}>A</span>
            <div>
              <h2 style={sectionTitleStyle}>The first complete Workbench loop</h2>
              <p style={mutedStyle}>Do not move to automation until all five checkpoints pass.</p>
            </div>
          </div>
          <div style={stepGridStyle}>
            {firstLoop.map(([number, title, summary]) => (
              <article key={number} style={stepStyle}>
                <span style={stepNumberStyle}>{number}</span>
                <strong>{title}</strong>
                <p style={cardCopyStyle}>{summary}</p>
              </article>
            ))}
          </div>
          <div style={checkpointStyle}>
            <strong>Completion gate</strong>
            <span>
              A click is not completion. The same project, model, terminal run, result, and evidence
              identity must be recoverable after leaving the active view.
            </span>
          </div>
        </section>

        <section style={sectionStyle}>
          <div style={sectionHeadingStyle}>
            <span style={sectionNumberStyle}>B</span>
            <div>
              <h2 style={sectionTitleStyle}>Choose the next route by ownership</h2>
              <p style={mutedStyle}>Use the narrowest surface that actually owns the task.</p>
            </div>
          </div>
          <div style={pathGridStyle}>
            {paths.map((path) => {
              const content = (
                <>
                  <span style={pathCodeStyle}>{path.code}</span>
                  <strong style={{ fontSize: 18 }}>{path.title}</strong>
                  <p style={cardCopyStyle}>{path.summary}</p>
                  <span style={proofStyle}>Proof: {path.proof}</span>
                </>
              );
              return path.href ? (
                <Link href={path.href} key={path.code} style={pathStyle}>{content}</Link>
              ) : (
                <article key={path.code} style={pathStyle}>{content}</article>
              );
            })}
          </div>
        </section>

        <section style={sectionStyle}>
          <div style={sectionHeadingStyle}>
            <span style={sectionNumberStyle}>C</span>
            <div>
              <h2 style={sectionTitleStyle}>Three SDK boundaries</h2>
              <p style={mutedStyle}>The names can look adjacent; their authority is not.</p>
            </div>
          </div>
          <div style={boundaryStyle}>
            {boundaries.map(([name, owns, excludes]) => (
              <div key={name} style={boundaryRowStyle}>
                <strong style={{ color: "#dce9f8" }}>{name}</strong>
                <span>{owns}</span>
                <span style={mutedStyle}>{excludes}</span>
              </div>
            ))}
          </div>
        </section>

        <section style={recoveryStyle}>
          <div>
            <p style={eyebrowStyle}>Recovery rule</p>
            <h2 style={sectionTitleStyle}>Keep the first failure receipt.</h2>
          </div>
          <p style={{ ...cardCopyStyle, maxWidth: 620 }}>
            Classify the failure as storage, validation, dispatch, execution, or result retrieval.
            Preserve its id and report, then retry only that bounded stage. Do not erase partial work
            or resubmit a solve merely because the result view is missing.
          </p>
        </section>
      </div>
    </main>
  );
}

const mainStyle: CSSProperties = {
  minHeight: "100vh",
  padding: "36px 18px 72px",
  background:
    "linear-gradient(135deg, rgba(25, 62, 81, 0.26), transparent 42%), repeating-linear-gradient(90deg, rgba(120, 165, 186, 0.025) 0, rgba(120, 165, 186, 0.025) 1px, transparent 1px, transparent 64px), #10151a",
  color: "#dce9f8",
  fontFamily: '"Avenir Next", "IBM Plex Sans", sans-serif',
};

const shellStyle: CSSProperties = { maxWidth: 1120, margin: "0 auto", display: "grid", gap: 18 };
const heroStyle: CSSProperties = {
  padding: "clamp(24px, 4vw, 48px)", borderRadius: 24, border: "1px solid #2c4b5c",
  background: "linear-gradient(145deg, #17242d, #11181e 72%)", boxShadow: "0 28px 70px rgba(0,0,0,0.28)",
};
const heroGridStyle: CSSProperties = { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 260px), 1fr))", gap: 28, alignItems: "end" };
const eyebrowStyle: CSSProperties = { margin: 0, color: "#79c6df", letterSpacing: "0.14em", textTransform: "uppercase", fontSize: 12, fontWeight: 700 };
const titleStyle: CSSProperties = { margin: "10px 0 14px", maxWidth: 760, fontSize: "clamp(2.25rem, 6vw, 4.8rem)", lineHeight: 0.98, letterSpacing: "-0.055em" };
const heroCopyStyle: CSSProperties = { margin: 0, maxWidth: 720, color: "#a8bdca", lineHeight: 1.7, fontSize: 17 };
const statusStyle: CSSProperties = { display: "grid", gap: 8, padding: 20, borderRadius: 18, border: "1px solid #315568", background: "#0d1419" };
const statusLabelStyle: CSSProperties = { color: "#f0b95d", fontSize: 11, letterSpacing: "0.16em", fontWeight: 800 };
const mutedStyle: CSSProperties = { color: "#8fa6b4", lineHeight: 1.55 };
const navStyle: CSSProperties = { display: "flex", flexWrap: "wrap", gap: 10, marginTop: 24 };
const buttonStyle: CSSProperties = { padding: "10px 14px", borderRadius: 10, background: "#79c6df", color: "#0d151a", textDecoration: "none", fontWeight: 800 };
const quietButtonStyle: CSSProperties = { ...buttonStyle, background: "transparent", color: "#cfe3ed", border: "1px solid #315568" };
const sectionStyle: CSSProperties = { padding: "clamp(20px, 3vw, 30px)", borderRadius: 20, border: "1px solid #263b47", background: "rgba(16, 24, 30, 0.94)" };
const sectionHeadingStyle: CSSProperties = { display: "flex", gap: 14, alignItems: "flex-start", marginBottom: 20 };
const sectionNumberStyle: CSSProperties = { display: "grid", placeItems: "center", width: 34, height: 34, borderRadius: 9, color: "#10171c", background: "#f0b95d", fontWeight: 900 };
const sectionTitleStyle: CSSProperties = { margin: "0 0 4px", fontSize: "clamp(1.35rem, 3vw, 2rem)", letterSpacing: "-0.025em" };
const stepGridStyle: CSSProperties = { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 10 };
const stepStyle: CSSProperties = { minHeight: 150, padding: 16, display: "grid", alignContent: "start", gap: 9, borderRadius: 14, border: "1px solid #263f4d", background: "#131d23" };
const stepNumberStyle: CSSProperties = { color: "#79c6df", fontFamily: '"IBM Plex Mono", monospace', fontSize: 12, letterSpacing: "0.12em" };
const cardCopyStyle: CSSProperties = { margin: 0, color: "#9db2bf", lineHeight: 1.6 };
const checkpointStyle: CSSProperties = { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 180px), 1fr))", gap: 16, marginTop: 14, padding: 16, borderRadius: 12, borderLeft: "3px solid #79c6df", background: "#101b22", color: "#b9cbd5", lineHeight: 1.6 };
const pathGridStyle: CSSProperties = { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: 12 };
const pathStyle: CSSProperties = { minHeight: 190, padding: 18, display: "grid", alignContent: "start", gap: 10, borderRadius: 16, border: "1px solid #2a4655", background: "linear-gradient(155deg, #17252d, #11191e)", color: "inherit", textDecoration: "none" };
const pathCodeStyle: CSSProperties = { width: "fit-content", padding: "4px 7px", borderRadius: 6, background: "#213d49", color: "#8bd6ed", fontSize: 11, fontWeight: 900, letterSpacing: "0.12em" };
const proofStyle: CSSProperties = { marginTop: 6, color: "#e5bb72", fontSize: 13 };
const boundaryStyle: CSSProperties = { display: "grid", border: "1px solid #29414d", borderRadius: 14, overflow: "hidden" };
const boundaryRowStyle: CSSProperties = { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 220px), 1fr))", gap: 16, padding: 16, borderBottom: "1px solid #223640", alignItems: "start" };
const recoveryStyle: CSSProperties = { display: "flex", flexWrap: "wrap", justifyContent: "space-between", gap: 24, padding: "clamp(22px, 4vw, 38px)", borderRadius: 20, border: "1px solid #66502e", background: "linear-gradient(135deg, #2b2519, #151815 70%)" };
