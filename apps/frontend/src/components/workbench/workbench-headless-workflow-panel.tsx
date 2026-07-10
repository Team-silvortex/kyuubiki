"use client";

import { useMemo, useState } from "react";
import { WorkbenchHeadlessHandoffHistory } from "@/components/workbench/workbench-headless-handoff-history";
import type { DraftStep, PayloadObject } from "@/components/workbench/workbench-headless-workflow-contract";
import { formatPayload, parsePayloadText, updatePayloadField } from "@/components/workbench/workbench-headless-workflow-contract";
import {
  buildHeadlessWorkflowExecutionBatch,
  buildHeadlessWorkflowExportDocument,
  buildHeadlessWorkflowExportHtml,
  parseHeadlessWorkflowImportDocument,
} from "@/components/workbench/workbench-headless-workflow-export";
import {
  buildHeadlessWorkflowPanelCopy,
  readStoredWorkbenchAuth,
} from "@/components/workbench/workbench-headless-workflow-panel-helpers";
import {
  buildHeadlessAgentDispatchPlanFromBackend,
  buildHeadlessOrchestraHandoffFromBackend,
  describeHeadlessHandoffReceiptForLog,
  submitHeadlessOrchestraHandoffFromBackend,
} from "@/components/workbench/workbench-headless-workflow-panel-actions";
import {
  buildFrontendMacroBridgePayload,
  type FrontendMacroAssetRecord,
  moveItem,
  parseFrontendMacroBridgePayload,
} from "@/components/workbench/workbench-headless-workflow-panel-state";
import { buildReferenceTokens, buildStepFromTemplate, HEADLESS_ACTIONS, HEADLESS_WORKFLOW_TEMPLATES, localizeWorkflowText } from "@/components/workbench/workbench-headless-workflow-registry";
import { WorkbenchHeadlessWorkflowStepEditor } from "@/components/workbench/workbench-headless-workflow-step-editor";
import { downloadHtmlArtifact, downloadJsonArtifact, slugifyWorkflowAssetName } from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import {
  type HeadlessHandoffReceipt,
  type HeadlessHandoffSnapshot,
} from "@/lib/api/headless-handoff-client";
import { runHeadlessExecutionBatch } from "@/lib/scripting/workbench-headless-execution";
import type { WorkbenchRecordedMacroDraft, WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";
import { defaultWorkbenchHeadlessWorkflowBackendService } from "@/lib/workbench/headless-workflow-backend-service";

export type { FrontendMacroAssetRecord } from "@/components/workbench/workbench-headless-workflow-panel-state";

type WorkbenchHeadlessWorkflowPanelProps = {
  frontendMacroAssets: FrontendMacroAssetRecord[];
  language: WorkbenchScriptLanguage;
  onDeriveFrontendMacro: (asset: FrontendMacroAssetRecord) => void;
  onInsertMacroDraft: (draft: WorkbenchRecordedMacroDraft) => void;
  onRestoreFrontendMacro: (draft: WorkbenchRecordedMacroDraft) => void;
};

export function WorkbenchHeadlessWorkflowPanel({
  frontendMacroAssets,
  language,
  onDeriveFrontendMacro,
  onInsertMacroDraft,
  onRestoreFrontendMacro,
}: WorkbenchHeadlessWorkflowPanelProps) {
  const [draftId, setDraftId] = useState("macro/headless-service-workflow");
  const [steps, setSteps] = useState<DraftStep[]>(() => HEADLESS_WORKFLOW_TEMPLATES[0].steps.map((step) => buildStepFromTemplate(step.action, step.payload)));
  const [error, setError] = useState<string | null>(null);
  const [executionLog, setExecutionLog] = useState<string[]>([]);
  const [executionRunning, setExecutionRunning] = useState(false);
  const [handoffHistory, setHandoffHistory] = useState<HeadlessHandoffReceipt[]>([]);
  const [latestHandoffId, setLatestHandoffId] = useState<string | null>(null);
  const [selectedHandoffSnapshot, setSelectedHandoffSnapshot] = useState<HeadlessHandoffSnapshot | null>(null);
  const actionMap = useMemo(() => new Map(HEADLESS_ACTIONS.map((action) => [action.id, action])), []);

  const draft = useMemo<WorkbenchRecordedMacroDraft | null>(() => {
    try {
      return {
        id: draftId.trim() || "macro/headless-service-workflow",
        steps: steps.map((step) => {
          const payload = parsePayloadText(step.payloadText);
          if (!payload) throw new Error("invalid payload");
          return { action: step.action, payload };
        }),
      };
    } catch {
      return null;
    }
  }, [draftId, steps]);

  const patchStepPayload = (stepId: string, updater: (payload: PayloadObject | null) => PayloadObject | null) => {
    setSteps((current) =>
      current.map((step) => {
        if (step.id !== stepId) return step;
        const nextPayload = updater(parsePayloadText(step.payloadText));
        return nextPayload ? { ...step, payloadText: formatPayload(nextPayload) } : step;
      }),
    );
  };
  const exportDocument = useMemo(
    () => (draft ? buildHeadlessWorkflowExportDocument({ actionMap, draft, language }) : null),
    [actionMap, draft, language],
  );

  const ui = buildHeadlessWorkflowPanelCopy(language);

  const importHeadlessWorkflowJson = async (file: File | undefined) => {
    if (!file) return;
    try {
      const imported = parseHeadlessWorkflowImportDocument(JSON.parse(await file.text()) as unknown);
      setDraftId(imported.id);
      setSteps(imported.steps.map((step) => buildStepFromTemplate(step.action, step.payload ?? {})));
      setError(ui.imported);
    } catch (importError) {
      setError(importError instanceof Error ? importError.message : String(importError));
    }
  };

  const runExecutionBatch = async () => {
    if (!draft) {
      setError(ui.invalidJson);
      return;
    }
    const batch = buildHeadlessWorkflowExecutionBatch({ actionMap, draft, language });
    setExecutionRunning(true);
    setExecutionLog(batch.warnings.map((warning) => `[warning] ${warning}`));
    setError(null);
    try {
      const result = await runHeadlessExecutionBatch(batch, (event) => {
        setExecutionLog((current) => [...current, event.message]);
      });
      setExecutionLog((current) => [
        ...current,
        ...result.steps.map((step) => `[result] step ${step.index} ${step.action}: ${JSON.stringify(step.result)}`),
      ]);
      setError(ui.executionDone);
    } catch (runError) {
      setError(runError instanceof Error ? runError.message : String(runError));
    } finally {
      setExecutionRunning(false);
    }
  };

  const exportAgentDispatchPlan = async () => {
    if (!draft) {
      setError(ui.invalidJson);
      return;
    }
    try {
      const batch = buildHeadlessWorkflowExecutionBatch({ actionMap, draft, language });
      const plan = await buildHeadlessAgentDispatchPlanFromBackend({
        backendService: defaultWorkbenchHeadlessWorkflowBackendService,
        batch,
      });
      downloadJsonArtifact(`${slugifyWorkflowAssetName(draft.id)}.headless-agent-dispatch.json`, plan);
      setError(null);
    } catch (dispatchError) {
      setError(dispatchError instanceof Error ? dispatchError.message : String(dispatchError));
    }
  };

  const exportOrchestraHandoff = async () => {
    if (!draft) {
      setError(ui.invalidJson);
      return;
    }
    try {
      const batch = buildHeadlessWorkflowExecutionBatch({ actionMap, draft, language });
      const handoff = await buildHeadlessOrchestraHandoffFromBackend({
        auth: readStoredWorkbenchAuth(),
        backendService: defaultWorkbenchHeadlessWorkflowBackendService,
        batch,
      });
      downloadJsonArtifact(`${slugifyWorkflowAssetName(draft.id)}.headless-orchestra-handoff.json`, handoff);
      setError(null);
    } catch (handoffError) {
      setError(handoffError instanceof Error ? handoffError.message : String(handoffError));
    }
  };

  const submitOrchestraHandoff = async () => {
    if (!draft) {
      setError(ui.invalidJson);
      return;
    }
    try {
      const batch = buildHeadlessWorkflowExecutionBatch({ actionMap, draft, language });
      const { receipt } = await submitHeadlessOrchestraHandoffFromBackend({
        backendService: defaultWorkbenchHeadlessWorkflowBackendService,
        buildAuthSnapshot: readStoredWorkbenchAuth,
        batch,
      });
      setLatestHandoffId(receipt.handoff_id);
      setHandoffHistory((current) => [receipt, ...current.filter((item) => item.handoff_id !== receipt.handoff_id)]);
      setExecutionLog((current) => [...current, describeHeadlessHandoffReceiptForLog(receipt)]);
      setError(ui.handoffSubmitted);
    } catch (submitError) {
      setError(submitError instanceof Error ? submitError.message : String(submitError));
    }
  };

  const refreshOrchestraHandoff = async () => {
    if (!latestHandoffId) {
      setError(ui.noHandoffYet);
      return;
    }
    try {
      const status = await defaultWorkbenchHeadlessWorkflowBackendService.fetchHandoffStatus(latestHandoffId);
      setExecutionLog((current) => [...current, `[handoff-status] ${JSON.stringify(status)}`]);
      setError(ui.handoffRefreshed);
    } catch (statusError) {
      setError(statusError instanceof Error ? statusError.message : String(statusError));
    }
  };

  const refreshHandoffHistory = async () => {
    try {
      const payload = await defaultWorkbenchHeadlessWorkflowBackendService.fetchHandoffHistory();
      setHandoffHistory(payload.handoffs);
      setError(ui.historyRefreshed);
    } catch (historyError) {
      setError(historyError instanceof Error ? historyError.message : String(historyError));
    }
  };

  const inspectHandoffSnapshot = async (handoffId: string) => {
    try {
      const snapshot = await defaultWorkbenchHeadlessWorkflowBackendService.fetchHandoffSnapshot(handoffId);
      setSelectedHandoffSnapshot(snapshot);
      setLatestHandoffId(snapshot.handoff_id);
      setExecutionLog((current) => [...current, `[handoff-snapshot] ${snapshot.handoff_id}`]);
      setError(null);
    } catch (snapshotError) {
      setError(snapshotError instanceof Error ? snapshotError.message : String(snapshotError));
    }
  };

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{ui.title}</h2>
        <span>{steps.length}</span>
      </div>
      <p className="card-copy">{ui.subtitle}</p>
      <label className="field-label">
        <span>{ui.draftId}</span>
        <input className="text-input" onChange={(event) => setDraftId(event.target.value)} type="text" value={draftId} />
      </label>

      <div className="card-subhead">
        <strong>{ui.frontendAssets}</strong>
        <span>{frontendMacroAssets.length}</span>
      </div>
      <p className="card-copy">{ui.frontendAssetsHint}</p>
      {frontendMacroAssets.length === 0 ? (
        <p className="card-copy">{ui.frontendAssetEmpty}</p>
      ) : (
        <div className="script-panel__catalog">
          {frontendMacroAssets.map((asset) => (
            <article className="script-panel__action" key={asset.assetId}>
              <div className="script-panel__action-head">
                <strong>{asset.draft.id}</strong>
                <span>{`${ui.bridgeSteps}: ${asset.draft.steps.length}`}</span>
              </div>
              <div className="script-panel__payload">
                <span>{ui.assetSnapshotId}</span>
                <code>{asset.assetId}</code>
              </div>
              <div className="script-panel__payload">
                <span>{ui.assetSource}</span>
                <code>
                  {asset.source === "bridge_restore"
                    ? ui.assetSourceBridge
                    : asset.source === "snapshot_derived"
                      ? ui.assetSourceDerived
                      : ui.assetSourceTimeline}
                </code>
              </div>
              <div className="script-panel__payload">
                <span>{ui.assetUpdatedAt}</span>
                <code>{asset.updatedAt}</code>
              </div>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => onDeriveFrontendMacro(asset)} type="button">
                  {ui.bridgeDerive}
                </button>
                <button
                  className="ghost-button ghost-button--compact"
                  onClick={() => setSteps((current) => [...current, buildStepFromTemplate("frontend_macro_bridge", buildFrontendMacroBridgePayload(asset.draft))])}
                  type="button"
                >
                  {ui.bridgeInsert}
                </button>
              </div>
            </article>
          ))}
        </div>
      )}

      <div className="script-panel__catalog">
        {HEADLESS_WORKFLOW_TEMPLATES.map((template) => (
          <article className="script-panel__action" key={template.id}>
            <div className="script-panel__action-head">
              <strong>{localizeWorkflowText(language, template.title)}</strong>
              <span>{template.steps.length}</span>
            </div>
            <p className="card-copy">{localizeWorkflowText(language, template.description)}</p>
            <div className="button-row">
              <button
                className="ghost-button ghost-button--compact"
                onClick={() => {
                  setSteps(template.steps.map((step) => buildStepFromTemplate(step.action, step.payload)));
                  setDraftId(`macro/${template.id}`);
                  setError(null);
                }}
                type="button"
              >
                {ui.loadTemplate}
              </button>
            </div>
          </article>
        ))}
      </div>

      <div className="button-row">
        {HEADLESS_ACTIONS.map((action) => (
          <button
            className="ghost-button ghost-button--compact"
            key={action.id}
            onClick={() => setSteps((current) => [...current, buildStepFromTemplate(action.id, action.payloadExample)])}
            type="button"
          >
            {action.id}
          </button>
        ))}
      </div>

      <div className="script-panel__catalog">
        {steps.map((step, index) => {
          const descriptor = actionMap.get(step.action);
          const references = buildReferenceTokens(steps, index, actionMap);
          const riskLabel =
            descriptor?.risk === "destructive" ? ui.destructive : descriptor?.risk === "sensitive" ? ui.sensitive : ui.normal;

          return (
            <article className="script-panel__action" key={step.id}>
              <div className="script-panel__action-head">
                <strong>{`${index + 1}. ${step.action}`}</strong>
                <span>{riskLabel}</span>
              </div>
              {descriptor ? <p className="card-copy">{localizeWorkflowText(language, descriptor.summary)}</p> : null}
              <WorkbenchHeadlessWorkflowStepEditor
                bridgeActionListLabel={ui.bridgeActionList}
                bridgeMacroIdLabel={ui.bridgeMacroId}
                bridgePreviewHideLabel={ui.bridgePreviewHide}
                bridgePreviewPayloadLabel={ui.bridgePreviewPayload}
                bridgePreviewShowLabel={ui.bridgePreviewShow}
                bridgeReplayModeHint={ui.bridgeReplayModeHint}
                bridgeReplayModeLabel={ui.bridgeReplayMode}
                bridgeRestoreLabel={ui.bridgeRestore}
                bridgeStepCountLabel={ui.bridgeSteps}
                endpointsHint={ui.endpointsHint}
                endpointsLabel={ui.endpoints}
                contract={descriptor}
                guidanceTitle={ui.guidanceTitle}
                language={language}
                noReferencesLabel={ui.noReferences}
                onRestoreBridgeMacro={
                  step.action === "frontend_macro_bridge"
                    ? () => {
                        const restored = parseFrontendMacroBridgePayload(parsePayloadText(step.payloadText));
                        if (!restored) return;
                        onRestoreFrontendMacro(restored);
                      }
                    : undefined
                }
                parsePayloadText={parsePayloadText}
                patchStepPayload={patchStepPayload}
                referenceApplyLabel={ui.referenceApply}
                referenceClearLabel={ui.referenceClear}
                referenceCurrentLabel={ui.referenceCurrent}
                references={references}
                referenceTitle={ui.referenceTitle}
                step={step}
              />

              {step.action === "frontend_macro_bridge" ? (
                <label className="field-label">
                  <span>{ui.rawPayloadJson}</span>
                  <pre className="script-panel__snapshot">{step.payloadText}</pre>
                </label>
              ) : (
                <label className="field-label">
                  <span>{ui.payloadJson}</span>
                  <textarea
                    className="script-panel__editor"
                    onChange={(event) =>
                      setSteps((current) => current.map((entry) => (entry.id === step.id ? { ...entry, payloadText: event.target.value } : entry)))
                    }
                    rows={6}
                    spellCheck={false}
                    value={step.payloadText}
                  />
                </label>
              )}

              <div className="button-row">
                <button
                  className="ghost-button ghost-button--compact"
                  onClick={() => setSteps((current) => moveItem(current, index, index - 1))}
                  type="button"
                >
                  {ui.moveUp}
                </button>
                <button
                  className="ghost-button ghost-button--compact"
                  onClick={() => setSteps((current) => moveItem(current, index, index + 1))}
                  type="button"
                >
                  {ui.moveDown}
                </button>
                <button className="ghost-button ghost-button--compact" onClick={() => setSteps((current) => current.filter((entry) => entry.id !== step.id))} type="button">
                  {ui.remove}
                </button>
              </div>
            </article>
          );
        })}
      </div>

      {error ? <p className="card-copy">{error}</p> : null}
      <div className="button-row">
        <label className="ghost-button" style={{ display: "inline-flex", alignItems: "center", cursor: "pointer" }}>
          {ui.importJson}
          <input
            accept="application/json,.json"
            hidden
            onChange={(event) => {
              void importHeadlessWorkflowJson(event.target.files?.[0]);
              event.currentTarget.value = "";
            }}
            type="file"
          />
        </label>
        <button
          className="ghost-button"
          disabled={executionRunning}
          onClick={() => {
            void runExecutionBatch();
          }}
          type="button"
        >
          {ui.runBatch}
        </button>
        <button
          className="ghost-button"
          onClick={() => {
            if (!exportDocument) return setError(ui.invalidJson);
            downloadJsonArtifact(`${slugifyWorkflowAssetName(exportDocument.workflow.id)}.headless-workflow.json`, exportDocument);
            setError(null);
          }}
          type="button"
        >
          {ui.exportJson}
        </button>
        <button
          className="ghost-button"
          onClick={() => {
            if (!draft) return setError(ui.invalidJson);
            downloadJsonArtifact(
              `${slugifyWorkflowAssetName(draft.id)}.headless-execution-batch.json`,
              buildHeadlessWorkflowExecutionBatch({ actionMap, draft, language }),
            );
            setError(null);
          }}
          type="button"
        >
          {ui.exportBatch}
        </button>
        <button className="ghost-button" onClick={() => void exportAgentDispatchPlan()} type="button">
          {ui.exportDispatch}
        </button>
        <button className="ghost-button" onClick={() => void exportOrchestraHandoff()} type="button">
          {ui.exportHandoff}
        </button>
        <button className="ghost-button" onClick={() => void submitOrchestraHandoff()} type="button">
          {ui.submitHandoff}
        </button>
        <button className="ghost-button" onClick={() => void refreshOrchestraHandoff()} type="button">
          {ui.refreshHandoff}
        </button>
        <button className="ghost-button" onClick={() => void refreshHandoffHistory()} type="button">
          {ui.refreshHistory}
        </button>
        <button
          className="ghost-button"
          onClick={() => {
            if (!exportDocument) return setError(ui.invalidJson);
            downloadHtmlArtifact(
              `${slugifyWorkflowAssetName(exportDocument.workflow.id)}.headless-workflow.html`,
              buildHeadlessWorkflowExportHtml(exportDocument),
            );
            setError(null);
          }}
          type="button"
        >
          {ui.exportHtml}
        </button>
        <button
          className="ghost-button"
          onClick={() => {
            if (!draft) return setError(ui.invalidInsert);
            onInsertMacroDraft(draft);
            setError(null);
          }}
          type="button"
        >
          {ui.insert}
        </button>
      </div>
      <WorkbenchHeadlessHandoffHistory
        allStagesLabel={ui.handoffAllStages}
        copy={ui}
        dispatchPlannedCountLabel={ui.handoffDispatchPlannedCount}
        emptyLabel={ui.handoffHistoryEmpty}
        filterLabel={ui.handoffFilter}
        inspectLabel={ui.handoffInspect}
        items={handoffHistory}
        onInspect={(handoffId) => void inspectHandoffSnapshot(handoffId)}
        queuedCountLabel={ui.handoffQueuedCount}
        readyCountLabel={ui.handoffReadyCount}
        receivedCountLabel={ui.handoffReceivedCount}
        selectedSnapshot={selectedHandoffSnapshot}
        snapshotEmptyLabel={ui.handoffSnapshotEmpty}
        snapshotLabel={ui.handoffSnapshot}
        stageLabel={ui.handoffStage}
        statusMessageLabel={ui.handoffStatusMessage}
        title={ui.handoffHistory}
        workflowFilterLabel={ui.handoffWorkflowFilter}
      />
      {executionLog.length > 0 ? (
        <label className="field-label">
          <span>{ui.executionLog}</span>
          <pre className="script-panel__snapshot">{executionLog.join("\n")}</pre>
        </label>
      ) : null}
      <pre className="script-panel__snapshot">{draft ? JSON.stringify(draft, null, 2) : "{\n  \"error\": \"invalid payload json\"\n}"}</pre>
    </section>
  );
}
