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
      style={{ position: "absolute", inset: 0, pointerEvents: "none", overflow: "visible", zIndex: 0 }}
    >
      <line
        stroke="rgba(125, 211, 252, 0.9)"
        strokeDasharray="8 6"
        strokeWidth="2.5"
        x1={source.x}
        x2={target.x}
        y1={source.y}
        y2={target.y}
      />
    </svg>
  );
}
