"use client";

import type { JobState } from "@/lib/api";
import { getWorkbenchScriptErrorCopy } from "@/components/workbench/workbench-extended-language-copy";
import { getWorkbenchScriptMacroSummary } from "@/components/workbench/workbench-script-catalog-copy";
import type { WorkbenchSecurityAuditSource } from "@/lib/workbench/security-audit";
import {
  getWorkbenchScriptMacroDefinition,
  resolveWorkbenchMacroPayloadTemplates,
  type WorkbenchScriptSnapshot,
} from "@/lib/scripting/workbench-script-runtime";

type ScriptMacroDataControllerDeps = {
  action: string;
  payload: Record<string, unknown>;
  source: WorkbenchSecurityAuditSource;
  note?: string;
  language: string;
  getScriptSnapshot: () => WorkbenchScriptSnapshot;
  invokeScriptAction: (
    action: string,
    payload?: Record<string, unknown>,
    source?: WorkbenchSecurityAuditSource,
    note?: string,
  ) => Promise<Record<string, unknown>>;
  setSystemDataTab: (value: "jobs" | "results") => void;
  setAdminFilterProjectId: (value: string) => void;
  setAdminFilterModelVersionId: (value: string) => void;
  setSelectedAdminJobId: (value: string) => void;
  setSelectedAdminResultJobId: (value: string) => void;
  setSidebarSection: (value: "study" | "model" | "workflow" | "library" | "system") => void;
  setSystemPanelTab: (value: "config" | "scripts" | "runtime" | "data") => void;
  resolveScriptLinkedJob: (payload: Record<string, unknown>) => JobState | null;
  openModelVersionById: (modelVersionId: string) => void;
  openProjectContextById: (projectId: string) => void;
  applyJobContextToWorkbench: (linkedJob: JobState) => void;
  downloadDatabaseSnapshot: () => Promise<void>;
};

export async function handleWorkbenchScriptMacroDataAction({
  action,
  payload,
  source,
  note,
  language,
  getScriptSnapshot,
  invokeScriptAction,
  setSystemDataTab,
  setAdminFilterProjectId,
  setAdminFilterModelVersionId,
  setSelectedAdminJobId,
  setSelectedAdminResultJobId,
  setSidebarSection,
  setSystemPanelTab,
  resolveScriptLinkedJob,
  openModelVersionById,
  openProjectContextById,
  applyJobContextToWorkbench,
  downloadDatabaseSnapshot,
}: ScriptMacroDataControllerDeps): Promise<Record<string, unknown> | null> {
  const copy = getWorkbenchScriptErrorCopy(language);
  switch (action) {
    case "macro/run": {
      const macroId = typeof payload.macroId === "string" ? payload.macroId : null;
      const macro = macroId ? getWorkbenchScriptMacroDefinition(macroId) : null;

      if (!macro) {
        throw new Error(copy.macroMissing);
      }

      const macroPayload = Object.fromEntries(Object.entries(payload).filter(([key]) => key !== "macroId"));
      const macroSnapshot = getScriptSnapshot();

      for (const step of macro.steps) {
        const nextPayload = resolveWorkbenchMacroPayloadTemplates(step.payload ?? {}, macroPayload, macroSnapshot) as Record<string, unknown>;
        await invokeScriptAction(step.action, nextPayload, source, note ?? getWorkbenchScriptMacroSummary(macro, language));
      }

      return { ok: true, action, macroId: macro.id, stepCount: macro.steps.length };
    }
    case "data/setFilters": {
      if (payload.activeTab === "jobs" || payload.activeTab === "results") {
        setSystemDataTab(payload.activeTab);
      }
      if (typeof payload.projectId === "string" || payload.projectId === null) {
        setAdminFilterProjectId(typeof payload.projectId === "string" ? payload.projectId : "");
      }
      if (typeof payload.modelVersionId === "string" || payload.modelVersionId === null) {
        setAdminFilterModelVersionId(typeof payload.modelVersionId === "string" ? payload.modelVersionId : "");
      }
      setSidebarSection("system");
      setSystemPanelTab("data");
      return { ok: true, action };
    }
    case "data/selectRecord": {
      if (payload.activeTab === "jobs" || payload.activeTab === "results") {
        setSystemDataTab(payload.activeTab);
      }
      if (typeof payload.jobId === "string") {
        setSelectedAdminJobId(payload.jobId);
      }
      if (typeof payload.resultJobId === "string") {
        setSelectedAdminResultJobId(payload.resultJobId);
      }
      setSidebarSection("system");
      setSystemPanelTab("data");
      return { ok: true, action };
    }
    case "data/openLinkedContext": {
      const mode =
        payload.mode === "apply" || payload.mode === "project" || payload.mode === "version" ? payload.mode : "apply";
      const linkedJob = resolveScriptLinkedJob(payload);

      if (!linkedJob) {
        throw new Error(copy.linkedContextMissing);
      }

      if (mode === "version") {
        if (!linkedJob.model_version_id) {
          throw new Error(copy.linkedVersionMissing);
        }
        openModelVersionById(linkedJob.model_version_id);
      } else if (mode === "project") {
        if (!linkedJob.project_id) {
          throw new Error(copy.linkedProjectMissing);
        }
        openProjectContextById(linkedJob.project_id);
      } else {
        applyJobContextToWorkbench(linkedJob);
      }

      return {
        ok: true,
        action,
        mode,
        jobId: linkedJob.job_id,
        projectId: linkedJob.project_id ?? null,
        modelVersionId: linkedJob.model_version_id ?? null,
      };
    }
    case "data/exportDatabase": {
      await downloadDatabaseSnapshot();
      return { ok: true, action };
    }
    default:
      return null;
  }
}
