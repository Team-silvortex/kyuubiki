"use client";

import type { AssetStoreEntryKind } from "@/lib/api";
import type { WorkbenchStoreBackendService } from "@/lib/workbench/store-backend-service-core";
import {
  buildWorkspaceStoreManifestExport,
  removeWorkspaceStoreEntry,
  stageWorkspaceStoreEntry,
} from "@/lib/workbench/store-command-service";

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
      const entry = await storeBackendService.fetchEntry(kind, entryId);
      if (!entry || entry.kind !== kind || entry.id !== entryId) {
        throw new Error(`Store entry response mismatch: ${kind}:${entryId}`);
      }
      const manifest = stageWorkspaceStoreEntry(selectedProjectId, entry);
      setMessage(storeMessage(language, "staged", entry.title));
      return {
        ok: true,
        action,
        projectId: manifest.project_id,
        kind,
        entryId,
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
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${key} is required.`);
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
