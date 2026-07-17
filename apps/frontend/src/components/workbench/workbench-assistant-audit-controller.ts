"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  createAssistantTransactionEntry,
  type AssistantTransactionEntry,
  type WorkbenchSnapshot,
} from "@/lib/workbench/history";
import {
  createSecurityAuditEntry,
  readSecurityAuditLog,
  writeSecurityAuditLog,
  type WorkbenchSecurityAuditEntry,
  type WorkbenchSecurityAuditRisk,
  type WorkbenchSecurityAuditSource,
} from "@/lib/workbench/security-audit";
import type { WorkbenchScriptActionLogEntry, WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";
import { buildWorkbenchGovernanceRuntimeDiagnostics } from "@/lib/workbench/governance";
import {
  workbenchSecurityEventBackendService,
} from "@/lib/workbench/security-event-backend-service";
import { getWorkbenchAssistantAuditCopy } from "@/components/workbench/workbench-extended-language-copy";
import type {
  WorkbenchSecurityEventBackendService,
} from "@/lib/workbench/security-event-backend-service-core";

type AssistantPlanAction = {
  action: string;
  payload?: Record<string, unknown>;
  reason?: string;
};

type AssistantAuditControllerDeps = {
  language: string;
  scriptRecordingMode: boolean;
  frontendRuntimeMode: string;
  directMeshEndpointsText: string;
  controlPlaneApiToken: string;
  clusterApiToken: string;
  directMeshApiToken: string;
  protocolAgents: any[];
  securityEventBackendService?: WorkbenchSecurityEventBackendService;
  studyKind: string;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  selectedVersionId: string | null;
  immersiveViewport: boolean;
  setMessage: (value: string) => void;
  buildScriptSnapshot: () => WorkbenchScriptSnapshot;
  buildWorkbenchSnapshot: () => WorkbenchSnapshot;
  restoreWorkbenchSnapshot: (snapshot: WorkbenchSnapshot) => void;
};

export function useWorkbenchAssistantAuditController({
  language,
  scriptRecordingMode,
  frontendRuntimeMode,
  directMeshEndpointsText,
  controlPlaneApiToken,
  clusterApiToken,
  directMeshApiToken,
  protocolAgents,
  securityEventBackendService = workbenchSecurityEventBackendService,
  studyKind,
  selectedProjectId,
  selectedModelId,
  selectedVersionId,
  immersiveViewport,
  setMessage,
  buildScriptSnapshot,
  buildWorkbenchSnapshot,
  restoreWorkbenchSnapshot,
}: AssistantAuditControllerDeps) {
  const assistantAuditCopy = getWorkbenchAssistantAuditCopy(language);
  const [scriptActionLog, setScriptActionLog] = useState<WorkbenchScriptActionLogEntry[]>([]);
  const [assistantTransactions, setAssistantTransactions] = useState<AssistantTransactionEntry[]>([]);
  const [securityAuditLog, setSecurityAuditLog] = useState<WorkbenchSecurityAuditEntry[]>([]);
  const governanceAuditSignatureRef = useRef<string | null>(null);

  useEffect(() => {
    setSecurityAuditLog(readSecurityAuditLog());
  }, []);

  useEffect(() => {
    writeSecurityAuditLog(securityAuditLog);
  }, [securityAuditLog]);

  const governanceRuntime = useMemo(
    () =>
      buildWorkbenchGovernanceRuntimeDiagnostics({
        frontendRuntimeMode:
          frontendRuntimeMode === "direct_mesh_gui" ? "direct_mesh_gui" : "orchestrated_gui",
        directMeshEndpointsText,
        protocolAgents,
        controlPlaneApiToken,
        clusterApiToken,
        directMeshApiToken,
      }),
    [
      clusterApiToken,
      controlPlaneApiToken,
      directMeshApiToken,
      directMeshEndpointsText,
      frontendRuntimeMode,
      protocolAgents,
    ],
  );

  const getScriptSnapshot = (): WorkbenchScriptSnapshot => buildScriptSnapshot();

  const appendScriptActionLog = (entry: Omit<WorkbenchScriptActionLogEntry, "id" | "at">) => {
    setScriptActionLog((current) => [
      {
        id: `${Date.now()}-${current.length}`,
        at: new Date().toISOString(),
        ...entry,
      },
      ...current,
    ].slice(0, 40));
  };

  const recordManualDslAction = (action: string, payload: Record<string, unknown>) => {
    if (!scriptRecordingMode) return;

    appendScriptActionLog({
      action,
      source: "manual",
      status: "completed",
      summary: JSON.stringify(payload),
      payload,
      note: assistantAuditCopy.manualRecording,
    });
  };

  const persistSecurityAuditEvent = async (entry: WorkbenchSecurityAuditEntry) => {
    try {
      await securityEventBackendService.createEvent({
        event_id: entry.id,
        event_type: "security_high_risk_action",
        source: entry.source,
        action: entry.action,
        risk: entry.risk,
        status: entry.status,
        note: entry.note,
        occurred_at: entry.at,
        context: {
          frontend_runtime_mode: frontendRuntimeMode,
          study_kind: studyKind,
          project_id: selectedProjectId,
          model_id: selectedModelId,
          model_version_id: selectedVersionId,
          language,
          immersive_viewport: immersiveViewport,
          ...(entry.context ?? {}),
        },
      });
    } catch {
      // Keep local audit logging available even when the control plane is unreachable.
    }
  };

  const recordSecurityAuditEvent = (entry: Omit<WorkbenchSecurityAuditEntry, "id" | "at">) => {
    const event = createSecurityAuditEntry(entry);
    setSecurityAuditLog((current) => [event, ...current].slice(0, 80));
    void persistSecurityAuditEvent(event);
  };

  useEffect(() => {
    const nextSignature = JSON.stringify({
      authority: governanceRuntime.authorityLabel,
      exposure: governanceRuntime.exposureLabel,
      drift: governanceRuntime.driftLabel,
      clusters: governanceRuntime.visibleClusterIds,
      runtimeModes: governanceRuntime.visibleRuntimeModes,
      hasViolation: governanceRuntime.hasViolation,
    });

    if (governanceAuditSignatureRef.current === nextSignature) return;
    governanceAuditSignatureRef.current = nextSignature;

    if (!governanceRuntime.hasViolation) return;

    recordSecurityAuditEvent({
      action: "governance/runtime-drift",
      source: "governance",
      risk: "sensitive",
      status: "completed",
      note: assistantAuditCopy.governanceDriftDetected(governanceRuntime.driftLabel),
      context: {
        authority_label: governanceRuntime.authorityLabel,
        exposure_label: governanceRuntime.exposureLabel,
        drift_label: governanceRuntime.driftLabel,
        visible_cluster_ids: governanceRuntime.visibleClusterIds,
        visible_runtime_modes: governanceRuntime.visibleRuntimeModes,
      },
    });
  }, [governanceRuntime, language]);

  const recordAssistantTransaction = (summary: string, executedActions: string[]) => {
    const entry: AssistantTransactionEntry = createAssistantTransactionEntry(
      summary,
      executedActions,
      buildWorkbenchSnapshot(),
    );
    setAssistantTransactions((current) => [entry, ...current].slice(0, 12));
    return entry.id;
  };

  const rollbackAssistantTransaction = (transactionId: string) => {
    const entry = assistantTransactions.find((transaction) => transaction.id === transactionId);
    if (!entry) return;
    restoreWorkbenchSnapshot(entry.snapshot);
    setAssistantTransactions((current) => current.filter((transaction) => transaction.id !== transactionId));
    setMessage(assistantAuditCopy.transactionRolledBack);
  };

  const executeAssistantPlan = async (
    actions: AssistantPlanAction[],
    summary: string,
    invokeScriptAction: (
      action: string,
      payload?: Record<string, unknown>,
      source?: WorkbenchSecurityAuditSource,
      note?: string,
    ) => Promise<Record<string, unknown>>,
  ) => {
    const transactionId = recordAssistantTransaction(summary, actions.map((entry) => entry.action));
    try {
      for (const entry of actions) {
        await invokeScriptAction(entry.action, entry.payload ?? {}, "assistant", entry.reason);
      }
      setMessage(assistantAuditCopy.planExecuted);
      return transactionId;
    } catch (error) {
      rollbackAssistantTransaction(transactionId);
      throw error;
    }
  };

  return {
    assistantTransactions,
    appendScriptActionLog,
    executeAssistantPlan,
    getScriptSnapshot,
    recordManualDslAction,
    recordSecurityAuditEvent,
    rollbackAssistantTransaction,
    scriptActionLog,
    securityAuditLog,
    setSecurityAuditLog,
  };
}
