"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  type AssetStoreEntry,
  type AssetStoreEntryKind,
  type AssetStorePayload,
} from "@/lib/api";
import { downloadTextFile } from "@/components/workbench/workbench-file-helpers";
import {
  blankWorkspaceStoreManifest,
  manifestForSelectedProject,
  manifestEntryKey,
  readWorkspaceStoreManifestResult,
  STORE_MANIFEST_CHANGED_EVENT,
  type WorkspaceStoreManifestEntry,
} from "@/lib/workbench/store-manifest";
import {
  buildWorkspaceStoreManifestExport,
  removeWorkspaceStoreEntry,
  stageWorkspaceStoreEntry,
  WorkspaceStoreCommandError,
} from "@/lib/workbench/store-command-service";
import {
  workbenchStoreBackendService,
  type WorkbenchStoreBackendService,
} from "@/lib/workbench/store-backend-service";

type WorkbenchStoreSectionMountProps = {
  language: string;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  setMessage: (value: string) => void;
  storeBackendService?: WorkbenchStoreBackendService;
};

type StoreView = "catalog" | "project" | "sources";

const STORE_PAGE_SIZE = 5;

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
  storeBackendService = workbenchStoreBackendService,
}: WorkbenchStoreSectionMountProps) {
  const [payload, setPayload] = useState<AssetStorePayload | null>(null);
  const [kind, setKind] = useState<"" | AssetStoreEntryKind>("");
  const [query, setQuery] = useState("");
  const [view, setView] = useState<StoreView>("catalog");
  const [catalogPage, setCatalogPage] = useState(0);
  const [manifestPage, setManifestPage] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [storageError, setStorageError] = useState<string | null>(null);
  const [manifest, setManifest] = useState(() => blankWorkspaceStoreManifest(selectedProjectId));
  const refreshRequestRef = useRef(0);

  const copy = resolveStoreCopy(language);
  const entries = useMemo(() => payload?.entries ?? [], [payload?.entries]);
  const sources = payload?.sources ?? [];

  async function refreshStore() {
    const requestId = ++refreshRequestRef.current;
    setBusy(true);
    setError(null);

    try {
      const nextPayload = await storeBackendService.fetchCatalog({
        kind: kind || undefined,
        q: query.trim() || undefined,
      });
      if (requestId !== refreshRequestRef.current) return;
      setPayload(nextPayload);
      setCatalogPage(0);
      setMessage(copy.loaded(nextPayload.summary.entry_count));
    } catch (refreshError) {
      if (requestId !== refreshRequestRef.current) return;
      const message = refreshError instanceof Error ? refreshError.message : copy.failed;
      setError(message);
      setMessage(copy.failed);
    } finally {
      if (requestId === refreshRequestRef.current) setBusy(false);
    }
  }

  useEffect(() => {
    void refreshStore();
  }, [kind]);

  useEffect(() => {
    const syncManifest = () => {
      const result = readWorkspaceStoreManifestResult(selectedProjectId);
      setManifest(result.manifest);
      setStorageError(result.readable ? null : copy.storageReadFailed);
    };
    syncManifest();
    window.addEventListener(STORE_MANIFEST_CHANGED_EVENT, syncManifest);
    return () => window.removeEventListener(STORE_MANIFEST_CHANGED_EVENT, syncManifest);
  }, [language, selectedProjectId]);

  const selectedProjectLabel = selectedProjectId ?? copy.noProject;
  const activeManifest = manifestForSelectedProject(manifest, selectedProjectId);
  const installedKeys = useMemo(
    () => new Set(activeManifest.entries.map((entry) => manifestEntryKey(entry.kind, entry.id))),
    [activeManifest.entries],
  );
  const catalogPageCount = Math.max(1, Math.ceil(entries.length / STORE_PAGE_SIZE));
  const activeCatalogPage = Math.min(catalogPage, catalogPageCount - 1);
  const catalogStart = activeCatalogPage * STORE_PAGE_SIZE;
  const visibleEntries = entries.slice(catalogStart, catalogStart + STORE_PAGE_SIZE);
  const manifestPageCount = Math.max(1, Math.ceil(activeManifest.entries.length / STORE_PAGE_SIZE));
  const activeManifestPage = Math.min(manifestPage, manifestPageCount - 1);
  const manifestStart = activeManifestPage * STORE_PAGE_SIZE;
  const visibleManifestEntries = activeManifest.entries.slice(manifestStart, manifestStart + STORE_PAGE_SIZE);

  function reportStorageFailure(message: string) {
    setStorageError(message);
    setMessage(message);
  }

  function reportCommandFailure(commandError: unknown) {
    if (!(commandError instanceof WorkspaceStoreCommandError)) {
      reportStorageFailure(copy.storageWriteFailed);
      return;
    }
    const message = commandErrorMessage(commandError.code, copy);
    reportStorageFailure(message);
  }

  function installEntry(entry: AssetStoreEntry) {
    if (!selectedProjectId) {
      setMessage(copy.selectProjectFirst);
      return;
    }

    try {
      const nextManifest = stageWorkspaceStoreEntry(selectedProjectId, entry);
      setManifest(nextManifest);
      setStorageError(null);
      setMessage(copy.installed(entry));
    } catch (commandError) {
      reportCommandFailure(commandError);
    }
  }

  function removeEntry(entry: WorkspaceStoreManifestEntry) {
    if (!selectedProjectId) {
      setMessage(copy.selectProjectFirst);
      return;
    }
    try {
      const nextManifest = removeWorkspaceStoreEntry(selectedProjectId, entry.kind, entry.id);
      setManifest(nextManifest);
      setStorageError(null);
      setMessage(copy.removed(entry.title));
    } catch (commandError) {
      reportCommandFailure(commandError);
    }
  }

  function exportManifest() {
    try {
      const exported = buildWorkspaceStoreManifestExport(selectedProjectId);
      downloadTextFile(exported.filename, exported.contents);
      setStorageError(null);
      setMessage(copy.exported);
    } catch (commandError) {
      reportCommandFailure(commandError);
    }
  }

  return (
    <div
      className="sidebar-stack panel-scroll-window"
      data-workbench-store-panel="true"
      data-workbench-store-manifest-count={activeManifest.entries.length}
      data-workbench-store-model={selectedModelId ?? ""}
      data-workbench-store-status={busy ? "loading" : error ? "error" : "ready"}
      data-workbench-store-view={view}
      data-workbench-store-visible-count={view === "catalog" ? visibleEntries.length : visibleManifestEntries.length}
    >
      <section className="sidebar-card sidebar-card--compact store-context-card">
        <div className="card-head">
          <div>
            <p className="eyebrow">{copy.eyebrow}</p>
            <h2>{copy.title}</h2>
          </div>
          <span className="status-pill">{busy ? copy.loading : copy.ready}</span>
        </div>
        <div className="store-context-grid">
          <div className="sidebar-list__row">
            <span>{copy.project}</span>
            <strong>{selectedProjectLabel}</strong>
          </div>
          <div className="sidebar-list__row">
            <span>{copy.installedAssets}</span>
            <strong>{activeManifest.entries.length}</strong>
          </div>
        </div>
        {error ? <p className="warning-copy">{error}</p> : null}
        {storageError ? <p className="warning-copy">{storageError}</p> : null}
      </section>

      <div className="panel-tabs panel-tabs--editor" data-workbench-store-navigation="true">
        <button className={`panel-tab${view === "catalog" ? " panel-tab--active" : ""}`} data-workbench-store-view-tab="catalog" onClick={() => setView("catalog")} type="button">
          {copy.browse}
        </button>
        <button className={`panel-tab${view === "project" ? " panel-tab--active" : ""}`} data-workbench-store-view-tab="project" onClick={() => setView("project")} type="button">
          {copy.projectAssets} · {activeManifest.entries.length}
        </button>
        <button className={`panel-tab${view === "sources" ? " panel-tab--active" : ""}`} data-workbench-store-view-tab="sources" onClick={() => setView("sources")} type="button">
          {copy.sources} · {sources.length}
        </button>
      </div>

      {view === "catalog" ? (
        <section className="sidebar-card sidebar-card--compact">
          <div className="card-head">
            <h2>{copy.entries}</h2>
            <button className="ghost-button ghost-button--compact" disabled={busy} onClick={() => void refreshStore()} type="button">
              {copy.refresh}
            </button>
          </div>
          <label className="form-field">
            <span>{copy.search}</span>
            <input
              data-workbench-store-search="query"
              onChange={(event) => {
                setQuery(event.target.value);
                setCatalogPage(0);
              }}
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
                data-workbench-store-kind={filter.kind || "all"}
                key={filter.kind || "all"}
                onClick={() => {
                  setCatalogPage(0);
                  setKind(filter.kind);
                }}
                type="button"
              >
                {labelForLanguage(filter, language)}
              </button>
            ))}
          </div>
          <StorePager
            copy={copy}
            count={entries.length}
            page={activeCatalogPage}
            pageCount={catalogPageCount}
            pageStart={catalogStart}
            onPageChange={setCatalogPage}
          />
          <div className="history-list store-entry-list">
            {entries.length > 0 ? visibleEntries.map((entry) => (
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
      ) : null}

      {view === "project" ? <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{copy.manifestTitle}</h2>
          <button
            className="ghost-button ghost-button--compact"
            data-workbench-store-manifest-action="export"
            disabled={activeManifest.entries.length === 0}
            onClick={exportManifest}
            type="button"
          >
            {copy.exportManifest}
          </button>
        </div>
        <p className="card-copy">{copy.manifestHint}</p>
        <StorePager
          copy={copy}
          count={activeManifest.entries.length}
          page={activeManifestPage}
          pageCount={manifestPageCount}
          pageStart={manifestStart}
          onPageChange={setManifestPage}
        />
        <div className="history-list">
          {activeManifest.entries.length > 0 ? visibleManifestEntries.map((entry) => (
            <article
              className="history-item"
              data-workbench-store-manifest-entry-id={entry.id}
              data-workbench-store-manifest-entry-kind={entry.kind}
              key={manifestEntryKey(entry.kind, entry.id)}
            >
              <div>
                <strong>{entry.title}</strong>
                <small>{copy.kindLabel(entry.kind)} · {entry.source_id} · {entry.version ?? "v0"}</small>
              </div>
              <div className="button-row">
                <button
                  className="ghost-button ghost-button--compact"
                  data-workbench-store-manifest-action="remove"
                  onClick={() => removeEntry(entry)}
                  type="button"
                >
                  {copy.remove}
                </button>
              </div>
            </article>
          )) : <p className="card-copy">{copy.manifestEmpty}</p>}
        </div>
      </section> : null}

      {view === "sources" ? <section className="sidebar-card sidebar-card--compact">
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
      </section> : null}
    </div>
  );
}

function StorePager({
  copy,
  count,
  page,
  pageCount,
  pageStart,
  onPageChange,
}: {
  copy: ReturnType<typeof resolveStoreCopy>;
  count: number;
  page: number;
  pageCount: number;
  pageStart: number;
  onPageChange: (page: number) => void;
}) {
  if (count === 0) return null;
  const pageEnd = Math.min(count, pageStart + STORE_PAGE_SIZE);
  return (
    <div className="store-page-controls" data-workbench-store-page={`${page + 1}/${pageCount}`}>
      <span>{copy.pageRange(pageStart + 1, pageEnd, count)}</span>
      <div className="button-row">
        <button className="ghost-button ghost-button--compact" data-workbench-store-page-action="previous" disabled={page === 0} onClick={() => onPageChange(page - 1)} type="button">
          {copy.previous}
        </button>
        <button className="ghost-button ghost-button--compact" data-workbench-store-page-action="next" disabled={page >= pageCount - 1} onClick={() => onPageChange(page + 1)} type="button">
          {copy.next}
        </button>
      </div>
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
    <article
      className="history-item store-entry-card"
      data-workbench-store-entry-id={entry.id}
      data-workbench-store-entry-kind={entry.kind}
    >
      <div>
        <strong>{entry.title}</strong>
        <small>{copy.kindLabel(entry.kind)} · {entry.source_id} · {entry.version ?? "v0"}</small>
      </div>
      <p>{entry.summary ?? entry.id}</p>
      <div className="button-row">
        <button
          className="ghost-button ghost-button--compact"
          data-workbench-store-entry-action="stage"
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

function commandErrorMessage(
  code: WorkspaceStoreCommandError["code"],
  copy: ReturnType<typeof resolveStoreCopy>,
) {
  if (code === "project_required") return copy.selectProjectFirst;
  if (code === "manifest_unreadable") return copy.storageReadFailed;
  if (code === "invalid_asset") return copy.invalidAsset;
  if (code === "entry_missing") return copy.entryMissing;
  return copy.storageWriteFailed;
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
    projectAssets: zh ? "项目资产" : ja ? "プロジェクト資産" : "Project assets",
    sourceTitle: zh ? "来源配置" : ja ? "ソース設定" : "Source configuration",
    loading: zh ? "加载中" : ja ? "読み込み中" : "Loading",
    ready: zh ? "就绪" : ja ? "準備完了" : "Ready",
    empty: zh ? "没有匹配的资产。" : ja ? "一致する資産がありません。" : "No assets matched this search.",
    failed: zh ? "商店目录加载失败。" : ja ? "ストアカタログの読み込みに失敗しました。" : "Failed to load store catalog.",
    storageReadFailed: zh
      ? "项目商店 manifest 无法读取；为避免覆盖现有数据，本次修改已阻止。"
      : ja
        ? "プロジェクトの store manifest を読み取れないため、既存データを保護して変更を停止しました。"
        : "The project store manifest could not be read. Changes were blocked to protect existing data.",
    storageWriteFailed: zh
      ? "项目商店 manifest 写入失败，界面没有应用这次修改。"
      : ja
        ? "プロジェクトの store manifest を保存できなかったため、画面にも変更を適用しませんでした。"
        : "The project store manifest could not be saved, so the UI did not apply the change.",
    invalidAsset: zh
      ? "商店返回了无效资产条目，未加入当前项目。"
      : ja
        ? "ストアから無効な資産エントリが返されたため、プロジェクトには追加しませんでした。"
        : "The store returned an invalid asset entry, so it was not added to the project.",
    entryMissing: zh
      ? "该资产已不在当前项目 manifest 中。"
      : ja
        ? "この資産は現在のプロジェクト manifest にありません。"
        : "The asset is no longer present in the current project manifest.",
    stage: zh ? "加入当前项目" : ja ? "プロジェクトに追加" : "Add to project",
    installedBadge: zh ? "已加入" : ja ? "追加済み" : "Added",
    installedAssets: zh ? "项目资产" : ja ? "追加済み資産" : "Project assets",
    previous: zh ? "上一页" : ja ? "前へ" : "Previous",
    next: zh ? "下一页" : ja ? "次へ" : "Next",
    pageRange: (start: number, end: number, count: number) =>
      zh
        ? `${start}-${end} / ${count}`
        : ja
          ? `${start}-${end} / ${count}`
          : `${start}-${end} of ${count}`,
    manifestTitle: zh ? "项目资产 manifest" : ja ? "プロジェクト資産 manifest" : "Project asset manifest",
    manifestHint: zh
      ? "商店资产随当前项目保存，并自动写入导出的 .kyuubiki 项目包。"
      : ja
        ? "ストア資産は現在のプロジェクトに保存され、書き出した .kyuubiki バンドルにも自動的に含まれます。"
        : "Store assets are saved with the current project and included automatically in exported .kyuubiki bundles.",
    manifestEmpty: zh ? "当前项目还没有加入商店资产。" : ja ? "まだ追加済み資産はありません。" : "No store assets added to this project yet.",
    exportManifest: zh ? "导出" : ja ? "書き出し" : "Export",
    remove: zh ? "移除" : ja ? "削除" : "Remove",
    exported: zh ? "已导出项目商店 manifest。" : ja ? "プロジェクトの store manifest を書き出しました。" : "Exported the project Store manifest.",
    selectProjectFirst: zh ? "先选择或创建一个项目，再加入商店资产。" : ja ? "先にプロジェクトを選択してください。" : "Select or create a project before adding store assets.",
    loaded: (count: number) => zh ? `工作区商店已加载：${count} 个资产。` : ja ? `ストアを読み込みました: ${count} 件。` : `Workspace Store loaded ${count} assets.`,
    kindLabel: (kind: AssetStoreEntryKind) => labelForLanguage(
      KIND_FILTERS.find((filter) => filter.kind === kind) ?? KIND_FILTERS[0],
      language,
    ),
    installed: (entry: AssetStoreEntry) =>
      zh
        ? `已把 ${entry.title} 加入当前项目资产 manifest。`
        : ja
          ? `${entry.title} を現在のプロジェクト資産 manifest へ追加しました。`
          : `Added ${entry.title} to the current project asset manifest.`,
    removed: (title: string) =>
      zh
        ? `已从当前项目资产 manifest 移除 ${title}。`
        : ja
          ? `${title} をプロジェクト資産 manifest から削除しました。`
          : `Removed ${title} from the current project asset manifest.`,
  };
}
