import type { JobState } from "@/lib/api/fem-shared";
import type { ModelRecord, ModelVersionRecord, ProjectRecord } from "@/lib/api/project-types";

export type DatabaseExportPayload = {
  exported_at: string;
  projects: ProjectRecord[];
  models: ModelRecord[];
  model_versions: ModelVersionRecord[];
  jobs: JobState[];
  results: Array<{
    job_id: string;
    result: Record<string, unknown>;
    inserted_at?: string;
    updated_at?: string;
  }>;
  security_events?: SecurityEventRecord[];
};

export type SecurityEventRecord = {
  event_id: string;
  event_type: string;
  source: string;
  action: string;
  risk: string;
  status: string;
  note?: string | null;
  context: Record<string, unknown>;
  occurred_at: string;
  inserted_at?: string;
  updated_at?: string;
};

export type SecurityEventListPayload = {
  events: SecurityEventRecord[];
};

export type SecurityEventEnvelope = {
  event: SecurityEventRecord;
};

export type SecurityEventExportPayload = {
  exported_at: string;
  schema: {
    name: string;
    version: number;
    fields: Array<{ name: string; type: string }>;
  };
  filters: Record<string, string>;
  summary: {
    total: number;
    by_source: Record<string, number>;
    by_risk: Record<string, number>;
    by_status: Record<string, number>;
  };
  events: SecurityEventRecord[];
};

export type ResultRecord = {
  job_id: string;
  result: Record<string, unknown>;
  inserted_at?: string;
  updated_at?: string;
};

export type ResultListPayload = {
  results: ResultRecord[];
};

export type ResultChunkKind = "nodes" | "elements";

export type ResultChunkPayload<TItem = Record<string, unknown>> = {
  job_id: string;
  kind: ResultChunkKind;
  offset: number;
  limit: number;
  returned: number;
  total: number;
  items: TItem[];
};
