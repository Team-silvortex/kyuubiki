"use client";

import { useEffect, useMemo, useState } from "react";
import {
  fetchAssetStore,
  type AssetStoreEntry,
  type AssetStoreEntryKind,
  type AssetStorePayload,
} from "@/lib/api";
import { downloadTextFile } from "@/components/workbench/workbench-file-helpers";
import {
  addManifestEntry,
  manifestEntryKey,
  persistWorkspaceStoreManifest,
  readWorkspaceStoreManifest,
  removeManifestEntry,
  type WorkspaceStoreManifestEntry,
} from "@/lib/workbench/store-manifest";

type WorkbenchStoreSectionMountProps = {
  language: string;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  setMessage: (value: string) => void;
};

const KIND_FILTERS: Array<{ kind: "" | AssetStoreEntryKind; en: string; zh: string; ja: string }> = [
  { kind: "", en: "All", zh: "全部", ja: "すべて" },
  { kind: "operator", en: "Operators", zh: "算子", ja: "オペレーター" },
  { kind: "workflow_template", en: "Workflow templates", zh: "工作流模板", ja: "ワークフローテンプレート" },
  { kind: "frontend_dsl_template", en: "Frontend DSL", zh: "前端 DSL", ja: "フロント DSL" },
];

export function WorkbenchStoreSectionMount({
  language,
  selectedProjectId,
  selectedModelId,
  setMessage,
}: WorkbenchStoreSectionMountProps) {
  const [payload, setPayload] = useState<AssetStorePayload | null>(null);
  const [kind, setKind] = useState<"" | AssetStoreEntryKind>("");
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [manifest, setManifest] = useState(() => readWorkspaceStoreManifest(selectedProjectId));

  const copy = resolveStoreCopy(language);
  const entries = useMemo(() => payload?.entries ?? [], [payload?.entries]);
  const sources = payload?.sources ?? [];

  async function refreshStore() {
    setBusy(true);
    setError(null);

    try {
      const nextPayload = await fetchAssetStore({
        kind: kind || undefined,
        q: query.trim() || undefined,
      });
      setPayload(nextPayload);
      setMessage(copy.loaded(nextPayload.summary.entry_count));
    } catch (refreshError) {
      const message = refreshError instanceof Error ? refreshError.message : copy.failed;
      setError(message);
      setMessage(copy.failed);
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void refreshStore();
  }, [kind]);

  useEffect(() => {
    setManifest(readWorkspaceStoreManifest(selectedProjectId));
  }, [selectedProjectId]);

  const selectedProjectLabel = selectedProjectId ?? copy.noProject;
  const selectedModelLabel = selectedModelId ?? copy.noModel;
  const installedKeys = useMemo(
    () => new Set(manifest.entries.map((entry) => manifestEntryKey(entry.kind, entry.id))),
    [manifest.entries],
  );

  function installEntry(entry: AssetStoreEntry) {
    if (!selectedProjectId) {
      setMessage(copy.selectProjectFirst);
      return;
    }

    setManifest((current) => {
      const nextManifest = addManifestEntry(current, entry);
      persistWorkspaceStoreManifest(nextManifest);
      return nextManifest;
    });
    setMessage(copy.installed(entry));
  }

  function removeEntry(entry: WorkspaceStoreManifestEntry) {
    setManifest((current) => {
      const nextManifest = removeManifestEntry(current, entry);
      persistWorkspaceStoreManifest(nextManifest);
      return nextManifest;
    });
    setMessage(copy.removed(entry.title));
  }

  function exportManifest() {
    const filename = `${selectedProjectId ?? "kyuubiki-workspace"}.store-manifest.json`;
    downloadTextFile(filename, `${JSON.stringify(manifest, null, 2)}\n`);
    setMessage(copy.exported);
  }

  return (
    <div className="sidebar-stack panel-scroll-window" data-workbench-store-panel="true">
      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <div>
            <p className="eyebrow">{copy.eyebrow}</p>
            <h2>{copy.title}</h2>
          </div>
          <span className="status-pill">{busy ? copy.loading : copy.ready}</span>
        </div>
        <p className="card-copy">{copy.hint}</p>
        <div className="sidebar-list">
          <div className="sidebar-list__row">
            <span>{copy.project}</span>
            <strong>{selectedProjectLabel}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.model}</span>
            <strong>{selectedModelLabel}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.sources}</span>
            <strong>{sources.filter((source) => source.enabled).length}/{sources.length}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.installedAssets}</span>
            <strong>{manifest.entries.length}</strong>
          </div>
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{copy.browse}</h2>
          <button className="ghost-button ghost-button--compact" disabled={busy} onClick={() => void refreshStore()} type="button">
            {copy.refresh}
          </button>
        </div>
        <label className="form-field">
          <span>{copy.search}</span>
          <input
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") void refreshStore();
            }}
            placeholder={copy.searchPlaceholder}
            value={query}
          />
        </label>
        <div className="panel-tabs panel-tabs--wrap" role="tablist">
          {KIND_FILTERS.map((filter) => (
            <button
              className={kind === filter.kind ? "active" : ""}
              key={filter.kind || "all"}
              onClick={() => setKind(filter.kind)}
              type="button"
            >
              {labelForLanguage(filter, language)}
            </button>
          ))}
        </div>
        {error ? <p className="warning-copy">{error}</p> : null}
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{copy.entries}</h2>
          <span className="status-pill">{entries.length}</span>
        </div>
        <div className="history-list">
          {entries.length > 0 ? entries.map((entry) => (
            <StoreEntryCard
              copy={copy}
              entry={entry}
              installed={installedKeys.has(manifestEntryKey(entry.kind, entry.id))}
              key={`${entry.kind}:${entry.id}`}
              onInstall={installEntry}
            />
          )) : <p className="card-copy">{busy ? copy.loading : copy.empty}</p>}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{copy.manifestTitle}</h2>
          <button
            className="ghost-button ghost-button--compact"
            disabled={manifest.entries.length === 0}
            onClick={exportManifest}
            type="button"
          >
            {copy.exportManifest}
          </button>
        </div>
        <p className="card-copy">{copy.manifestHint}</p>
        <div className="history-list">
          {manifest.entries.length > 0 ? manifest.entries.map((entry) => (
            <article className="history-item" key={manifestEntryKey(entry.kind, entry.id)}>
              <div>
                <strong>{entry.title}</strong>
                <small>{copy.kindLabel(entry.kind)} · {entry.source_id} · {entry.version ?? "v0"}</small>
              </div>
              <div className="button-row">
                <button className="ghost-button ghost-button--compact" onClick={() => removeEntry(entry)} type="button">
                  {copy.remove}
                </button>
              </div>
            </article>
          )) : <p className="card-copy">{copy.manifestEmpty}</p>}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{copy.sourceTitle}</h2>
          <span className="status-pill">{sources.length}</span>
        </div>
        <div className="sidebar-list">
          {sources.map((source) => (
            <div className="sidebar-list__row" key={source.id}>
              <span>{source.label}</span>
              <strong>{source.status}</strong>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function StoreEntryCard({
  copy,
  entry,
  installed,
  onInstall,
}: {
  copy: ReturnType<typeof resolveStoreCopy>;
  entry: AssetStoreEntry;
  installed: boolean;
  onInstall: (entry: AssetStoreEntry) => void;
}) {
  return (
    <article className="history-item">
      <div>
        <strong>{entry.title}</strong>
        <small>{copy.kindLabel(entry.kind)} · {entry.source_id} · {entry.version ?? "v0"}</small>
      </div>
      <p>{entry.summary ?? entry.id}</p>
      <div className="button-row">
        <button
          className="ghost-button ghost-button--compact"
          disabled={installed}
          onClick={() => onInstall(entry)}
          type="button"
        >
          {installed ? copy.installedBadge : copy.stage}
        </button>
      </div>
    </article>
  );
}

function labelForLanguage(
  labels: { en: string; zh: string; ja: string },
  language: string,
) {
  if (language === "zh") return labels.zh;
  if (language === "ja") return labels.ja;
  return labels.en;
}

function resolveStoreCopy(language: string) {
  const zh = language === "zh";
  const ja = language === "ja";

  return {
    eyebrow: zh ? "项目级资产" : ja ? "プロジェクト資産" : "Project assets",
    title: zh ? "工作区商店" : ja ? "ワークスペースストア" : "Workspace Store",
    hint: zh
      ? "像 Unity Editor 一样，在当前 Workbench 项目里挑选算子、工作流模板和前端 DSL 模板。"
      : ja
        ? "Unity Editor のように、現在の Workbench プロジェクトへ資産を追加します。"
        : "Install operators, workflow templates, and frontend DSL templates into the current Workbench project.",
    project: zh ? "当前项目" : ja ? "現在のプロジェクト" : "Current project",
    model: zh ? "当前模型" : ja ? "現在のモデル" : "Current model",
    noProject: zh ? "未选择项目" : ja ? "未選択" : "No project selected",
    noModel: zh ? "未选择模型" : ja ? "未選択" : "No model selected",
    sources: zh ? "可用源" : ja ? "有効なソース" : "Enabled sources",
    browse: zh ? "浏览资产" : ja ? "資産を探す" : "Browse assets",
    refresh: zh ? "刷新" : ja ? "更新" : "Refresh",
    search: zh ? "搜索" : ja ? "検索" : "Search",
    searchPlaceholder: zh ? "按名称、ID、领域或标签搜索" : ja ? "名前、ID、タグで検索" : "Search by name, id, domain, or tags",
    entries: zh ? "资产条目" : ja ? "資産エントリ" : "Store entries",
    sourceTitle: zh ? "来源配置" : ja ? "ソース設定" : "Source configuration",
    loading: zh ? "加载中" : ja ? "読み込み中" : "Loading",
    ready: zh ? "就绪" : ja ? "準備完了" : "Ready",
    empty: zh ? "没有匹配的资产。" : ja ? "一致する資産がありません。" : "No assets matched this search.",
    failed: zh ? "商店目录加载失败。" : ja ? "ストアカタログの読み込みに失敗しました。" : "Failed to load store catalog.",
    stage: zh ? "加入当前项目" : ja ? "プロジェクトに追加" : "Add to project",
    installedBadge: zh ? "已加入" : ja ? "追加済み" : "Added",
    installedAssets: zh ? "项目资产" : ja ? "追加済み資産" : "Project assets",
    manifestTitle: zh ? "项目 manifest 草稿" : ja ? "manifest 草稿" : "Project manifest draft",
    manifestHint: zh
      ? "先把商店资产写入当前项目的前端 manifest 草稿，后续会迁移到 project bundle/lock。"
      : ja
        ? "まず現在のプロジェクト manifest 草稿に保存し、後で bundle/lock に移します。"
        : "Store assets are staged in a project manifest draft before backend bundle/lock persistence lands.",
    manifestEmpty: zh ? "当前项目还没有加入商店资产。" : ja ? "まだ追加済み資産はありません。" : "No store assets added to this project yet.",
    exportManifest: zh ? "导出" : ja ? "書き出し" : "Export",
    remove: zh ? "移除" : ja ? "削除" : "Remove",
    exported: zh ? "已导出项目商店 manifest 草稿。" : ja ? "manifest 草稿を書き出しました。" : "Exported workspace store manifest draft.",
    selectProjectFirst: zh ? "先选择或创建一个项目，再加入商店资产。" : ja ? "先にプロジェクトを選択してください。" : "Select or create a project before adding store assets.",
    loaded: (count: number) => zh ? `工作区商店已加载：${count} 个资产。` : ja ? `ストアを読み込みました: ${count} 件。` : `Workspace Store loaded ${count} assets.`,
    kindLabel: (kind: AssetStoreEntryKind) => labelForLanguage(
      KIND_FILTERS.find((filter) => filter.kind === kind) ?? KIND_FILTERS[0],
      language,
    ),
    installed: (entry: AssetStoreEntry) =>
      zh
        ? `已把 ${entry.title} 加入当前项目 manifest 草稿。`
        : ja
          ? `${entry.title} を現在の manifest 草稿へ追加しました。`
          : `Added ${entry.title} to the current project manifest draft.`,
    removed: (title: string) =>
      zh
        ? `已从当前项目 manifest 草稿移除 ${title}。`
        : ja
          ? `${title} を manifest 草稿から削除しました。`
          : `Removed ${title} from the current project manifest draft.`,
  };
}
