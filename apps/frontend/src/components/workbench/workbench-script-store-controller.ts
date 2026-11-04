"use client";

import type { AssetStoreEntryKind } from "@/lib/api";
import type { WorkbenchStoreBackendService } from "@/lib/workbench/store-backend-service-core";
import {
  buildWorkspaceStoreManifestExport,
  removeWorkspaceStoreEntry,
  stageWorkspaceStoreEntry,
  WorkspaceStoreCommandError,
} from "@/lib/workbench/store-command-service";
import { normalizeStoreEntrySelection, resolveStoreCatalogEntry } from "@/lib/workbench/store-catalog";

const STORE_ENTRY_KINDS = ["operator", "workflow_template", "frontend_dsl_template"] as const;

type ScriptStoreControllerDeps = {
  action: string;
  payload: Record<string, unknown>;
  selectedProjectId: string | null;
  language: string;
  setMessage: (value: string) => void;
  storeBackendService: WorkbenchStoreBackendService;
  downloadTextFile: (filename: string, contents: string) => void;
};

export async function handleWorkbenchScriptStoreAction({
  action,
  payload,
  selectedProjectId,
  language,
  setMessage,
  storeBackendService,
  downloadTextFile,
}: ScriptStoreControllerDeps): Promise<Record<string, unknown> | null> {
  switch (action) {
    case "store/stageEntry": {
      const kind = requiredStoreKind(payload);
      const entryId = requiredText(payload, "entryId");
      if (!selectedProjectId?.trim()) {
        throw new WorkspaceStoreCommandError("project_required", "A selected project is required.");
      }
      const selection = normalizeStoreEntrySelection(Object.fromEntries(
        Object.entries(payload).filter(([key]) => key !== "kind" && key !== "entryId"),
      ));
      const entry = await storeBackendService.fetchEntry(kind, entryId, selection);
      if (!entry || entry.kind !== kind || entry.id !== entryId) {
        throw new Error(`Store entry response mismatch: ${kind}:${entryId}`);
      }
      resolveStoreCatalogEntry([entry], kind, entryId, selection);
      const manifest = stageWorkspaceStoreEntry(selectedProjectId, entry);
      const staged = manifest.entries.find((item) => item.kind === kind && item.id === entryId)!;
      setMessage(storeMessage(language, "staged", entry.title));
      return {
        ok: true,
        action,
        projectId: manifest.project_id,
        kind,
        entryId,
        sourceId: staged.source_id,
        version: staged.version ?? null,
        packageRef: staged.package_ref ?? null,
        target: staged.target ?? null,
        manifestEntryCount: manifest.entries.length,
      };
    }
    case "store/removeEntry": {
      const kind = requiredStoreKind(payload);
      const entryId = requiredText(payload, "entryId");
      const manifest = removeWorkspaceStoreEntry(selectedProjectId, kind, entryId);
      setMessage(storeMessage(language, "removed", entryId));
      return {
        ok: true,
        action,
        projectId: manifest.project_id,
        kind,
        entryId,
        manifestEntryCount: manifest.entries.length,
      };
    }
    case "store/exportManifest": {
      const exported = buildWorkspaceStoreManifestExport(selectedProjectId);
      downloadTextFile(exported.filename, exported.contents);
      setMessage(storeMessage(language, "exported", exported.filename));
      return {
        ok: true,
        action,
        projectId: exported.manifest.project_id,
        filename: exported.filename,
        schemaVersion: exported.manifest.schema_version,
        manifestEntryCount: exported.manifest.entries.length,
      };
    }
    default:
      return null;
  }
}

function requiredStoreKind(payload: Record<string, unknown>): AssetStoreEntryKind {
  const kind = payload.kind;
  if (typeof kind !== "string" || !(STORE_ENTRY_KINDS as readonly string[]).includes(kind)) {
    throw new Error(`Invalid kind: ${String(kind)}`);
  }
  return kind as AssetStoreEntryKind;
}

function requiredText(payload: Record<string, unknown>, key: string) {
  const value = payload[key];
  if (typeof value !== "string" || !value.trim() || value.trim().length > 256) {
    throw new Error(`${key} must be nonempty text of at most 256 characters.`);
  }
  return value.trim();
}

function storeMessage(language: string, action: "staged" | "removed" | "exported", value: string) {
  if (language === "zh") {
    if (action === "staged") return `已把 ${value} 加入当前项目 manifest。`;
    if (action === "removed") return `已从当前项目 manifest 移除 ${value}。`;
    return `已导出项目商店 manifest：${value}`;
  }
  if (language === "ja") {
    if (action === "staged") return `${value} を現在のプロジェクト manifest に追加しました。`;
    if (action === "removed") return `${value} を現在のプロジェクト manifest から削除しました。`;
    return `プロジェクト store manifest を書き出しました: ${value}`;
  }
  if (action === "staged") return `Added ${value} to the current project manifest.`;
  if (action === "removed") return `Removed ${value} from the current project manifest.`;
  return `Exported project store manifest: ${value}`;
}
