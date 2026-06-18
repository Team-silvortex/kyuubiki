import { readFile } from "node:fs/promises";
import path from "node:path";
import { listAutomationActionContracts, validateAutomationStep } from "./kyuubiki-automation-actions.mjs";
import { runHeadlessAutomationEnvelope } from "./kyuubiki-automation-runner.mjs";
import { buildHeadlessAutomationEnvelope, normalizeMacroDraft } from "./kyuubiki-macro-headless.mjs";
import { fail, printFailedAutomationStep, readOptionalJsonFile, resolveRunModeLabel, withAutomationExecutor } from "./kyuubiki-cli-runtime.mjs";
import { writeOutputFile } from "./kyuubiki-project-bundle-io.mjs";

function validateMacroDraft(macro) {
  const issues = [];
  if (!macro.id || typeof macro.id !== "string") issues.push("macro id is missing");
  if (!Array.isArray(macro.steps) || macro.steps.length === 0) issues.push("macro has no steps");
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

function buildMacroEnvelope(inputPath, macro, payload, state) {
  return buildHeadlessAutomationEnvelope(
    { kind: "macro_file", input_path: path.resolve(inputPath) },
    macro,
    { payload, state },
  );
}

export async function handleMacroActions(flags) {
  const contracts = listAutomationActionContracts();
  if (flags.json) return void console.log(JSON.stringify({ action_count: contracts.length, actions: contracts }, null, 2));
  console.log(`Automation actions: ${contracts.length}`);
  for (const contract of contracts) {
    console.log(`- ${contract.id} [${contract.risk}]`);
    console.log(`  engine: ${contract.engine}`);
    console.log(`  aliases: ${contract.aliases.join(", ") || "--"}`);
    console.log(`  summary: ${contract.summary}`);
    console.log(`  required payload: ${contract.requiredPayloadKeys.join(", ") || "--"}`);
  }
}

export async function handleMacroInspect(inputPath, flags) {
  const macro = normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  const summary = { id: macro.id, step_count: macro.steps.length, actions: macro.steps.map((step) => step.action) };
  if (flags.json) return void console.log(JSON.stringify(summary, null, 2));
  console.log(`Macro: ${summary.id}`);
  console.log(`Steps: ${summary.step_count}`);
  console.log(`Actions: ${summary.actions.join(", ")}`);
}

export async function handleMacroValidate(inputPath, flags) {
  const report = validateMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exitCode = 1;
    return;
  }
  console.log(`Macro validation: ${report.ok ? "ok" : "failed"}`);
  console.log(`Macro: ${report.summary.id}`);
  console.log(`Steps: ${report.summary.step_count}`);
  if (report.issues.length > 0) {
    for (const issue of report.issues) console.log(`- ${issue}`);
    process.exitCode = 1;
  }
}

export async function handleMacroNormalize(inputPath, flags) {
  const outputPath = typeof flags.out === "string" ? flags.out : null;
  if (!outputPath) fail("macro normalize requires --out <output>");
  await writeOutputFile(outputPath, JSON.stringify(normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8"))), null, 2));
  console.log(`normalized macro -> ${path.resolve(outputPath)}`);
}

export async function handleMacroRender(inputPath, flags) {
  const macro = normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  const envelope = buildMacroEnvelope(
    inputPath,
    macro,
    await readOptionalJsonFile(typeof flags.payload === "string" ? flags.payload : null),
    await readOptionalJsonFile(typeof flags.state === "string" ? flags.state : null),
  );
  if (flags.json) return void console.log(JSON.stringify(envelope, null, 2));
  console.log(`Macro render: ${envelope.plan.id}`);
  console.log(`Steps: ${envelope.plan.step_count}`);
  console.log(`Highest risk: ${envelope.risk_summary.highest_risk}`);
  for (const [index, step] of envelope.plan.steps.entries()) {
    console.log(`${index + 1}. ${step.action} [${step.risk}]`);
    console.log(`   payload: ${JSON.stringify(step.payload)}`);
  }
}

export async function handleMacroRun(inputPath, flags) {
  const macro = normalizeMacroDraft(JSON.parse(await readFile(path.resolve(inputPath), "utf8")));
  const envelope = buildMacroEnvelope(
    inputPath,
    macro,
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
  console.log(`Macro run: ${report.metadata.macro_id}`);
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
