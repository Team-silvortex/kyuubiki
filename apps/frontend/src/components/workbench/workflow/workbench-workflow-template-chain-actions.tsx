"use client";

import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert, upsertWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import { WorkbenchPanelNotice } from "@/components/workbench/workbench-panel-notice";
import {
  showWorkbenchNotice,
  type WorkbenchNoticeItem,
} from "@/components/workbench/workbench-notice-state";
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
import { scoreWorkflowTemplateChainSearch } from "@/components/workbench/workflow/workbench-workflow-template-chain-search";
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
  setSystemAlerts: Dispatch<SetStateAction<WorkbenchAlertItem[]>>;
};

const TEMPLATE_CHAIN_STORAGE_ALERT_ID = "workflow-template-chain-storage-write-failed";
const TEMPLATE_CHAIN_PAGE_SIZE = 5;

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
  setSystemAlerts,
}: WorkbenchWorkflowTemplateChainActionsProps) {
  const [favoriteChainIds, setFavoriteChainIds] = useState<string[]>([]);
  const [favoriteChainAliases, setFavoriteChainAliases] = useState<Record<string, string>>({});
  const [importedChains, setImportedChains] = useState<WorkflowTemplateChainDefinition[]>([]);
  const [notice, setNotice] = useState<WorkbenchNoticeItem | null>(null);
  const [query, setQuery] = useState("");
  const [builtInPage, setBuiltInPage] = useState(0);
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
    const normalized = query.trim();
    if (!normalized) return availableChains;
    return availableChains
      .flatMap((chain) => {
        const score = scoreWorkflowTemplateChainSearch(chain, normalized);
        return score == null ? [] : [{ chain, score }];
      })
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
        .filter((chain) => filteredChains.some((entry) => entry.id === chain?.id))
        .slice(0, 3) as WorkflowTemplateChainDefinition[],
    [availableChains, favoriteChainIds, filteredChains],
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
  const catalogBuiltInChains = useMemo(
    () => filteredBuiltInChains.filter((chain) => !favoriteChainIds.includes(chain.id)),
    [favoriteChainIds, filteredBuiltInChains],
  );
  const builtInPageCount = Math.max(1, Math.ceil(catalogBuiltInChains.length / TEMPLATE_CHAIN_PAGE_SIZE));
  const visibleBuiltInChains = useMemo(
    () => catalogBuiltInChains.slice(
      builtInPage * TEMPLATE_CHAIN_PAGE_SIZE,
      (builtInPage + 1) * TEMPLATE_CHAIN_PAGE_SIZE,
    ),
    [builtInPage, catalogBuiltInChains],
  );
  const groupedBuiltInChains = useMemo(
    () => groupTemplateChainsByDomain(visibleBuiltInChains),
    [visibleBuiltInChains],
  );

  useEffect(() => setBuiltInPage(0), [query]);
  useEffect(() => {
    setBuiltInPage((current) => Math.min(current, builtInPageCount - 1));
  }, [builtInPageCount]);

  function reportStorageWriteFailure() {
    upsertWorkbenchAlert(setSystemAlerts, {
      id: TEMPLATE_CHAIN_STORAGE_ALERT_ID,
      message: labels.storageWriteFailedLabel,
      tone: "warning",
    });
    showWorkbenchNotice(setNotice, {
      id: TEMPLATE_CHAIN_STORAGE_ALERT_ID,
      message: labels.storageWriteFailedLabel,
      tone: "warning",
    });
  }

  function clearStorageWriteFailure() {
    dismissWorkbenchAlert(setSystemAlerts, TEMPLATE_CHAIN_STORAGE_ALERT_ID);
  }

  function writePreferences(nextIds: string[], nextAliases = favoriteChainAliases) {
    const persisted = writeWorkflowTemplateChainPreferences({
      favoriteChainIds: nextIds,
      favoriteChainAliases: nextAliases,
    });
    if (persisted) {
      setFavoriteChainIds(nextIds);
      setFavoriteChainAliases(nextAliases);
      clearStorageWriteFailure();
    } else {
      reportStorageWriteFailure();
    }
    return persisted;
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
    if (!removeImportedWorkflowTemplateChain(chainId)) {
      reportStorageWriteFailure();
      return;
    }
    clearStorageWriteFailure();
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
        upsertWorkbenchAlert(setSystemAlerts, {
          id: "workflow-template-chain-import-error",
          message: labels.templateChainImportInvalidLabel,
          tone: "error",
        });
        showWorkbenchNotice(setNotice, {
          id: "workflow-template-chain-import-invalid",
          message: labels.templateChainImportInvalidLabel,
          tone: "error",
        });
        return;
      }
      const saved = saveImportedWorkflowTemplateChain(
        packageToWorkflowTemplateChainDefinition(pkg),
      );
      if (!saved) {
        reportStorageWriteFailure();
        return;
      }
      setImportedChains(listStoredWorkflowTemplateChains());
      clearStorageWriteFailure();
      dismissWorkbenchAlert(setSystemAlerts, "workflow-template-chain-import-error");
      showWorkbenchNotice(setNotice, {
        id: "workflow-template-chain-import-success",
        message: labels.templateChainImportSuccessLabel,
        tone: "info",
      });
    } catch {
      upsertWorkbenchAlert(setSystemAlerts, {
        id: "workflow-template-chain-import-error",
        message: labels.templateChainImportInvalidLabel,
        tone: "error",
      });
      showWorkbenchNotice(setNotice, {
        id: "workflow-template-chain-import-invalid",
        message: labels.templateChainImportInvalidLabel,
        tone: "error",
      });
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
    const saved = saveImportedWorkflowTemplateChain(
      buildImportedTemplateChainFromNodes({ label: nextLabel, nodes: selectedNodes }),
    );
    if (!saved) {
      reportStorageWriteFailure();
      return;
    }
    setImportedChains(listStoredWorkflowTemplateChains());
    clearStorageWriteFailure();
    showWorkbenchNotice(setNotice, {
      id: "workflow-template-chain-save-selection-success",
      message: labels.templateChainSaveSelectionSuccessLabel,
      tone: "info",
    });
  }

  function renameImportedChain(chain: WorkflowTemplateChainDefinition) {
    const nextLabel = window.prompt(labels.templateChainRenamePrompt, chain.label)?.trim();
    if (!nextLabel) return;
    const updated = updateImportedWorkflowTemplateChain(chain.id, (current) => ({
      ...current,
      label: nextLabel,
    }));
    if (!updated) {
      reportStorageWriteFailure();
      return;
    }
    setImportedChains(listStoredWorkflowTemplateChains());
    clearStorageWriteFailure();
  }

  function editImportedChainSummary(chain: WorkflowTemplateChainDefinition) {
    const nextSummary = window.prompt(labels.templateChainSummaryPrompt, chain.summary ?? "");
    if (nextSummary === null) return;
    const updated = updateImportedWorkflowTemplateChain(chain.id, (current) => ({
      ...current,
      summary: nextSummary.trim() || undefined,
    }));
    if (!updated) {
      reportStorageWriteFailure();
      return;
    }
    setImportedChains(listStoredWorkflowTemplateChains());
    clearStorageWriteFailure();
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
            data-workflow-template-chain-search="query"
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
        <WorkbenchPanelNotice notice={notice} setNotice={setNotice} />
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
          <strong>{catalogBuiltInChains.length}</strong>
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
        <div className="workflow-template-chain-pager" data-workflow-template-chain-page={builtInPage + 1}>
          <button
            aria-label={`${labels.templateChainBuiltInLabel} ${builtInPage}`}
            data-workflow-template-chain-page-action="previous"
            disabled={builtInPage === 0}
            onClick={() => setBuiltInPage((current) => Math.max(0, current - 1))}
            type="button"
          >
            ←
          </button>
          <strong>{builtInPage + 1}/{builtInPageCount}</strong>
          <button
            aria-label={`${labels.templateChainBuiltInLabel} ${builtInPage + 2}`}
            data-workflow-template-chain-page-action="next"
            disabled={builtInPage + 1 >= builtInPageCount}
            onClick={() => setBuiltInPage((current) => Math.min(builtInPageCount - 1, current + 1))}
            type="button"
          >
            →
          </button>
        </div>
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
