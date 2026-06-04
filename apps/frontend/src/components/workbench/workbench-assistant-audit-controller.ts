"use client";

import { useEffect, useState } from "react";
import { createSecurityEvent } from "@/lib/api";
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

type AssistantPlanAction = {
  action: string;
  payload?: Record<string, unknown>;
  reason?: string;
};

type AssistantAuditControllerDeps = {
  language: "en" | "zh" | "ja" | "es";
  scriptRecordingMode: boolean;
  frontendRuntimeMode: string;
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
  const [scriptActionLog, setScriptActionLog] = useState<WorkbenchScriptActionLogEntry[]>([]);
  const [assistantTransactions, setAssistantTransactions] = useState<AssistantTransactionEntry[]>([]);
  const [securityAuditLog, setSecurityAuditLog] = useState<WorkbenchSecurityAuditEntry[]>([]);

  useEffect(() => {
    setSecurityAuditLog(readSecurityAuditLog());
  }, []);

  useEffect(() => {
    writeSecurityAuditLog(securityAuditLog);
  }, [securityAuditLog]);

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
      note: language === "zh" ? "手动 UI 录制" : language === "ja" ? "手動 UI 操作から記録" : "Recorded from manual UI interaction",
    });
  };

  const persistSecurityAuditEvent = async (entry: WorkbenchSecurityAuditEntry) => {
    try {
      await createSecurityEvent({
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
    setMessage(language === "zh" ? "已回滚上一轮助手事务。" : language === "ja" ? "直前のアシスタント操作をロールバックしました。" : "Rolled back the last assistant transaction.");
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
      setMessage(language === "zh" ? "助手计划已执行。" : language === "ja" ? "アシスタントのプランを実行しました。" : "Assistant plan executed.");
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
