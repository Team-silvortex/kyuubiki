import JSZip from "jszip";
import type { WorkbenchMacroPresetRecord, WorkbenchScriptSnippetPresetRecord } from "@/lib/scripting/workbench-script-runtime";
import type { JobResultRecord, ModelRecord, ModelVersionRecord, ProjectRecord } from "@/lib/api";
import { extractAnalysisMetadata } from "@/lib/projects/project-format-analysis";

export const PROJECT_SCHEMA_VERSION = "kyuubiki.project/v2";
export const LEGACY_PROJECT_SCHEMA_VERSION = "kyuubiki.project/v1";
export const PROJECT_FILE_LAYOUT_VERSION = "kyuubiki.project-layout/v1";
const PROJECT_MANIFEST_PATH = "project.json";
const PROJECT_ENGINE_MANIFEST_PATH = ".kyuubiki/project.json";
const PROJECT_RECORD_PATH = "project/project.json";
const STANDARD_PROJECT_RECORD_PATH = "Assets/project/project.json";
const STANDARD_MODELS_DIRECTORY = "Assets/models";
const STANDARD_VERSIONS_DIRECTORY = "Assets/versions";
const STANDARD_PROJECT_SETTINGS_DIRECTORY = "ProjectSettings";
const STANDARD_WORKSPACE_DIRECTORY = "Workspace";
const STANDARD_ANALYSIS_DIRECTORY = "Analysis";
const STANDARD_JOBS_DIRECTORY = `${STANDARD_ANALYSIS_DIRECTORY}/jobs`;
const STANDARD_RESULTS_DIRECTORY = `${STANDARD_ANALYSIS_DIRECTORY}/results`;
const WORKSPACE_SNAPSHOT_PATH = "workspace/current-model.json";
const STANDARD_WORKSPACE_SNAPSHOT_PATH = `${STANDARD_WORKSPACE_DIRECTORY}/current-model.json`;
const STANDARD_WORKSPACE_SETTINGS_PATH = `${STANDARD_PROJECT_SETTINGS_DIRECTORY}/workspace.json`;
const STANDARD_AUTOMATION_PRESETS_PATH = `${STANDARD_PROJECT_SETTINGS_DIRECTORY}/automation-presets.json`;
const STANDARD_SNIPPET_PRESETS_PATH = `${STANDARD_PROJECT_SETTINGS_DIRECTORY}/snippet-presets.json`;
const STANDARD_ASSET_CATALOG_PATH = `${STANDARD_PROJECT_SETTINGS_DIRECTORY}/asset-catalog.json`;
const STANDARD_ASSET_REFERENCES_PATH = `${STANDARD_PROJECT_SETTINGS_DIRECTORY}/asset-references.json`;
const JOBS_INDEX_PATH = "jobs/jobs.json";
const STANDARD_JOBS_INDEX_PATH = `${STANDARD_JOBS_DIRECTORY}/index.json`;
const RESULTS_INDEX_PATH = "results/results.json";
const STANDARD_RESULTS_INDEX_PATH = `${STANDARD_RESULTS_DIRECTORY}/index.json`;

export type ProjectFileManifest = {
  layout_version: string;
  engine_manifest_path: string;
  root_manifest_path: string;
  project_record_path: string;
  workspace_settings_path: string;
  workspace_snapshot_path: string;
  automation_presets_path: string;
  snippet_presets_path: string;
  asset_catalog_path: string;
  asset_references_path: string;
  model_directory: string;
  version_directory: string;
  job_directory: string;
  result_directory: string;
};

export type ProjectAssetMetaRecord = {
  guid: string;
  meta_version: "kyuubiki.asset-meta/v1";
  kind:
    | "project"
    | "workspace_settings"
    | "workspace_snapshot"
    | "automation_preset"
    | "snippet_preset"
    | "model"
    | "model_version"
    | "job"
    | "result";
  path: string;
  source_id: string;
  name?: string;
  updated_at?: string | null;
  analysis_domain?: "mechanical" | "thermal" | "thermo_mechanical";
  analysis_family?: "axial_and_springs" | "beams_and_frames" | "trusses" | "planes";
  thermal_intent?: string[];
};

export type ProjectAssetReferenceRecord = {
  from_guid: string;
  relation:
    | "contains"
    | "active_model"
    | "active_version"
    | "workspace_snapshot_of"
    | "workspace_settings_for"
    | "automation_for"
    | "snippet_for"
    | "version_of"
    | "job_for_project"
    | "job_for_version"
    | "result_for_job";
  to_guid: string;
};

export type ProjectBundle = {
  project_schema_version: string;
  exported_at?: string;
  project_file_manifest?: ProjectFileManifest;
  project: ProjectRecord;
  models: ModelRecord[];
  model_versions: ModelVersionRecord[];
  active_model_id?: string | null;
  active_version_id?: string | null;
  workspace_snapshot?: Record<string, unknown> | null;
  automation_presets?: WorkbenchMacroPresetRecord[];
  snippet_presets?: WorkbenchScriptSnippetPresetRecord[];
  asset_catalog?: ProjectAssetMetaRecord[];
  asset_references?: ProjectAssetReferenceRecord[];
  jobs?: Array<Record<string, unknown>>;
  results?: JobResultRecord[];
};

export function defaultProjectFileManifest(): ProjectFileManifest {
  return {
    layout_version: PROJECT_FILE_LAYOUT_VERSION,
    engine_manifest_path: PROJECT_ENGINE_MANIFEST_PATH,
    root_manifest_path: PROJECT_MANIFEST_PATH,
    project_record_path: STANDARD_PROJECT_RECORD_PATH,
    workspace_settings_path: STANDARD_WORKSPACE_SETTINGS_PATH,
    workspace_snapshot_path: STANDARD_WORKSPACE_SNAPSHOT_PATH,
    automation_presets_path: STANDARD_AUTOMATION_PRESETS_PATH,
    snippet_presets_path: STANDARD_SNIPPET_PRESETS_PATH,
    asset_catalog_path: STANDARD_ASSET_CATALOG_PATH,
    asset_references_path: STANDARD_ASSET_REFERENCES_PATH,
    model_directory: STANDARD_MODELS_DIRECTORY,
    version_directory: STANDARD_VERSIONS_DIRECTORY,
    job_directory: STANDARD_JOBS_DIRECTORY,
    result_directory: STANDARD_RESULTS_DIRECTORY,
  };
}

function normalizeBundle(raw: Partial<ProjectBundle>): ProjectBundle {
  if (raw.project_schema_version !== PROJECT_SCHEMA_VERSION && raw.project_schema_version !== LEGACY_PROJECT_SCHEMA_VERSION) {
    throw new Error(`unsupported project_schema_version: ${String(raw.project_schema_version)}`);
  }

  if (!raw.project || !raw.models || !raw.model_versions) {
    throw new Error("project bundle is missing required sections");
  }

  return {
    project_schema_version: raw.project_schema_version === LEGACY_PROJECT_SCHEMA_VERSION ? LEGACY_PROJECT_SCHEMA_VERSION : PROJECT_SCHEMA_VERSION,
    exported_at: raw.exported_at,
    project_file_manifest: raw.project_file_manifest ?? defaultProjectFileManifest(),
    project: raw.project,
    models: raw.models,
    model_versions: raw.model_versions,
    active_model_id: raw.active_model_id ?? null,
    active_version_id: raw.active_version_id ?? null,
    workspace_snapshot: raw.workspace_snapshot ?? null,
    automation_presets: raw.automation_presets ?? [],
    snippet_presets: raw.snippet_presets ?? [],
    asset_catalog: raw.asset_catalog ?? [],
    asset_references: raw.asset_references ?? [],
    jobs: raw.jobs ?? [],
    results: raw.results ?? [],
  };
}

function slugifyPathSegment(value: string) {
  const normalized = value.trim().toLowerCase().replace(/[^a-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
  return normalized || "asset";
}

function stableAssetGuid(seed: string) {
  let hash = 0;
  for (let index = 0; index < seed.length; index += 1) {
    hash = (hash * 31 + seed.charCodeAt(index)) >>> 0;
  }
  const first = hash.toString(16).padStart(8, "0");
  const second = ((hash ^ 0x9e3779b9) >>> 0).toString(16).padStart(8, "0");
  const third = ((hash ^ 0x85ebca6b) >>> 0).toString(16).padStart(8, "0");
  const fourth = ((hash ^ 0xc2b2ae35) >>> 0).toString(16).padStart(8, "0");
  return `${first}-${second.slice(0, 4)}-${second.slice(4, 8)}-${third.slice(0, 4)}-${third.slice(4, 8)}${fourth.slice(0, 4)}`;
}

function buildProjectAssetCatalog(bundle: ProjectBundle, fileManifest: ProjectFileManifest): ProjectAssetMetaRecord[] {
  const catalog: ProjectAssetMetaRecord[] = [
    {
      guid: stableAssetGuid(`project:${bundle.project.project_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "project",
      path: fileManifest.project_record_path,
      source_id: bundle.project.project_id,
      name: bundle.project.name,
      updated_at: bundle.project.updated_at,
    },
    {
      guid: stableAssetGuid(`workspace-settings:${bundle.project.project_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "workspace_settings",
      path: fileManifest.workspace_settings_path,
      source_id: bundle.project.project_id,
      name: `${bundle.project.name} workspace settings`,
      updated_at: bundle.exported_at ?? null,
    },
  ];

  if (bundle.workspace_snapshot) {
    catalog.push({
      guid: stableAssetGuid(`workspace-snapshot:${bundle.project.project_id}:${bundle.active_model_id ?? "none"}:${bundle.active_version_id ?? "none"}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "workspace_snapshot",
      path: fileManifest.workspace_snapshot_path,
      source_id: bundle.active_version_id ?? bundle.active_model_id ?? bundle.project.project_id,
      name: "Current workspace snapshot",
      updated_at: bundle.exported_at ?? null,
    });
  }

  bundle.models.forEach((model) => {
    const analysisMetadata = extractAnalysisMetadata(model.payload);
    catalog.push({
      guid: stableAssetGuid(`model:${model.model_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "model",
      path: `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`,
      source_id: model.model_id,
      name: model.name,
      updated_at: model.updated_at,
      ...analysisMetadata,
    });
  });

  bundle.model_versions.forEach((version) => {
    const analysisMetadata = extractAnalysisMetadata(version.payload);
    catalog.push({
      guid: stableAssetGuid(`version:${version.version_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "model_version",
      path: `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`,
      source_id: version.version_id,
      name: version.name ?? `${version.kind} v${version.version_number}`,
      updated_at: version.updated_at,
      ...analysisMetadata,
    });
  });

  (bundle.automation_presets ?? []).forEach((preset) => {
    catalog.push({
      guid: stableAssetGuid(`preset:${preset.presetId}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "automation_preset",
      path: fileManifest.automation_presets_path,
      source_id: preset.presetId,
      name: preset.name,
      updated_at: preset.updatedAt,
    });
  });
  (bundle.snippet_presets ?? []).forEach((preset) => {
    catalog.push({
      guid: stableAssetGuid(`snippet-preset:${preset.presetId}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "snippet_preset",
      path: fileManifest.snippet_presets_path,
      source_id: preset.presetId,
      name: preset.name,
      updated_at: preset.updatedAt,
    });
  });

  (bundle.jobs ?? []).forEach((job) => {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    const updatedAt = typeof job.updated_at === "string" ? job.updated_at : bundle.exported_at ?? null;
    catalog.push({
      guid: stableAssetGuid(`job:${jobId}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "job",
      path: `${fileManifest.job_directory}/${slugifyPathSegment(jobId)}.json`,
      source_id: jobId,
      name: typeof job.simulation_case_id === "string" ? job.simulation_case_id : jobId,
      updated_at: updatedAt,
    });
  });

  (bundle.results ?? []).forEach((result) => {
    const resultId = result.job_id;
    catalog.push({
      guid: stableAssetGuid(`result:${resultId}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "result",
      path: `${fileManifest.result_directory}/${slugifyPathSegment(resultId)}.json`,
      source_id: resultId,
      name: result.status ?? resultId,
      updated_at: bundle.exported_at ?? null,
    });
  });

  return catalog;
}

function buildProjectAssetReferences(bundle: ProjectBundle, assetCatalog: ProjectAssetMetaRecord[]): ProjectAssetReferenceRecord[] {
  const guidByKindAndSource = new Map(assetCatalog.map((entry) => [`${entry.kind}:${entry.source_id}`, entry.guid] as const));
  const refs: ProjectAssetReferenceRecord[] = [];
  const projectGuid = guidByKindAndSource.get(`project:${bundle.project.project_id}`);
  const workspaceSettingsGuid = guidByKindAndSource.get(`workspace_settings:${bundle.project.project_id}`);

  if (projectGuid && workspaceSettingsGuid) {
    refs.push({ from_guid: projectGuid, relation: "workspace_settings_for", to_guid: workspaceSettingsGuid });
  }

  if (projectGuid && bundle.active_model_id) {
    const activeModelGuid = guidByKindAndSource.get(`model:${bundle.active_model_id}`);
    if (activeModelGuid) {
      refs.push({ from_guid: projectGuid, relation: "active_model", to_guid: activeModelGuid });
    }
  }

  if (projectGuid && bundle.active_version_id) {
    const activeVersionGuid = guidByKindAndSource.get(`model_version:${bundle.active_version_id}`);
    if (activeVersionGuid) {
      refs.push({ from_guid: projectGuid, relation: "active_version", to_guid: activeVersionGuid });
    }
  }

  const workspaceSnapshotSourceId = bundle.active_version_id ?? bundle.active_model_id ?? bundle.project.project_id;
  const workspaceSnapshotGuid = guidByKindAndSource.get(`workspace_snapshot:${workspaceSnapshotSourceId}`);
  if (projectGuid && workspaceSnapshotGuid) {
    refs.push({ from_guid: projectGuid, relation: "workspace_snapshot_of", to_guid: workspaceSnapshotGuid });
  }

  bundle.models.forEach((model) => {
    const modelGuid = guidByKindAndSource.get(`model:${model.model_id}`);
    if (projectGuid && modelGuid) {
      refs.push({ from_guid: projectGuid, relation: "contains", to_guid: modelGuid });
    }
  });

  bundle.model_versions.forEach((version) => {
    const versionGuid = guidByKindAndSource.get(`model_version:${version.version_id}`);
    const modelGuid = guidByKindAndSource.get(`model:${version.model_id}`);
    if (projectGuid && versionGuid) {
      refs.push({ from_guid: projectGuid, relation: "contains", to_guid: versionGuid });
    }
    if (modelGuid && versionGuid) {
      refs.push({ from_guid: versionGuid, relation: "version_of", to_guid: modelGuid });
    }
  });

  (bundle.automation_presets ?? []).forEach((preset) => {
    const presetGuid = guidByKindAndSource.get(`automation_preset:${preset.presetId}`);
    if (projectGuid && presetGuid) {
      refs.push({ from_guid: projectGuid, relation: "automation_for", to_guid: presetGuid });
    }
  });
  (bundle.snippet_presets ?? []).forEach((preset) => {
    const presetGuid = guidByKindAndSource.get(`snippet_preset:${preset.presetId}`);
    if (projectGuid && presetGuid) {
      refs.push({ from_guid: projectGuid, relation: "snippet_for", to_guid: presetGuid });
    }
  });

  (bundle.jobs ?? []).forEach((job) => {
    const jobId = typeof job.job_id === "string" ? job.job_id : null;
    if (!jobId) return;
    const jobGuid = guidByKindAndSource.get(`job:${jobId}`);
    if (projectGuid && jobGuid) {
      refs.push({ from_guid: projectGuid, relation: "job_for_project", to_guid: jobGuid });
    }
    if (typeof job.model_version_id === "string") {
      const versionGuid = guidByKindAndSource.get(`model_version:${job.model_version_id}`);
      if (jobGuid && versionGuid) {
        refs.push({ from_guid: jobGuid, relation: "job_for_version", to_guid: versionGuid });
      }
    }
  });

  (bundle.results ?? []).forEach((result) => {
    const resultGuid = guidByKindAndSource.get(`result:${result.job_id}`);
    const jobGuid = guidByKindAndSource.get(`job:${result.job_id}`);
    if (resultGuid && jobGuid) {
      refs.push({ from_guid: resultGuid, relation: "result_for_job", to_guid: jobGuid });
    }
  });

  return refs;
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
    const fileManifest = bundle.project_file_manifest ?? defaultProjectFileManifest();

    if (!bundle.workspace_snapshot) {
      const workspaceSnapshot = zip.file(fileManifest.workspace_snapshot_path) ?? zip.file(WORKSPACE_SNAPSHOT_PATH);
      if (workspaceSnapshot) {
        bundle.workspace_snapshot = JSON.parse(await workspaceSnapshot.async("string")) as Record<string, unknown>;
      }
    }

    if ((bundle.automation_presets?.length ?? 0) === 0) {
      const automationPresets = zip.file(fileManifest.automation_presets_path);
      if (automationPresets) {
        bundle.automation_presets = JSON.parse(await automationPresets.async("string")) as WorkbenchMacroPresetRecord[];
      }
    }
    if ((bundle.snippet_presets?.length ?? 0) === 0) {
      const snippetPresets = zip.file(fileManifest.snippet_presets_path);
      if (snippetPresets) {
        bundle.snippet_presets = JSON.parse(await snippetPresets.async("string")) as WorkbenchScriptSnippetPresetRecord[];
      }
    }

    if ((bundle.asset_catalog?.length ?? 0) === 0) {
      const assetCatalog = zip.file(fileManifest.asset_catalog_path);
      if (assetCatalog) {
        bundle.asset_catalog = JSON.parse(await assetCatalog.async("string")) as ProjectAssetMetaRecord[];
      }
    }

    if ((bundle.asset_references?.length ?? 0) === 0) {
      const assetReferences = zip.file(fileManifest.asset_references_path) ?? zip.file(STANDARD_ASSET_REFERENCES_PATH);
      if (assetReferences) {
        bundle.asset_references = JSON.parse(await assetReferences.async("string")) as ProjectAssetReferenceRecord[];
      }
    }

    if ((bundle.jobs?.length ?? 0) === 0) {
      const jobsIndex = zip.file(`${fileManifest.job_directory}/index.json`) ?? zip.file(JOBS_INDEX_PATH);
      if (jobsIndex) {
        bundle.jobs = JSON.parse(await jobsIndex.async("string")) as Array<Record<string, unknown>>;
      }
    }

    if ((bundle.results?.length ?? 0) === 0) {
      const resultsIndex = zip.file(`${fileManifest.result_directory}/index.json`) ?? zip.file(RESULTS_INDEX_PATH);
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
  const fileManifest = bundle.project_file_manifest ?? defaultProjectFileManifest();
  const assetCatalog = bundle.asset_catalog?.length ? bundle.asset_catalog : buildProjectAssetCatalog(bundle, fileManifest);
  const assetReferences = bundle.asset_references?.length ? bundle.asset_references : buildProjectAssetReferences(bundle, assetCatalog);
  const zip = new JSZip();
  zip.file(PROJECT_MANIFEST_PATH, bundleJson);
  zip.file(PROJECT_ENGINE_MANIFEST_PATH, JSON.stringify(fileManifest, null, 2));
  zip.file(PROJECT_RECORD_PATH, JSON.stringify(bundle.project, null, 2));
  zip.file(fileManifest.project_record_path, JSON.stringify(bundle.project, null, 2));
  zip.file("workspace/manifest.json", JSON.stringify({
    active_model_id: bundle.active_model_id ?? null,
    active_version_id: bundle.active_version_id ?? null,
    exported_at: bundle.exported_at ?? null,
  }, null, 2));
  zip.file(fileManifest.workspace_settings_path, JSON.stringify({
    active_model_id: bundle.active_model_id ?? null,
    active_version_id: bundle.active_version_id ?? null,
    exported_at: bundle.exported_at ?? null,
    project_schema_version: bundle.project_schema_version,
    layout_version: fileManifest.layout_version,
  }, null, 2));

  if (bundle.workspace_snapshot) {
    zip.file(WORKSPACE_SNAPSHOT_PATH, JSON.stringify(bundle.workspace_snapshot, null, 2));
    zip.file(fileManifest.workspace_snapshot_path, JSON.stringify(bundle.workspace_snapshot, null, 2));
  }

  if ((bundle.automation_presets?.length ?? 0) > 0) {
    zip.file(fileManifest.automation_presets_path, JSON.stringify(bundle.automation_presets, null, 2));
  }
  if ((bundle.snippet_presets?.length ?? 0) > 0) {
    zip.file(fileManifest.snippet_presets_path, JSON.stringify(bundle.snippet_presets, null, 2));
  }

  zip.file(fileManifest.asset_catalog_path, JSON.stringify(assetCatalog, null, 2));
  zip.file(fileManifest.asset_references_path, JSON.stringify(assetReferences, null, 2));

  bundle.models.forEach((model) => {
    const path = `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model" && entry.source_id === model.model_id);
    zip.file(`models/${model.model_id}.json`, JSON.stringify(model, null, 2));
    zip.file(path, JSON.stringify(model, null, 2));
    if (meta) {
      zip.file(`${path}.meta`, JSON.stringify(meta, null, 2));
    }
  });

  bundle.model_versions.forEach((version) => {
    const path = `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model_version" && entry.source_id === version.version_id);
    zip.file(`versions/${version.version_id}.json`, JSON.stringify(version, null, 2));
    zip.file(path, JSON.stringify(version, null, 2));
    if (meta) {
      zip.file(`${path}.meta`, JSON.stringify(meta, null, 2));
    }
  });

  zip.file(JOBS_INDEX_PATH, JSON.stringify(bundle.jobs ?? [], null, 2));
  zip.file(`${fileManifest.job_directory}/index.json`, JSON.stringify(bundle.jobs ?? [], null, 2));
  (bundle.jobs ?? []).forEach((job) => {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    const path = `${fileManifest.job_directory}/${slugifyPathSegment(jobId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "job" && entry.source_id === jobId);
    zip.file(`jobs/${jobId}.json`, JSON.stringify(job, null, 2));
    zip.file(path, JSON.stringify(job, null, 2));
    if (meta) {
      zip.file(`${path}.meta`, JSON.stringify(meta, null, 2));
    }
  });

  zip.file(RESULTS_INDEX_PATH, JSON.stringify(bundle.results ?? [], null, 2));
  zip.file(`${fileManifest.result_directory}/index.json`, JSON.stringify(bundle.results ?? [], null, 2));
  (bundle.results ?? []).forEach((result) => {
    const jobId = typeof result.job_id === "string" ? result.job_id : "result";
    const path = `${fileManifest.result_directory}/${slugifyPathSegment(jobId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "result" && entry.source_id === jobId);
    zip.file(`results/${jobId}.json`, JSON.stringify(result, null, 2));
    zip.file(path, JSON.stringify(result, null, 2));
    if (meta) {
      zip.file(`${path}.meta`, JSON.stringify(meta, null, 2));
    }
  });

  const projectMeta = assetCatalog.find((entry) => entry.kind === "project" && entry.source_id === bundle.project.project_id);
  if (projectMeta) {
    zip.file(`${fileManifest.project_record_path}.meta`, JSON.stringify(projectMeta, null, 2));
  }

  const workspaceSettingsMeta = assetCatalog.find((entry) => entry.kind === "workspace_settings" && entry.source_id === bundle.project.project_id);
  if (workspaceSettingsMeta) {
    zip.file(`${fileManifest.workspace_settings_path}.meta`, JSON.stringify(workspaceSettingsMeta, null, 2));
  }

  const workspaceSnapshotMeta = assetCatalog.find((entry) => entry.kind === "workspace_snapshot");
  if (workspaceSnapshotMeta && bundle.workspace_snapshot) {
    zip.file(`${fileManifest.workspace_snapshot_path}.meta`, JSON.stringify(workspaceSnapshotMeta, null, 2));
  }

  if ((bundle.automation_presets?.length ?? 0) > 0) {
    const presetMetas = assetCatalog.filter((entry) => entry.kind === "automation_preset");
    zip.file(`${fileManifest.automation_presets_path}.meta`, JSON.stringify(presetMetas, null, 2));
  }
  if ((bundle.snippet_presets?.length ?? 0) > 0) {
    const presetMetas = assetCatalog.filter((entry) => entry.kind === "snippet_preset");
    zip.file(`${fileManifest.snippet_presets_path}.meta`, JSON.stringify(presetMetas, null, 2));
  }

  zip.file(
    "README.txt",
    [
      "Kyuubiki project bundle",
      "",
      `Schema: ${bundle.project_schema_version}`,
      `Layout: ${fileManifest.layout_version}`,
      `Manifest: ${PROJECT_MANIFEST_PATH}`,
      `Engine manifest: ${fileManifest.engine_manifest_path}`,
      "Standard project layout:",
      `  ${fileManifest.project_record_path}`,
      `  ${fileManifest.model_directory}/*.json`,
      `  ${fileManifest.version_directory}/*.json`,
      `  ${fileManifest.workspace_settings_path}`,
      `  ${fileManifest.workspace_snapshot_path}`,
      `  ${fileManifest.automation_presets_path}`,
      `  ${fileManifest.snippet_presets_path}`,
      `  ${fileManifest.asset_catalog_path}`,
      `  ${fileManifest.asset_references_path}`,
      `  ${fileManifest.job_directory}/index.json`,
      `  ${fileManifest.job_directory}/*.json`,
      `  ${fileManifest.result_directory}/index.json`,
      `  ${fileManifest.result_directory}/*.json`,
      "",
      "Meta sidecars:",
      "  core assets also emit *.meta files with stable guid records",
      "Guid graph:",
      "  asset references describe stable guid-to-guid relations across the project",
      "",
      "Legacy compatibility aliases:",
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
