import JSZip from "jszip";
import type { ModelRecord, ModelVersionRecord, ProjectRecord } from "@/lib/api";

export const PROJECT_SCHEMA_VERSION = "kyuubiki.project/v1";
const PROJECT_MANIFEST_PATH = "project.json";

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
};

export function parseProjectBundleJson(text: string): ProjectBundle {
  const raw = JSON.parse(text) as Partial<ProjectBundle>;

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
  };
}

export async function parseProjectBundleFile(file: File): Promise<ProjectBundle> {
  if (file.name.endsWith(".kyuubiki")) {
    const zip = await JSZip.loadAsync(await file.arrayBuffer());
    const manifest = zip.file(PROJECT_MANIFEST_PATH);

    if (!manifest) {
      throw new Error(`missing ${PROJECT_MANIFEST_PATH} in project archive`);
    }

    return parseProjectBundleJson(await manifest.async("string"));
  }

  return parseProjectBundleJson(await file.text());
}

export async function exportProjectBundleZip(bundleJson: string): Promise<Blob> {
  const zip = new JSZip();
  zip.file(PROJECT_MANIFEST_PATH, bundleJson);
  zip.file(
    "README.txt",
    [
      "Kyuubiki project bundle",
      "",
      `Schema: ${PROJECT_SCHEMA_VERSION}`,
      `Manifest: ${PROJECT_MANIFEST_PATH}`,
    ].join("\n"),
  );

  return zip.generateAsync({
    type: "blob",
    compression: "DEFLATE",
    compressionOptions: { level: 6 },
  });
}
