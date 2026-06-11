"use client";

import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type ControlFlowPlaneSnapshot = {
  source: string[];
  conditionId: string;
  trueTargets: string[];
  falseTargets: string[];
  mergeId: string | null;
  mergeOutgoing: string[];
};

type WorkbenchWorkflowControlFlowSnapshotCardProps = {
  labels: WorkflowSidebarLabels;
  snapshot: ControlFlowPlaneSnapshot;
  selectedLaneKey: string | null;
  onSelectLane: (lane: string | null) => void;
};

function buildLaneHighlightStyle(active: boolean, tone: "good" | "risk" | "watch") {
  if (!active) return undefined;
  const color =
    tone === "good"
      ? "rgba(34, 197, 94, 0.9)"
      : tone === "risk"
        ? "rgba(248, 113, 113, 0.9)"
        : "rgba(125, 211, 252, 0.95)";
  return {
    outline: `2px solid ${color}`,
    outlineOffset: "2px",
    boxShadow: `0 0 0 1px ${color.replace("0.9", "0.22").replace("0.95", "0.24")}, 0 0 18px ${color.replace("0.9", "0.14").replace("0.95", "0.16")}`,
  };
}

function laneKey(nodeId: string, lane: string) {
  return `${nodeId}:${lane}`;
}

function renderEndpointChips(values: string[], tone: "good" | "risk" | "watch") {
  if (values.length === 0) return <span className="card-copy">--</span>;
  return (
    <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
      {values.map((value) => (
        <span className={`status-pill status-pill--${tone}`} key={value}>
          {value}
        </span>
      ))}
    </div>
  );
}

function renderStageNode(label: string, tone: "good" | "risk" | "watch", detail: string) {
  return (
    <div
      style={{
        display: "grid",
        gap: "0.35rem",
        padding: "0.55rem 0.6rem",
        borderRadius: "10px",
        border: "1px solid var(--line)",
        background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.2))",
      }}
    >
      <span className={`status-pill status-pill--${tone}`}>{label}</span>
      <span className="card-copy">{detail}</span>
    </div>
  );
}

function renderLaneButton(
  active: boolean,
  tone: "good" | "risk" | "watch",
  title: string,
  values: string[],
  onClick: () => void,
) {
  return (
    <button onClick={onClick} style={{ all: "unset", cursor: "pointer" }} type="button">
      <div
        style={{
          display: "grid",
          gap: "0.35rem",
          padding: "0.55rem 0.6rem",
          borderRadius: "10px",
          border: "1px solid var(--line)",
          background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.18))",
          ...buildLaneHighlightStyle(active, tone),
        }}
      >
        <span className="card-copy">{title}</span>
        {renderEndpointChips(values, tone)}
      </div>
    </button>
  );
}

export function WorkbenchWorkflowControlFlowSnapshotCard({
  labels,
  snapshot,
  selectedLaneKey,
  onSelectLane,
}: WorkbenchWorkflowControlFlowSnapshotCardProps) {
  const trueLaneActive = selectedLaneKey === laneKey(snapshot.conditionId, "true");
  const falseLaneActive = selectedLaneKey === laneKey(snapshot.conditionId, "false");
  const mergeLaneActive = selectedLaneKey === laneKey(snapshot.conditionId, "merge");
  const downstreamLaneActive = selectedLaneKey === laneKey(snapshot.conditionId, "downstream");
  return (
    <div
      style={{
        display: "grid",
        gap: "0.75rem",
        padding: "0.85rem",
        border: "1px solid var(--line)",
        borderRadius: "12px",
        background: "linear-gradient(180deg, rgba(255,255,255,0.05), rgba(0,0,0,0.24))",
        marginBottom: "0.8rem",
      }}
    >
      <div style={{ display: "grid", gap: "0.55rem", gridTemplateColumns: "minmax(0,1fr) 28px 120px 28px minmax(0,1fr)" }}>
        {renderStageNode(labels.localWorkflowSourceLabel, "watch", `${snapshot.source.length} upstream`)}
        <div style={{ alignSelf: "center", height: "2px", borderRadius: "999px", background: "linear-gradient(90deg, rgba(125,211,252,0.2), rgba(125,211,252,0.8))" }} />
        {renderStageNode("Branch", "watch", snapshot.conditionId)}
        <div style={{ alignSelf: "center", height: "2px", borderRadius: "999px", background: "linear-gradient(90deg, rgba(125,211,252,0.8), rgba(34,197,94,0.7))" }} />
        {renderStageNode(labels.controlFlowPlaneMergeLabel, "good", snapshot.mergeId ?? "pending")}
      </div>
      <div style={{ display: "grid", gap: "0.65rem", gridTemplateColumns: "minmax(0,1fr) 120px minmax(0,1fr)", alignItems: "stretch" }}>
        <div style={{ display: "grid", gap: "0.35rem", alignContent: "start" }}>
          <span className="card-copy">{labels.localWorkflowSourceLabel}</span>
          {renderEndpointChips(snapshot.source, "watch")}
        </div>
        <div style={{ display: "grid", gap: "0.4rem", justifyItems: "center", alignContent: "center", padding: "0.5rem", borderRadius: "12px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(125,211,252,0.08), rgba(0,0,0,0.16))" }}>
          <span className="status-pill status-pill--watch">{snapshot.conditionId}</span>
          <span className="card-copy">branch switch</span>
        </div>
        <div style={{ display: "grid", gap: "0.45rem" }}>
          {renderLaneButton(trueLaneActive, "good", "true lane", snapshot.trueTargets, () => onSelectLane(trueLaneActive ? null : laneKey(snapshot.conditionId, "true")))}
          {renderLaneButton(falseLaneActive, "risk", "false lane", snapshot.falseTargets, () => onSelectLane(falseLaneActive ? null : laneKey(snapshot.conditionId, "false")))}
        </div>
      </div>
      <div style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "minmax(0,1fr) 120px minmax(0,1fr)", alignItems: "center" }}>
        <div style={{ height: "2px", borderRadius: "999px", background: "linear-gradient(90deg, rgba(34,197,94,0.7), rgba(125,211,252,0.7))" }} />
        <div style={{ textAlign: "center" }}>
          <span className="card-copy">lane convergence</span>
        </div>
        <div style={{ height: "2px", borderRadius: "999px", background: "linear-gradient(90deg, rgba(248,113,113,0.7), rgba(125,211,252,0.7))" }} />
      </div>
      <div style={{ display: "grid", gap: "0.65rem", gridTemplateColumns: "minmax(0,1fr) 120px minmax(0,1fr)", alignItems: "stretch" }}>
        {renderLaneButton(mergeLaneActive, "watch", "lane merge", [...new Set([...snapshot.trueTargets, ...snapshot.falseTargets])], () => onSelectLane(mergeLaneActive ? null : laneKey(snapshot.conditionId, "merge")))}
        <div style={{ display: "grid", gap: "0.4rem", justifyItems: "center", alignContent: "center", padding: "0.5rem", borderRadius: "12px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(34,197,94,0.08), rgba(0,0,0,0.16))" }}>
          <span className="status-pill status-pill--good">{snapshot.mergeId ?? "--"}</span>
          <span className="card-copy">{labels.controlFlowPlaneMergeLabel}</span>
        </div>
        {renderLaneButton(downstreamLaneActive, "good", "downstream", snapshot.mergeOutgoing, () => onSelectLane(downstreamLaneActive ? null : laneKey(snapshot.conditionId, "downstream")))}
      </div>
    </div>
  );
}
