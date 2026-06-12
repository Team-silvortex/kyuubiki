"use client";

import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowGraphNode } from "@/lib/api";
import {
  buildWorkflowTemplateChainPackage,
  asWorkflowTemplateChainPackage,
  packageToWorkflowTemplateChainDefinition,
} from "@/components/workbench/workflow/workbench-workflow-template-chain-package";
import {
  listBuiltInWorkflowTemplateChains,
  listStoredWorkflowTemplateChains,
  removeImportedWorkflowTemplateChain,
  saveImportedWorkflowTemplateChain,
  updateImportedWorkflowTemplateChain,
  type WorkflowTemplateChainDefinition,
} from "@/components/workbench/workflow/workbench-workflow-template-chain-library";
import {
  readWorkflowTemplateChainPreferences,
  writeWorkflowTemplateChainPreferences,
} from "@/components/workbench/workflow/workbench-workflow-template-chain-storage";
import { buildImportedTemplateChainFromNodes, buildSuggestedTemplateChainLabel } from "@/components/workbench/workflow/workbench-workflow-template-chain-build";
import { downloadJsonArtifact } from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import { readJsonFile } from "@/components/workbench/workflow/workbench-workflow-builder-import";

type WorkbenchWorkflowTemplateChainActionsProps = {
  labels: WorkflowSidebarLabels;
  selectedSourceNodeId?: string | null;
  onInsertTemplateChain: (
    chain: WorkflowTemplateChainDefinition,
    sourceNodeId?: string | null,
  ) => void;
  selectedNodes?: WorkflowGraphNode[];
};

function compactTemplateStepLabel(template: WorkflowNodeTemplateSelection) {
  const base = template.operatorId?.split(".").pop() || template.kind || "step";
  return base.replaceAll("_", " ");
}

function describeTemplateChainPreview(chain: WorkflowTemplateChainDefinition) {
  return chain.templates
    .slice(0, 4)
    .map(compactTemplateStepLabel)
    .join(" -> ");
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

function describeTemplateStep(template: WorkflowNodeTemplateSelection, index: number) {
  return `step ${index + 1}: ${compactTemplateStepLabel(template)}`;
}

function scoreTemplateChainQueryMatch(
  chain: WorkflowTemplateChainDefinition,
  normalizedQuery: string,
) {
  const haystack = [chain.id, chain.label, chain.summary ?? "", chain.tags?.join(" ") ?? ""]
    .join(" ")
    .toLowerCase();
  if (!normalizedQuery) return 0;
  if (!haystack.includes(normalizedQuery)) return -1;
  let score = haystack.startsWith(normalizedQuery) ? 180 : 0;
  if (chain.label.toLowerCase().includes(normalizedQuery)) score += 120;
  if (chain.id.toLowerCase().includes(normalizedQuery)) score += 80;
  score += (chain.tags ?? []).filter((tag) => tag.toLowerCase().includes(normalizedQuery)).length * 40;
  score += chain.templates.filter((template) => (template.operatorId ?? template.kind ?? "").toLowerCase().includes(normalizedQuery)).length * 12;
  return score;
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

  return { connections, positionByIndex, width: orderedColumns.length, height: Math.max(...orderedColumns.map(([, indexes]) => indexes.length), 1) };
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

function sortChainsByPriority(
  chains: WorkflowTemplateChainDefinition[],
  favoriteChainIds: string[],
) {
  const favoriteOrder = new Map(
    favoriteChainIds.map((chainId, index) => [chainId, index] as const),
  );
  return [...chains].sort((left, right) => {
    const leftFavorite = favoriteOrder.has(left.id);
    const rightFavorite = favoriteOrder.has(right.id);
    if (leftFavorite !== rightFavorite) return leftFavorite ? -1 : 1;
    if (leftFavorite && rightFavorite) {
      return (favoriteOrder.get(left.id) ?? 0) - (favoriteOrder.get(right.id) ?? 0);
    }
    const leftTime = left.updatedAt ?? "";
    const rightTime = right.updatedAt ?? "";
    return rightTime.localeCompare(leftTime) || left.label.localeCompare(right.label);
  });
}

export function WorkbenchWorkflowTemplateChainActions({
  labels,
  selectedSourceNodeId,
  onInsertTemplateChain,
  selectedNodes = [],
}: WorkbenchWorkflowTemplateChainActionsProps) {
  const [favoriteChainIds, setFavoriteChainIds] = useState<string[]>([]);
  const [favoriteChainAliases, setFavoriteChainAliases] = useState<Record<string, string>>({});
  const [importedChains, setImportedChains] = useState<WorkflowTemplateChainDefinition[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const importInputRef = useRef<HTMLInputElement | null>(null);
  const builtInChains = useMemo(() => listBuiltInWorkflowTemplateChains(), []);
  const availableChains = useMemo(() => [...builtInChains, ...importedChains], [builtInChains, importedChains]);

  useEffect(() => {
    const preferences = readWorkflowTemplateChainPreferences();
    setFavoriteChainIds(preferences.favoriteChainIds);
    setFavoriteChainAliases(preferences.favoriteChainAliases);
    setImportedChains(listStoredWorkflowTemplateChains());
  }, []);

  const favoriteChains = useMemo(
    () =>
      favoriteChainIds
        .map((chainId) => availableChains.find((preset) => preset.id === chainId))
        .filter(Boolean)
        .slice(0, 6) as WorkflowTemplateChainDefinition[],
    [availableChains, favoriteChainIds],
  );
  const filteredChains = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return availableChains;
    return availableChains
      .map((chain) => ({ chain, score: scoreTemplateChainQueryMatch(chain, normalized) }))
      .filter((entry) => entry.score >= 0)
      .sort((left, right) => right.score - left.score || left.chain.label.localeCompare(right.chain.label))
      .map((entry) => entry.chain);
  }, [availableChains, query]);
  const filteredBuiltInChains = useMemo(
    () => sortChainsByPriority(filteredChains.filter((chain) => chain.source === "built-in"), favoriteChainIds),
    [favoriteChainIds, filteredChains],
  );
  const filteredImportedChains = useMemo(
    () => sortChainsByPriority(filteredChains.filter((chain) => chain.source === "imported"), favoriteChainIds),
    [favoriteChainIds, filteredChains],
  );
  const pinnedFavoriteChains = useMemo(
    () => sortChainsByPriority(favoriteChains, favoriteChainIds),
    [favoriteChainIds, favoriteChains],
  );

  function insertChain(chain: WorkflowTemplateChainDefinition) {
    onInsertTemplateChain(chain, selectedSourceNodeId);
  }

  function toggleFavorite(chainId: string) {
    const next = favoriteChainIds.includes(chainId)
      ? favoriteChainIds.filter((value) => value !== chainId)
      : [chainId, ...favoriteChainIds].slice(0, 12);
    setFavoriteChainIds(next);
    writeWorkflowTemplateChainPreferences({
      favoriteChainIds: next,
      favoriteChainAliases,
    });
  }
  function renameFavorite(chainId: string) {
    const preset = availableChains.find((entry) => entry.id === chainId);
    const current = favoriteChainAliases[chainId] ?? preset?.label ?? chainId;
    const next = window.prompt(labels.templateChainRenamePrompt, current)?.trim();
    if (!next) return;
    const aliases = { ...favoriteChainAliases, [chainId]: next };
    setFavoriteChainAliases(aliases);
    writeWorkflowTemplateChainPreferences({
      favoriteChainIds,
      favoriteChainAliases: aliases,
    });
  }
  function chainDisplayLabel(preset: WorkflowTemplateChainDefinition) {
    return favoriteChainAliases[preset.id] || preset.label;
  }
  function exportChainPackage(chain: WorkflowTemplateChainDefinition) {
    downloadJsonArtifact(
      `${chain.id}.workflow-template-chain.json`,
      buildWorkflowTemplateChainPackage(chain),
    );
  }
  function deleteImportedChain(chainId: string) {
    removeImportedWorkflowTemplateChain(chainId);
    setImportedChains(listStoredWorkflowTemplateChains());
    const nextFavoriteChainIds = favoriteChainIds.filter((value) => value !== chainId);
    setFavoriteChainIds(nextFavoriteChainIds);
    const aliases = Object.fromEntries(
      Object.entries(favoriteChainAliases).filter(([key]) => key !== chainId),
    );
    setFavoriteChainAliases(aliases);
    writeWorkflowTemplateChainPreferences({
      favoriteChainIds: nextFavoriteChainIds,
      favoriteChainAliases: aliases,
    });
  }
  async function importChainPackage(file: File) {
    try {
      const json = await readJsonFile(file);
      const pkg = asWorkflowTemplateChainPackage(json);
      if (!pkg) {
        setMessage(labels.templateChainImportInvalidLabel);
        return;
      }
      saveImportedWorkflowTemplateChain(packageToWorkflowTemplateChainDefinition(pkg));
      setImportedChains(listStoredWorkflowTemplateChains());
      setMessage(labels.templateChainImportSuccessLabel);
    } catch {
      setMessage(labels.templateChainImportInvalidLabel);
    }
  }
  function handleImportChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) void importChainPackage(file);
    event.target.value = "";
  }
  function chainInsertLabel(chain: WorkflowTemplateChainDefinition) {
    if (chain.source === "imported") return chainDisplayLabel(chain);
    const chainId = chain.id;
    return chainId === "frame_2d_summary"
      ? labels.insertFrame2dSolveExportLabel
      : chainId === "thermal_frame_2d_summary"
        ? labels.insertThermalFrame2dSolveExportLabel
        : chainId === "truss_3d_summary"
          ? labels.insertTruss3dSolveExportLabel
          : chainId === "frame_3d_summary"
            ? labels.insertSolveExtractExportLabel
            : chainId === "heat_bridge_thermo"
              ? labels.insertHeatBridgeThermoLabel
              : chainId === "electrostatic_bridge_heat"
                ? labels.insertElectrostaticBridgeHeatLabel
                : chainId === "electrostatic_summary"
                  ? labels.insertElectrostaticSolveExportLabel
                  : chain.label;
  }
  function saveCurrentSelectionAsChain() {
    if (selectedNodes.length === 0) return;
    const nextLabel = window.prompt(
      labels.templateChainSaveSelectionPrompt,
      buildSuggestedTemplateChainLabel(selectedNodes),
    )?.trim();
    if (!nextLabel) return;
    saveImportedWorkflowTemplateChain(
      buildImportedTemplateChainFromNodes({ label: nextLabel, nodes: selectedNodes }),
    );
    setImportedChains(listStoredWorkflowTemplateChains());
    setMessage(labels.templateChainSaveSelectionSuccessLabel);
  }
  function renameImportedChain(chain: WorkflowTemplateChainDefinition) {
    const nextLabel = window.prompt(labels.templateChainRenamePrompt, chain.label)?.trim();
    if (!nextLabel) return;
    updateImportedWorkflowTemplateChain(chain.id, (current) => ({
      ...current,
      label: nextLabel,
    }));
    setImportedChains(listStoredWorkflowTemplateChains());
  }
  function editImportedChainSummary(chain: WorkflowTemplateChainDefinition) {
    const nextSummary = window.prompt(labels.templateChainSummaryPrompt, chain.summary ?? "");
    if (nextSummary === null) return;
    updateImportedWorkflowTemplateChain(chain.id, (current) => ({
      ...current,
      summary: nextSummary.trim() || undefined,
    }));
    setImportedChains(listStoredWorkflowTemplateChains());
  }
  function selectTag(tag: string) {
    setQuery(tag);
  }

  return (
    <div className="sidebar-stack">
      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.templateChainLibraryLabel}</span>
          <strong>{availableChains.length}</strong>
        </div>
        <label>
          <span>{labels.templateChainSearchLabel}</span>
          <input
            onChange={(event) => setQuery(event.target.value)}
            placeholder={labels.templateChainSearchPlaceholder}
            value={query}
          />
        </label>
        <div className="button-row">
          <button onClick={() => importInputRef.current?.click()} type="button">
            {labels.templateChainImportLabel}
          </button>
          <button disabled={selectedNodes.length === 0} onClick={saveCurrentSelectionAsChain} type="button">
            {labels.templateChainSaveSelectionLabel}
          </button>
        </div>
        <input accept="application/json,.json" hidden onChange={handleImportChange} ref={importInputRef} type="file" />
        {message ? <p className="card-copy">{message}</p> : null}
      </div>
      {favoriteChains.length > 0 ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.templateChainPinnedLabel}</span>
            <strong>{pinnedFavoriteChains.length}</strong>
          </div>
          <div className="button-row button-row--adaptive">
            {pinnedFavoriteChains.map((preset) => (
              <div key={`favorite-chain:${preset.id}`} className="sidebar-card sidebar-card--compact">
                <div className="sidebar-list__row">
                  <span>{chainDisplayLabel(preset)}</span>
                  <strong>{preset.templates.length}</strong>
                </div>
                <WorkbenchWorkflowTemplateChainPreview chain={preset} />
                <p className="card-copy">{describeTemplateChainPreview(preset)}</p>
                <TemplateChainTagRow activeQuery={query} label={labels.templateChainTagsLabel} onSelectTag={selectTag} tags={preset.tags} />
                {preset.summary ? <p className="card-copy">{preset.summary}</p> : null}
                <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                  <button
                    onClick={() => insertChain(preset)}
                    type="button"
                  >
                    {chainDisplayLabel(preset)}
                  </button>
                  <button onClick={() => exportChainPackage(preset)} type="button">
                    {labels.templateChainExportLabel}
                  </button>
                  <button onClick={() => renameFavorite(preset.id)} type="button">
                    {labels.templateChainRenameLabel}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : null}
      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.templateChainBuiltInLabel}</span>
          <strong>{filteredBuiltInChains.length}</strong>
        </div>
        <div className="button-row button-row--adaptive">
          {filteredBuiltInChains.map((preset) => {
            const favorite = favoriteChainIds.includes(preset.id);
            return (
              <div key={preset.id} className="sidebar-card sidebar-card--compact">
                <div className="sidebar-list__row">
                  <span>{chainInsertLabel(preset)}</span>
                  <strong>{preset.templates.length}</strong>
                </div>
                <WorkbenchWorkflowTemplateChainPreview chain={preset} />
                <p className="card-copy">{describeTemplateChainPreview(preset)}</p>
                <TemplateChainTagRow activeQuery={query} label={labels.templateChainTagsLabel} onSelectTag={selectTag} tags={preset.tags} />
                {preset.summary ? <p className="card-copy">{preset.summary}</p> : null}
                <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                  <button onClick={() => insertChain(preset)} type="button">
                    {chainInsertLabel(preset)}
                  </button>
                  <button onClick={() => exportChainPackage(preset)} type="button">
                    {labels.templateChainExportLabel}
                  </button>
                  <button onClick={() => toggleFavorite(preset.id)} type="button">
                    {favorite
                      ? labels.templateChainFavoriteRemoveLabel
                      : labels.templateChainFavoriteAddLabel}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </div>
      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.templateChainAllLabel}</span>
          <strong>{filteredChains.length}</strong>
        </div>
      </div>
      <div className="button-row button-row--adaptive">
        {filteredImportedChains.map((preset) => {
          const favorite = favoriteChainIds.includes(preset.id);
          return (
            <div key={preset.id} className="sidebar-card sidebar-card--compact">
              <div className="sidebar-list__row">
                <span>{chainInsertLabel(preset)}</span>
                <strong>{preset.templates.length}</strong>
              </div>
              <WorkbenchWorkflowTemplateChainPreview chain={preset} />
              <p className="card-copy">{describeTemplateChainPreview(preset)}</p>
              <TemplateChainTagRow activeQuery={query} label={labels.templateChainTagsLabel} onSelectTag={selectTag} tags={preset.tags} />
              {preset.summary ? <p className="card-copy">{preset.summary}</p> : null}
              <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                <button onClick={() => insertChain(preset)} type="button">
                  {chainInsertLabel(preset)}
                </button>
                <button onClick={() => exportChainPackage(preset)} type="button">
                  {labels.templateChainExportLabel}
                </button>
                <button onClick={() => toggleFavorite(preset.id)} type="button">
                  {favorite
                    ? labels.templateChainFavoriteRemoveLabel
                    : labels.templateChainFavoriteAddLabel}
                </button>
                <button onClick={() => renameImportedChain(preset)} type="button">
                  {labels.templateChainRenameTemplateLabel}
                </button>
                <button onClick={() => editImportedChainSummary(preset)} type="button">
                  {labels.templateChainSummaryEditLabel}
                </button>
                <button onClick={() => deleteImportedChain(preset.id)} type="button">
                  {labels.templateChainDeleteImportedLabel}
                </button>
              </div>
            </div>
          );
        })}
      </div>
      {importedChains.length > 0 ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.templateChainImportedLabel}</span>
            <strong>{importedChains.length}</strong>
          </div>
          {importedChains.map((chain) => (
            <div className="sidebar-list__row" key={`imported-summary:${chain.id}`}>
              <span>{chainDisplayLabel(chain)}</span>
              <strong>{chain.summary ?? "--"}</strong>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
