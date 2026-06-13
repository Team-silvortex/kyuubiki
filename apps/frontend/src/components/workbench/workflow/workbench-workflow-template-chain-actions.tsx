"use client";

import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowGraphNode } from "@/lib/api";
import {
  asWorkflowTemplateChainPackage,
  buildWorkflowTemplateChainPackage,
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
import { groupTemplateChainsByDomain } from "@/components/workbench/workflow/workbench-workflow-domain-groups";
import {
  buildImportedTemplateChainFromNodes,
  buildSuggestedTemplateChainLabel,
} from "@/components/workbench/workflow/workbench-workflow-template-chain-build";
import { WorkbenchWorkflowTemplateChainCard } from "@/components/workbench/workflow/workbench-workflow-template-chain-card";
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
  score +=
    (chain.tags ?? []).filter((tag) => tag.toLowerCase().includes(normalizedQuery)).length * 40;
  score +=
    chain.templates.filter((template) =>
      (template.operatorId ?? template.kind ?? "").toLowerCase().includes(normalizedQuery),
    ).length * 12;
  return score;
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
    return (
      (right.updatedAt ?? "").localeCompare(left.updatedAt ?? "") ||
      left.label.localeCompare(right.label)
    );
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
  const availableChains = useMemo(
    () => [...builtInChains, ...importedChains],
    [builtInChains, importedChains],
  );

  useEffect(() => {
    const preferences = readWorkflowTemplateChainPreferences();
    setFavoriteChainIds(preferences.favoriteChainIds);
    setFavoriteChainAliases(preferences.favoriteChainAliases);
    setImportedChains(listStoredWorkflowTemplateChains());
  }, []);

  const filteredChains = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return availableChains;
    return availableChains
      .map((chain) => ({ chain, score: scoreTemplateChainQueryMatch(chain, normalized) }))
      .filter((entry) => entry.score >= 0)
      .sort(
        (left, right) =>
          right.score - left.score || left.chain.label.localeCompare(right.chain.label),
      )
      .map((entry) => entry.chain);
  }, [availableChains, query]);

  const favoriteChains = useMemo(
    () =>
      favoriteChainIds
        .map((chainId) => availableChains.find((entry) => entry.id === chainId))
        .filter(Boolean)
        .slice(0, 6) as WorkflowTemplateChainDefinition[],
    [availableChains, favoriteChainIds],
  );
  const pinnedFavoriteChains = useMemo(
    () => sortChainsByPriority(favoriteChains, favoriteChainIds),
    [favoriteChainIds, favoriteChains],
  );
  const filteredBuiltInChains = useMemo(
    () =>
      sortChainsByPriority(
        filteredChains.filter((chain) => chain.source === "built-in"),
        favoriteChainIds,
      ),
    [favoriteChainIds, filteredChains],
  );
  const filteredImportedChains = useMemo(
    () =>
      sortChainsByPriority(
        filteredChains.filter((chain) => chain.source === "imported"),
        favoriteChainIds,
      ),
    [favoriteChainIds, filteredChains],
  );
  const groupedBuiltInChains = useMemo(
    () => groupTemplateChainsByDomain(filteredBuiltInChains),
    [filteredBuiltInChains],
  );

  function writePreferences(nextIds: string[], nextAliases = favoriteChainAliases) {
    setFavoriteChainIds(nextIds);
    setFavoriteChainAliases(nextAliases);
    writeWorkflowTemplateChainPreferences({
      favoriteChainIds: nextIds,
      favoriteChainAliases: nextAliases,
    });
  }

  function insertChain(chain: WorkflowTemplateChainDefinition) {
    onInsertTemplateChain(chain, selectedSourceNodeId);
  }

  function toggleFavorite(chainId: string) {
    const next = favoriteChainIds.includes(chainId)
      ? favoriteChainIds.filter((value) => value !== chainId)
      : [chainId, ...favoriteChainIds].slice(0, 12);
    writePreferences(next);
  }

  function renameFavorite(chainId: string) {
    const preset = availableChains.find((entry) => entry.id === chainId);
    const current = favoriteChainAliases[chainId] ?? preset?.label ?? chainId;
    const next = window.prompt(labels.templateChainRenamePrompt, current)?.trim();
    if (!next) return;
    writePreferences(favoriteChainIds, { ...favoriteChainAliases, [chainId]: next });
  }

  function deleteImportedChain(chainId: string) {
    removeImportedWorkflowTemplateChain(chainId);
    setImportedChains(listStoredWorkflowTemplateChains());
    writePreferences(
      favoriteChainIds.filter((value) => value !== chainId),
      Object.fromEntries(
        Object.entries(favoriteChainAliases).filter(([key]) => key !== chainId),
      ),
    );
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

  function chainDisplayLabel(chain: WorkflowTemplateChainDefinition) {
    return favoriteChainAliases[chain.id] || chain.label;
  }

  function exportChainPackage(chain: WorkflowTemplateChainDefinition) {
    downloadJsonArtifact(
      `${chain.id}.workflow-template-chain.json`,
      buildWorkflowTemplateChainPackage(chain),
    );
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
                : chainId === "electrostatic_heat_thermo_summary"
                  ? labels.insertElectrostaticHeatThermoSummaryLabel
                  : chainId === "electrostatic_triangle_heat_thermo_triangle_summary"
                    ? labels.insertElectrostaticTriangleHeatThermoTriangleSummaryLabel
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
          <button
            disabled={selectedNodes.length === 0}
            onClick={saveCurrentSelectionAsChain}
            type="button"
          >
            {labels.templateChainSaveSelectionLabel}
          </button>
        </div>
        <input
          accept="application/json,.json"
          hidden
          onChange={handleImportChange}
          ref={importInputRef}
          type="file"
        />
        {message ? <p className="card-copy">{message}</p> : null}
      </div>

      {pinnedFavoriteChains.length > 0 ? (
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{labels.templateChainPinnedLabel}</span>
            <strong>{pinnedFavoriteChains.length}</strong>
          </div>
          <div className="button-row button-row--adaptive">
            {pinnedFavoriteChains.map((chain) => (
              <WorkbenchWorkflowTemplateChainCard
                activeQuery={query}
                chain={chain}
                key={`favorite-chain:${chain.id}`}
                labels={labels}
                onExport={() => exportChainPackage(chain)}
                onInsert={() => insertChain(chain)}
                onPrimaryAction={() => renameFavorite(chain.id)}
                onPrimaryLabel={chainDisplayLabel(chain)}
                onSelectTag={selectTag}
              />
            ))}
          </div>
        </div>
      ) : null}

      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.templateChainBuiltInLabel}</span>
          <strong>{filteredBuiltInChains.length}</strong>
        </div>
        {groupedBuiltInChains.map((group) => (
          <div className="sidebar-stack" key={group.key}>
            <div className="sidebar-list__row">
              <span>{group.label}</span>
              <strong>{group.entries.length}</strong>
            </div>
            <div className="button-row button-row--adaptive">
              {group.entries.map((chain) => (
                <WorkbenchWorkflowTemplateChainCard
                  activeQuery={query}
                  chain={chain}
                  favorite={favoriteChainIds.includes(chain.id)}
                  key={chain.id}
                  labels={labels}
                  onExport={() => exportChainPackage(chain)}
                  onInsert={() => insertChain(chain)}
                  onPrimaryAction={() => toggleFavorite(chain.id)}
                  onPrimaryLabel={chainInsertLabel(chain)}
                  onSelectTag={selectTag}
                />
              ))}
            </div>
          </div>
        ))}
      </div>

      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.templateChainAllLabel}</span>
          <strong>{filteredChains.length}</strong>
        </div>
      </div>

      <div className="button-row button-row--adaptive">
        {filteredImportedChains.map((chain) => (
          <div key={chain.id} className="sidebar-card sidebar-card--compact">
            <div className="sidebar-list__row">
              <span>{chainInsertLabel(chain)}</span>
              <strong>{chain.templates.length}</strong>
            </div>
            {chain.summary ? <p className="card-copy">{chain.summary}</p> : null}
            <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
              <button onClick={() => insertChain(chain)} type="button">
                {chainInsertLabel(chain)}
              </button>
              <button onClick={() => exportChainPackage(chain)} type="button">
                {labels.templateChainExportLabel}
              </button>
              <button onClick={() => renameImportedChain(chain)} type="button">
                {labels.templateChainRenameTemplateLabel}
              </button>
              <button onClick={() => editImportedChainSummary(chain)} type="button">
                {labels.templateChainSummaryEditLabel}
              </button>
              <button onClick={() => deleteImportedChain(chain.id)} type="button">
                {labels.templateChainDeleteImportedLabel}
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
