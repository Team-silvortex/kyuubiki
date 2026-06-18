import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { createHybridAutomationExecutor } from "./kyuubiki-hybrid-executor.mjs";

export function fail(message) {
  console.error(message);
  process.exit(1);
}

export function parseFlags(args) {
  const flags = {};
  for (let index = 0; index < args.length; index += 1) {
    const token = args[index];
    if (!token.startsWith("--")) continue;
    const key = token.slice(2);
    const next = args[index + 1];
    if (!next || next.startsWith("--")) flags[key] = true;
    else {
      flags[key] = next;
      index += 1;
    }
  }
  return flags;
}

export function positionalArgs(args) {
  const result = [];
  for (let index = 0; index < args.length; index += 1) {
    const token = args[index];
    if (token.startsWith("--")) {
      const next = args[index + 1];
      if (next && !next.startsWith("--")) index += 1;
      continue;
    }
    result.push(token);
  }
  return result;
}

export function resolveRunModeLabel(report) {
  return report.dry_run ? "dry-run" : "live-execute";
}

export function printFailedAutomationStep(report) {
  if (!report.failed_step) return;
  console.log(`Failed: step ${report.failed_step.index + 1} ${report.failed_step.action}`);
  console.log(`Reason: ${report.failed_step.message}`);
  if (report.failed_step.error_code === "PLAYWRIGHT_RESTRICTED") {
    console.log("Hint: run this browser-backed automation in a desktop session that can launch a local browser.");
  }
}

export async function readOptionalJsonFile(inputPath) {
  if (!inputPath) return {};
  return JSON.parse(await readFile(path.resolve(inputPath), "utf8"));
}

export async function withAutomationExecutor(flags, callback) {
  if (!flags.execute) return callback({ executor: null, artifactsDir: null });
  const runtime = await createHybridAutomationExecutor({
    artifactsDir: typeof flags["artifacts-dir"] === "string" ? flags["artifacts-dir"] : undefined,
    apiBaseUrl: typeof flags["api-base-url"] === "string" ? flags["api-base-url"] : undefined,
  });
  try {
    return await callback({
      executor: runtime.executor,
      artifactsDir: runtime.getArtifactsDir?.() ?? runtime.artifactsDir ?? null,
      apiBaseUrl: runtime.getApiBaseUrl?.() ?? null,
    });
  } finally {
    await runtime.dispose();
  }
}

async function collectArtifactManifestEntries(rootDir, currentDir) {
  const entries = await readdir(currentDir, { withFileTypes: true });
  const manifest = [];
  for (const entry of entries) {
    const absolutePath = path.join(currentDir, entry.name);
    if (entry.isDirectory()) {
      manifest.push(...await collectArtifactManifestEntries(rootDir, absolutePath));
      continue;
    }
    const details = await stat(absolutePath);
    manifest.push({
      path: absolutePath,
      relative_path: path.relative(rootDir, absolutePath) || entry.name,
      size_bytes: details.size,
      modified_at: details.mtime.toISOString(),
    });
  }
  manifest.sort((left, right) => left.relative_path.localeCompare(right.relative_path));
  return manifest;
}

export async function collectArtifactManifest(artifactsDir) {
  if (!artifactsDir) return [];
  try {
    const details = await stat(artifactsDir);
    if (!details.isDirectory()) return [];
  } catch {
    return [];
  }
  return collectArtifactManifestEntries(artifactsDir, artifactsDir);
}
