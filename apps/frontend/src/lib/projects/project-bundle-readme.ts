import type { ProjectFileManifest } from "@/lib/projects/project-format";

export function buildProjectBundleReadme(params: {
  projectSchemaVersion: string;
  projectManifestPath: string;
  fileManifest: ProjectFileManifest;
}) {
  const { projectSchemaVersion, projectManifestPath, fileManifest } = params;
  return [
    "Kyuubiki project bundle",
    "",
    `Schema: ${projectSchemaVersion}`,
    `Layout: ${fileManifest.layout_version}`,
    `Manifest: ${projectManifestPath}`,
    `Engine manifest: ${fileManifest.engine_manifest_path}`,
    "Standard project layout:",
    `  ${fileManifest.project_record_path}`,
    `  ${fileManifest.model_directory}/*.json`,
    `  ${fileManifest.version_directory}/*.json`,
    `  ${fileManifest.workspace_settings_path}`,
    `  ${fileManifest.workspace_snapshot_path}`,
    `  ${fileManifest.automation_presets_path}`,
    `  ${fileManifest.snippet_presets_path}`,
    `  ${fileManifest.store_manifest_path}`,
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
  ].join("\n");
}
