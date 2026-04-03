import JSZip from "jszip";
import type { JobResultRecord, ModelRecord, ModelVersionRecord, ProjectRecord } from "@/lib/api";

export const PROJECT_SCHEMA_VERSION = "kyuubiki.project/v1";
const PROJECT_MANIFEST_PATH = "project.json";
const PROJECT_RECORD_PATH = "project/project.json";
const WORKSPACE_SNAPSHOT_PATH = "workspace/current-model.json";
const JOBS_INDEX_PATH = "jobs/jobs.json";
const RESULTS_INDEX_PATH = "results/results.json";

export type ProjectBundle = {
  project_schema_version: string;
  exported_at?: string;
  project: ProjectRecord;
  models: ModelRecord[];
  model_versions: ModelVersionRecord[];
  active_model_id?: string | null;
  active_version_id?: string | null;
  workspace_snapshot?: Record<string, unknown> | null;
  jobs?: Array<Record<string, unknown>>;
  results?: JobResultRecord[];
};

function normalizeBundle(raw: Partial<ProjectBundle>): ProjectBundle {
  if (raw.project_schema_version !== PROJECT_SCHEMA_VERSION) {
    throw new Error(`unsupported project_schema_version: ${String(raw.project_schema_version)}`);
  }

  if (!raw.project || !raw.models || !raw.model_versions) {
    throw new Error("project bundle is missing required sections");
  }

  return {
    project_schema_version: raw.project_schema_version,
    exported_at: raw.exported_at,
    project: raw.project,
    models: raw.models,
    model_versions: raw.model_versions,
    active_model_id: raw.active_model_id ?? null,
    active_version_id: raw.active_version_id ?? null,
    workspace_snapshot: raw.workspace_snapshot ?? null,
    jobs: raw.jobs ?? [],
    results: raw.results ?? [],
  };
}

export function parseProjectBundleJson(text: string): ProjectBundle {
  return normalizeBundle(JSON.parse(text) as Partial<ProjectBundle>);
}

export async function parseProjectBundleFile(file: File): Promise<ProjectBundle> {
  if (file.name.endsWith(".kyuubiki")) {
    const zip = await JSZip.loadAsync(await file.arrayBuffer());
    const manifest = zip.file(PROJECT_MANIFEST_PATH) ?? zip.file(PROJECT_RECORD_PATH);

    if (!manifest) {
      throw new Error(`missing ${PROJECT_MANIFEST_PATH} in project archive`);
    }

    const bundle = parseProjectBundleJson(await manifest.async("string"));

    if (!bundle.workspace_snapshot) {
      const workspaceSnapshot = zip.file(WORKSPACE_SNAPSHOT_PATH);
      if (workspaceSnapshot) {
        bundle.workspace_snapshot = JSON.parse(await workspaceSnapshot.async("string")) as Record<string, unknown>;
      }
    }

    if ((bundle.jobs?.length ?? 0) === 0) {
      const jobsIndex = zip.file(JOBS_INDEX_PATH);
      if (jobsIndex) {
        bundle.jobs = JSON.parse(await jobsIndex.async("string")) as Array<Record<string, unknown>>;
      }
    }

    if ((bundle.results?.length ?? 0) === 0) {
      const resultsIndex = zip.file(RESULTS_INDEX_PATH);
      if (resultsIndex) {
        bundle.results = JSON.parse(await resultsIndex.async("string")) as JobResultRecord[];
      }
    }

    return bundle;
  }

  return parseProjectBundleJson(await file.text());
}

export async function exportProjectBundleZip(bundleJson: string): Promise<Blob> {
  const bundle = parseProjectBundleJson(bundleJson);
  const zip = new JSZip();
  zip.file(PROJECT_MANIFEST_PATH, bundleJson);
  zip.file(PROJECT_RECORD_PATH, JSON.stringify(bundle.project, null, 2));
  zip.file("workspace/manifest.json", JSON.stringify({
    active_model_id: bundle.active_model_id ?? null,
    active_version_id: bundle.active_version_id ?? null,
    exported_at: bundle.exported_at ?? null,
  }, null, 2));

  if (bundle.workspace_snapshot) {
    zip.file(WORKSPACE_SNAPSHOT_PATH, JSON.stringify(bundle.workspace_snapshot, null, 2));
  }

  bundle.models.forEach((model) => {
    zip.file(`models/${model.model_id}.json`, JSON.stringify(model, null, 2));
  });

  bundle.model_versions.forEach((version) => {
    zip.file(`versions/${version.version_id}.json`, JSON.stringify(version, null, 2));
  });

  zip.file(JOBS_INDEX_PATH, JSON.stringify(bundle.jobs ?? [], null, 2));
  (bundle.jobs ?? []).forEach((job) => {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    zip.file(`jobs/${jobId}.json`, JSON.stringify(job, null, 2));
  });

  zip.file(RESULTS_INDEX_PATH, JSON.stringify(bundle.results ?? [], null, 2));
  (bundle.results ?? []).forEach((result) => {
    const jobId = typeof result.job_id === "string" ? result.job_id : "result";
    zip.file(`results/${jobId}.json`, JSON.stringify(result, null, 2));
  });

  zip.file(
    "README.txt",
    [
      "Kyuubiki project bundle",
      "",
      `Schema: ${PROJECT_SCHEMA_VERSION}`,
      `Manifest: ${PROJECT_MANIFEST_PATH}`,
      "Structured payload:",
      "  project/project.json",
      "  models/*.json",
      "  versions/*.json",
      "  jobs/jobs.json",
      "  jobs/*.json",
      "  results/results.json",
      "  results/*.json",
      "  workspace/current-model.json",
    ].join("\n"),
  );

  return zip.generateAsync({
    type: "blob",
    compression: "DEFLATE",
    compressionOptions: { level: 6 },
  });
}
