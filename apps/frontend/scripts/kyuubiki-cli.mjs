#!/usr/bin/env node

import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import JSZip from "jszip";
import { listAutomationActionContracts, validateAutomationStep } from "./kyuubiki-automation-actions.mjs";
import { runHeadlessAutomationEnvelope } from "./kyuubiki-automation-runner.mjs";
import { createPlaywrightExecutor } from "./kyuubiki-playwright-executor.mjs";
import { buildHeadlessAutomationEnvelope, normalizeMacroDraft } from "./kyuubiki-macro-headless.mjs";

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

function usage() {
  console.log(`kyuubiki frontend CLI

Usage:
  kyuubiki help
  kyuubiki project inspect <bundle> [--json]
  kyuubiki project validate <input> [--json]
  kyuubiki project normalize <input> --out <output>
  kyuubiki project unpack <bundle> --out <directory>
  kyuubiki project pack <input> --out <bundle>
  kyuubiki project diff <left> <right> [--json]
  kyuubiki project automation-presets <input> [--json]
  kyuubiki project automation-render <input> --preset <id|name> [--payload payload.json] [--state state.json] [--json]
  kyuubiki project automation-run <input> --preset <id|name> [--payload payload.json] [--state state.json] [--json] [--execute] [--allow-sensitive] [--allow-destructive] [--artifacts-dir dir]
  kyuubiki macro inspect <macro.json> [--json]
  kyuubiki macro actions [--json]
  kyuubiki macro validate <input> [--json]
  kyuubiki macro normalize <input> --out <output>
  kyuubiki macro render <input> [--payload payload.json] [--state state.json] [--json]
  kyuubiki macro run <input> [--payload payload.json] [--state state.json] [--json] [--execute] [--allow-sensitive] [--allow-destructive] [--artifacts-dir dir]

Examples:
  kyuubiki project inspect demo.kyuubiki
  kyuubiki project validate demo.kyuubiki --json
  kyuubiki project normalize demo.kyuubiki --out demo.normalized.kyuubiki
  kyuubiki project unpack demo.kyuubiki --out ./tmp/demo-project
  kyuubiki project pack ./tmp/demo-project --out demo.repacked.kyuubiki
  kyuubiki project diff before.kyuubiki after.kyuubiki
  kyuubiki project automation-presets demo.kyuubiki --json
  kyuubiki project automation-render demo.kyuubiki --preset preset_123 --payload payload.json --json
  kyuubiki project automation-run demo.kyuubiki --preset preset_123 --execute --allow-sensitive --artifacts-dir ./artifacts
  kyuubiki macro inspect review-result.json
  kyuubiki macro actions --json
  kyuubiki macro validate review-result.json --json
  kyuubiki macro normalize review-result.json --out review-result.normalized.json
  kyuubiki macro render review-result.json --payload payload.json --state state.json --json
  kyuubiki macro run review-result.json --execute --allow-sensitive --allow-destructive --artifacts-dir ./artifacts
`);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

function parseFlags(args) {
  const flags = {};
  for (let index = 0; index < args.length; index += 1) {
    const token = args[index];
    if (!token.startsWith("--")) continue;
    const key = token.slice(2);
    const next = args[index + 1];
    if (!next || next.startsWith("--")) {
      flags[key] = true;
      continue;
    }
    flags[key] = next;
    index += 1;
  }
  return flags;
}

function positionalArgs(args) {
  const result = [];
  for (let index = 0; index < args.length; index += 1) {
    const token = args[index];
    if (token.startsWith("--")) {
      const next = args[index + 1];
      if (next && !next.startsWith("--")) {
        index += 1;
      }
      continue;
    }
    result.push(token);
  }
  return result;
}

function defaultProjectFileManifest() {
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

function normalizeProjectBundle(raw) {
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
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  const metadata = value.analysis_metadata;
  if (!metadata || typeof metadata !== "object" || Array.isArray(metadata)) {
    return {};
  }
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
    const analysisMetadata = extractAnalysisMetadata(model.payload);
    catalog.push({
      guid: stableAssetGuid(`model:${model.model_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "model",
      path: `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`,
      source_id: model.model_id,
      name: model.name,
      updated_at: model.updated_at ?? bundle.exported_at ?? null,
      ...analysisMetadata,
    });
  }

  for (const version of bundle.model_versions) {
    const analysisMetadata = extractAnalysisMetadata(version.payload);
    catalog.push({
      guid: stableAssetGuid(`version:${version.version_id}`),
      meta_version: "kyuubiki.asset-meta/v1",
      kind: "model_version",
      path: `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`,
      source_id: version.version_id,
      name: version.name ?? `${version.kind} v${version.version_number}`,
      updated_at: version.updated_at ?? bundle.exported_at ?? null,
      ...analysisMetadata,
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

  if (projectGuid && workspaceSettingsGuid) {
    refs.push({ from_guid: projectGuid, relation: "workspace_settings_for", to_guid: workspaceSettingsGuid });
  }
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
  if (projectGuid && workspaceSnapshotGuid) {
    refs.push({ from_guid: projectGuid, relation: "workspace_snapshot_of", to_guid: workspaceSnapshotGuid });
  }

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
  if (absolutePath.endsWith(".kyuubiki")) {
    const zip = await JSZip.loadAsync(await readFile(absolutePath));
    const manifestFile = zip.file(PROJECT_MANIFEST_PATH) ?? zip.file(PROJECT_RECORD_PATH);
    if (!manifestFile) throw new Error(`missing ${PROJECT_MANIFEST_PATH} in project archive`);
    const bundle = normalizeProjectBundle(JSON.parse(await manifestFile.async("string")));
    const fileManifest = bundle.project_file_manifest ?? defaultProjectFileManifest();

    if (!bundle.workspace_snapshot) {
      const workspaceSnapshot = zip.file(fileManifest.workspace_snapshot_path) ?? zip.file(WORKSPACE_SNAPSHOT_PATH);
      if (workspaceSnapshot) bundle.workspace_snapshot = JSON.parse(await workspaceSnapshot.async("string"));
    }
    if ((bundle.automation_presets?.length ?? 0) === 0) {
      const automationPresets = zip.file(fileManifest.automation_presets_path);
      if (automationPresets) bundle.automation_presets = JSON.parse(await automationPresets.async("string"));
    }
    if ((bundle.asset_catalog?.length ?? 0) === 0) {
      const assetCatalog = zip.file(fileManifest.asset_catalog_path);
      if (assetCatalog) bundle.asset_catalog = JSON.parse(await assetCatalog.async("string"));
    }
    if ((bundle.asset_references?.length ?? 0) === 0) {
      const assetReferences = zip.file(fileManifest.asset_references_path) ?? zip.file(STANDARD_ASSET_REFERENCES_PATH);
      if (assetReferences) bundle.asset_references = JSON.parse(await assetReferences.async("string"));
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
  return normalizeProjectBundle(JSON.parse(await readFile(absolutePath, "utf8")));
}

async function pathIsDirectory(targetPath) {
  try {
    return (await stat(path.resolve(targetPath))).isDirectory();
  } catch {
    return false;
  }
}

async function parseProjectBundleInput(inputPath) {
  const absolutePath = path.resolve(inputPath);
  if (await pathIsDirectory(absolutePath)) {
    return normalizeProjectBundle(JSON.parse(await readFile(path.join(absolutePath, PROJECT_MANIFEST_PATH), "utf8")));
  }
  return parseProjectBundleFile(absolutePath);
}

function finalizeProjectBundle(bundle) {
  const normalized = normalizeProjectBundle(bundle);
  const fileManifest = normalized.project_file_manifest ?? defaultProjectFileManifest();
  const assetCatalog = normalized.asset_catalog?.length ? normalized.asset_catalog : buildProjectAssetCatalog(normalized, fileManifest);
  const assetReferences =
    normalized.asset_references?.length ? normalized.asset_references : buildProjectAssetReferences(normalized, assetCatalog);
  return {
    ...normalized,
    project_file_manifest: fileManifest,
    asset_catalog: assetCatalog,
    asset_references: assetReferences,
  };
}

async function exportProjectBundleZip(bundle) {
  const finalized = finalizeProjectBundle(bundle);
  const fileManifest = finalized.project_file_manifest;
  const assetCatalog = finalized.asset_catalog;
  const assetReferences = finalized.asset_references;
  const zip = new JSZip();

  zip.file(PROJECT_MANIFEST_PATH, JSON.stringify(finalized, null, 2));
  zip.file(PROJECT_ENGINE_MANIFEST_PATH, JSON.stringify(fileManifest, null, 2));
  zip.file(PROJECT_RECORD_PATH, JSON.stringify(finalized.project, null, 2));
  zip.file(fileManifest.project_record_path, JSON.stringify(finalized.project, null, 2));
  zip.file(fileManifest.workspace_settings_path, JSON.stringify({
    active_model_id: finalized.active_model_id ?? null,
    active_version_id: finalized.active_version_id ?? null,
    exported_at: finalized.exported_at ?? null,
    project_schema_version: finalized.project_schema_version,
    layout_version: fileManifest.layout_version,
  }, null, 2));

  if (finalized.workspace_snapshot) {
    zip.file(WORKSPACE_SNAPSHOT_PATH, JSON.stringify(finalized.workspace_snapshot, null, 2));
    zip.file(fileManifest.workspace_snapshot_path, JSON.stringify(finalized.workspace_snapshot, null, 2));
  }
  if ((finalized.automation_presets?.length ?? 0) > 0) {
    zip.file(fileManifest.automation_presets_path, JSON.stringify(finalized.automation_presets, null, 2));
  }
  zip.file(fileManifest.asset_catalog_path, JSON.stringify(assetCatalog, null, 2));
  zip.file(fileManifest.asset_references_path, JSON.stringify(assetReferences, null, 2));

  for (const model of finalized.models) {
    const assetPath = `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model" && entry.source_id === model.model_id);
    zip.file(`models/${model.model_id}.json`, JSON.stringify(model, null, 2));
    zip.file(assetPath, JSON.stringify(model, null, 2));
    if (meta) zip.file(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  for (const version of finalized.model_versions) {
    const assetPath = `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model_version" && entry.source_id === version.version_id);
    zip.file(`versions/${version.version_id}.json`, JSON.stringify(version, null, 2));
    zip.file(assetPath, JSON.stringify(version, null, 2));
    if (meta) zip.file(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  zip.file(JOBS_INDEX_PATH, JSON.stringify(finalized.jobs ?? [], null, 2));
  zip.file(fileManifest.job_directory + "/index.json", JSON.stringify(finalized.jobs ?? [], null, 2));
  for (const job of finalized.jobs ?? []) {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    const assetPath = `${fileManifest.job_directory}/${slugifyPathSegment(jobId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "job" && entry.source_id === jobId);
    zip.file(`jobs/${jobId}.json`, JSON.stringify(job, null, 2));
    zip.file(assetPath, JSON.stringify(job, null, 2));
    if (meta) zip.file(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  zip.file(RESULTS_INDEX_PATH, JSON.stringify(finalized.results ?? [], null, 2));
  zip.file(fileManifest.result_directory + "/index.json", JSON.stringify(finalized.results ?? [], null, 2));
  for (const result of finalized.results ?? []) {
    const resultId = typeof result.job_id === "string" ? result.job_id : "result";
    const assetPath = `${fileManifest.result_directory}/${slugifyPathSegment(resultId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "result" && entry.source_id === resultId);
    zip.file(`results/${resultId}.json`, JSON.stringify(result, null, 2));
    zip.file(assetPath, JSON.stringify(result, null, 2));
    if (meta) zip.file(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  const projectMeta = assetCatalog.find((entry) => entry.kind === "project" && entry.source_id === finalized.project.project_id);
  if (projectMeta) zip.file(`${fileManifest.project_record_path}.meta`, JSON.stringify(projectMeta, null, 2));

  const workspaceSettingsMeta = assetCatalog.find((entry) => entry.kind === "workspace_settings" && entry.source_id === finalized.project.project_id);
  if (workspaceSettingsMeta) zip.file(`${fileManifest.workspace_settings_path}.meta`, JSON.stringify(workspaceSettingsMeta, null, 2));

  const workspaceSnapshotSourceId = finalized.active_version_id ?? finalized.active_model_id ?? finalized.project.project_id;
  const workspaceSnapshotMeta = assetCatalog.find((entry) => entry.kind === "workspace_snapshot" && entry.source_id === workspaceSnapshotSourceId);
  if (workspaceSnapshotMeta && finalized.workspace_snapshot) {
    zip.file(`${fileManifest.workspace_snapshot_path}.meta`, JSON.stringify(workspaceSnapshotMeta, null, 2));
  }

  if ((finalized.automation_presets?.length ?? 0) > 0) {
    const presetMetas = assetCatalog.filter((entry) => entry.kind === "automation_preset");
    zip.file(`${fileManifest.automation_presets_path}.meta`, JSON.stringify(presetMetas, null, 2));
  }

  zip.file("README.txt", [
    "Kyuubiki project bundle",
    "",
    `Schema: ${finalized.project_schema_version}`,
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
    `  ${fileManifest.asset_catalog_path}`,
    `  ${fileManifest.asset_references_path}`,
    `  ${fileManifest.job_directory}/index.json`,
    `  ${fileManifest.result_directory}/index.json`,
  ].join("\n"));

  return zip.generateAsync({ type: "nodebuffer", compression: "DEFLATE", compressionOptions: { level: 6 } });
}

async function writeOutputFile(outputPath, contents, binary = false) {
  const absolutePath = path.resolve(outputPath);
  await mkdir(path.dirname(absolutePath), { recursive: true });
  await writeFile(absolutePath, contents, binary ? undefined : "utf8");
}

async function writeProjectDirectory(bundle, outputDirectory) {
  const finalized = finalizeProjectBundle(bundle);
  const fileManifest = finalized.project_file_manifest;
  const assetCatalog = finalized.asset_catalog;
  const absoluteOutputDirectory = path.resolve(outputDirectory);

  const writeProjectFile = async (relativePath, contents) => {
    await writeOutputFile(path.join(absoluteOutputDirectory, relativePath), contents);
  };

  await writeProjectFile(PROJECT_MANIFEST_PATH, JSON.stringify(finalized, null, 2));
  await writeProjectFile(PROJECT_ENGINE_MANIFEST_PATH, JSON.stringify(fileManifest, null, 2));
  await writeProjectFile(PROJECT_RECORD_PATH, JSON.stringify(finalized.project, null, 2));
  await writeProjectFile(fileManifest.project_record_path, JSON.stringify(finalized.project, null, 2));
  await writeProjectFile(
    fileManifest.workspace_settings_path,
    JSON.stringify(
      {
        active_model_id: finalized.active_model_id ?? null,
        active_version_id: finalized.active_version_id ?? null,
        exported_at: finalized.exported_at ?? null,
        project_schema_version: finalized.project_schema_version,
        layout_version: fileManifest.layout_version,
      },
      null,
      2,
    ),
  );

  if (finalized.workspace_snapshot) {
    await writeProjectFile(WORKSPACE_SNAPSHOT_PATH, JSON.stringify(finalized.workspace_snapshot, null, 2));
    await writeProjectFile(fileManifest.workspace_snapshot_path, JSON.stringify(finalized.workspace_snapshot, null, 2));
  }

  if ((finalized.automation_presets?.length ?? 0) > 0) {
    await writeProjectFile(fileManifest.automation_presets_path, JSON.stringify(finalized.automation_presets, null, 2));
  }

  await writeProjectFile(fileManifest.asset_catalog_path, JSON.stringify(finalized.asset_catalog, null, 2));
  await writeProjectFile(fileManifest.asset_references_path, JSON.stringify(finalized.asset_references, null, 2));

  for (const model of finalized.models) {
    const assetPath = `${fileManifest.model_directory}/${slugifyPathSegment(model.model_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model" && entry.source_id === model.model_id);
    await writeProjectFile(`models/${model.model_id}.json`, JSON.stringify(model, null, 2));
    await writeProjectFile(assetPath, JSON.stringify(model, null, 2));
    if (meta) await writeProjectFile(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  for (const version of finalized.model_versions) {
    const assetPath = `${fileManifest.version_directory}/${slugifyPathSegment(version.version_id)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "model_version" && entry.source_id === version.version_id);
    await writeProjectFile(`versions/${version.version_id}.json`, JSON.stringify(version, null, 2));
    await writeProjectFile(assetPath, JSON.stringify(version, null, 2));
    if (meta) await writeProjectFile(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  await writeProjectFile(JOBS_INDEX_PATH, JSON.stringify(finalized.jobs ?? [], null, 2));
  await writeProjectFile(STANDARD_JOBS_INDEX_PATH, JSON.stringify(finalized.jobs ?? [], null, 2));
  for (const job of finalized.jobs ?? []) {
    const jobId = typeof job.job_id === "string" ? job.job_id : "job";
    const assetPath = `${fileManifest.job_directory}/${slugifyPathSegment(jobId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "job" && entry.source_id === jobId);
    await writeProjectFile(`jobs/${jobId}.json`, JSON.stringify(job, null, 2));
    await writeProjectFile(assetPath, JSON.stringify(job, null, 2));
    if (meta) await writeProjectFile(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  await writeProjectFile(RESULTS_INDEX_PATH, JSON.stringify(finalized.results ?? [], null, 2));
  await writeProjectFile(STANDARD_RESULTS_INDEX_PATH, JSON.stringify(finalized.results ?? [], null, 2));
  for (const result of finalized.results ?? []) {
    const resultId = typeof result.job_id === "string" ? result.job_id : "result";
    const assetPath = `${fileManifest.result_directory}/${slugifyPathSegment(resultId)}.json`;
    const meta = assetCatalog.find((entry) => entry.kind === "result" && entry.source_id === resultId);
    await writeProjectFile(`results/${resultId}.json`, JSON.stringify(result, null, 2));
    await writeProjectFile(assetPath, JSON.stringify(result, null, 2));
    if (meta) await writeProjectFile(`${assetPath}.meta`, JSON.stringify(meta, null, 2));
  }

  const projectMeta = assetCatalog.find((entry) => entry.kind === "project" && entry.source_id === finalized.project.project_id);
  if (projectMeta) await writeProjectFile(`${fileManifest.project_record_path}.meta`, JSON.stringify(projectMeta, null, 2));

  const workspaceSettingsMeta = assetCatalog.find((entry) => entry.kind === "workspace_settings" && entry.source_id === finalized.project.project_id);
  if (workspaceSettingsMeta) {
    await writeProjectFile(`${fileManifest.workspace_settings_path}.meta`, JSON.stringify(workspaceSettingsMeta, null, 2));
  }

  const workspaceSnapshotSourceId = finalized.active_version_id ?? finalized.active_model_id ?? finalized.project.project_id;
  const workspaceSnapshotMeta = assetCatalog.find((entry) => entry.kind === "workspace_snapshot" && entry.source_id === workspaceSnapshotSourceId);
  if (workspaceSnapshotMeta && finalized.workspace_snapshot) {
    await writeProjectFile(`${fileManifest.workspace_snapshot_path}.meta`, JSON.stringify(workspaceSnapshotMeta, null, 2));
  }

  if ((finalized.automation_presets?.length ?? 0) > 0) {
    const presetMetas = assetCatalog.filter((entry) => entry.kind === "automation_preset");
    await writeProjectFile(`${fileManifest.automation_presets_path}.meta`, JSON.stringify(presetMetas, null, 2));
  }

  await writeProjectFile(
    "README.txt",
    [
      "Kyuubiki project directory",
      "",
      `Schema: ${finalized.project_schema_version}`,
      `Layout: ${fileManifest.layout_version}`,
      `Manifest: ${PROJECT_MANIFEST_PATH}`,
      `Engine manifest: ${fileManifest.engine_manifest_path}`,
      `Asset catalog: ${fileManifest.asset_catalog_path}`,
      `Asset references: ${fileManifest.asset_references_path}`,
    ].join("\n"),
  );
}

function projectInspectSummary(bundle) {
  const modelAssets = (bundle.asset_catalog ?? []).filter((entry) => entry.kind === "model");
  const analysisDomains = Array.from(new Set(modelAssets.map((entry) => entry.analysis_domain).filter(Boolean))).sort();
  const analysisFamilies = Array.from(new Set(modelAssets.map((entry) => entry.analysis_family).filter(Boolean))).sort();
  const thermalIntents = Array.from(new Set(modelAssets.flatMap((entry) => (Array.isArray(entry.thermal_intent) ? entry.thermal_intent : [])).filter(Boolean))).sort();
  return {
    schema: bundle.project_schema_version,
    layout: bundle.project_file_manifest?.layout_version ?? null,
    project_id: bundle.project.project_id,
    project_name: bundle.project.name,
    model_count: bundle.models.length,
    version_count: bundle.model_versions.length,
    job_count: bundle.jobs?.length ?? 0,
    result_count: bundle.results?.length ?? 0,
    automation_preset_count: bundle.automation_presets?.length ?? 0,
    asset_count: bundle.asset_catalog?.length ?? 0,
    asset_reference_count: bundle.asset_references?.length ?? 0,
    active_model_id: bundle.active_model_id ?? null,
    active_version_id: bundle.active_version_id ?? null,
    has_workspace_snapshot: Boolean(bundle.workspace_snapshot),
    analysis_domains: analysisDomains,
    analysis_families: analysisFamilies,
    thermal_intents: thermalIntents,
  };
}

async function handleProjectInspect(inputPath, flags) {
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  const summary = projectInspectSummary(bundle);
  if (flags.json) {
    console.log(JSON.stringify(summary, null, 2));
    return;
  }
  console.log(`Project: ${summary.project_name} (${summary.project_id})`);
  console.log(`Schema: ${summary.schema}`);
  console.log(`Layout: ${summary.layout}`);
  console.log(`Models: ${summary.model_count}`);
  console.log(`Versions: ${summary.version_count}`);
  console.log(`Jobs: ${summary.job_count}`);
  console.log(`Results: ${summary.result_count}`);
  console.log(`Automation presets: ${summary.automation_preset_count}`);
  console.log(`Assets: ${summary.asset_count}`);
  console.log(`Asset references: ${summary.asset_reference_count}`);
  console.log(`Active model: ${summary.active_model_id ?? "--"}`);
  console.log(`Active version: ${summary.active_version_id ?? "--"}`);
  console.log(`Workspace snapshot: ${summary.has_workspace_snapshot ? "yes" : "no"}`);
  console.log(`Analysis domains: ${summary.analysis_domains.length > 0 ? summary.analysis_domains.join(", ") : "--"}`);
  console.log(`Analysis families: ${summary.analysis_families.length > 0 ? summary.analysis_families.join(", ") : "--"}`);
  console.log(`Thermal intents: ${summary.thermal_intents.length > 0 ? summary.thermal_intents.join(", ") : "--"}`);
}

async function handleProjectValidate(inputPath, flags) {
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  const report = validateProjectBundle(bundle);
  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exitCode = 1;
    return;
  }
  console.log(`Project validation: ${report.ok ? "ok" : "failed"}`);
  console.log(`Project: ${report.summary.project_name} (${report.summary.project_id})`);
  console.log(`Issues: ${report.issue_count}`);
  if (report.issues.length > 0) {
    for (const issue of report.issues) {
      console.log(`- ${issue}`);
    }
    process.exitCode = 1;
  }
}

async function handleProjectNormalize(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("project normalize requires --out <output>");
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  if (outputPath.endsWith(".kyuubiki")) {
    const zip = await exportProjectBundleZip(bundle);
    await writeOutputFile(outputPath, zip, true);
  } else {
    await writeOutputFile(outputPath, JSON.stringify(bundle, null, 2));
  }
  console.log(`normalized project bundle -> ${path.resolve(outputPath)}`);
}

async function handleProjectUnpack(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("project unpack requires --out <directory>");
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  await writeProjectDirectory(bundle, outputPath);
  console.log(`unpacked project bundle -> ${path.resolve(outputPath)}`);
}

async function handleProjectPack(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("project pack requires --out <bundle>");
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  if (outputPath.endsWith(".kyuubiki")) {
    const zip = await exportProjectBundleZip(bundle);
    await writeOutputFile(outputPath, zip, true);
  } else {
    await writeOutputFile(outputPath, JSON.stringify(bundle, null, 2));
  }
  console.log(`packed project bundle -> ${path.resolve(outputPath)}`);
}

function buildKindIndex(entries = []) {
  const index = new Map();
  for (const entry of entries) {
    const bucket = index.get(entry.kind) ?? [];
    bucket.push(entry.source_id);
    index.set(entry.kind, bucket);
  }
  for (const bucket of index.values()) {
    bucket.sort();
  }
  return index;
}

function diffSortedLists(left = [], right = []) {
  const leftSet = new Set(left);
  const rightSet = new Set(right);
  return {
    added: right.filter((entry) => !leftSet.has(entry)),
    removed: left.filter((entry) => !rightSet.has(entry)),
  };
}

function projectDiffSummary(left, right) {
  const leftKinds = buildKindIndex(left.asset_catalog);
  const rightKinds = buildKindIndex(right.asset_catalog);
  const kindNames = Array.from(new Set([...leftKinds.keys(), ...rightKinds.keys()])).sort();
  return {
    left: projectInspectSummary(left),
    right: projectInspectSummary(right),
    changed_project_identity:
      left.project.project_id !== right.project.project_id || left.project.name !== right.project.name,
    active_model_changed: left.active_model_id !== right.active_model_id,
    active_version_changed: left.active_version_id !== right.active_version_id,
    asset_kind_diff: Object.fromEntries(
      kindNames.map((kind) => [kind, diffSortedLists(leftKinds.get(kind) ?? [], rightKinds.get(kind) ?? [])]),
    ),
    automation_preset_ids: diffSortedLists(
      (left.automation_presets ?? []).map((entry) => entry.presetId).sort(),
      (right.automation_presets ?? []).map((entry) => entry.presetId).sort(),
    ),
  };
}

function validateProjectBundle(bundle) {
  const issues = [];
  const fileManifest = bundle.project_file_manifest ?? defaultProjectFileManifest();
  const assetCatalog = bundle.asset_catalog ?? [];
  const assetReferences = bundle.asset_references ?? [];
  const guidSet = new Set();
  const guidByKindAndSource = new Map();

  for (const entry of assetCatalog) {
    if (!entry?.guid || typeof entry.guid !== "string") {
      issues.push("asset_catalog contains an entry without a valid guid");
      continue;
    }
    if (guidSet.has(entry.guid)) {
      issues.push(`duplicate asset guid detected: ${entry.guid}`);
    }
    guidSet.add(entry.guid);
    guidByKindAndSource.set(`${entry.kind}:${entry.source_id}`, entry.guid);
  }

  const ensureGuid = (key, label) => {
    if (!guidByKindAndSource.has(key)) {
      issues.push(`missing asset catalog entry for ${label}`);
    }
  };

  ensureGuid(`project:${bundle.project.project_id}`, `project ${bundle.project.project_id}`);
  ensureGuid(`workspace_settings:${bundle.project.project_id}`, `workspace settings for project ${bundle.project.project_id}`);

  if (bundle.workspace_snapshot) {
    const workspaceSnapshotSourceId = bundle.active_version_id ?? bundle.active_model_id ?? bundle.project.project_id;
    ensureGuid(`workspace_snapshot:${workspaceSnapshotSourceId}`, "workspace snapshot");
  }

  for (const model of bundle.models) {
    ensureGuid(`model:${model.model_id}`, `model ${model.model_id}`);
  }

  for (const version of bundle.model_versions) {
    ensureGuid(`model_version:${version.version_id}`, `model version ${version.version_id}`);
    if (!bundle.models.some((model) => model.model_id === version.model_id)) {
      issues.push(`model version ${version.version_id} points to missing model ${version.model_id}`);
    }
  }

  for (const preset of bundle.automation_presets ?? []) {
    ensureGuid(`automation_preset:${preset.presetId}`, `automation preset ${preset.presetId}`);
  }

  for (const job of bundle.jobs ?? []) {
    const jobId = typeof job.job_id === "string" ? job.job_id : null;
    if (!jobId) {
      issues.push("job record is missing job_id");
      continue;
    }
    ensureGuid(`job:${jobId}`, `job ${jobId}`);
    if (typeof job.model_version_id === "string" && !bundle.model_versions.some((version) => version.version_id === job.model_version_id)) {
      issues.push(`job ${jobId} points to missing model version ${job.model_version_id}`);
    }
  }

  for (const result of bundle.results ?? []) {
    ensureGuid(`result:${result.job_id}`, `result ${result.job_id}`);
    if (!(bundle.jobs ?? []).some((job) => job.job_id === result.job_id)) {
      issues.push(`result ${result.job_id} has no matching job record`);
    }
  }

  if (bundle.active_model_id && !bundle.models.some((model) => model.model_id === bundle.active_model_id)) {
    issues.push(`active_model_id points to missing model ${bundle.active_model_id}`);
  }
  if (bundle.active_version_id && !bundle.model_versions.some((version) => version.version_id === bundle.active_version_id)) {
    issues.push(`active_version_id points to missing model version ${bundle.active_version_id}`);
  }

  const expectedPaths = new Set([
    fileManifest.project_record_path,
    fileManifest.workspace_settings_path,
    fileManifest.workspace_snapshot_path,
    fileManifest.automation_presets_path,
    fileManifest.asset_catalog_path,
    fileManifest.asset_references_path,
  ]);
  for (const entry of assetCatalog) {
    if (!entry.path || typeof entry.path !== "string") {
      issues.push(`asset ${entry.guid} is missing a valid path`);
      continue;
    }
    if (
      entry.path !== fileManifest.project_record_path &&
      entry.path !== fileManifest.workspace_settings_path &&
      entry.path !== fileManifest.workspace_snapshot_path &&
      entry.path !== fileManifest.automation_presets_path &&
      !entry.path.startsWith(`${fileManifest.model_directory}/`) &&
      !entry.path.startsWith(`${fileManifest.version_directory}/`) &&
      !entry.path.startsWith(`${fileManifest.job_directory}/`) &&
      !entry.path.startsWith(`${fileManifest.result_directory}/`)
    ) {
      issues.push(`asset ${entry.guid} uses unexpected path ${entry.path}`);
    } else {
      expectedPaths.add(entry.path);
    }
  }

  for (const reference of assetReferences) {
    if (!guidSet.has(reference.from_guid)) {
      issues.push(`asset reference has unknown from_guid ${reference.from_guid}`);
    }
    if (!guidSet.has(reference.to_guid)) {
      issues.push(`asset reference has unknown to_guid ${reference.to_guid}`);
    }
  }

  return {
    ok: issues.length === 0,
    issue_count: issues.length,
    issues,
    summary: projectInspectSummary(bundle),
    expected_paths: Array.from(expectedPaths).sort(),
  };
}

function validateMacroDraft(macro) {
  const issues = [];
  if (!macro.id || typeof macro.id !== "string") {
    issues.push("macro id is missing");
  }
  if (!Array.isArray(macro.steps) || macro.steps.length === 0) {
    issues.push("macro has no steps");
  }
  for (const [index, step] of (macro.steps ?? []).entries()) {
    const validation = validateAutomationStep(step, index);
    issues.push(...validation.issues);
  }
  return {
    ok: issues.length === 0,
    issue_count: issues.length,
    issues,
    summary: {
      id: macro.id,
      step_count: macro.steps.length,
      actions: macro.steps.map((step) => step.action),
    },
  };
}

async function handleMacroActions(flags) {
  const contracts = listAutomationActionContracts();
  if (flags.json) {
    console.log(JSON.stringify({ action_count: contracts.length, actions: contracts }, null, 2));
    return;
  }
  console.log(`Automation actions: ${contracts.length}`);
  for (const contract of contracts) {
    console.log(`- ${contract.id} [${contract.risk}]`);
    console.log(`  aliases: ${contract.aliases.join(", ") || "--"}`);
    console.log(`  summary: ${contract.summary}`);
    console.log(`  required payload: ${contract.requiredPayloadKeys.join(", ") || "--"}`);
  }
}

async function readOptionalJsonFile(inputPath) {
  if (!inputPath) return {};
  return JSON.parse(await readFile(path.resolve(inputPath), "utf8"));
}

async function withAutomationExecutor(flags, callback) {
  if (!flags.execute) return callback({ executor: null, artifactsDir: null });
  const runtime = await createPlaywrightExecutor({
    artifactsDir: typeof flags["artifacts-dir"] === "string" ? flags["artifacts-dir"] : undefined,
  });
  try {
    return await callback({
      executor: runtime.executor,
      artifactsDir: runtime.artifactsDir ?? null,
    });
  } finally {
    await runtime.dispose();
  }
}

async function handleProjectDiff(leftInputPath, rightInputPath, flags) {
  const left = finalizeProjectBundle(await parseProjectBundleInput(leftInputPath));
  const right = finalizeProjectBundle(await parseProjectBundleInput(rightInputPath));
  const summary = projectDiffSummary(left, right);
  if (flags.json) {
    console.log(JSON.stringify(summary, null, 2));
    return;
  }
  console.log(`Left:  ${summary.left.project_name} (${summary.left.project_id})`);
  console.log(`Right: ${summary.right.project_name} (${summary.right.project_id})`);
  console.log(`Schema: ${summary.left.schema} -> ${summary.right.schema}`);
  console.log(`Layout: ${summary.left.layout} -> ${summary.right.layout}`);
  console.log(`Active model changed: ${summary.active_model_changed ? "yes" : "no"}`);
  console.log(`Active version changed: ${summary.active_version_changed ? "yes" : "no"}`);
  console.log(`Project identity changed: ${summary.changed_project_identity ? "yes" : "no"}`);
  console.log("Asset kind diff:");
  for (const [kind, diff] of Object.entries(summary.asset_kind_diff)) {
    console.log(`  ${kind}: +${diff.added.length} / -${diff.removed.length}`);
  }
  console.log(`Automation presets: +${summary.automation_preset_ids.added.length} / -${summary.automation_preset_ids.removed.length}`);
}

async function handleProjectAutomationPresets(inputPath, flags) {
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  const presets = (bundle.automation_presets ?? []).map((preset) => ({
    preset_id: preset.presetId,
    project_id: preset.projectId,
    name: preset.name,
    updated_at: preset.updatedAt,
    macro_id: preset.macro?.id ?? null,
    step_count: Array.isArray(preset.macro?.steps) ? preset.macro.steps.length : 0,
    actions: Array.isArray(preset.macro?.steps) ? preset.macro.steps.map((step) => step.action) : [],
  }));
  if (flags.json) {
    console.log(JSON.stringify({ preset_count: presets.length, presets }, null, 2));
    return;
  }
  console.log(`Automation presets: ${presets.length}`);
  for (const preset of presets) {
    console.log(`- ${preset.name} (${preset.preset_id})`);
    console.log(`  steps: ${preset.step_count}`);
    console.log(`  actions: ${preset.actions.join(", ") || "--"}`);
  }
}

function findAutomationPreset(bundle, presetSelector) {
  const normalized = String(presetSelector ?? "").trim();
  if (!normalized) {
    throw new Error("automation-render requires --preset <id|name>");
  }
  const presets = bundle.automation_presets ?? [];
  return (
    presets.find((preset) => preset.presetId === normalized) ??
    presets.find((preset) => preset.name === normalized) ??
    null
  );
}

async function handleProjectAutomationRender(inputPath, flags) {
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  const preset = findAutomationPreset(bundle, flags.preset);
  if (!preset) {
    throw new Error(`Could not find automation preset "${String(flags.preset ?? "")}".`);
  }
  const payload = await readOptionalJsonFile(typeof flags.payload === "string" ? flags.payload : null);
  const state = await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null);
  const result = buildHeadlessAutomationEnvelope(
    {
      kind: "project_automation_preset",
      preset_id: preset.presetId,
      preset_name: preset.name,
      project_id: preset.projectId,
      updated_at: preset.updatedAt,
    },
    preset.macro,
    { payload, state },
  );
  if (flags.json) {
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  console.log(`Automation preset: ${result.source.preset_name} (${result.source.preset_id})`);
  console.log(`Project: ${result.source.project_id}`);
  console.log(`Steps: ${result.plan.step_count}`);
  console.log(`Highest risk: ${result.risk_summary.highest_risk}`);
  for (const [index, step] of result.plan.steps.entries()) {
    console.log(`${index + 1}. ${step.action} [${step.risk}]`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
}

async function handleProjectAutomationRun(inputPath, flags) {
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  const preset = findAutomationPreset(bundle, flags.preset);
  if (!preset) {
    throw new Error(`Could not find automation preset "${String(flags.preset ?? "")}".`);
  }
  const payload = await readOptionalJsonFile(typeof flags.payload === "string" ? flags.payload : null);
  const state = await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null);
  const envelope = buildHeadlessAutomationEnvelope(
    {
      kind: "project_automation_preset",
      preset_id: preset.presetId,
      preset_name: preset.name,
      project_id: preset.projectId,
      updated_at: preset.updatedAt,
    },
    preset.macro,
    { payload, state },
  );
  const report = await withAutomationExecutor(flags, ({ executor, artifactsDir }) =>
    runHeadlessAutomationEnvelope(envelope, {
      dryRun: !flags.execute,
      allowSensitive: flags["allow-sensitive"],
      allowDestructive: flags["allow-destructive"],
      executor,
      context: artifactsDir ? { artifactsDir } : {},
    }),
  );
  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    return;
  }
  console.log(`Automation run: ${report.metadata.macro_id}`);
  console.log(`Mode: ${report.dry_run ? "dry-run" : "stub-execute"}`);
  console.log(`Status: ${report.status}`);
  console.log(`Executed steps: ${report.executed_step_count}/${report.metadata.step_count}`);
  if (report.blocked_by_confirmation) {
    console.log(
      `Blocked: step ${report.blocked_by_confirmation.index + 1} requires ${report.blocked_by_confirmation.risk} confirmation`,
    );
  }
  for (const [index, step] of report.steps.entries()) {
    console.log(`${index + 1}. ${step.action} -> ${step.status}`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
}

async function handleMacroInspect(inputPath, flags) {
  const macro = normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  const summary = { id: macro.id, step_count: macro.steps.length, actions: macro.steps.map((step) => step.action) };
  if (flags.json) {
    console.log(JSON.stringify(summary, null, 2));
    return;
  }
  console.log(`Macro: ${summary.id}`);
  console.log(`Steps: ${summary.step_count}`);
  console.log(`Actions: ${summary.actions.join(", ")}`);
}

async function handleMacroValidate(inputPath, flags) {
  const macro = JSON.parse(await readFile(path.resolve(inputPath), "utf8"));
  const report = validateMacroDraft(macro);
  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exitCode = 1;
    return;
  }
  console.log(`Macro validation: ${report.ok ? "ok" : "failed"}`);
  console.log(`Macro: ${report.summary.id}`);
  console.log(`Steps: ${report.summary.step_count}`);
  if (report.issues.length > 0) {
    for (const issue of report.issues) {
      console.log(`- ${issue}`);
    }
    process.exitCode = 1;
  }
}

async function handleMacroNormalize(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("macro normalize requires --out <output>");
  const macro = normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  await writeOutputFile(outputPath, JSON.stringify(macro, null, 2));
  console.log(`normalized macro -> ${path.resolve(outputPath)}`);
}

async function handleMacroRender(inputPath, flags) {
  const macro = normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  const payload = await readOptionalJsonFile(typeof flags.payload === "string" ? flags.payload : null);
  const state = await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null);
  const envelope = buildHeadlessAutomationEnvelope(
    { kind: "macro_file", input_path: path.resolve(inputPath) },
    macro,
    { payload, state },
  );
  if (flags.json) {
    console.log(JSON.stringify(envelope, null, 2));
    return;
  }
  console.log(`Macro render: ${envelope.plan.id}`);
  console.log(`Steps: ${envelope.plan.step_count}`);
  console.log(`Highest risk: ${envelope.risk_summary.highest_risk}`);
  for (const [index, step] of envelope.plan.steps.entries()) {
    console.log(`${index + 1}. ${step.action} [${step.risk}]`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
}

async function handleMacroRun(inputPath, flags) {
  const macro = normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  const payload = await readOptionalJsonFile(typeof flags.payload === "string" ? flags.payload : null);
  const state = await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null);
  const envelope = buildHeadlessAutomationEnvelope(
    { kind: "macro_file", input_path: path.resolve(inputPath) },
    macro,
    { payload, state },
  );
  const report = await withAutomationExecutor(flags, ({ executor, artifactsDir }) =>
    runHeadlessAutomationEnvelope(envelope, {
      dryRun: !flags.execute,
      allowSensitive: flags["allow-sensitive"],
      allowDestructive: flags["allow-destructive"],
      executor,
      context: artifactsDir ? { artifactsDir } : {},
    }),
  );
  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    return;
  }
  console.log(`Macro run: ${report.metadata.macro_id}`);
  console.log(`Mode: ${report.dry_run ? "dry-run" : "stub-execute"}`);
  console.log(`Status: ${report.status}`);
  console.log(`Executed steps: ${report.executed_step_count}/${report.metadata.step_count}`);
  if (report.blocked_by_confirmation) {
    console.log(
      `Blocked: step ${report.blocked_by_confirmation.index + 1} requires ${report.blocked_by_confirmation.risk} confirmation`,
    );
  }
  for (const [index, step] of report.steps.entries()) {
    console.log(`${index + 1}. ${step.action} -> ${step.status}`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
}

async function main() {
  const args = process.argv.slice(2);
  const [scope = "help", command = ""] = positionalArgs(args);
  const flags = parseFlags(args);

  if (scope === "help" || scope === "--help" || scope === "-h") {
    usage();
    return;
  }

  if (scope === "project") {
    const [, , firstInputPath, secondInputPath] = positionalArgs(args);
    if (!firstInputPath) fail("project command requires an input path");
    if (command === "inspect") return handleProjectInspect(firstInputPath, flags);
    if (command === "validate") return handleProjectValidate(firstInputPath, flags);
    if (command === "normalize") return handleProjectNormalize(firstInputPath, flags);
    if (command === "unpack") return handleProjectUnpack(firstInputPath, flags);
    if (command === "pack") return handleProjectPack(firstInputPath, flags);
    if (command === "diff") {
      if (!secondInputPath) fail("project diff requires <left> <right>");
      return handleProjectDiff(firstInputPath, secondInputPath, flags);
    }
    if (command === "automation-presets") return handleProjectAutomationPresets(firstInputPath, flags);
    if (command === "automation-render") return handleProjectAutomationRender(firstInputPath, flags);
    if (command === "automation-run") return handleProjectAutomationRun(firstInputPath, flags);
    fail(`unknown project command: ${command}`);
  }

  if (scope === "macro") {
    const [, , inputPath] = positionalArgs(args);
    if (command === "actions") return handleMacroActions(flags);
    if (!inputPath) fail("macro command requires an input path");
    if (command === "inspect") return handleMacroInspect(inputPath, flags);
    if (command === "validate") return handleMacroValidate(inputPath, flags);
    if (command === "normalize") return handleMacroNormalize(inputPath, flags);
    if (command === "render") return handleMacroRender(inputPath, flags);
    if (command === "run") return handleMacroRun(inputPath, flags);
    fail(`unknown macro command: ${command}`);
  }

  fail(`unknown command: ${scope}`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
