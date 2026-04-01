"use client";

import { memo } from "react";
import { VirtualList } from "@/components/virtual-list";

type NodeRow = {
  id: string;
  x: number;
  y: number;
  load_y: number;
};

type ElementRow = {
  id: string;
  node_i: number;
  node_j?: number;
  node_k?: number;
};

type WorkbenchObjectTreeProps = {
  title: string;
  countLabel: string;
  hint: string;
  diagnosticsLabel: string;
  loadCaseLabel: string;
  nodeJLabel: string;
  nodeKLabel: string;
  nodeRows: NodeRow[];
  elementRows: ElementRow[];
  isPlane: boolean;
  isTruss: boolean;
  selectedNode: number | null;
  selectedElement: number | null;
  nodeIssueCounts: Record<number, number>;
  onSelectNode: (index: number) => void;
  onSelectElement: (index: number) => void;
};

function WorkbenchObjectTreeInner({
  title,
  countLabel,
  hint,
  diagnosticsLabel,
  loadCaseLabel,
  nodeJLabel,
  nodeKLabel,
  nodeRows,
  elementRows,
  isPlane,
  isTruss,
  selectedNode,
  selectedElement,
  nodeIssueCounts,
  onSelectNode,
  onSelectElement,
}: WorkbenchObjectTreeProps) {
  return (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{countLabel}</span>
      </div>
      <p className="card-copy">{hint}</p>

      <div className="table-like">
        <div className="table-like__head">
          <span>ID</span>
          <span>X</span>
          <span>Y</span>
          <span>{isTruss ? diagnosticsLabel : loadCaseLabel}</span>
        </div>
        <VirtualList
          className="table-like__body"
          items={nodeRows}
          itemHeight={46}
          maxHeight={240}
          itemKey={(node) => node.id}
          renderItem={(node, index) => (
            <button
              className={[
                "table-like__row",
                selectedNode === index ? "table-like__row--active" : "",
                isTruss && (nodeIssueCounts[index] ?? 0) > 0 ? "table-like__row--warning" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => onSelectNode(index)}
              type="button"
            >
              <strong>{node.id}</strong>
              <span>{node.x.toFixed(2)}</span>
              <span>{node.y.toFixed(2)}</span>
              <span>
                {isTruss && (nodeIssueCounts[index] ?? 0) > 0 ? (
                  <span className="issue-badge">{nodeIssueCounts[index]}</span>
                ) : (
                  node.load_y.toFixed(0)
                )}
              </span>
            </button>
          )}
        />
      </div>

      <div className="table-like model-tree-spacer">
        <div className="table-like__head">
          <span>ID</span>
          <span>i</span>
          <span>{isPlane ? nodeKLabel : nodeJLabel}</span>
        </div>
        <VirtualList
          className="table-like__body"
          items={elementRows}
          itemHeight={44}
          maxHeight={240}
          itemKey={(element) => element.id}
          renderItem={(element, index) => (
            <button
              className={`table-like__row${selectedElement === index ? " table-like__row--active" : ""}`}
              onClick={() => onSelectElement(index)}
              type="button"
            >
              <strong>{element.id}</strong>
              <span>{element.node_i}</span>
              <span>{typeof element.node_k === "number" ? element.node_k : element.node_j}</span>
            </button>
          )}
        />
      </div>
    </section>
  );
}

export const WorkbenchObjectTree = memo(WorkbenchObjectTreeInner);
