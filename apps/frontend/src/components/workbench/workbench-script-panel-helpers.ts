"use client";

import type { FrontendMacroAssetRecord } from "@/components/workbench/workbench-headless-workflow-panel";
import type { WorkbenchScriptActionLogEntry } from "@/lib/scripting/workbench-script-runtime";
import { serializeWorkbenchPythonLiteral } from "@/lib/scripting/workbench-script-python-format";

function stringifyPayload(payload: Record<string, unknown> | undefined): string {
  return serializeWorkbenchPythonLiteral(payload ?? {});
}

function sanitizePythonComment(value: string | undefined): string | null {
  if (!value) return null;
  const collapsed = value.replace(/\s+/g, " ").trim();
  return collapsed ? collapsed.replaceAll("#", "") : null;
}

export function buildTimelineReplaySnippet(entry: WorkbenchScriptActionLogEntry): string {
  const failureReason =
    entry.status !== "failed"
      ? null
      : sanitizePythonComment(entry.note ?? (entry.result ? JSON.stringify(entry.result) : entry.payload ? JSON.stringify(entry.payload) : entry.summary));
  const commentLines = [
    "# Replay snippet from timeline",
    `# action: ${entry.action}`,
    `# source: ${entry.source ?? "unknown"}`,
    `# status: ${entry.status}`,
    `# at: ${entry.at}`,
    sanitizePythonComment(entry.summary) ? `# summary: ${sanitizePythonComment(entry.summary)}` : null,
    sanitizePythonComment(entry.note) ? `# note: ${sanitizePythonComment(entry.note)}` : null,
    failureReason ? `# last failure: ${failureReason}` : null,
  ].filter(Boolean);
  const actionLiteral = JSON.stringify(entry.action);
  const payloadLiteral = stringifyPayload(entry.payload);

  return `${commentLines.join("\n")}
replay_payload = ${payloadLiteral}
replay_result = await ky.invoke(${actionLiteral}, replay_payload)
ky.log("Replay result:", replay_result)
latest_state = ky.state()
ky.log("Replay message:", latest_state.get("message"))
# await ky.sleep(0.25)  # Un-comment if the next UI action needs a short settle window.
`;
}

export function buildTimelineContinuationSnippet(actionLog: WorkbenchScriptActionLogEntry[], entry: WorkbenchScriptActionLogEntry): string {
  const entryIndex = actionLog.findIndex((candidate) => candidate.id === entry.id);
  const replayEntries = (entryIndex >= 0 ? actionLog.slice(0, entryIndex + 1) : [entry]).reverse();
  const header = [
    "# Continue timeline from selected action",
    `# start action: ${entry.action}`,
    `# steps: ${replayEntries.length}`,
    "# Re-run the recorded flow in chronological order.",
  ].join("\n");
  const body = replayEntries.map((step, index) => {
    const actionLiteral = JSON.stringify(step.action);
    const payloadLiteral = stringifyPayload(step.payload);
    const summary = sanitizePythonComment(step.summary);
    const note = sanitizePythonComment(step.note);
    return [
      `# step ${index + 1}: ${step.action}`,
      summary ? `# summary: ${summary}` : null,
      note ? `# note: ${note}` : null,
      `step_${index + 1}_payload = ${payloadLiteral}`,
      `step_${index + 1}_result = await ky.invoke(${actionLiteral}, step_${index + 1}_payload)`,
      `replay_results.append(step_${index + 1}_result)`,
      `ky.log("Step ${index + 1} result:", step_${index + 1}_result)`,
      "# await ky.sleep(0.25)",
    ].filter(Boolean).join("\n");
  }).join("\n\n");

  return `${header}
replay_results = []

${body}

latest_state = ky.state()
ky.log("Continuation message:", latest_state.get("message"))
`;
}

export function downloadTextFile(filename: string, contents: string) {
  const blob = new Blob([contents], { type: "application/json;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function buildTimelinePresetName(entry: WorkbenchScriptActionLogEntry): string {
  const actionName = entry.action.replaceAll("/", " ");
  const timestamp = entry.at.replace("T", " ").slice(0, 16);
  return `${actionName} ${timestamp}`.trim();
}

export function buildFrontendMacroAssetId(source: FrontendMacroAssetRecord["source"]) {
  return `asset_${source}_${Math.random().toString(36).slice(2, 10)}`;
}

export function buildDerivedMacroDraftId(baseId: string) {
  const normalized = (baseId.startsWith("macro/") ? baseId.slice("macro/".length) : baseId)
    .replace(/-derived-[a-z0-9]+$/i, "")
    .replace(/[^a-zA-Z0-9/_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^[-/]+|[-/]+$/g, "");
  const suffix = Date.now().toString(36).slice(-6);
  return `macro/${normalized || "frontend-subflow"}-derived-${suffix}`;
}
