"use client";

import { useCallback, useMemo, useState, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import {
  fetchJobStatus,
  fetchWorkflowCatalog,
  fetchWorkflowOperators,
  submitWorkflowCatalogJob,
  submitWorkflowGraphJob,
  type JobEnvelope,
  type WorkflowCatalogEntry,
  type WorkflowGraphDefinition,
  type WorkflowGraphJobResult,
  type WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { WorkflowRunRecord, WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";
import { builtInWorkflowSampleInputArtifacts } from "@/components/workbench/workflow/workbench-workflow-sample-inputs";
import {
  buildStoredLocalWorkflowCatalogEntries,
  findStoredLocalWorkflow,
} from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { parseWorkflowInputArtifactTexts } from "@/components/workbench/workflow/workbench-workflow-input-artifacts";
import { summarizeWorkflowRunTrace } from "@/components/workbench/workflow/workbench-workflow-run-trace-summary";

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
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>;
  setMessage: Dispatch<SetStateAction<string>>;
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
  const exported = result.artifacts["json_output.json"];
  if (!exported || typeof exported !== "object" || exported === null || !("content" in exported)) {
    return null;
  }

  const content = (exported as { content?: unknown }).content;
  if (typeof content !== "string") {
    return null;
  }

  try {
    const parsed = JSON.parse(content) as Record<string, unknown>;
    const summary = Object.entries(parsed)
      .slice(0, 3)
      .map(([key, value]) => `${key}=${typeof value === "number" ? value.toExponential(3) : String(value)}`)
      .join(", ");
    return summary || null;
  } catch {
    return null;
  }
}

export function upsertWorkflowRunRecord(current: WorkflowRunRecord[], next: WorkflowRunRecord): WorkflowRunRecord[] {
  const withoutMatch = current.filter((entry) => entry.jobId !== next.jobId);
  return [next, ...withoutMatch].slice(0, 12);
}

export function useWorkbenchWorkflowController({
  labels,
  jobPollTokenRef,
  refreshJobHistory,
  setJob,
  setMessage,
  openWorkflowRunsSurface,
}: UseWorkbenchWorkflowControllerArgs) {
  const [workflowCatalog, setWorkflowCatalog] = useState<WorkflowCatalogEntry[]>([]);
  const [workflowOperatorDescriptors, setWorkflowOperatorDescriptors] = useState<WorkflowOperatorDescriptor[]>([]);
  const [workflowCatalogBusy, setWorkflowCatalogBusy] = useState(false);
  const [workflowPanelTab, setWorkflowPanelTab] = useState<WorkflowSurfaceTab>("overview");
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(null);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRunRecord[]>([]);

  const refreshWorkflowCatalog = useCallback(async () => {
    setWorkflowCatalogBusy(true);

    try {
      const [payload, operatorPayload] = await Promise.all([
        fetchWorkflowCatalog(),
        fetchWorkflowOperators().catch(() => ({ operators: [] as WorkflowOperatorDescriptor[] })),
      ]);
      const localEntries = buildStoredLocalWorkflowCatalogEntries();
      setWorkflowOperatorDescriptors(operatorPayload.operators ?? []);
      setWorkflowCatalog([...localEntries, ...payload.workflows]);
      setSelectedWorkflowId((current) =>
        current && [...localEntries, ...payload.workflows].some((entry) => entry.id === current)
          ? current
          : [...localEntries, ...payload.workflows][0]?.id ?? null,
      );
      setMessage(labels.workflowCatalogLoaded);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : labels.initialFailed);
    } finally {
      setWorkflowCatalogBusy(false);
    }
  }, [labels.initialFailed, labels.workflowCatalogLoaded, setMessage]);

  const runWorkflowJob = useCallback(async (params: {
    sourceWorkflowId: string;
    displayWorkflowId: string;
    graph?: WorkflowGraphDefinition;
    inputArtifacts?: Record<string, unknown>;
  }) => {
    const inputArtifacts =
      params.inputArtifacts ?? builtInWorkflowSampleInputArtifacts(params.sourceWorkflowId);
    if (!inputArtifacts) {
      setMessage(labels.workflowCatalogUnsupported);
      return;
    }

    setWorkflowCatalogBusy(true);

    try {
      const payload = params.graph
        ? await submitWorkflowGraphJob(params.graph, inputArtifacts)
        : await submitWorkflowCatalogJob(params.sourceWorkflowId, inputArtifacts);
      openWorkflowRunsSurface(params.displayWorkflowId);
      setJob(payload.job);
      setWorkflowRuns((current) =>
        upsertWorkflowRunRecord(current, {
          jobId: payload.job.job_id,
          workflowId: params.displayWorkflowId,
          status: payload.job.status,
          progress: payload.job.progress ?? 0,
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
            progress: next.job.progress ?? 0,
            currentNode:
              (next.result && isWorkflowGraphResult(next.result) ? next.result.current_node : null) ??
              next.job.message ??
              null,
            summary: next.result && isWorkflowGraphResult(next.result) ? summarizeWorkflowArtifacts(next.result) : null,
            skippedNodes: next.result && isWorkflowGraphResult(next.result) ? next.result.skipped_nodes ?? [] : [],
            branchDecisions: next.result && isWorkflowGraphResult(next.result) ? next.result.branch_decisions ?? [] : [],
            nodeRuns: next.result && isWorkflowGraphResult(next.result) ? next.result.node_runs ?? [] : [],
            artifactLineage: next.result && isWorkflowGraphResult(next.result) ? next.result.artifact_lineage ?? [] : [],
            traceSummary: next.result && isWorkflowGraphResult(next.result) ? summarizeWorkflowRunTrace(next.result) : undefined,
            updatedAt: next.job.updated_at ?? null,
          }),
        );

        if (next.job.status === "completed" && next.result && isWorkflowGraphResult(next.result)) {
          await refreshJobHistory();
          const summary = summarizeWorkflowArtifacts(next.result);
          setMessage(
            summary
              ? `${labels.workflowCatalogCompleted}: ${next.result.workflow_id} (${summary})`
              : `${labels.workflowCatalogCompleted}: ${next.result.workflow_id}`,
          );
          return;
        }

        if (next.job.status === "failed" || next.job.status === "cancelled") {
          await refreshJobHistory();
          setMessage(labels.workflowCatalogFailed);
          return;
        }

        await new Promise((resolve) => window.setTimeout(resolve, 350));
      }

      await refreshJobHistory();
      setMessage(labels.pollingDetached);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : labels.initialFailed);
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
  ]);

  const runWorkflowCatalogEntry = useCallback(
    async (workflowId: string) => {
      const localWorkflow = findStoredLocalWorkflow(workflowId);
      if (localWorkflow) {
        const parsedInputs = parseWorkflowInputArtifactTexts(localWorkflow.inputArtifactTexts ?? {});
        if (parsedInputs.invalidKeys.length > 0) {
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
    [labels.workflowCatalogUnsupported, runWorkflowJob, setMessage],
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
