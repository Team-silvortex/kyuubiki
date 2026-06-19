"use client";

export type WorkflowControlFlowWirePoint = {
  x: number;
  y: number;
};

type WorkbenchWorkflowControlFlowWireOverlayProps = {
  source: WorkflowControlFlowWirePoint | null;
  target: WorkflowControlFlowWirePoint | null;
};

export function WorkbenchWorkflowControlFlowWireOverlay({
  source,
  target,
}: WorkbenchWorkflowControlFlowWireOverlayProps) {
  if (!source || !target) return null;
  return (
    <svg
      aria-hidden="true"
      className="workflow-control-flow-wire-overlay"
      style={{ position: "absolute", inset: 0, pointerEvents: "none", overflow: "visible", zIndex: 0 }}
    >
      <line
        className="workflow-control-flow-wire-overlay__glow"
        x1={source.x}
        x2={target.x}
        y1={source.y}
        y2={target.y}
      />
      <line
        className="workflow-control-flow-wire-overlay__line"
        x1={source.x}
        x2={target.x}
        y1={source.y}
        y2={target.y}
      />
      <circle className="workflow-control-flow-wire-overlay__socket" cx={source.x} cy={source.y} r="4.5" />
      <circle className="workflow-control-flow-wire-overlay__socket workflow-control-flow-wire-overlay__socket--target" cx={target.x} cy={target.y} r="4.5" />
    </svg>
  );
}
