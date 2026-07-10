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
  WorkbenchHeadlessActionButtons,
  WorkbenchHeadlessFrontendAssetCatalog,
  WorkbenchHeadlessTemplateCatalog,
} from "@/components/workbench/workbench-headless-workflow-panel-library";
import { WorkbenchHeadlessWorkflowPanelControls } from "@/components/workbench/workbench-headless-workflow-panel-controls";
import { WorkbenchHeadlessWorkflowStepList } from "@/components/workbench/workbench-headless-workflow-step-list";
import {
  buildHeadlessAgentDispatchPlanFromBackend,
  buildHeadlessOrchestraHandoffFromBackend,
  describeHeadlessHandoffReceiptForLog,
  submitHeadlessOrchestraHandoffFromBackend,
} from "@/components/workbench/workbench-headless-workflow-panel-actions";
import {
  buildFrontendMacroBridgePayload,
  formatHeadlessExecutionResultLogs,
  formatHeadlessHandoffSnapshotLog,
  formatHeadlessHandoffStatusLog,
  type FrontendMacroAssetRecord,
  moveItem,
  upsertLatestHeadlessHandoffReceipt,
} from "@/components/workbench/workbench-headless-workflow-panel-state";
import { buildStepFromTemplate, HEADLESS_ACTIONS, HEADLESS_WORKFLOW_TEMPLATES } from "@/components/workbench/workbench-headless-workflow-registry";
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
      setExecutionLog((current) => [...current, ...formatHeadlessExecutionResultLogs(result)]);
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
      setHandoffHistory((current) => upsertLatestHeadlessHandoffReceipt(current, receipt));
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
      setExecutionLog((current) => [...current, formatHeadlessHandoffStatusLog(status)]);
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
      setExecutionLog((current) => [...current, formatHeadlessHandoffSnapshotLog(snapshot)]);
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

      <WorkbenchHeadlessFrontendAssetCatalog
        assets={frontendMacroAssets}
        onDerive={onDeriveFrontendMacro}
        onInsertBridge={(asset) =>
          setSteps((current) => [...current, buildStepFromTemplate("frontend_macro_bridge", buildFrontendMacroBridgePayload(asset.draft))])
        }
        ui={ui}
      />

      <WorkbenchHeadlessTemplateCatalog
        language={language}
        onLoadTemplate={(template) => {
          setSteps(template.steps.map((step) => buildStepFromTemplate(step.action, step.payload)));
          setDraftId(`macro/${template.id}`);
          setError(null);
        }}
        ui={ui}
      />

      <WorkbenchHeadlessActionButtons
        onInsertAction={(actionId, payload) =>
          setSteps((current) => [...current, buildStepFromTemplate(actionId, payload)])
        }
      />

      <WorkbenchHeadlessWorkflowStepList
        actionMap={actionMap}
        language={language}
        onMoveStep={(index, direction) => setSteps((current) => moveItem(current, index, index + direction))}
        onPayloadTextChange={(stepId, payloadText) =>
          setSteps((current) => current.map((entry) => (entry.id === stepId ? { ...entry, payloadText } : entry)))
        }
        onRemoveStep={(stepId) => setSteps((current) => current.filter((entry) => entry.id !== stepId))}
        onRestoreFrontendMacro={onRestoreFrontendMacro}
        patchStepPayload={patchStepPayload}
        steps={steps}
        ui={ui}
      />

      {error ? <p className="card-copy">{error}</p> : null}
      <WorkbenchHeadlessWorkflowPanelControls
        draft={draft}
        executionLog={executionLog}
        executionRunning={executionRunning}
        onExportBatch={() => {
          if (!draft) return setError(ui.invalidJson);
          downloadJsonArtifact(
            `${slugifyWorkflowAssetName(draft.id)}.headless-execution-batch.json`,
            buildHeadlessWorkflowExecutionBatch({ actionMap, draft, language }),
          );
          setError(null);
        }}
        onExportDispatch={() => void exportAgentDispatchPlan()}
        onExportHandoff={() => void exportOrchestraHandoff()}
        onExportHtml={() => {
          if (!exportDocument) return setError(ui.invalidJson);
          downloadHtmlArtifact(
            `${slugifyWorkflowAssetName(exportDocument.workflow.id)}.headless-workflow.html`,
            buildHeadlessWorkflowExportHtml(exportDocument),
          );
          setError(null);
        }}
        onExportJson={() => {
          if (!exportDocument) return setError(ui.invalidJson);
          downloadJsonArtifact(`${slugifyWorkflowAssetName(exportDocument.workflow.id)}.headless-workflow.json`, exportDocument);
          setError(null);
        }}
        onImportJson={(file) => void importHeadlessWorkflowJson(file)}
        onInsert={() => {
          if (!draft) return setError(ui.invalidInsert);
          onInsertMacroDraft(draft);
          setError(null);
        }}
        onRefreshHandoff={() => void refreshOrchestraHandoff()}
        onRefreshHistory={() => void refreshHandoffHistory()}
        onRunBatch={() => void runExecutionBatch()}
        onSubmitHandoff={() => void submitOrchestraHandoff()}
        ui={ui}
      />
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
    </section>
  );
}
