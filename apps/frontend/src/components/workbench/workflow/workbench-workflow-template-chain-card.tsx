"use client";

import { useState } from "react";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

function compactTemplateStepLabel(template: WorkflowNodeTemplateSelection) {
  const base = template.operatorId?.split(".").pop() || template.kind || "step";
  return base.replaceAll("_", " ");
}

function describeTemplateStep(template: WorkflowNodeTemplateSelection, index: number) {
  return `step ${index + 1}: ${compactTemplateStepLabel(template)}`;
}

function summarizeTemplateKind(template: WorkflowNodeTemplateSelection) {
  const operatorId = template.operatorId ?? "";
  if (operatorId.startsWith("solve.")) return "solve";
  if (operatorId.startsWith("bridge.")) return "bridge";
  if (operatorId.startsWith("extract.")) return "extract";
  if (operatorId.startsWith("export.")) return "export";
  if (operatorId.startsWith("transform.")) return "merge";
  return template.kind || "step";
}

function describeTemplateChainPreview(chain: WorkflowTemplateChainDefinition) {
  return chain.templates
    .slice(0, 4)
    .map(compactTemplateStepLabel)
    .join(" -> ");
}

function buildTemplateChainPreviewLayout(chain: WorkflowTemplateChainDefinition) {
  const connections =
    chain.connections?.length
      ? chain.connections
      : chain.templates.slice(1).map((_, index) => ({ from: index, to: index + 1 }));
  const columnByIndex = new Map<number, number>();
  for (let index = 0; index < chain.templates.length; index += 1) {
    columnByIndex.set(index, 0);
  }
  let changed = true;
  while (changed) {
    changed = false;
    for (const connection of connections) {
      const nextColumn = (columnByIndex.get(connection.from) ?? 0) + 1;
      if ((columnByIndex.get(connection.to) ?? 0) < nextColumn) {
        columnByIndex.set(connection.to, nextColumn);
        changed = true;
      }
    }
  }

  const groupedRows = new Map<number, number[]>();
  for (let index = 0; index < chain.templates.length; index += 1) {
    const column = columnByIndex.get(index) ?? 0;
    const row = groupedRows.get(column) ?? [];
    row.push(index);
    groupedRows.set(column, row);
  }

  const positionByIndex = new Map<number, { x: number; y: number }>();
  const orderedColumns = [...groupedRows.entries()].sort((left, right) => left[0] - right[0]);
  for (const [column, indexes] of orderedColumns) {
    indexes.forEach((index, row) => {
      positionByIndex.set(index, { x: column, y: row });
    });
  }

  return {
    connections,
    positionByIndex,
    width: orderedColumns.length,
    height: Math.max(...orderedColumns.map(([, indexes]) => indexes.length), 1),
  };
}

function TemplateChainTagRow(props: {
  tags?: string[];
  label: string;
  onSelectTag: (tag: string) => void;
  activeQuery: string;
}) {
  const { tags, label, onSelectTag, activeQuery } = props;
  if (!tags || tags.length === 0) return null;
  return (
    <div className="card-copy" style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", alignItems: "center" }}>
      <span>{label}:</span>
      {tags.map((tag) => (
        <button
          key={tag}
          onClick={() => onSelectTag(tag)}
          type="button"
          style={activeQuery.trim().toLowerCase() === tag.toLowerCase() ? { outline: "1px solid var(--accent, #4f46e5)" } : undefined}
        >
          {tag}
        </button>
      ))}
    </div>
  );
}

function WorkbenchWorkflowTemplateChainPreview(props: {
  chain: WorkflowTemplateChainDefinition;
}) {
  const { chain } = props;
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const { connections, positionByIndex, width, height } = buildTemplateChainPreviewLayout(chain);
  const cellWidth = 84;
  const cellHeight = 40;
  const padding = 16;
  const svgWidth = padding * 2 + Math.max(width - 1, 0) * cellWidth + 72;
  const svgHeight = padding * 2 + Math.max(height - 1, 0) * cellHeight + 28;
  const activeTemplate = activeIndex === null ? null : chain.templates[activeIndex];

  return (
    <div
      aria-label={`${chain.label} topology preview`}
      style={{
        border: "1px solid rgba(148, 163, 184, 0.22)",
        borderRadius: "0.7rem",
        padding: "0.35rem",
        background:
          "linear-gradient(180deg, rgba(15, 23, 42, 0.18), rgba(15, 23, 42, 0.08))",
      }}
    >
      <svg
        viewBox={`0 0 ${svgWidth} ${svgHeight}`}
        width="100%"
        height={Math.min(svgHeight, 144)}
        role="img"
      >
        {connections.map((connection, index) => {
          const from = positionByIndex.get(connection.from);
          const to = positionByIndex.get(connection.to);
          if (!from || !to) return null;
          const x1 = padding + from.x * cellWidth + 56;
          const y1 = padding + from.y * cellHeight + 14;
          const x2 = padding + to.x * cellWidth;
          const y2 = padding + to.y * cellHeight + 14;
          const midX = x1 + (x2 - x1) / 2;
          const active =
            activeIndex !== null &&
            (connection.from === activeIndex || connection.to === activeIndex);
          return (
            <path
              key={`edge:${chain.id}:${index}`}
              d={`M ${x1} ${y1} C ${midX} ${y1}, ${midX} ${y2}, ${x2} ${y2}`}
              fill="none"
              stroke={active ? "rgba(96, 165, 250, 0.96)" : "rgba(148, 163, 184, 0.7)"}
              strokeWidth={active ? "2" : "1.5"}
            />
          );
        })}
        {chain.templates.map((template, index) => {
          const position = positionByIndex.get(index);
          if (!position) return null;
          const x = padding + position.x * cellWidth;
          const y = padding + position.y * cellHeight;
          const active = index === activeIndex;
          return (
            <g
              key={`node:${chain.id}:${index}`}
              transform={`translate(${x}, ${y})`}
              onMouseEnter={() => setActiveIndex(index)}
              onMouseLeave={() => setActiveIndex((current) => (current === index ? null : current))}
            >
              <rect
                width="56"
                height="28"
                rx="8"
                fill={active ? "rgba(30, 64, 175, 0.92)" : "rgba(30, 41, 59, 0.94)"}
                stroke={active ? "rgba(191, 219, 254, 0.88)" : "rgba(96, 165, 250, 0.38)"}
              />
              <text
                x="28"
                y="18"
                textAnchor="middle"
                fontSize="10"
                fill="rgba(226, 232, 240, 0.92)"
              >
                {summarizeTemplateKind(template)}
              </text>
              <title>{describeTemplateStep(template, index)}</title>
            </g>
          );
        })}
      </svg>
      <p className="card-copy" style={{ margin: "0.2rem 0 0", minHeight: "1.1rem" }}>
        {activeTemplate && activeIndex !== null
          ? describeTemplateStep(activeTemplate, activeIndex)
          : `${chain.templates.length} steps${chain.connections?.length ? `, ${chain.connections.length} links` : ""}`}
      </p>
    </div>
  );
}

export function WorkbenchWorkflowTemplateChainCard(props: {
  activeQuery: string;
  chain: WorkflowTemplateChainDefinition;
  favorite?: boolean;
  labels: WorkflowSidebarLabels;
  onExport: () => void;
  onInsert: () => void;
  onPrimaryAction: () => void;
  onPrimaryLabel: string;
  onSelectTag: (tag: string) => void;
}) {
  const {
    activeQuery,
    chain,
    labels,
    onExport,
    onInsert,
    onPrimaryAction,
    onPrimaryLabel,
    onSelectTag,
  } = props;

  return (
    <div className="sidebar-card sidebar-card--compact">
      <div className="sidebar-list__row">
        <span>{onPrimaryLabel}</span>
        <strong>{chain.templates.length}</strong>
      </div>
      <WorkbenchWorkflowTemplateChainPreview chain={chain} />
      <p className="card-copy">{describeTemplateChainPreview(chain)}</p>
      <TemplateChainTagRow
        activeQuery={activeQuery}
        label={labels.templateChainTagsLabel}
        onSelectTag={onSelectTag}
        tags={chain.tags}
      />
      {chain.summary ? <p className="card-copy">{chain.summary}</p> : null}
      <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
        <button onClick={onInsert} type="button">
          {onPrimaryLabel}
        </button>
        <button onClick={onExport} type="button">
          {labels.templateChainExportLabel}
        </button>
        <button onClick={onPrimaryAction} type="button">
          {props.favorite === undefined
            ? labels.templateChainRenameLabel
            : props.favorite
              ? labels.templateChainFavoriteRemoveLabel
              : labels.templateChainFavoriteAddLabel}
        </button>
      </div>
    </div>
  );
}
