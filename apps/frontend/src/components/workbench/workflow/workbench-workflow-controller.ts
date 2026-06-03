"use client";

import { useCallback, useMemo, useState, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import { fetchJobStatus, fetchWorkflowCatalog, submitWorkflowCatalogJob, type JobEnvelope, type WorkflowCatalogEntry, type WorkflowGraphJobResult } from "@/lib/api";
import type { WorkflowRunRecord, WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";

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

function builtInWorkflowSampleInputArtifacts(workflowId: string): Record<string, unknown> | null {
  if (workflowId !== "workflow.heat-to-thermo-quad-2d") {
    return null;
  }

  return {
    heat_model: {
      nodes: [
        { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
        { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
        { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
        { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
      ],
      elements: [
        {
          id: "hq0",
          node_i: 0,
          node_j: 1,
          node_k: 2,
          node_l: 3,
          thickness: 0.02,
          conductivity: 45,
        },
      ],
    },
  };
}

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
  const [workflowCatalogBusy, setWorkflowCatalogBusy] = useState(false);
  const [workflowPanelTab, setWorkflowPanelTab] = useState<WorkflowSurfaceTab>("overview");
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(null);
  const [workflowRuns, setWorkflowRuns] = useState<WorkflowRunRecord[]>([]);

  const refreshWorkflowCatalog = useCallback(async () => {
    setWorkflowCatalogBusy(true);

    try {
      const payload = await fetchWorkflowCatalog();
      setWorkflowCatalog(payload.workflows);
      setSelectedWorkflowId((current) =>
        current && payload.workflows.some((entry) => entry.id === current) ? current : payload.workflows[0]?.id ?? null,
      );
      setMessage(labels.workflowCatalogLoaded);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : labels.initialFailed);
    } finally {
      setWorkflowCatalogBusy(false);
    }
  }, [labels.initialFailed, labels.workflowCatalogLoaded, setMessage]);

  const runWorkflowCatalogEntry = useCallback(async (workflowId: string) => {
    const inputArtifacts = builtInWorkflowSampleInputArtifacts(workflowId);
    if (!inputArtifacts) {
      setMessage(labels.workflowCatalogUnsupported);
      return;
    }

    setWorkflowCatalogBusy(true);

    try {
      const payload = await submitWorkflowCatalogJob(workflowId, inputArtifacts);
      openWorkflowRunsSurface(workflowId);
      setJob(payload.job);
      setWorkflowRuns((current) =>
        upsertWorkflowRunRecord(current, {
          jobId: payload.job.job_id,
          workflowId,
          status: payload.job.status,
          progress: payload.job.progress ?? 0,
          currentNode: payload.job.message ?? null,
          updatedAt: payload.job.updated_at ?? null,
        }),
      );
      await refreshJobHistory();
      setMessage(`${labels.workflowCatalogQueued}: ${workflowId}`);

      const pollToken = ++jobPollTokenRef.current;

      for (let attempt = 0; attempt < 80; attempt += 1) {
        if (pollToken !== jobPollTokenRef.current) return;

        const next = await fetchJobStatus<WorkflowGraphJobResult>(payload.job.job_id);
        if (pollToken !== jobPollTokenRef.current) return;

        setJob(next.job);
        setWorkflowRuns((current) =>
          upsertWorkflowRunRecord(current, {
            jobId: next.job.job_id,
            workflowId,
            status: next.job.status,
            progress: next.job.progress ?? 0,
            currentNode:
              (next.result && isWorkflowGraphResult(next.result) ? next.result.current_node : null) ??
              next.job.message ??
              null,
            summary: next.result && isWorkflowGraphResult(next.result) ? summarizeWorkflowArtifacts(next.result) : null,
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

  const selectedWorkflow = useMemo(
    () => workflowCatalog.find((entry) => entry.id === selectedWorkflowId) ?? workflowCatalog[0] ?? null,
    [selectedWorkflowId, workflowCatalog],
  );
  const latestWorkflowRun = workflowRuns[0] ?? null;
  const latestWorkflowSummary = latestWorkflowRun?.summary ?? null;

  return {
    workflowCatalog,
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
  };
}
