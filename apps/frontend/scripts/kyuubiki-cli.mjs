#!/usr/bin/env node

import { fail, parseFlags, positionalArgs } from "./kyuubiki-cli-runtime.mjs";
import { assertHeadlessInputPath, handleHeadlessInit, handleHeadlessInspect, handleHeadlessRender, handleHeadlessRun, handleHeadlessTemplates, handleHeadlessValidate } from "./kyuubiki-cli-headless.mjs";
import {
  handleProjectAutomationPresets,
  handleProjectAutomationRender,
  handleProjectAutomationRun,
  handleProjectDiff,
  handleProjectInspect,
  handleProjectNormalize,
  handleProjectPack,
  handleProjectUnpack,
  handleProjectValidate,
} from "./kyuubiki-cli-project.mjs";
import {
  handleMacroActions,
  handleMacroInspect,
  handleMacroNormalize,
  handleMacroRender,
  handleMacroRun,
  handleMacroValidate,
} from "./kyuubiki-cli-macro.mjs";

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
  kyuubiki project automation-run <input> --preset <id|name> [--payload payload.json] [--state state.json] [--json] [--execute] [--allow-sensitive] [--allow-destructive] [--artifacts-dir dir] [--api-base-url url]
  kyuubiki headless templates [--runtime service_only|browser_only|hybrid] [--query text] [--json]
  kyuubiki headless init [--template <id>] [--runtime-style service_only|browser_only|hybrid] [--query text] [--workflow-id workflow.id] [--out output.json] [--json]
  kyuubiki headless inspect <input> [--json]
  kyuubiki headless validate <input> [--json]
  kyuubiki headless render <input> [--json] [--out output.json]
  kyuubiki headless run <input> [--state context.json] [--json] [--report-out report.json] [--execute] [--allow-sensitive] [--allow-destructive] [--artifacts-dir dir] [--api-base-url url]
  kyuubiki macro inspect <macro.json> [--json]
  kyuubiki macro actions [--json]
  kyuubiki macro validate <input> [--json]
  kyuubiki macro normalize <input> --out <output>
  kyuubiki macro render <input> [--payload payload.json] [--state state.json] [--json]
  kyuubiki macro run <input> [--payload payload.json] [--state state.json] [--json] [--execute] [--allow-sensitive] [--allow-destructive] [--artifacts-dir dir] [--api-base-url url]
`);
}

async function handleProjectScope(command, args, flags) {
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

async function handleMacroScope(command, args, flags) {
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

async function handleHeadlessScope(command, args, flags) {
  if (command === "templates") return handleHeadlessTemplates(flags);
  if (command === "init") return handleHeadlessInit(flags);
  const [, , inputPath] = positionalArgs(args);
  assertHeadlessInputPath(inputPath);
  if (command === "inspect") return handleHeadlessInspect(inputPath, flags);
  if (command === "validate") return handleHeadlessValidate(inputPath, flags);
  if (command === "render") return handleHeadlessRender(inputPath, flags);
  if (command === "run") return handleHeadlessRun(inputPath, flags);
  fail(`unknown headless command: ${command}`);
}

async function main() {
  const args = process.argv.slice(2);
  const [scope = "help", command = ""] = positionalArgs(args);
  const flags = parseFlags(args);

  if (scope === "help" || scope === "--help" || scope === "-h") {
    usage();
    return;
  }
  if (scope === "project") return handleProjectScope(command, args, flags);
  if (scope === "headless") return handleHeadlessScope(command, args, flags);
  if (scope === "macro") return handleMacroScope(command, args, flags);
  fail(`unknown command: ${scope}`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
