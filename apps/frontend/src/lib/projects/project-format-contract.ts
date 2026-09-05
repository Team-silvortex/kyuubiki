export const PROJECT_SCHEMA_VERSION = "kyuubiki.project/v2";
export const LEGACY_PROJECT_SCHEMA_VERSION = "kyuubiki.project/v1";
export const PROJECT_FILE_LAYOUT_VERSION = "kyuubiki.project-layout/v1";

export const PROJECT_FORMAT_PATHS = {
  projectManifest: "project.json",
  projectEngineManifest: ".kyuubiki/project.json",
  legacyProjectRecord: "project/project.json",
  projectRecord: "Assets/project/project.json",
  modelsDirectory: "Assets/models",
  versionsDirectory: "Assets/versions",
  workspaceSnapshot: "Workspace/current-model.json",
  legacyWorkspaceSnapshot: "workspace/current-model.json",
  workspaceSettings: "ProjectSettings/workspace.json",
  automationPresets: "ProjectSettings/automation-presets.json",
  snippetPresets: "ProjectSettings/snippet-presets.json",
  storeManifest: "ProjectSettings/store-manifest.json",
  assetCatalog: "ProjectSettings/asset-catalog.json",
  assetReferences: "ProjectSettings/asset-references.json",
  jobsDirectory: "Analysis/jobs",
  legacyJobsIndex: "jobs/jobs.json",
  resultsDirectory: "Analysis/results",
  legacyResultsIndex: "results/results.json",
} as const;

export type ProjectFileManifest = {
  layout_version: string;
  engine_manifest_path: string;
  root_manifest_path: string;
  project_record_path: string;
  workspace_settings_path: string;
  workspace_snapshot_path: string;
  automation_presets_path: string;
  snippet_presets_path: string;
  store_manifest_path: string;
  asset_catalog_path: string;
  asset_references_path: string;
  model_directory: string;
  version_directory: string;
  job_directory: string;
  result_directory: string;
};

export function defaultProjectFileManifest(): ProjectFileManifest {
  return {
    layout_version: PROJECT_FILE_LAYOUT_VERSION,
    engine_manifest_path: PROJECT_FORMAT_PATHS.projectEngineManifest,
    root_manifest_path: PROJECT_FORMAT_PATHS.projectManifest,
    project_record_path: PROJECT_FORMAT_PATHS.projectRecord,
    workspace_settings_path: PROJECT_FORMAT_PATHS.workspaceSettings,
    workspace_snapshot_path: PROJECT_FORMAT_PATHS.workspaceSnapshot,
    automation_presets_path: PROJECT_FORMAT_PATHS.automationPresets,
    snippet_presets_path: PROJECT_FORMAT_PATHS.snippetPresets,
    store_manifest_path: PROJECT_FORMAT_PATHS.storeManifest,
    asset_catalog_path: PROJECT_FORMAT_PATHS.assetCatalog,
    asset_references_path: PROJECT_FORMAT_PATHS.assetReferences,
    model_directory: PROJECT_FORMAT_PATHS.modelsDirectory,
    version_directory: PROJECT_FORMAT_PATHS.versionsDirectory,
    job_directory: PROJECT_FORMAT_PATHS.jobsDirectory,
    result_directory: PROJECT_FORMAT_PATHS.resultsDirectory,
  };
}
