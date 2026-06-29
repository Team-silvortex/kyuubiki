export type ModelVersionRecord = {
  version_id: string;
  project_id: string;
  model_id: string;
  name: string;
  version_number: number;
  kind: string;
  material?: string | null;
  model_schema_version: string;
  payload: Record<string, unknown>;
  inserted_at: string;
  updated_at: string;
};

export type ModelRecord = {
  model_id: string;
  project_id: string;
  name: string;
  kind: string;
  material?: string | null;
  model_schema_version: string;
  payload: Record<string, unknown>;
  latest_version_id?: string | null;
  latest_version_number?: number | null;
  inserted_at: string;
  updated_at: string;
  versions?: ModelVersionRecord[];
};

export type ProjectRecord = {
  project_id: string;
  name: string;
  description?: string | null;
  inserted_at: string;
  updated_at: string;
  models?: ModelRecord[];
};

export type ProjectListPayload = {
  projects: ProjectRecord[];
};

export type ProjectEnvelope = {
  project: ProjectRecord;
};

export type ModelEnvelope = {
  model: ModelRecord;
};

export type ModelListPayload = {
  models: ModelRecord[];
};

export type ModelVersionEnvelope = {
  version: ModelVersionRecord;
};

export type ModelVersionListPayload = {
  versions: ModelVersionRecord[];
};
