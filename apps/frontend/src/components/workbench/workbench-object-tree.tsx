"use client";

import { memo, useState } from "react";
import { VirtualList } from "@/components/ui/virtual-list";

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
  resultValue?: string;
  resultMagnitude?: number;
};

type WorkbenchObjectTreeProps = {
  title: string;
  countLabel: string;
  hint: string;
  geometryLabel?: string;
  resultsLabel?: string;
  sortByLabel?: string;
  diagnosticsLabel: string;
  loadCaseLabel: string;
  nodeJLabel: string;
  nodeKLabel: string;
  elementValueLabel?: string;
  nodeRows: NodeRow[];
  elementRows: ElementRow[];
  isPlane: boolean;
  isTruss: boolean;
  enableElementResultsMode?: boolean;
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
  geometryLabel,
  resultsLabel,
  sortByLabel,
  diagnosticsLabel,
  loadCaseLabel,
  nodeJLabel,
  nodeKLabel,
  elementValueLabel,
  nodeRows,
  elementRows,
  isPlane,
  isTruss,
  enableElementResultsMode,
  selectedNode,
  selectedElement,
  nodeIssueCounts,
  onSelectNode,
  onSelectElement,
}: WorkbenchObjectTreeProps) {
  const [elementTreeMode, setElementTreeMode] = useState<"geometry" | "results">("geometry");
  const [resultSortMode, setResultSortMode] = useState<"index" | "value">("index");
  const showElementResults = Boolean(enableElementResultsMode && elementValueLabel && elementTreeMode === "results");
  const visibleElementRows =
    showElementResults && resultSortMode === "value"
      ? [...elementRows].sort((left, right) => (right.resultMagnitude ?? Number.NEGATIVE_INFINITY) - (left.resultMagnitude ?? Number.NEGATIVE_INFINITY))
      : elementRows;

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
        {enableElementResultsMode ? (
          <div className="panel-tabs">
            <button className={`panel-tab${elementTreeMode === "geometry" ? " panel-tab--active" : ""}`} onClick={() => setElementTreeMode("geometry")} type="button">
              {geometryLabel ?? "Geometry"}
            </button>
            <button className={`panel-tab${elementTreeMode === "results" ? " panel-tab--active" : ""}`} onClick={() => setElementTreeMode("results")} type="button">
              {resultsLabel ?? "Results"}
            </button>
          </div>
        ) : null}
        {showElementResults ? (
          <div className="button-row">
            <span className="card-copy">{sortByLabel ?? "Sort by"}</span>
            <button className={`ghost-button ghost-button--compact${resultSortMode === "index" ? " ghost-button--active" : ""}`} onClick={() => setResultSortMode("index")} type="button">#</button>
            <button className={`ghost-button ghost-button--compact${resultSortMode === "value" ? " ghost-button--active" : ""}`} onClick={() => setResultSortMode("value")} type="button">
              {elementValueLabel}
            </button>
          </div>
        ) : null}
        <div className={`table-like__head${showElementResults ? " table-like__head--wide" : ""}`}>
          <span>ID</span>
          <span>i</span>
          <span>{isPlane ? nodeKLabel : nodeJLabel}</span>
          {showElementResults ? <span>{elementValueLabel}</span> : null}
        </div>
        <VirtualList
          className="table-like__body"
          items={visibleElementRows}
          itemHeight={44}
          maxHeight={240}
          itemKey={(element) => element.id}
          renderItem={(element, index) => (
            <button
              className={`${showElementResults ? "table-like__row table-like__row--wide" : "table-like__row"}${selectedElement === index ? " table-like__row--active" : ""}`}
              onClick={() => onSelectElement(index)}
              type="button"
            >
              <strong>{element.id}</strong>
              <span>{element.node_i}</span>
              <span>{typeof element.node_k === "number" ? element.node_k : element.node_j}</span>
              {showElementResults ? <span>{element.resultValue ?? "--"}</span> : null}
            </button>
          )}
        />
      </div>
    </section>
  );
}

export const WorkbenchObjectTree = memo(WorkbenchObjectTreeInner);
