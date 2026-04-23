"use client";

import { VirtualList } from "@/components/ui/virtual-list";

type Truss3dTreeNodeRow = {
  index: number;
  id: string;
  x: string;
  y: string;
  z: string;
  active: boolean;
  draft: boolean;
};

type Truss3dTreeElementRow = {
  index: number;
  id: string;
  nodeI: string | number;
  nodeJ: string | number;
  area: string;
  active: boolean;
};

type WorkbenchTruss3dTreeCardProps = {
  title: string;
  countLabel: string;
  hint: string;
  nodeILabel: string;
  nodeJLabel: string;
  areaLabel: string;
  nodes: Truss3dTreeNodeRow[];
  elements: Truss3dTreeElementRow[];
  onSelectNode: (index: number) => void;
  onSelectElement: (index: number) => void;
};

export function WorkbenchTruss3dTreeCard({
  title,
  countLabel,
  hint,
  nodeILabel,
  nodeJLabel,
  areaLabel,
  nodes,
  elements,
  onSelectNode,
  onSelectElement,
}: WorkbenchTruss3dTreeCardProps) {
  return (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{countLabel}</span>
      </div>
      <p className="card-copy">{hint}</p>
      <div className="table-like">
        <div className="table-like__head table-like__head--space">
          <span>ID</span>
          <span>X</span>
          <span>Y</span>
          <span>Z</span>
        </div>
        <VirtualList
          className="table-like__body"
          items={nodes}
          itemHeight={46}
          maxHeight={240}
          itemKey={(node) => node.id}
          renderItem={(node) => (
            <button
              className={`table-like__row table-like__row--space${node.active ? " table-like__row--active" : ""}${node.draft ? " table-like__row--draft" : ""}`}
              onClick={() => onSelectNode(node.index)}
              type="button"
            >
              <strong>{node.id}</strong>
              <span>{node.x}</span>
              <span>{node.y}</span>
              <span>{node.z}</span>
            </button>
          )}
        />
      </div>
      <div className="table-like model-tree-spacer">
        <div className="table-like__head">
          <span>ID</span>
          <span>{nodeILabel}</span>
          <span>{nodeJLabel}</span>
          <span>{areaLabel}</span>
        </div>
        <VirtualList
          className="table-like__body"
          items={elements}
          itemHeight={44}
          maxHeight={240}
          itemKey={(element) => element.id}
          renderItem={(element) => (
            <button
              className={`table-like__row${element.active ? " table-like__row--active" : ""}`}
              onClick={() => onSelectElement(element.index)}
              type="button"
            >
              <strong>{element.id}</strong>
              <span>{element.nodeI}</span>
              <span>{element.nodeJ}</span>
              <span>{element.area}</span>
            </button>
          )}
        />
      </div>
    </section>
  );
}
