import path from "node:path";
import { buildHeadlessAutomationEnvelope } from "./kyuubiki-macro-headless.mjs";
import { runHeadlessAutomationEnvelope } from "./kyuubiki-automation-runner.mjs";
import { printFailedAutomationStep, readOptionalJsonFile, resolveRunModeLabel, withAutomationExecutor, fail } from "./kyuubiki-cli-runtime.mjs";
import { exportProjectBundleZip, finalizeProjectBundle, parseProjectBundleInput, writeOutputFile, writeProjectDirectory } from "./kyuubiki-project-bundle-io.mjs";
import { projectDiffSummary, projectInspectSummary, validateProjectBundle } from "./kyuubiki-project-bundle-analysis.mjs";

export async function handleProjectInspect(inputPath, flags) {
  const summary = projectInspectSummary(finalizeProjectBundle(await parseProjectBundleInput(inputPath)));
  if (flags.json) return void console.log(JSON.stringify(summary, null, 2));
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

export async function handleProjectValidate(inputPath, flags) {
  const report = validateProjectBundle(finalizeProjectBundle(await parseProjectBundleInput(inputPath)));
  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exitCode = 1;
    return;
  }
  console.log(`Project validation: ${report.ok ? "ok" : "failed"}`);
  console.log(`Project: ${report.summary.project_name} (${report.summary.project_id})`);
  console.log(`Issues: ${report.issue_count}`);
  if (report.issues.length > 0) {
    for (const issue of report.issues) console.log(`- ${issue}`);
    process.exitCode = 1;
  }
}

async function writeProjectNormalizationOutput(bundle, outputPath) {
  if (outputPath.endsWith(".kyuubiki")) await writeOutputFile(outputPath, await exportProjectBundleZip(bundle), true);
  else await writeOutputFile(outputPath, JSON.stringify(bundle, null, 2));
}

export async function handleProjectNormalize(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("project normalize requires --out <output>");
  await writeProjectNormalizationOutput(finalizeProjectBundle(await parseProjectBundleInput(inputPath)), outputPath);
  console.log(`normalized project bundle -> ${path.resolve(outputPath)}`);
}

export async function handleProjectUnpack(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("project unpack requires --out <directory>");
  await writeProjectDirectory(finalizeProjectBundle(await parseProjectBundleInput(inputPath)), outputPath);
  console.log(`unpacked project bundle -> ${path.resolve(outputPath)}`);
}

export async function handleProjectPack(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("project pack requires --out <bundle>");
  await writeProjectNormalizationOutput(finalizeProjectBundle(await parseProjectBundleInput(inputPath)), outputPath);
  console.log(`packed project bundle -> ${path.resolve(outputPath)}`);
}

export async function handleProjectDiff(leftInputPath, rightInputPath, flags) {
  const summary = projectDiffSummary(
    finalizeProjectBundle(await parseProjectBundleInput(leftInputPath)),
    finalizeProjectBundle(await parseProjectBundleInput(rightInputPath)),
  );
  if (flags.json) return void console.log(JSON.stringify(summary, null, 2));
  console.log(`Left:  ${summary.left.project_name} (${summary.left.project_id})`);
  console.log(`Right: ${summary.right.project_name} (${summary.right.project_id})`);
  console.log(`Schema: ${summary.left.schema} -> ${summary.right.schema}`);
  console.log(`Layout: ${summary.left.layout} -> ${summary.right.layout}`);
  console.log(`Active model changed: ${summary.active_model_changed ? "yes" : "no"}`);
  console.log(`Active version changed: ${summary.active_version_changed ? "yes" : "no"}`);
  console.log(`Project identity changed: ${summary.changed_project_identity ? "yes" : "no"}`);
  console.log("Asset kind diff:");
  for (const [kind, diff] of Object.entries(summary.asset_kind_diff)) console.log(`  ${kind}: +${diff.added.length} / -${diff.removed.length}`);
  console.log(`Automation presets: +${summary.automation_preset_ids.added.length} / -${summary.automation_preset_ids.removed.length}`);
}

export async function handleProjectAutomationPresets(inputPath, flags) {
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
  if (flags.json) return void console.log(JSON.stringify({ preset_count: presets.length, presets }, null, 2));
  console.log(`Automation presets: ${presets.length}`);
  for (const preset of presets) {
    console.log(`- ${preset.name} (${preset.preset_id})`);
    console.log(`  steps: ${preset.step_count}`);
    console.log(`  actions: ${preset.actions.join(", ") || "--"}`);
  }
}

function findAutomationPreset(bundle, presetSelector, commandName) {
  const normalized = String(presetSelector ?? "").trim();
  if (!normalized) throw new Error(`${commandName} requires --preset <id|name>`);
  const presets = bundle.automation_presets ?? [];
  return presets.find((preset) => preset.presetId === normalized) ?? presets.find((preset) => preset.name === normalized) ?? null;
}

function buildProjectAutomationEnvelope(preset, payload, state) {
  return buildHeadlessAutomationEnvelope(
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
}

export async function handleProjectAutomationRender(inputPath, flags) {
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  const preset = findAutomationPreset(bundle, flags.preset, "automation-render");
  if (!preset) throw new Error(`Could not find automation preset "${String(flags.preset ?? "")}".`);
  const envelope = buildProjectAutomationEnvelope(
    preset,
    await readOptionalJsonFile(typeof flags.payload === "string" ? flags.payload : null),
    await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null),
  );
  if (flags.json) return void console.log(JSON.stringify(envelope, null, 2));
  console.log(`Automation preset: ${envelope.source.preset_name} (${envelope.source.preset_id})`);
  console.log(`Project: ${envelope.source.project_id}`);
  console.log(`Steps: ${envelope.plan.step_count}`);
  console.log(`Highest risk: ${envelope.risk_summary.highest_risk}`);
  for (const [index, step] of envelope.plan.steps.entries()) {
    console.log(`${index + 1}. ${step.action} [${step.risk}]`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
}

export async function handleProjectAutomationRun(inputPath, flags) {
  const bundle = finalizeProjectBundle(await parseProjectBundleInput(inputPath));
  const preset = findAutomationPreset(bundle, flags.preset, "automation-run");
  if (!preset) throw new Error(`Could not find automation preset "${String(flags.preset ?? "")}".`);
  const envelope = buildProjectAutomationEnvelope(
    preset,
    await readOptionalJsonFile(typeof flags.payload === "string" ? flags.payload : null),
    await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null),
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
  if (flags.json) return void console.log(JSON.stringify(report, null, 2));
  console.log(`Automation run: ${report.metadata.macro_id}`);
  console.log(`Mode: ${resolveRunModeLabel(report)}`);
  console.log(`Status: ${report.status}`);
  console.log(`Executed steps: ${report.executed_step_count}/${report.metadata.step_count}`);
  if (report.blocked_by_confirmation) console.log(`Blocked: step ${report.blocked_by_confirmation.index + 1} requires ${report.blocked_by_confirmation.risk} confirmation`);
  printFailedAutomationStep(report);
  for (const [index, step] of report.steps.entries()) {
    console.log(`${index + 1}. ${step.action} -> ${step.status}`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
}
