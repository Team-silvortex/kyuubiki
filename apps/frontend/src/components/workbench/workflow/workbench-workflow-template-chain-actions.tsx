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
    templates: WorkflowNodeTemplateSelection[],
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
    return availableChains.filter((chain) => {
      const operatorText = chain.templates
        .map((template) => template.operatorId ?? template.kind ?? "")
        .join(" ");
      return [chain.id, chain.label, chain.summary ?? "", operatorText, chain.tags?.join(" ") ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(normalized);
    });
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

  function insertChain(templates: WorkflowNodeTemplateSelection[]) {
    onInsertTemplateChain(templates, selectedSourceNodeId);
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
                  : chainId;
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
                <p className="card-copy">{describeTemplateChainPreview(preset)}</p>
                <TemplateChainTagRow activeQuery={query} label={labels.templateChainTagsLabel} onSelectTag={selectTag} tags={preset.tags} />
                {preset.summary ? <p className="card-copy">{preset.summary}</p> : null}
                <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                  <button
                    onClick={() => insertChain(preset.templates)}
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
                <p className="card-copy">{describeTemplateChainPreview(preset)}</p>
                <TemplateChainTagRow activeQuery={query} label={labels.templateChainTagsLabel} onSelectTag={selectTag} tags={preset.tags} />
                {preset.summary ? <p className="card-copy">{preset.summary}</p> : null}
                <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                  <button onClick={() => insertChain(preset.templates)} type="button">
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
              <p className="card-copy">{describeTemplateChainPreview(preset)}</p>
              <TemplateChainTagRow activeQuery={query} label={labels.templateChainTagsLabel} onSelectTag={selectTag} tags={preset.tags} />
              {preset.summary ? <p className="card-copy">{preset.summary}</p> : null}
              <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                <button onClick={() => insertChain(preset.templates)} type="button">
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
