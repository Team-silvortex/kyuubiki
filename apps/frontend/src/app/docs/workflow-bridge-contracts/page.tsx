import type { CSSProperties } from "react";
import type { Metadata } from "next";
import {
  createElectrostaticToHeatBridgeContract,
  createHeatToThermoBridgeContract,
  ELECTROSTATIC_TO_HEAT_BRIDGE_CONTRACT_SCHEMA,
  HEAT_TO_THERMO_BRIDGE_CONTRACT_SCHEMA,
} from "@/lib/workbench/workflow-bridge-contract";

export const metadata: Metadata = {
  title: "Workflow Bridge Contracts | tamamono 1.15.0",
  description: "Bridge contract reference for cross-operator workflow transforms.",
};

function pretty(value: unknown) {
  return JSON.stringify(value, null, 2);
}

export default function WorkflowBridgeContractsPage() {
  return (
    <main
      style={{
        minHeight: "100vh",
        background:
          "radial-gradient(circle at top, rgba(103, 132, 166, 0.18), transparent 36%), #10151b",
        color: "#d9e3f0",
        padding: "48px 20px 80px",
        fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
      }}
    >
      <div style={{ maxWidth: 980, margin: "0 auto" }}>
        <p style={{ color: "#7fb3ff", letterSpacing: "0.12em", textTransform: "uppercase" }}>
          tamamono 1.15.0
        </p>
        <h1 style={{ fontSize: "clamp(2rem, 4vw, 3.4rem)", margin: "0 0 12px" }}>
          Workflow Bridge Contracts
        </h1>
        <p style={{ color: "#9eb1c8", maxWidth: 760, lineHeight: 1.7 }}>
          This page documents the shared contract shape used by workflow bridge operators so
          cross-operator transforms can describe what field they read, how values are mapped, and
          which downstream field they populate.
        </p>

        <section style={{ marginTop: 32 }}>
          <h2>Contract Shape</h2>
          <pre style={preStyle}>{pretty({
            version: "kyuubiki.bridge-contract/v1",
            source: { field: "string", distribution: "string", node_index_fields: ["string"] },
            transform: { scale: "number", reduction: "string", default_value: "number" },
            target: { field: "string" },
          })}</pre>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Electrostatic to Heat</h2>
          <p style={copyStyle}>
            Schema: {ELECTROSTATIC_TO_HEAT_BRIDGE_CONTRACT_SCHEMA.schema}@
            {ELECTROSTATIC_TO_HEAT_BRIDGE_CONTRACT_SCHEMA.version}
          </p>
          <pre style={preStyle}>{pretty(createElectrostaticToHeatBridgeContract())}</pre>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Heat to Thermo</h2>
          <p style={copyStyle}>
            Schema: {HEAT_TO_THERMO_BRIDGE_CONTRACT_SCHEMA.schema}@
            {HEAT_TO_THERMO_BRIDGE_CONTRACT_SCHEMA.version}
          </p>
          <pre style={preStyle}>{pretty(createHeatToThermoBridgeContract())}</pre>
        </section>

        <section style={{ marginTop: 32 }}>
          <h2>Notes</h2>
          <ul style={{ color: "#b9c7d8", lineHeight: 1.8, paddingLeft: 20 }}>
            <li>`source.field` identifies the upstream solver/result field that the bridge reads.</li>
            <li>`transform.scale` provides a deterministic unit or magnitude conversion point.</li>
            <li>`target.field` is the downstream model field populated before the next solve step.</li>
            <li>`distribution` and `reduction` let element-wise and node-wise bridges share one contract family.</li>
          </ul>
        </section>
      </div>
    </main>
  );
}

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
};
