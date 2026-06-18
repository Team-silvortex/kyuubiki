import { defaultProjectFileManifest } from "./kyuubiki-project-bundle-io.mjs";

export function projectInspectSummary(bundle) {
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

function buildKindIndex(entries = []) {
  const index = new Map();
  for (const entry of entries) {
    const bucket = index.get(entry.kind) ?? [];
    bucket.push(entry.source_id);
    index.set(entry.kind, bucket);
  }
  for (const bucket of index.values()) bucket.sort();
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

export function projectDiffSummary(left, right) {
  const leftKinds = buildKindIndex(left.asset_catalog);
  const rightKinds = buildKindIndex(right.asset_catalog);
  const kindNames = Array.from(new Set([...leftKinds.keys(), ...rightKinds.keys()])).sort();
  return {
    left: projectInspectSummary(left),
    right: projectInspectSummary(right),
    changed_project_identity: left.project.project_id !== right.project.project_id || left.project.name !== right.project.name,
    active_model_changed: left.active_model_id !== right.active_model_id,
    active_version_changed: left.active_version_id !== right.active_version_id,
    asset_kind_diff: Object.fromEntries(kindNames.map((kind) => [kind, diffSortedLists(leftKinds.get(kind) ?? [], rightKinds.get(kind) ?? [])])),
    automation_preset_ids: diffSortedLists(
      (left.automation_presets ?? []).map((entry) => entry.presetId).sort(),
      (right.automation_presets ?? []).map((entry) => entry.presetId).sort(),
    ),
  };
}

export function validateProjectBundle(bundle) {
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
    if (guidSet.has(entry.guid)) issues.push(`duplicate asset guid detected: ${entry.guid}`);
    guidSet.add(entry.guid);
    guidByKindAndSource.set(`${entry.kind}:${entry.source_id}`, entry.guid);
  }

  const ensureGuid = (key, label) => {
    if (!guidByKindAndSource.has(key)) issues.push(`missing asset catalog entry for ${label}`);
  };

  ensureGuid(`project:${bundle.project.project_id}`, `project ${bundle.project.project_id}`);
  ensureGuid(`workspace_settings:${bundle.project.project_id}`, `workspace settings for project ${bundle.project.project_id}`);

  if (bundle.workspace_snapshot) {
    const sourceId = bundle.active_version_id ?? bundle.active_model_id ?? bundle.project.project_id;
    ensureGuid(`workspace_snapshot:${sourceId}`, "workspace snapshot");
  }

  for (const model of bundle.models) ensureGuid(`model:${model.model_id}`, `model ${model.model_id}`);
  for (const version of bundle.model_versions) {
    ensureGuid(`model_version:${version.version_id}`, `model version ${version.version_id}`);
    if (!bundle.models.some((model) => model.model_id === version.model_id)) {
      issues.push(`model version ${version.version_id} points to missing model ${version.model_id}`);
    }
  }

  for (const preset of bundle.automation_presets ?? []) ensureGuid(`automation_preset:${preset.presetId}`, `automation preset ${preset.presetId}`);
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
    if (!(bundle.jobs ?? []).some((job) => job.job_id === result.job_id)) issues.push(`result ${result.job_id} has no matching job record`);
  }

  if (bundle.active_model_id && !bundle.models.some((model) => model.model_id === bundle.active_model_id)) {
    issues.push(`active_model_id points to missing model ${bundle.active_model_id}`);
  }
  if (bundle.active_version_id && !bundle.model_versions.some((version) => version.version_id === bundle.active_version_id)) {
    issues.push(`active_version_id points to missing model version ${bundle.active_version_id}`);
  }

  const expected_paths = new Set([
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
    const knownPath =
      entry.path === fileManifest.project_record_path ||
      entry.path === fileManifest.workspace_settings_path ||
      entry.path === fileManifest.workspace_snapshot_path ||
      entry.path === fileManifest.automation_presets_path ||
      entry.path.startsWith(`${fileManifest.model_directory}/`) ||
      entry.path.startsWith(`${fileManifest.version_directory}/`) ||
      entry.path.startsWith(`${fileManifest.job_directory}/`) ||
      entry.path.startsWith(`${fileManifest.result_directory}/`);
    if (!knownPath) issues.push(`asset ${entry.guid} uses unexpected path ${entry.path}`);
    else expected_paths.add(entry.path);
  }

  for (const reference of assetReferences) {
    if (!guidSet.has(reference.from_guid)) issues.push(`asset reference has unknown from_guid ${reference.from_guid}`);
    if (!guidSet.has(reference.to_guid)) issues.push(`asset reference has unknown to_guid ${reference.to_guid}`);
  }

  return {
    ok: issues.length === 0,
    issue_count: issues.length,
    issues,
    summary: projectInspectSummary(bundle),
    expected_paths: Array.from(expected_paths).sort(),
  };
}
