"use client";

import { useMemo, useState } from "react";
import { downloadJsonArtifact, slugifyWorkflowAssetName } from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import type { HeadlessHandoffReceipt, HeadlessHandoffSnapshot } from "@/lib/api";
import {
  asHeadlessDispatchOverrideDocument,
  buildHeadlessDispatchOverrideDocument,
  toLocalDispatchOverrides,
} from "@/lib/scripting/workbench-headless-dispatch-override";
import { attachDispatchOverrideDraft } from "@/lib/scripting/workbench-headless-orchestra-handoff";

type WorkbenchHeadlessHandoffHistoryProps = {
  allStagesLabel: string;
  copy: {
    handoffAuthority: string;
    handoffAuthorityLabel: string;
    handoffCapabilities: string;
    handoffCandidateCluster: string;
    handoffCandidateEndpoint: string;
    handoffCandidateHealth: string;
    handoffCandidateHeadlessBonus: string;
    handoffCandidateRuntimeBonus: string;
    handoffCandidateRuntime: string;
    handoffCandidateScore: string;
    handoffCandidates: string;
    handoffChosenAgent: string;
    handoffClusterScope: string;
    handoffControlMode: string;
    handoffDispatchPlan: string;
    handoffHideCandidates: string;
    handoffLane: string;
    handoffLaneDirectMesh: string;
    handoffLaneFrontendBridge: string;
    handoffLaneOrchestrator: string;
    handoffGovernance: string;
    handoffInspectCandidates: string;
    handoffDrift: string;
    handoffDriftLabel: string;
    handoffExposureLabel: string;
    handoffNote: string;
    handoffOverrideClear: string;
    handoffOverrideAcknowledged: string;
    handoffOverrideCount: string;
    handoffOverrideExport: string;
    handoffOverrideExportHandoff: string;
    handoffOverrideHandoffExported: string;
    handoffOverrideImport: string;
    handoffOverrideImported: string;
    handoffOverrideLocal: string;
    handoffOverrideNote: string;
    handoffOverridePromote: string;
    handoffOverrideBadge: string;
    handoffRawSnapshot: string;
    handoffRuntimeManifest: string;
    handoffRuntimeModes: string;
    handoffSelected: string;
    handoffTopology: string;
    handoffViolation: string;
    handoffWarning: string;
    handoffWinnerReason: string;
  };
  emptyLabel: string;
  filterLabel: string;
  inspectLabel: string;
  readyCountLabel: string;
  receivedCountLabel: string;
  queuedCountLabel: string;
  dispatchPlannedCountLabel: string;
  snapshotEmptyLabel: string;
  snapshotLabel: string;
  stageLabel: string;
  statusMessageLabel: string;
  workflowFilterLabel: string;
  title: string;
  items: HeadlessHandoffReceipt[];
  onInspect: (handoffId: string) => void;
  selectedSnapshot: HeadlessHandoffSnapshot | null;
};

export function WorkbenchHeadlessHandoffHistory({
  allStagesLabel,
  copy,
  dispatchPlannedCountLabel,
  emptyLabel,
  filterLabel,
  inspectLabel,
  items,
  onInspect,
  queuedCountLabel,
  readyCountLabel,
  receivedCountLabel,
  selectedSnapshot,
  snapshotEmptyLabel,
  snapshotLabel,
  stageLabel,
  statusMessageLabel,
  title,
  workflowFilterLabel,
}: WorkbenchHeadlessHandoffHistoryProps) {
  const [stageFilter, setStageFilter] = useState("all");
  const [workflowFilter, setWorkflowFilter] = useState("");
  const [expandedCandidates, setExpandedCandidates] = useState<Record<string, boolean>>({});
  const [localOverrides, setLocalOverrides] = useState<Record<string, string>>({});
  const [overrideMessage, setOverrideMessage] = useState<string | null>(null);

  const filteredItems = useMemo(() => {
    const workflowNeedle = workflowFilter.trim().toLowerCase();
    return items.filter((item) => {
      const matchesStage = stageFilter === "all" || item.stage === stageFilter;
      if (!matchesStage) return false;
      if (!workflowNeedle) return true;
      return (
        item.workflow_id.toLowerCase().includes(workflowNeedle) ||
        item.handoff_id.toLowerCase().includes(workflowNeedle)
      );
    });
  }, [items, stageFilter, workflowFilter]);

  const stats = useMemo(
    () => ({
      received: items.filter((item) => item.stage === "received").length,
      queued: items.filter((item) => item.stage === "queued").length,
      dispatch_planned: items.filter((item) => item.stage === "dispatch_planned").length,
      ready_for_orchestra: items.filter((item) => item.stage === "ready_for_orchestra").length,
    }),
    [items],
  );
  const selectedDispatchSteps = selectedSnapshot?.envelope.dispatch_plan.steps ?? [];
  const selectedWarnings = selectedSnapshot?.envelope.dispatch_plan.warnings ?? [];
  const selectedGovernance = selectedSnapshot?.envelope.governance;
  const selectedRuntimeManifest = selectedSnapshot?.envelope.runtime_manifest;
  const dispatchGroups = useMemo(
    () =>
      [
        {
          lane: "orchestrator_service",
          label: copy.handoffLaneOrchestrator,
          steps: selectedDispatchSteps.filter((step) => step.lane === "orchestrator_service"),
        },
        {
          lane: "direct_mesh_solver",
          label: copy.handoffLaneDirectMesh,
          steps: selectedDispatchSteps.filter((step) => step.lane === "direct_mesh_solver"),
        },
        {
          lane: "frontend_bridge",
          label: copy.handoffLaneFrontendBridge,
          steps: selectedDispatchSteps.filter((step) => step.lane === "frontend_bridge"),
        },
      ].filter((group) => group.steps.length > 0),
    [copy.handoffLaneDirectMesh, copy.handoffLaneFrontendBridge, copy.handoffLaneOrchestrator, selectedDispatchSteps],
  );

  const exportDispatchOverrideDraft = () => {
    if (!selectedSnapshot) return;
    const document = buildHeadlessDispatchOverrideDocument({
      localOverrides,
      snapshot: selectedSnapshot,
    });
    downloadJsonArtifact(
      `${slugifyWorkflowAssetName(selectedSnapshot.workflow_id)}.headless-dispatch-override.json`,
      document,
    );
    setOverrideMessage(null);
  };

  const exportHandoffDraftWithOverride = () => {
    if (!selectedSnapshot) return;
    const document = buildHeadlessDispatchOverrideDocument({
      localOverrides,
      snapshot: selectedSnapshot,
    });
    const handoffDraft = attachDispatchOverrideDraft({
      handoff: selectedSnapshot.envelope,
      document,
    });
    downloadJsonArtifact(
      `${slugifyWorkflowAssetName(selectedSnapshot.workflow_id)}.headless-orchestra-handoff.override-draft.json`,
      handoffDraft,
    );
    setOverrideMessage(copy.handoffOverrideHandoffExported);
  };

  const importDispatchOverrideDraft = async (file: File | undefined) => {
    if (!file) return;
    try {
      const document = asHeadlessDispatchOverrideDocument(JSON.parse(await file.text()) as unknown);
      if (!document) throw new Error("invalid dispatch override document");
      setLocalOverrides((current) => ({ ...current, ...toLocalDispatchOverrides(document) }));
      setOverrideMessage(copy.handoffOverrideImported);
    } catch (error) {
      setOverrideMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <>
      <div className="card-subhead">
        <strong>{title}</strong>
        <span>{items.length}</span>
      </div>
      <div className="script-panel__catalog">
        <article className="script-panel__action">
          <div className="script-panel__action-head">
            <strong>{receivedCountLabel}</strong>
            <span>{stats.received}</span>
          </div>
        </article>
        <article className="script-panel__action">
          <div className="script-panel__action-head">
            <strong>{queuedCountLabel}</strong>
            <span>{stats.queued}</span>
          </div>
        </article>
        <article className="script-panel__action">
          <div className="script-panel__action-head">
            <strong>{dispatchPlannedCountLabel}</strong>
            <span>{stats.dispatch_planned}</span>
          </div>
        </article>
        <article className="script-panel__action">
          <div className="script-panel__action-head">
            <strong>{readyCountLabel}</strong>
            <span>{stats.ready_for_orchestra}</span>
          </div>
        </article>
      </div>
      <div className="button-row">
        <label className="field-label" style={{ minWidth: 180, flex: "1 1 180px" }}>
          <span>{filterLabel}</span>
          <select className="text-input" onChange={(event) => setStageFilter(event.target.value)} value={stageFilter}>
            <option value="all">{allStagesLabel}</option>
            <option value="received">received</option>
            <option value="queued">queued</option>
            <option value="dispatch_planned">dispatch_planned</option>
            <option value="ready_for_orchestra">ready_for_orchestra</option>
          </select>
        </label>
        <label className="field-label" style={{ minWidth: 220, flex: "2 1 220px" }}>
          <span>{workflowFilterLabel}</span>
          <input className="text-input" onChange={(event) => setWorkflowFilter(event.target.value)} type="text" value={workflowFilter} />
        </label>
      </div>
      {filteredItems.length === 0 ? (
        <p className="card-copy">{emptyLabel}</p>
      ) : (
        <div className="script-panel__catalog">
          {filteredItems.map((item) => (
            <article className="script-panel__action" key={item.handoff_id}>
              <div className="script-panel__action-head">
                <strong>{item.workflow_id}</strong>
                <span>{item.has_dispatch_override ? copy.handoffOverrideBadge : item.stage}</span>
              </div>
              <div className="script-panel__payload">
                <span>ID</span>
                <code>{item.handoff_id}</code>
              </div>
              <div className="script-panel__payload">
                <span>{stageLabel}</span>
                <code>{item.stage}</code>
              </div>
              <div className="script-panel__payload">
                <span>{statusMessageLabel}</span>
                <code>{item.status_message}</code>
              </div>
              <div className="script-panel__payload">
                <span>{copy.handoffOverrideCount}</span>
                <code>{item.dispatch_override_count}</code>
              </div>
              <div className="script-panel__payload">
                <span>{copy.handoffOverrideAcknowledged}</span>
                <code>{item.override_acknowledged ? "true" : "false"}</code>
              </div>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => onInspect(item.handoff_id)} type="button">
                  {inspectLabel}
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
      <label className="field-label">
        <span>{snapshotLabel}</span>
        {!selectedSnapshot ? (
          <pre className="script-panel__snapshot">{snapshotEmptyLabel}</pre>
        ) : (
          <>
            <div className="script-panel__catalog">
              <article className="script-panel__action">
                <div className="script-panel__action-head">
                  <strong>{copy.handoffSelected}</strong>
                  <span>{selectedSnapshot.has_dispatch_override ? copy.handoffOverrideBadge : selectedSnapshot.stage}</span>
                </div>
                <div className="script-panel__payload">
                  <span>ID</span>
                  <code>{selectedSnapshot.handoff_id}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffWarning}</span>
                  <code>{selectedWarnings.length}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffOverrideCount}</span>
                  <code>{selectedSnapshot.dispatch_override_count}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffOverrideAcknowledged}</span>
                  <code>{selectedSnapshot.override_acknowledged ? "true" : "false"}</code>
                </div>
                {selectedSnapshot.override_note ? (
                  <div className="script-panel__payload">
                    <span>{copy.handoffOverrideNote}</span>
                    <code>{selectedSnapshot.override_note}</code>
                  </div>
                ) : null}
              </article>
            </div>
            <div className="card-subhead">
              <strong>{copy.handoffDispatchPlan}</strong>
              <span>{selectedDispatchSteps.length}</span>
            </div>
            <div className="button-row">
              <button className="ghost-button ghost-button--compact" onClick={() => exportDispatchOverrideDraft()} type="button">
                {copy.handoffOverrideExport}
              </button>
              <button className="ghost-button ghost-button--compact" onClick={() => exportHandoffDraftWithOverride()} type="button">
                {copy.handoffOverrideExportHandoff}
              </button>
              <label className="ghost-button ghost-button--compact" style={{ display: "inline-flex", alignItems: "center", cursor: "pointer" }}>
                {copy.handoffOverrideImport}
                <input
                  accept="application/json,.json"
                  hidden
                  onChange={(event) => {
                    void importDispatchOverrideDraft(event.target.files?.[0]);
                    event.currentTarget.value = "";
                  }}
                  type="file"
                />
              </label>
            </div>
            {overrideMessage ? <p className="card-copy">{overrideMessage}</p> : null}
            {dispatchGroups.map((group) => (
              <div key={group.lane}>
                <div className="card-subhead">
                  <strong>{group.label}</strong>
                  <span>{group.steps.length}</span>
                </div>
                <div className="script-panel__catalog">
                  {group.steps.map((step) => {
                    const stepKey = `${step.index}:${step.action}`;
                    const expanded = expandedCandidates[stepKey] ?? false;
                    const overriddenAgentId = localOverrides[stepKey] ?? null;
                    const displayedChosenAgentId = overriddenAgentId ?? step.chosen_agent_id;
                    const displayedWinnerReason =
                      overriddenAgentId && overriddenAgentId !== step.chosen_agent_id
                        ? `${copy.handoffOverrideLocal}: ${overriddenAgentId}`
                        : step.winner_reason_summary;
                    return (
                      <article className="script-panel__action" key={stepKey}>
                        <div className="script-panel__action-head">
                          <strong>{`${step.index}. ${step.action}`}</strong>
                          <span>{group.label}</span>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.handoffLane}</span>
                          <code>{step.lane}</code>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.handoffChosenAgent}</span>
                          <code>{displayedChosenAgentId ?? "--"}</code>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.handoffCapabilities}</span>
                          <code>{step.required_capabilities.join(", ") || "--"}</code>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.handoffCandidates}</span>
                          <code>{step.candidates.length}</code>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.handoffNote}</span>
                          <code>{step.note}</code>
                        </div>
                        <div className="script-panel__payload">
                          <span>{copy.handoffWinnerReason}</span>
                          <code>{displayedWinnerReason}</code>
                        </div>
                        <div className="button-row">
                          <button
                            className="ghost-button ghost-button--compact"
                            onClick={() => setExpandedCandidates((current) => ({ ...current, [stepKey]: !expanded }))}
                            type="button"
                          >
                            {expanded ? copy.handoffHideCandidates : copy.handoffInspectCandidates}
                          </button>
                          {overriddenAgentId ? (
                            <button
                              className="ghost-button ghost-button--compact"
                              onClick={() =>
                                setLocalOverrides((current) => {
                                  const next = { ...current };
                                  delete next[stepKey];
                                  return next;
                                })
                              }
                              type="button"
                            >
                              {copy.handoffOverrideClear}
                            </button>
                          ) : null}
                        </div>
                        {expanded ? (
                          <div className="script-panel__catalog">
                            {step.candidates.length === 0 ? (
                              <article className="script-panel__action">
                                <div className="script-panel__payload">
                                  <span>{copy.handoffCandidates}</span>
                                  <code>0</code>
                                </div>
                              </article>
                            ) : (
                              step.candidates.map((candidate) => (
                                <article className="script-panel__action" key={`${stepKey}:${candidate.agent_id}`}>
                                  <div className="script-panel__action-head">
                                    <strong>{candidate.agent_id}</strong>
                                    <span>{candidate.score}</span>
                                  </div>
                                  <div className="script-panel__payload">
                                    <span>{copy.handoffCandidateEndpoint}</span>
                                    <code>{candidate.endpoint}</code>
                                  </div>
                                  <div className="script-panel__payload">
                                    <span>{copy.handoffCandidateCluster}</span>
                                    <code>{candidate.cluster_id ?? "--"}</code>
                                  </div>
                                  <div className="script-panel__payload">
                                    <span>{copy.handoffCandidateRuntime}</span>
                                    <code>{candidate.runtime_mode}</code>
                                  </div>
                                  <div className="script-panel__payload">
                                    <span>{copy.handoffCandidateScore}</span>
                                    <code>{candidate.score}</code>
                                  </div>
                                  <div className="script-panel__payload">
                                    <span>{copy.handoffCandidateHealth}</span>
                                    <code>{candidate.score_breakdown.health}</code>
                                  </div>
                                  <div className="script-panel__payload">
                                    <span>{copy.handoffCandidateHeadlessBonus}</span>
                                    <code>{candidate.score_breakdown.headless_bonus}</code>
                                  </div>
                                  <div className="script-panel__payload">
                                    <span>{copy.handoffCandidateRuntimeBonus}</span>
                                    <code>{candidate.score_breakdown.runtime_match_bonus}</code>
                                  </div>
                                  <div className="button-row">
                                    <button
                                      className="ghost-button ghost-button--compact"
                                      onClick={() =>
                                        setLocalOverrides((current) => ({
                                          ...current,
                                          [stepKey]: candidate.agent_id,
                                        }))
                                      }
                                      type="button"
                                    >
                                      {copy.handoffOverridePromote}
                                    </button>
                                  </div>
                                </article>
                              ))
                            )}
                          </div>
                        ) : null}
                      </article>
                    );
                  })}
                </div>
              </div>
            ))}
            <div className="card-subhead">
              <strong>{copy.handoffGovernance}</strong>
              <span>{selectedGovernance?.diagnostics.hasViolation ? "warning" : "aligned"}</span>
            </div>
            <div className="script-panel__catalog">
              <article className="script-panel__action">
                <div className="script-panel__action-head">
                  <strong>{copy.handoffAuthority}</strong>
                  <span>{selectedGovernance?.diagnostics.authorityLabel ?? "--"}</span>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffControlMode}</span>
                  <code>{selectedGovernance?.config.controlMode ?? "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffAuthorityLabel}</span>
                  <code>{selectedGovernance?.diagnostics.authorityLabel ?? "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffTopology}</span>
                  <code>{selectedGovernance?.config.orchestration.topology ?? "--"}</code>
                </div>
              </article>
              <article className="script-panel__action">
                <div className="script-panel__action-head">
                  <strong>{copy.handoffClusterScope}</strong>
                  <span>{selectedGovernance?.diagnostics.exposureLabel ?? "--"}</span>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffViolation}</span>
                  <code>{selectedGovernance?.diagnostics.hasViolation ? "true" : "false"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffExposureLabel}</span>
                  <code>{selectedGovernance?.diagnostics.exposureLabel ?? "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffRuntimeModes}</span>
                  <code>{selectedGovernance?.diagnostics.visibleRuntimeModes.join(", ") || "--"}</code>
                </div>
              </article>
              <article className="script-panel__action">
                <div className="script-panel__action-head">
                  <strong>{copy.handoffDrift}</strong>
                  <span>{selectedGovernance?.diagnostics.driftLabel ?? "--"}</span>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffDriftLabel}</span>
                  <code>{selectedGovernance?.diagnostics.driftLabel ?? "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffViolation}</span>
                  <code>{selectedGovernance?.diagnostics.hasViolation ? "true" : "false"}</code>
                </div>
              </article>
            </div>
            <div className="card-subhead">
              <strong>{copy.handoffRuntimeManifest}</strong>
              <span>{selectedRuntimeManifest?.authority_mode ?? "--"}</span>
            </div>
            <div className="script-panel__catalog">
              <article className="script-panel__action">
                <div className="script-panel__payload">
                  <span>Authority</span>
                  <code>{selectedRuntimeManifest?.authority_mode ?? "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>Source</span>
                  <code>{selectedRuntimeManifest?.source_of_truth ?? "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>Library</span>
                  <code>{selectedRuntimeManifest?.agent_library_replication ?? "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffClusterScope}</span>
                  <code>{selectedRuntimeManifest?.target_clusters.join(", ") || "--"}</code>
                </div>
                <div className="script-panel__payload">
                  <span>{copy.handoffRuntimeModes}</span>
                  <code>{selectedRuntimeManifest?.target_runtime_modes.join(", ") || "--"}</code>
                </div>
              </article>
            </div>
            <label className="field-label">
              <span>{copy.handoffRawSnapshot}</span>
              <pre className="script-panel__snapshot">{JSON.stringify(selectedSnapshot, null, 2)}</pre>
            </label>
          </>
        )}
      </label>
    </>
  );
}
