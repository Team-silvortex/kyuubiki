"use client";

import { useCallback, useMemo, useState, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert, upsertWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import {
  fetchJobStatus,
  fetchWorkflowCatalog,
  fetchWorkflowOperators,
  submitWorkflowCatalogJob,
  submitWorkflowGraphJob,
} from "@/lib/api/runtime-client";
import {
  isWorkflowRunFailureStatus,
  isWorkflowRunTerminalStatus,
} from "@/lib/api/job-status";
import type { JobEnvelope } from "@/lib/api/fem-shared";
import type {
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowGraphJobResult,
  WorkflowOperatorDescriptor,
  WorkflowOperatorModuleSummary,
} from "@/lib/api/workflow-types";
import type { WorkflowRunRecord, WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";
import { builtInWorkflowSampleInputArtifacts } from "@/components/workbench/workflow/workbench-workflow-sample-inputs";
import {
  buildStoredLocalWorkflowCatalogEntries,
  findStoredLocalWorkflow,
} from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { parseWorkflowInputArtifactTexts } from "@/components/workbench/workflow/workbench-workflow-input-artifacts";
import { summarizeWorkflowRunTrace } from "@/components/workbench/workflow/workbench-workflow-run-trace-summary";
import { summarizeWorkflowResultArtifacts } from "@/components/workbench/workflow/workbench-workflow-summary-contract";
import {
  clearWorkbenchRuntimeRecoveryIssue,
  upsertWorkbenchRuntimeRecoveryIssue,
  type WorkbenchRuntimeRecoveryState,
} from "@/components/workbench/workbench-runtime-recovery";
import { normalizeWorkbenchRequestError } from "@/lib/api/request-errors";

type WorkflowControllerLabels = {
  workflowCatalogLoaded: string;
  workflowCatalogUnsupported: string;
  workflowCatalogQueued: string;
  workflowCatalogCompleted: string;
  workflowCatalogFailed: string;
  initialFailed: string;
  pollingDetached: string;
};

type UseWorkbenchWorkflowControllerArgs = {
  labels: WorkflowControllerLabels;
  jobPollTokenRef: MutableRefObject<number>;
  refreshJobHistory: () => Promise<void>;
  setRuntimeRecovery: Dispatch<SetStateAction<WorkbenchRuntimeRecoveryState>>;
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>;
  setMessage: Dispatch<SetStateAction<string>>;
  setSystemAlerts: Dispatch<SetStateAction<WorkbenchAlertItem[]>>;
  openWorkflowRunsSurface: (workflowId: string) => void;
};

export function isWorkflowGraphResult(value: unknown): value is WorkflowGraphJobResult {
  return (
    typeof value === "object" &&
    value !== null &&
    "workflow_id" in value &&
    "completed_nodes" in value &&
    "artifacts" in value
  );
}

export function summarizeWorkflowArtifacts(result: WorkflowGraphJobResult): string | null {
  return summarizeWorkflowResultArtifacts(result);
}

export function upsertWorkflowRunRecord(current: WorkflowRunRecord[], next: WorkflowRunRecord): WorkflowRunRecord[] {
  const withoutMatch = current.filter((entry) => entry.jobId !== next.jobId);
  return [next, ...withoutMatch].slice(0, 12);
}

export function useWorkbenchWorkflowController({
  labels,
  jobPollTokenRef,
  refreshJobHistory,
  setRuntimeRecovery,
  setJob,
  setMessage,
  setSystemAlerts,
  openWorkflowRunsSurface,
}: UseWorkbenchWorkflowControllerArgs) {
  const [workflowCatalog, setWorkflowCatalog] = useState<WorkflowCatalogEntry[]>([]);
  const [workflowOperatorDescriptors, setWorkflowOperatorDescriptors] = useState<WorkflowOperatorDescriptor[]>([]);
  const [workflowOperatorModules, setWorkflowOperatorModules] = useState<WorkflowOperatorModuleSummary[]>([]);
  const [workflowCatalogBusy, setWorkflowCatalogBusy] = useState(false);
  const [workflowPanelTab, setWorkflowPanelTab] = useState<WorkflowSurfaceTab>("overview");
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(null);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRunRecord[]>([]);

  const refreshWorkflowCatalog = useCallback(async () => {
    setWorkflowCatalogBusy(true);

    try {
      const [payload, operatorPayload] = await Promise.all([
        fetchWorkflowCatalog(),
        fetchWorkflowOperators().catch(() => ({
          modules: [] as WorkflowOperatorModuleSummary[],
          operators: [] as WorkflowOperatorDescriptor[],
        })),
      ]);
      const localEntries = buildStoredLocalWorkflowCatalogEntries();
      setWorkflowOperatorDescriptors(operatorPayload.operators ?? []);
      setWorkflowOperatorModules(operatorPayload.modules ?? []);
      setWorkflowCatalog([...localEntries, ...payload.workflows]);
      setSelectedWorkflowId((current) =>
        current && [...localEntries, ...payload.workflows].some((entry) => entry.id === current)
          ? current
          : [...localEntries, ...payload.workflows][0]?.id ?? null,
      );
      dismissWorkbenchAlert(setSystemAlerts, "workflow-catalog-error");
      setRuntimeRecovery((current) => clearWorkbenchRuntimeRecoveryIssue(current, "workflow_catalog"));
      setMessage(labels.workflowCatalogLoaded);
    } catch (error) {
      const message = error instanceof Error ? error.message : labels.initialFailed;
      upsertWorkbenchAlert(setSystemAlerts, {
        id: "workflow-catalog-error",
        message,
        tone: "error",
      });
      setRuntimeRecovery((current) =>
        upsertWorkbenchRuntimeRecoveryIssue({
          channel: "workflow_catalog",
          current,
          error: normalizeWorkbenchRequestError(error, "Workflow catalog"),
          scopeLabel: "Workflow catalog",
        }),
      );
      setMessage(message);
    } finally {
      setWorkflowCatalogBusy(false);
    }
  }, [labels.initialFailed, labels.workflowCatalogLoaded, setMessage, setSystemAlerts]);

  const runWorkflowJob = useCallback(async (params: {
    sourceWorkflowId: string;
    displayWorkflowId: string;
    graph?: WorkflowGraphDefinition;
    inputArtifacts?: Record<string, unknown>;
  }) => {
    const inputArtifacts =
      params.inputArtifacts ?? builtInWorkflowSampleInputArtifacts(params.sourceWorkflowId);
    if (!inputArtifacts) {
      upsertWorkbenchAlert(setSystemAlerts, {
        id: "workflow-run-error",
        message: labels.workflowCatalogUnsupported,
        tone: "warning",
      });
      setMessage(labels.workflowCatalogUnsupported);
      return;
    }

    setWorkflowCatalogBusy(true);

    try {
      dismissWorkbenchAlert(setSystemAlerts, "workflow-run-error");
      const payload = params.graph
        ? await submitWorkflowGraphJob(params.graph, inputArtifacts)
        : await submitWorkflowCatalogJob(params.sourceWorkflowId, inputArtifacts);
      const activeJobId = payload.job.job_id;
      openWorkflowRunsSurface(params.displayWorkflowId);
      setJob(payload.job);
      setWorkflowRuns((current) =>
        upsertWorkflowRunRecord(current, {
          jobId: activeJobId,
          workflowId: params.displayWorkflowId,
          status: payload.job.status,
          statusDetail: payload.job.status_detail ?? null,
          progress: payload.job.progress ?? 0,
          pollingState: "attached",
          currentNode: payload.job.message ?? null,
          updatedAt: payload.job.updated_at ?? null,
        }),
      );
      await refreshJobHistory();
      setMessage(`${labels.workflowCatalogQueued}: ${params.displayWorkflowId}`);

      const pollToken = ++jobPollTokenRef.current;

      for (let attempt = 0; attempt < 80; attempt += 1) {
        if (pollToken !== jobPollTokenRef.current) return;

        const next = await fetchJobStatus<WorkflowGraphJobResult>(payload.job.job_id);
        if (pollToken !== jobPollTokenRef.current) return;

        setJob(next.job);
        setWorkflowRuns((current) =>
          upsertWorkflowRunRecord(current, {
            jobId: next.job.job_id,
            workflowId: params.displayWorkflowId,
            status: next.job.status,
            statusDetail: next.job.status_detail ?? null,
            progress: next.job.progress ?? 0,
            pollingState: "attached",
            currentNode:
              (next.result && isWorkflowGraphResult(next.result) ? next.result.current_node : null) ??
              next.job.message ??
              null,
            summary: next.result && isWorkflowGraphResult(next.result) ? summarizeWorkflowArtifacts(next.result) : null,
            skippedNodes: next.result && isWorkflowGraphResult(next.result) ? next.result.skipped_nodes ?? [] : [],
            branchDecisions: next.result && isWorkflowGraphResult(next.result) ? next.result.branch_decisions ?? [] : [],
            nodeRuns: next.result && isWorkflowGraphResult(next.result) ? next.result.node_runs ?? [] : [],
            artifactLineage: next.result && isWorkflowGraphResult(next.result) ? next.result.artifact_lineage ?? [] : [],
            result: next.result && isWorkflowGraphResult(next.result) ? next.result : undefined,
            traceSummary: next.result && isWorkflowGraphResult(next.result) ? summarizeWorkflowRunTrace(next.result) : undefined,
            updatedAt: next.job.updated_at ?? null,
          }),
        );

        if (next.job.status === "completed" && next.result && isWorkflowGraphResult(next.result)) {
          await refreshJobHistory();
          const summary = summarizeWorkflowArtifacts(next.result);
          dismissWorkbenchAlert(setSystemAlerts, "workflow-run-error");
          setMessage(
            summary
              ? `${labels.workflowCatalogCompleted}: ${next.result.workflow_id} (${summary})`
              : `${labels.workflowCatalogCompleted}: ${next.result.workflow_id}`,
          );
          return;
        }

        if (isWorkflowRunFailureStatus(next.job.status)) {
          await refreshJobHistory();
          upsertWorkbenchAlert(setSystemAlerts, {
            id: "workflow-run-error",
            message: labels.workflowCatalogFailed,
            tone: "error",
          });
          setMessage(labels.workflowCatalogFailed);
          return;
        }

        if (isWorkflowRunTerminalStatus(next.job.status)) {
          await refreshJobHistory();
          return;
        }

        await new Promise((resolve) => window.setTimeout(resolve, 350));
      }

      setWorkflowRuns((current) =>
        current.map((entry) =>
          entry.jobId === activeJobId
            ? {
                ...entry,
                pollingState: "detached",
                updatedAt: new Date().toISOString(),
              }
            : entry,
        ),
      );
      await refreshJobHistory();
      setMessage(labels.pollingDetached);
    } catch (error) {
      const message = error instanceof Error ? error.message : labels.initialFailed;
      upsertWorkbenchAlert(setSystemAlerts, {
        id: "workflow-run-error",
        message,
        tone: "error",
      });
      setMessage(message);
    } finally {
      setWorkflowCatalogBusy(false);
    }
  }, [
    jobPollTokenRef,
    labels.initialFailed,
    labels.pollingDetached,
    labels.workflowCatalogCompleted,
    labels.workflowCatalogFailed,
    labels.workflowCatalogQueued,
    labels.workflowCatalogUnsupported,
    openWorkflowRunsSurface,
    refreshJobHistory,
    setJob,
    setMessage,
    setSystemAlerts,
  ]);

  const runWorkflowCatalogEntry = useCallback(
    async (workflowId: string) => {
      const localWorkflow = findStoredLocalWorkflow(workflowId);
      if (localWorkflow) {
        const parsedInputs = parseWorkflowInputArtifactTexts(localWorkflow.inputArtifactTexts ?? {});
        if (parsedInputs.invalidKeys.length > 0) {
          upsertWorkbenchAlert(setSystemAlerts, {
            id: "workflow-run-error",
            message: labels.workflowCatalogUnsupported,
            tone: "warning",
          });
          setMessage(labels.workflowCatalogUnsupported);
          return;
        }
        return runWorkflowJob({
          sourceWorkflowId: localWorkflow.sourceWorkflowId,
          displayWorkflowId: localWorkflow.id,
          graph: localWorkflow.graph,
          inputArtifacts:
            Object.keys(parsedInputs.inputArtifacts).length > 0
              ? parsedInputs.inputArtifacts
              : builtInWorkflowSampleInputArtifacts(localWorkflow.sourceWorkflowId) ?? undefined,
        });
      }
      return runWorkflowJob({ sourceWorkflowId: workflowId, displayWorkflowId: workflowId });
    },
    [labels.workflowCatalogUnsupported, runWorkflowJob, setMessage, setSystemAlerts],
  );

  const runWorkflowDraft = useCallback(
    async (
      workflowId: string,
      graph: WorkflowGraphDefinition,
      inputArtifacts: Record<string, unknown>,
    ) =>
      runWorkflowJob({
        sourceWorkflowId: workflowId,
        displayWorkflowId: graph.id || workflowId,
        graph,
        inputArtifacts,
      }),
    [runWorkflowJob],
  );

  const selectedWorkflow = useMemo(
    () => workflowCatalog.find((entry) => entry.id === selectedWorkflowId) ?? workflowCatalog[0] ?? null,
    [selectedWorkflowId, workflowCatalog],
  );
  const latestWorkflowRun = workflowRuns[0] ?? null;
  const latestWorkflowSummary = latestWorkflowRun?.summary ?? null;

  return {
    workflowCatalog,
    workflowOperatorDescriptors,
    workflowOperatorModules,
    workflowCatalogBusy,
    workflowPanelTab,
    setWorkflowPanelTab,
    selectedWorkflowId,
    setSelectedWorkflowId,
    workflowRuns,
    setWorkflowRuns,
    selectedWorkflow,
    latestWorkflowSummary,
    refreshWorkflowCatalog,
    runWorkflowCatalogEntry,
    runWorkflowDraft,
  };
}
