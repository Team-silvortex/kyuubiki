import type { JobState } from "@/lib/api/fem-shared";
import { isWorkflowJobStatusContractValid } from "@/lib/api/job-status";

export function readWorkbenchCancellationReceipt(payload: unknown, jobId: string): JobState {
  const job = (payload as { job?: JobState } | null)?.job;
  if (job?.job_id !== jobId) throw new Error("Cancellation response belongs to a different job.");
  if (!isWorkflowJobStatusContractValid({
    status: job.status, progress: job.progress, statusDetail: job.status_detail,
  })) throw new Error("JOB_CANCELLATION_INVALID: invalid job status contract.");
  if (job.status !== "cancelled") {
    throw new Error(`JOB_CANCELLATION_NOT_CONFIRMED: backend reported ${job.status}; job observation remains active.`);
  }
  return job;
}
