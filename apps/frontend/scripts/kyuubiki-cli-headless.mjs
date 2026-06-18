import path from "node:path";
import { collectArtifactManifest, printFailedAutomationStep, readOptionalJsonFile, resolveRunModeLabel, withAutomationExecutor, fail } from "./kyuubiki-cli-runtime.mjs";
import { loadHeadlessInputDocument, normalizeHeadlessExecutionBatch, runHeadlessExecutionBatch, summarizeHeadlessExecutionBatch, validateHeadlessExecutionBatch } from "./kyuubiki-headless-batch.mjs";
import { buildHeadlessTemplateDocument, listHeadlessTemplates, resolveHeadlessTemplateSelection } from "./kyuubiki-headless-templates.mjs";
import { writeOutputFile } from "./kyuubiki-project-bundle-io.mjs";

export async function handleHeadlessTemplates(flags) {
  const runtimeStyle = typeof flags.runtime === "string"
    ? flags.runtime
    : typeof flags["runtime-style"] === "string" ? flags["runtime-style"] : null;
  const query = typeof flags.query === "string" ? flags.query : typeof flags.search === "string" ? flags.search : null;
  const templates = listHeadlessTemplates({ runtimeStyle, query });
  if (flags.json) return void console.log(JSON.stringify({ template_count: templates.length, templates }, null, 2));
  console.log(`Headless templates: ${templates.length}`);
  for (const template of templates) {
    console.log(`- ${template.id} (${template.step_count} steps)`);
    console.log(`  ${template.title}`);
    console.log(`  ${template.description}`);
    console.log(`  runtime: ${template.runtime_style}`);
    console.log(`  category: ${template.category}`);
    console.log(`  tags: ${template.tags.join(", ")}`);
    console.log(`  actions: ${template.actions.join(", ")}`);
    if (template.matched_fields.length > 0) console.log(`  matched: ${template.matched_fields.join(", ")}`);
  }
}

export async function handleHeadlessInit(flags) {
  const runtimeStyle = typeof flags.runtime === "string"
    ? flags.runtime
    : typeof flags["runtime-style"] === "string" ? flags["runtime-style"] : null;
  const query = typeof flags.query === "string" ? flags.query : typeof flags.search === "string" ? flags.search : null;
  const templateId = typeof flags.template === "string" ? flags.template : null;
  const template = resolveHeadlessTemplateSelection({ templateId, runtimeStyle, query });
  if (!template) {
    const available = listHeadlessTemplates({ runtimeStyle, query }).map((entry) => entry.id).join(", ");
    if (!templateId && (runtimeStyle || query)) {
      throw new Error(
        available
          ? `Template filters match multiple templates. Choose one with --template. Available templates: ${available}`
          : `No headless templates found for the current filters.`,
      );
    }
    if (!templateId) fail("headless init requires --template <id> or a unique filter set such as --runtime-style <style> or --query <text>");
    throw new Error(`Unknown headless template "${templateId}". Available templates: ${available || "none"}`);
  }
  const document = buildHeadlessTemplateDocument(template, {
    workflowId: typeof flags["workflow-id"] === "string" ? flags["workflow-id"] : undefined,
  });
  if (flags.json && typeof flags.out !== "string") {
    console.log(JSON.stringify(document, null, 2));
    return;
  }
  const outputPath = typeof flags.out === "string" ? flags.out : `${template.id}.headless-workflow.json`;
  await writeOutputFile(outputPath, JSON.stringify(document, null, 2));
  console.log(`initialized headless workflow -> ${path.resolve(outputPath)}`);
}

export async function handleHeadlessInspect(inputPath, flags) {
  const batch = normalizeHeadlessExecutionBatch(await loadHeadlessInputDocument(inputPath));
  const summary = summarizeHeadlessExecutionBatch(batch);
  if (flags.json) return void console.log(JSON.stringify(summary, null, 2));
  console.log(`Headless workflow: ${summary.workflow_id}`);
  console.log(`Schema: ${summary.schema_version}`);
  console.log(`Language: ${summary.language}`);
  console.log(`Steps: ${summary.step_count}`);
  console.log(`Warnings: ${summary.warning_count}`);
  console.log(`Actions: ${summary.actions.join(", ") || "--"}`);
}

export async function handleHeadlessRender(inputPath, flags) {
  const batch = normalizeHeadlessExecutionBatch(await loadHeadlessInputDocument(inputPath));
  if (flags.json) {
    console.log(JSON.stringify(batch, null, 2));
    return;
  }
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) {
    console.log(`Headless execution batch: ${batch.workflow_id}`);
    console.log(`Steps: ${batch.steps.length}`);
    console.log(`Warnings: ${batch.warnings.length}`);
    return;
  }
  await writeOutputFile(outputPath, JSON.stringify(batch, null, 2));
  console.log(`rendered headless execution batch -> ${outputPath}`);
}

export async function handleHeadlessValidate(inputPath, flags) {
  const batch = normalizeHeadlessExecutionBatch(await loadHeadlessInputDocument(inputPath));
  const report = validateHeadlessExecutionBatch(batch);
  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exitCode = 1;
    return;
  }
  console.log(`Headless validation: ${report.ok ? "ok" : "failed"}`);
  if (report.summary) {
    console.log(`Workflow: ${report.summary.workflow_id}`);
    console.log(`Schema: ${report.summary.schema_version}`);
    console.log(`Steps: ${report.summary.step_count}`);
  }
  if (report.policy) {
    console.log(`Runtime: ${report.policy.recommended_runtime}`);
    console.log(`Engines: ${report.policy.required_engines.join(", ") || "--"}`);
    console.log(
      `Risks: normal ${report.policy.risk_counts.normal}, sensitive ${report.policy.risk_counts.sensitive}, destructive ${report.policy.risk_counts.destructive}`,
    );
  }
  console.log(`Warnings: ${report.warning_count}`);
  console.log(`Issues: ${report.issue_count}`);
  for (const note of report.policy?.notes ?? []) console.log(`note: ${note}`);
  for (const warning of report.warnings) console.log(`warning: ${warning}`);
  for (const issue of report.issues) console.log(`- ${issue}`);
  if (!report.ok) process.exitCode = 1;
}

export async function handleHeadlessRun(inputPath, flags) {
  const batch = normalizeHeadlessExecutionBatch(await loadHeadlessInputDocument(inputPath));
  const contextInput = await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null);
  const execution = await withAutomationExecutor(flags, async ({ executor, artifactsDir, apiBaseUrl }) => ({
    report: await runHeadlessExecutionBatch(batch, {
      dryRun: !flags.execute,
      allowSensitive: flags["allow-sensitive"],
      allowDestructive: flags["allow-destructive"],
      executor,
      context: {
        ...contextInput,
        ...(artifactsDir ? { artifactsDir } : {}),
      },
    }),
    artifactsDir,
    apiBaseUrl,
    artifactManifest: await collectArtifactManifest(artifactsDir),
  }));
  const output = {
    report: execution.report,
    execution_context: {
      artifacts_dir: execution.artifactsDir ?? null,
      artifact_count: execution.artifactManifest.length,
      api_base_url: execution.apiBaseUrl ?? null,
      artifact_manifest: execution.artifactManifest,
    },
  };
  if (typeof flags["report-out"] === "string") {
    await writeOutputFile(flags["report-out"], JSON.stringify(output, null, 2));
  }
  if (flags.json) return void console.log(JSON.stringify(output, null, 2));
  const report = execution.report;
  console.log(`Headless run: ${report.workflow_id}`);
  console.log(`Mode: ${resolveRunModeLabel(report)}`);
  console.log(`Status: ${report.status}`);
  console.log(`Executed steps: ${report.executed_step_count}/${report.steps.length}`);
  if (report.warning_count > 0) console.log(`Warnings: ${report.warning_count}`);
  if (execution.apiBaseUrl) console.log(`API base: ${execution.apiBaseUrl}`);
  if (execution.artifactsDir) console.log(`Artifacts dir: ${execution.artifactsDir}`);
  console.log(`Artifacts: ${execution.artifactManifest.length}`);
  if (typeof flags["report-out"] === "string") console.log(`Report: ${path.resolve(flags["report-out"])}`);
  if (report.blocked_by_confirmation) {
    console.log(`Blocked: step ${report.blocked_by_confirmation.index} requires ${report.blocked_by_confirmation.risk} confirmation`);
  }
  printFailedAutomationStep(report);
  for (const step of report.steps) {
    console.log(`${step.index}. ${step.action} -> ${step.status}`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
  if (execution.artifactManifest.length > 0) {
    console.log("Artifact files:");
    for (const artifact of execution.artifactManifest) {
      console.log(`- ${artifact.relative_path} (${artifact.size_bytes} B)`);
    }
  }
}

export function assertHeadlessInputPath(inputPath) {
  if (!inputPath) fail("headless command requires an input path");
}
