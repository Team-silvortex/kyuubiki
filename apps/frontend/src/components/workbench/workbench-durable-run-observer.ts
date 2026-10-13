import type { DurableDirectEnvelope } from "@/lib/api/durable-direct-client";
import type { DurableDirectRecoveryHandle } from "@/lib/workbench/durable-direct-recovery";
import type { DurableDirectRunService } from "@/lib/workbench/durable-direct-run";
import type { JobState } from "@/lib/api/fem-shared";
import type { WorkbenchStudyResult } from "@/lib/workbench/study-run-backend-service-core";

export async function observeWorkbenchDurableRun(args: {
  handle: DurableDirectRecoveryHandle;
  envelope: DurableDirectEnvelope<WorkbenchStudyResult>;
  service: DurableDirectRunService;
  isCurrent: () => boolean;
  setJob: (job: JobState) => void;
  setResult: (result: WorkbenchStudyResult | null) => void;
  setMessage: (message: string) => void;
  sleep?: () => Promise<void>;
}) {
  const { handle, service, isCurrent, setJob, setResult, setMessage } = args;
  const jobId = `direct-durable-${handle.execution_id}`;
  const sleep = args.sleep ?? (() => new Promise<void>(resolve => setTimeout(resolve, 250)));
  let envelope = args.envelope;
  const admission = args.envelope.execution;
  for (let attempt = 0; attempt <= 40; attempt++) {
    if (!isCurrent()) throw new Error(`Direct execution observation was superseded: ${handle.execution_id}`);
    const execution = envelope.execution;
    if (admission.status !== "not_found" && execution.status !== "not_found"
      && (execution.request_id !== admission.request_id || execution.process_instance_id !== admission.process_instance_id
        || execution.request_digest !== admission.request_digest || execution.method !== admission.method)) {
      throw new Error(`direct:receipt_identity_changed: ${handle.execution_id}. Restore the original Agent store.`);
    }
    if (execution.status === "not_found" || execution.status === "outcome_unknown") {
      setMessage(`direct:recovery_required: ${execution.status} ${handle.execution_id}. No automatic replay.`);
      throw new Error(`direct:recovery_required: ${execution.status} ${handle.execution_id}`);
    }
    const terminal = execution.status !== "running";
    setJob({ job_id: jobId, status: execution.status === "running" ? "solving" : execution.status, worker_id: `direct-mesh@${handle.endpoint}`,
      direct_execution: handle, progress: terminal ? 1 : 0, has_result: execution.status === "completed" });
    if (terminal) {
      setResult(execution.status === "completed" ? execution.response.result : null);
      // Keep the identity on cleanup failure, but don't turn a completed calculation into a submission failure.
      let cleanupPending = false;
      try { await service.acknowledge(handle, envelope); } catch { cleanupPending = true; }
      if (!isCurrent()) throw new Error(`Direct execution observation was superseded: ${handle.execution_id}`);
      if (execution.status === "failed") throw new Error(`${execution.response.error.code}: ${execution.response.error.message}`);
      setMessage(`direct:completed: ${handle.execution_id}${cleanupPending ? "; recovery journal cleanup pending" : ""}`);
      return { ok: true as const, backend: "direct_mesh" as const, completion: "terminal" as const, jobId, status: "completed" as const };
    }
    setMessage(`direct:running: ${handle.execution_id}`);
    if (attempt === 40) break;
    await sleep();
    if (!isCurrent()) throw new Error(`Direct execution observation was superseded: ${handle.execution_id}`);
    try { envelope = await service.inspect<WorkbenchStudyResult>(handle); }
    catch (error) {
      if (isCurrent()) setMessage(`direct:observation_detached: ${handle.execution_id}. Inspect the original execution; do not resubmit.`);
      throw error;
    }
  }
  setMessage(`direct:observation_detached: ${handle.execution_id}. Inspect the original execution; do not resubmit.`);
  return { ok: true as const, backend: "direct_mesh" as const, completion: "detached" as const, jobId, status: "solving" as const };
}
