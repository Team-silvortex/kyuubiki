"use client";

import { downloadBlobFile, downloadTextFile } from "@/components/workbench/workbench-file-helpers";
import { SECURITY_EVENT_WINDOW_MS, type SecurityEventWindow } from "@/components/workbench/workbench-types";
import type { WorkbenchLanguage } from "@/components/workbench/workbench-copy";
import type { WorkbenchMacroPresetRecord, WorkbenchScriptSnippetPresetRecord } from "@/lib/scripting/workbench-script-runtime";
import {
  exportSecurityEvents,
  exportSecurityEventsCsv,
  fetchDatabaseExport,
  type JobResultRecord,
  type ProjectRecord,
} from "@/lib/api";
import type { WorkbenchSecurityAuditRisk, WorkbenchSecurityAuditSource } from "@/lib/workbench/security-audit";
import { exportProjectBundleZip } from "@/lib/projects/project-format";

export async function downloadWorkbenchProjectBundleJson(params: {
  selectedProject: ProjectRecord | null;
  buildBundle: () => Promise<{ bundle: string; partial: boolean }>;
  setMessage: (value: string) => void;
  labels: { projectExported: string; projectExportedPartial: string; initialFailed: string };
}) {
  const { selectedProject, buildBundle, setMessage, labels } = params;
  try {
    const { bundle, partial } = await buildBundle();
    downloadTextFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki.json`, bundle);
    setMessage(partial ? labels.projectExportedPartial : labels.projectExported);
  } catch (error) {
    setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function downloadWorkbenchProjectBundleZip(params: {
  selectedProject: ProjectRecord | null;
  buildBundle: () => Promise<{ bundle: string; partial: boolean }>;
  setMessage: (value: string) => void;
  labels: { projectExported: string; projectExportedPartial: string; initialFailed: string };
}) {
  const { selectedProject, buildBundle, setMessage, labels } = params;
  try {
    const { bundle, partial } = await buildBundle();
    const blob = await exportProjectBundleZip(bundle);
    downloadBlobFile(`${selectedProject?.name || "kyuubiki-project"}.kyuubiki`, blob);
    setMessage(partial ? labels.projectExportedPartial : labels.projectExported);
  } catch (error) {
    setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function downloadWorkbenchDatabaseSnapshot(params: {
  setMessage: (value: string) => void;
  labels: { databaseExported: string; initialFailed: string };
}) {
  const { setMessage, labels } = params;
  try {
    const snapshot = await fetchDatabaseExport();
    const timestamp = snapshot.exported_at.replaceAll(":", "-");
    downloadTextFile(`kyuubiki-database-${timestamp}.json`, JSON.stringify(snapshot, null, 2));
    setMessage(labels.databaseExported);
  } catch (error) {
    setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function downloadWorkbenchSecurityEventExport(params: {
  language: WorkbenchLanguage;
  securityEventWindowFilter: SecurityEventWindow;
  securityEventSourceFilter: WorkbenchSecurityAuditSource | "hub-assistant" | "";
  securityEventRiskFilter: WorkbenchSecurityAuditRisk | "";
  securityEventStatusFilter: "" | "allowed" | "blocked";
  securityEventActionFilter: string;
  setMessage: (value: string) => void;
  labels: { initialFailed: string };
}) {
  const {
    language,
    securityEventWindowFilter,
    securityEventSourceFilter,
    securityEventRiskFilter,
    securityEventStatusFilter,
    securityEventActionFilter,
    setMessage,
    labels,
  } = params;
  try {
    const windowMs =
      securityEventWindowFilter && securityEventWindowFilter in SECURITY_EVENT_WINDOW_MS
        ? SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter as keyof typeof SECURITY_EVENT_WINDOW_MS]
        : null;
    const occurredAfter = windowMs ? new Date(Date.now() - windowMs).toISOString() : undefined;
    const snapshot = await exportSecurityEvents({
      occurred_after: occurredAfter,
      source: securityEventSourceFilter || undefined,
      risk: securityEventRiskFilter || undefined,
      status: securityEventStatusFilter || undefined,
      action: securityEventActionFilter || undefined,
      limit: 500,
    });
    const timestamp = snapshot.exported_at.replaceAll(":", "-");
    downloadTextFile(`kyuubiki-security-events-${timestamp}.json`, JSON.stringify(snapshot, null, 2));
    setMessage(
      language === "zh"
        ? "安全事件分析包已下载。"
        : language === "ja"
          ? "セキュリティイベント分析バンドルを出力しました。"
          : "Security event export downloaded.",
    );
  } catch (error) {
    setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function downloadWorkbenchSecurityEventCsvExport(params: {
  language: WorkbenchLanguage;
  securityEventWindowFilter: SecurityEventWindow;
  securityEventSourceFilter: WorkbenchSecurityAuditSource | "hub-assistant" | "";
  securityEventRiskFilter: WorkbenchSecurityAuditRisk | "";
  securityEventStatusFilter: "" | "allowed" | "blocked";
  securityEventActionFilter: string;
  setMessage: (value: string) => void;
  labels: { initialFailed: string };
}) {
  const {
    language,
    securityEventWindowFilter,
    securityEventSourceFilter,
    securityEventRiskFilter,
    securityEventStatusFilter,
    securityEventActionFilter,
    setMessage,
    labels,
  } = params;
  try {
    const windowMs =
      securityEventWindowFilter && securityEventWindowFilter in SECURITY_EVENT_WINDOW_MS
        ? SECURITY_EVENT_WINDOW_MS[securityEventWindowFilter as keyof typeof SECURITY_EVENT_WINDOW_MS]
        : null;
    const occurredAfter = windowMs ? new Date(Date.now() - windowMs).toISOString() : undefined;
    const csv = await exportSecurityEventsCsv({
      occurred_after: occurredAfter,
      source: securityEventSourceFilter || undefined,
      risk: securityEventRiskFilter || undefined,
      status: securityEventStatusFilter || undefined,
      action: securityEventActionFilter || undefined,
      limit: 1000,
    });
    const timestamp = new Date().toISOString().replaceAll(":", "-");
    downloadTextFile(`kyuubiki-security-events-${timestamp}.csv`, csv);
    setMessage(
      language === "zh"
        ? "安全事件 CSV 已下载。"
        : language === "ja"
          ? "セキュリティイベント CSV を出力しました。"
          : "Security event CSV downloaded.",
    );
  } catch (error) {
    setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function buildWorkbenchProjectBundleJson(params: {
  project: ProjectRecord | null;
  models: unknown[];
  jobs: unknown[];
  results: JobResultRecord[];
  activeModelId: string | null;
  activeVersionId: string | null;
  workspaceSnapshot: Record<string, unknown>;
  automationPresets: WorkbenchMacroPresetRecord[];
  snippetPresets: WorkbenchScriptSnippetPresetRecord[];
  labels: { projectRequired: string };
}) {
  const { project, models, jobs, results, activeModelId, activeVersionId, workspaceSnapshot, automationPresets, snippetPresets, labels } =
    params;
  if (!project) {
    throw new Error(labels.projectRequired);
  }

  return JSON.stringify(
    {
      project,
      models,
      jobs,
      results,
      activeModelId,
      activeVersionId,
      workspaceSnapshot,
      automationPresets,
      snippetPresets,
    },
    null,
    2,
  );
}
