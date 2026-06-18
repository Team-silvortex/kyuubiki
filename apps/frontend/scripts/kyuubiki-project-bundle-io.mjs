import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import JSZip from "jszip";

const PROJECT_SCHEMA_VERSION = "kyuubiki.project/v2";
const LEGACY_PROJECT_SCHEMA_VERSION = "kyuubiki.project/v1";
const PROJECT_FILE_LAYOUT_VERSION = "kyuubiki.project-layout/v1";
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
const STANDARD_ASSET_CATALOG_PATH = `${STANDARD_PROJECT_SETTINGS_DIRECTORY}/asset-catalog.json`;
const STANDARD_ASSET_REFERENCES_PATH = `${STANDARD_PROJECT_SETTINGS_DIRECTORY}/asset-references.json`;
const JOBS_INDEX_PATH = "jobs/jobs.json";
const STANDARD_JOBS_INDEX_PATH = `${STANDARD_JOBS_DIRECTORY}/index.json`;
const RESULTS_INDEX_PATH = "results/results.json";
const STANDARD_RESULTS_INDEX_PATH = `${STANDARD_RESULTS_DIRECTORY}/index.json`;

export function defaultProjectFileManifest() {
  return {
    layout_version: PROJECT_FILE_LAYOUT_VERSION,
    engine_manifest_path: PROJECT_ENGINE_MANIFEST_PATH,
    root_manifest_path: PROJECT_MANIFEST_PATH,
    project_record_path: STANDARD_PROJECT_RECORD_PATH,
    workspace_settings_path: STANDARD_WORKSPACE_SETTINGS_PATH,
    workspace_snapshot_path: STANDARD_WORKSPACE_SNAPSHOT_PATH,
    automation_presets_path: STANDARD_AUTOMATION_PRESETS_PATH,
    asset_catalog_path: STANDARD_ASSET_CATALOG_PATH,
    asset_references_path: STANDARD_ASSET_REFERENCES_PATH,
    model_directory: STANDARD_MODELS_DIRECTORY,
    version_directory: STANDARD_VERSIONS_DIRECTORY,
    job_directory: STANDARD_JOBS_DIRECTORY,
    result_directory: STANDARD_RESULTS_DIRECTORY,
  };
}

export function normalizeProjectBundle(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("Invalid project bundle payload.");
  }
  if (raw.project_schema_version !== PROJECT_SCHEMA_VERSION && raw.project_schema_version !== LEGACY_PROJECT_SCHEMA_VERSION) {
    throw new Error(`unsupported project_schema_version: ${String(raw.project_schema_version)}`);
  }
  if (!raw.project || !Array.isArray(raw.models) || !Array.isArray(raw.model_versions)) {
    throw new Error("project bundle is missing required sections");
  }
  return {
    project_schema_version: PROJECT_SCHEMA_VERSION,
    exported_at: typeof raw.exported_at === "string" ? raw.exported_at : new Date().toISOString(),
    project_file_manifest: raw.project_file_manifest ?? defaultProjectFileManifest(),
    project: raw.project,
    models: raw.models,
    model_versions: raw.model_versions,
    active_model_id: raw.active_model_id ?? null,
    active_version_id: raw.active_version_id ?? null,
    workspace_snapshot: raw.workspace_snapshot ?? null,
    automation_presets: Array.isArray(raw.automation_presets) ? raw.automation_presets : [],
    asset_catalog: Array.isArray(raw.asset_catalog) ? raw.asset_catalog : [],
    asset_references: Array.isArray(raw.asset_references) ? raw.asset_references : [],
    jobs: Array.isArray(raw.jobs) ? raw.jobs : [],
    results: Array.isArray(raw.results) ? raw.results : [],
  };
}

function slugifyPathSegment(value) {
  const normalized = String(value).trim().toLowerCase().replace(/[^a-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
  return normalized || "asset";
}

function stableAssetGuid(seed) {
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

function extractAnalysisMetadata(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const metadata = value.analysis_metadata;
  if (!metadata || typeof metadata !== "object" || Array.isArray(metadata)) return {};
  return {
    analysis_domain:
      metadata.domain === "mechanical" || metadata.domain === "thermal" || metadata.domain === "thermo_mechanical" ? metadata.domain : undefined,
    analysis_family:
      metadata.family === "axial_and_springs" ||
      metadata.family === "beams_and_frames" ||
      metadata.family === "trusses" ||
      metadata.family === "planes"
        ? metadata.family
        : undefined,
    thermal_intent: Array.isArray(metadata.thermal_intent) ? metadata.thermal_intent.filter((item) => typeof item === "string") : undefined,
  };
}

function buildProjectAssetCatalog(bundle, fileManifest) {
  const catalog = [
    {
      guid: stableAssetGuid(`project:${bundle.project.project_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "project",
      path: fileManifest.project_record_path,
      source_id: bundle.project.project_id,
      name: bundle.project.name,
      updated_at: bundle.project.updated_at ?? bundle.exported_at ?? null,
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

  for (const model of bundle.models) {
    catalog.push({
      guid: stableAssetGuid(`model:${model.model_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "model",
      path: `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`,
      source_id: model.model_id,
      name: model.name,
      updated_at: model.updated_at ?? bundle.exported_at ?? null,
      ...extractAnalysisMetadata(model.payload),
    });
  }

  for (const version of bundle.model_versions) {
    catalog.push({
      guid: stableAssetGuid(`version:${version.version_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "model_version",
      path: `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`,
      source_id: version.version_id,
      name: version.name ?? `${version.kind} v${version.version_number}`,
      updated_at: version.updated_at ?? bundle.exported_at ?? null,
      ...extractAnalysisMetadata(version.payload),
    });
  }

  for (const preset of bundle.automation_presets ?? []) {
    catalog.push({
      guid: stableAssetGuid(`preset:${preset.presetId}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "automation_preset",
      path: fileManifest.automation_presets_path,
      source_id: preset.presetId,
      name: preset.name,
      updated_at: preset.updatedAt ?? bundle.exported_at ?? null,
    });
  }

  for (const job of bundle.jobs ?? []) {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    catalog.push({
      guid: stableAssetGuid(`job:${jobId}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "job",
      path: `${fileManifest.job_directory}/${slugifyPathSegment(jobId)}.json`,
      source_id: jobId,
      name: typeof job.simulation_case_id === "string" ? job.simulation_case_id : jobId,
      updated_at: typeof job.updated_at === "string" ? job.updated_at : bundle.exported_at ?? null,
    });
  }

  for (const result of bundle.results ?? []) {
    catalog.push({
      guid: stableAssetGuid(`result:${result.job_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "result",
      path: `${fileManifest.result_directory}/${slugifyPathSegment(result.job_id)}.json`,
      source_id: result.job_id,
      name: result.status ?? result.job_id,
      updated_at: bundle.exported_at ?? null,
    });
  }

  return catalog;
}

function buildProjectAssetReferences(bundle, assetCatalog) {
  const guidByKindAndSource = new Map(assetCatalog.map((entry) => [`${entry.kind}:${entry.source_id}`, entry.guid]));
  const refs = [];
  const projectGuid = guidByKindAndSource.get(`project:${bundle.project.project_id}`);
  const workspaceSettingsGuid = guidByKindAndSource.get(`workspace_settings:${bundle.project.project_id}`);

  if (projectGuid && workspaceSettingsGuid) refs.push({ from_guid: projectGuid, relation: "workspace_settings_for", to_guid: workspaceSettingsGuid });
  if (projectGuid && bundle.active_model_id) {
    const activeModelGuid = guidByKindAndSource.get(`model:${bundle.active_model_id}`);
    if (activeModelGuid) refs.push({ from_guid: projectGuid, relation: "active_model", to_guid: activeModelGuid });
  }
  if (projectGuid && bundle.active_version_id) {
    const activeVersionGuid = guidByKindAndSource.get(`model_version:${bundle.active_version_id}`);
    if (activeVersionGuid) refs.push({ from_guid: projectGuid, relation: "active_version", to_guid: activeVersionGuid });
  }

  const workspaceSnapshotSourceId = bundle.active_version_id ?? bundle.active_model_id ?? bundle.project.project_id;
  const workspaceSnapshotGuid = guidByKindAndSource.get(`workspace_snapshot:${workspaceSnapshotSourceId}`);
  if (projectGuid && workspaceSnapshotGuid) refs.push({ from_guid: projectGuid, relation: "workspace_snapshot_of", to_guid: workspaceSnapshotGuid });

  for (const model of bundle.models) {
    const modelGuid = guidByKindAndSource.get(`model:${model.model_id}`);
    if (projectGuid && modelGuid) refs.push({ from_guid: projectGuid, relation: "contains", to_guid: modelGuid });
  }

  for (const version of bundle.model_versions) {
    const versionGuid = guidByKindAndSource.get(`model_version:${version.version_id}`);
    const modelGuid = guidByKindAndSource.get(`model:${version.model_id}`);
    if (projectGuid && versionGuid) refs.push({ from_guid: projectGuid, relation: "contains", to_guid: versionGuid });
    if (modelGuid && versionGuid) refs.push({ from_guid: versionGuid, relation: "version_of", to_guid: modelGuid });
  }

  for (const preset of bundle.automation_presets ?? []) {
    const presetGuid = guidByKindAndSource.get(`automation_preset:${preset.presetId}`);
    if (projectGuid && presetGuid) refs.push({ from_guid: projectGuid, relation: "automation_for", to_guid: presetGuid });
  }

  for (const job of bundle.jobs ?? []) {
    const jobId = typeof job.job_id === "string" ? job.job_id : null;
    if (!jobId) continue;
    const jobGuid = guidByKindAndSource.get(`job:${jobId}`);
    if (projectGuid && jobGuid) refs.push({ from_guid: projectGuid, relation: "job_for_project", to_guid: jobGuid });
    if (typeof job.model_version_id === "string") {
      const versionGuid = guidByKindAndSource.get(`model_version:${job.model_version_id}`);
      if (jobGuid && versionGuid) refs.push({ from_guid: jobGuid, relation: "job_for_version", to_guid: versionGuid });
    }
  }

  for (const result of bundle.results ?? []) {
    const resultGuid = guidByKindAndSource.get(`result:${result.job_id}`);
    const jobGuid = guidByKindAndSource.get(`job:${result.job_id}`);
    if (resultGuid && jobGuid) refs.push({ from_guid: resultGuid, relation: "result_for_job", to_guid: jobGuid });
  }

  return refs;
}

async function parseProjectBundleFile(filePath) {
  const absolutePath = path.resolve(filePath);
  if (!absolutePath.endsWith(".kyuubiki")) {
    return normalizeProjectBundle(JSON.parse(await readFile(absolutePath, "utf8")));
  }

  const zip = await JSZip.loadAsync(await readFile(absolutePath));
  const manifestFile = zip.file(PROJECT_MANIFEST_PATH) ?? zip.file(PROJECT_RECORD_PATH);
  if (!manifestFile) throw new Error(`missing ${PROJECT_MANIFEST_PATH} in project archive`);

  const bundle = normalizeProjectBundle(JSON.parse(await manifestFile.async("string")));
  const fileManifest = bundle.project_file_manifest ?? defaultProjectFileManifest();

  if (!bundle.workspace_snapshot) {
    const snapshot = zip.file(fileManifest.workspace_snapshot_path) ?? zip.file(WORKSPACE_SNAPSHOT_PATH);
    if (snapshot) bundle.workspace_snapshot = JSON.parse(await snapshot.async("string"));
  }
  if ((bundle.automation_presets?.length ?? 0) === 0) {
    const presets = zip.file(fileManifest.automation_presets_path);
    if (presets) bundle.automation_presets = JSON.parse(await presets.async("string"));
  }
  if ((bundle.asset_catalog?.length ?? 0) === 0) {
    const catalog = zip.file(fileManifest.asset_catalog_path);
    if (catalog) bundle.asset_catalog = JSON.parse(await catalog.async("string"));
  }
  if ((bundle.asset_references?.length ?? 0) === 0) {
    const references = zip.file(fileManifest.asset_references_path) ?? zip.file(STANDARD_ASSET_REFERENCES_PATH);
    if (references) bundle.asset_references = JSON.parse(await references.async("string"));
  }
  if ((bundle.jobs?.length ?? 0) === 0) {
    const jobsIndex = zip.file(`${fileManifest.job_directory}/index.json`) ?? zip.file(JOBS_INDEX_PATH);
    if (jobsIndex) bundle.jobs = JSON.parse(await jobsIndex.async("string"));
  }
  if ((bundle.results?.length ?? 0) === 0) {
    const resultsIndex = zip.file(`${fileManifest.result_directory}/index.json`) ?? zip.file(RESULTS_INDEX_PATH);
    if (resultsIndex) bundle.results = JSON.parse(await resultsIndex.async("string"));
  }
  return bundle;
}

async function pathIsDirectory(targetPath) {
  try {
    return (await stat(path.resolve(targetPath))).isDirectory();
  } catch {
    return false;
  }
}

export async function parseProjectBundleInput(inputPath) {
  const absolutePath = path.resolve(inputPath);
  if (await pathIsDirectory(absolutePath)) {
    return normalizeProjectBundle(JSON.parse(await readFile(path.join(absolutePath, PROJECT_MANIFEST_PATH), "utf8")));
  }
  return parseProjectBundleFile(absolutePath);
}

export function finalizeProjectBundle(bundle) {
  const normalized = normalizeProjectBundle(bundle);
  const fileManifest = normalized.project_file_manifest ?? defaultProjectFileManifest();
  const asset_catalog = normalized.asset_catalog?.length ? normalized.asset_catalog : buildProjectAssetCatalog(normalized, fileManifest);
  const asset_references = normalized.asset_references?.length ? normalized.asset_references : buildProjectAssetReferences(normalized, asset_catalog);
  return { ...normalized, project_file_manifest: fileManifest, asset_catalog, asset_references };
}

function serializeWorkspaceSettings(finalized, fileManifest) {
  return JSON.stringify({
    active_model_id: finalized.active_model_id ?? null,
    active_version_id: finalized.active_version_id ?? null,
    exported_at: finalized.exported_at ?? null,
    project_schema_version: finalized.project_schema_version,
    layout_version: fileManifest.layout_version,
  }, null, 2);
}

function writeCatalogAssetMeta(container, relativePath, meta, writer) {
  if (!meta) return;
  writer(container, `${relativePath}.meta`, JSON.stringify(meta, null, 2));
}

function addBundleReadme(finalized, fileManifest, writer, container) {
  writer(container, "README.txt", [
    container === "zip" ? "Kyuubiki project bundle" : "Kyuubiki project directory",
    "",
    `Schema: ${finalized.project_schema_version}`,
    `Layout: ${fileManifest.layout_version}`,
    `Manifest: ${PROJECT_MANIFEST_PATH}`,
    `Engine manifest: ${fileManifest.engine_manifest_path}`,
    `Asset catalog: ${fileManifest.asset_catalog_path}`,
    `Asset references: ${fileManifest.asset_references_path}`,
  ].join("\n"));
}

export async function exportProjectBundleZip(bundle) {
  const finalized = finalizeProjectBundle(bundle);
  const fileManifest = finalized.project_file_manifest;
  const assetCatalog = finalized.asset_catalog;
  const zip = new JSZip();
  const writeZipFile = (_container, relativePath, contents) => zip.file(relativePath, contents);

  writeZipFile("zip", PROJECT_MANIFEST_PATH, JSON.stringify(finalized, null, 2));
  writeZipFile("zip", PROJECT_ENGINE_MANIFEST_PATH, JSON.stringify(fileManifest, null, 2));
  writeZipFile("zip", PROJECT_RECORD_PATH, JSON.stringify(finalized.project, null, 2));
  writeZipFile("zip", fileManifest.project_record_path, JSON.stringify(finalized.project, null, 2));
  writeZipFile("zip", fileManifest.workspace_settings_path, serializeWorkspaceSettings(finalized, fileManifest));
  writeZipFile("zip", fileManifest.asset_catalog_path, JSON.stringify(finalized.asset_catalog, null, 2));
  writeZipFile("zip", fileManifest.asset_references_path, JSON.stringify(finalized.asset_references, null, 2));
  writeZipFile("zip", JOBS_INDEX_PATH, JSON.stringify(finalized.jobs ?? [], null, 2));
  writeZipFile("zip", `${fileManifest.job_directory}/index.json`, JSON.stringify(finalized.jobs ?? [], null, 2));
  writeZipFile("zip", RESULTS_INDEX_PATH, JSON.stringify(finalized.results ?? [], null, 2));
  writeZipFile("zip", `${fileManifest.result_directory}/index.json`, JSON.stringify(finalized.results ?? [], null, 2));

  if (finalized.workspace_snapshot) {
    writeZipFile("zip", WORKSPACE_SNAPSHOT_PATH, JSON.stringify(finalized.workspace_snapshot, null, 2));
    writeZipFile("zip", fileManifest.workspace_snapshot_path, JSON.stringify(finalized.workspace_snapshot, null, 2));
  }
  if ((finalized.automation_presets?.length ?? 0) > 0) {
    writeZipFile("zip", fileManifest.automation_presets_path, JSON.stringify(finalized.automation_presets, null, 2));
    writeZipFile(
      "zip",
      `${fileManifest.automation_presets_path}.meta`,
      JSON.stringify(assetCatalog.filter((entry) => entry.kind === "automation_preset"), null, 2),
    );
  }

  for (const model of finalized.models) {
    const assetPath = `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model" && entry.source_id === model.model_id);
    writeZipFile("zip", `models/${model.model_id}.json`, JSON.stringify(model, null, 2));
    writeZipFile("zip", assetPath, JSON.stringify(model, null, 2));
    writeCatalogAssetMeta("zip", assetPath, meta, writeZipFile);
  }

  for (const version of finalized.model_versions) {
    const assetPath = `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model_version" && entry.source_id === version.version_id);
    writeZipFile("zip", `versions/${version.version_id}.json`, JSON.stringify(version, null, 2));
    writeZipFile("zip", assetPath, JSON.stringify(version, null, 2));
    writeCatalogAssetMeta("zip", assetPath, meta, writeZipFile);
  }

  for (const job of finalized.jobs ?? []) {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    const assetPath = `${fileManifest.job_directory}/${slugifyPathSegment(jobId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "job" && entry.source_id === jobId);
    writeZipFile("zip", `jobs/${jobId}.json`, JSON.stringify(job, null, 2));
    writeZipFile("zip", assetPath, JSON.stringify(job, null, 2));
    writeCatalogAssetMeta("zip", assetPath, meta, writeZipFile);
  }

  for (const result of finalized.results ?? []) {
    const resultId = typeof result.job_id === "string" ? result.job_id : "result";
    const assetPath = `${fileManifest.result_directory}/${slugifyPathSegment(resultId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "result" && entry.source_id === resultId);
    writeZipFile("zip", `results/${resultId}.json`, JSON.stringify(result, null, 2));
    writeZipFile("zip", assetPath, JSON.stringify(result, null, 2));
    writeCatalogAssetMeta("zip", assetPath, meta, writeZipFile);
  }

  writeCatalogAssetMeta("zip", fileManifest.project_record_path, assetCatalog.find((entry) => entry.kind === "project" && entry.source_id === finalized.project.project_id), writeZipFile);
  writeCatalogAssetMeta("zip", fileManifest.workspace_settings_path, assetCatalog.find((entry) => entry.kind === "workspace_settings" && entry.source_id === finalized.project.project_id), writeZipFile);

  const workspaceSnapshotSourceId = finalized.active_version_id ?? finalized.active_model_id ?? finalized.project.project_id;
  if (finalized.workspace_snapshot) {
    writeCatalogAssetMeta(
      "zip",
      fileManifest.workspace_snapshot_path,
      assetCatalog.find((entry) => entry.kind === "workspace_snapshot" && entry.source_id === workspaceSnapshotSourceId),
      writeZipFile,
    );
  }

  addBundleReadme(finalized, fileManifest, writeZipFile, "zip");
  return zip.generateAsync({ type: "nodebuffer", compression: "DEFLATE", compressionOptions: { level: 6 } });
}

export async function writeOutputFile(outputPath, contents, binary = false) {
  const absolutePath = path.resolve(outputPath);
  await mkdir(path.dirname(absolutePath), { recursive: true });
  await writeFile(absolutePath, contents, binary ? undefined : "utf8");
}

export async function writeProjectDirectory(bundle, outputDirectory) {
  const finalized = finalizeProjectBundle(bundle);
  const fileManifest = finalized.project_file_manifest;
  const assetCatalog = finalized.asset_catalog;
  const absoluteOutputDirectory = path.resolve(outputDirectory);
  const writeProjectFile = async (_container, relativePath, contents) => writeOutputFile(path.join(absoluteOutputDirectory, relativePath), contents);

  await writeProjectFile("dir", PROJECT_MANIFEST_PATH, JSON.stringify(finalized, null, 2));
  await writeProjectFile("dir", PROJECT_ENGINE_MANIFEST_PATH, JSON.stringify(fileManifest, null, 2));
  await writeProjectFile("dir", PROJECT_RECORD_PATH, JSON.stringify(finalized.project, null, 2));
  await writeProjectFile("dir", fileManifest.project_record_path, JSON.stringify(finalized.project, null, 2));
  await writeProjectFile("dir", fileManifest.workspace_settings_path, serializeWorkspaceSettings(finalized, fileManifest));
  await writeProjectFile("dir", fileManifest.asset_catalog_path, JSON.stringify(finalized.asset_catalog, null, 2));
  await writeProjectFile("dir", fileManifest.asset_references_path, JSON.stringify(finalized.asset_references, null, 2));
  await writeProjectFile("dir", JOBS_INDEX_PATH, JSON.stringify(finalized.jobs ?? [], null, 2));
  await writeProjectFile("dir", STANDARD_JOBS_INDEX_PATH, JSON.stringify(finalized.jobs ?? [], null, 2));
  await writeProjectFile("dir", RESULTS_INDEX_PATH, JSON.stringify(finalized.results ?? [], null, 2));
  await writeProjectFile("dir", STANDARD_RESULTS_INDEX_PATH, JSON.stringify(finalized.results ?? [], null, 2));

  if (finalized.workspace_snapshot) {
    await writeProjectFile("dir", WORKSPACE_SNAPSHOT_PATH, JSON.stringify(finalized.workspace_snapshot, null, 2));
    await writeProjectFile("dir", fileManifest.workspace_snapshot_path, JSON.stringify(finalized.workspace_snapshot, null, 2));
  }
  if ((finalized.automation_presets?.length ?? 0) > 0) {
    await writeProjectFile("dir", fileManifest.automation_presets_path, JSON.stringify(finalized.automation_presets, null, 2));
    await writeProjectFile(
      "dir",
      `${fileManifest.automation_presets_path}.meta`,
      JSON.stringify(assetCatalog.filter((entry) => entry.kind === "automation_preset"), null, 2),
    );
  }

  for (const model of finalized.models) {
    const assetPath = `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model" && entry.source_id === model.model_id);
    await writeProjectFile("dir", `models/${model.model_id}.json`, JSON.stringify(model, null, 2));
    await writeProjectFile("dir", assetPath, JSON.stringify(model, null, 2));
    if (meta) await writeProjectFile("dir", `${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  for (const version of finalized.model_versions) {
    const assetPath = `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model_version" && entry.source_id === version.version_id);
    await writeProjectFile("dir", `versions/${version.version_id}.json`, JSON.stringify(version, null, 2));
    await writeProjectFile("dir", assetPath, JSON.stringify(version, null, 2));
    if (meta) await writeProjectFile("dir", `${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  for (const job of finalized.jobs ?? []) {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    const assetPath = `${fileManifest.job_directory}/${slugifyPathSegment(jobId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "job" && entry.source_id === jobId);
    await writeProjectFile("dir", `jobs/${jobId}.json`, JSON.stringify(job, null, 2));
    await writeProjectFile("dir", assetPath, JSON.stringify(job, null, 2));
    if (meta) await writeProjectFile("dir", `${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  for (const result of finalized.results ?? []) {
    const resultId = typeof result.job_id === "string" ? result.job_id : "result";
    const assetPath = `${fileManifest.result_directory}/${slugifyPathSegment(resultId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "result" && entry.source_id === resultId);
    await writeProjectFile("dir", `results/${resultId}.json`, JSON.stringify(result, null, 2));
    await writeProjectFile("dir", assetPath, JSON.stringify(result, null, 2));
    if (meta) await writeProjectFile("dir", `${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  await writeProjectFile("dir", `${fileManifest.project_record_path}.meta`, JSON.stringify(assetCatalog.find((entry) => entry.kind === "project" && entry.source_id === finalized.project.project_id), null, 2));
  await writeProjectFile("dir", `${fileManifest.workspace_settings_path}.meta`, JSON.stringify(assetCatalog.find((entry) => entry.kind === "workspace_settings" && entry.source_id === finalized.project.project_id), null, 2));

  const workspaceSnapshotSourceId = finalized.active_version_id ?? finalized.active_model_id ?? finalized.project.project_id;
  if (finalized.workspace_snapshot) {
    await writeProjectFile(
      "dir",
      `${fileManifest.workspace_snapshot_path}.meta`,
      JSON.stringify(assetCatalog.find((entry) => entry.kind === "workspace_snapshot" && entry.source_id === workspaceSnapshotSourceId), null, 2),
    );
  }

  addBundleReadme(finalized, fileManifest, writeProjectFile, "dir");
}
