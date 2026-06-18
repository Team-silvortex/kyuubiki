import type { JobStatusDetail, WorkflowRunStatus } from "./job-status";

export type ModelMaterial = {
  id: string;
  name: string;
  youngs_modulus: number;
  poisson_ratio?: number | null;
};

export type JobState = {
  job_id: string;
  status: WorkflowRunStatus;
  status_detail?: JobStatusDetail | null;
  worker_id: string | null;
  model_version_id?: string | null;
  message?: string | null;
  progress: number;
  residual?: number | null;
  iteration?: number | null;
  has_result?: boolean;
  project_id?: string;
  simulation_case_id?: string;
  created_at?: string;
  updated_at?: string;
};

export type JobEnvelope<TResult = unknown> = {
  job: JobState;
  result?: TResult;
};

export type JobResultRecord<TResult = unknown> = {
  job_id: string;
  status?: string;
  worker_id?: string | null;
  result: TResult;
};

export type JobHistoryPayload = {
  jobs: JobState[];
};

export function resolveMaterialLookup(materials: ModelMaterial[] | undefined) {
  return new Map((materials ?? []).map((material) => [material.id, material]));
}
