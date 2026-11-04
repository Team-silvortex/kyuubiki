"use client";

import { useEffect, useRef, useState, type ReactNode, type RefObject } from "react";

export function useStorePageSize(expanded: boolean) {
  const [narrow, setNarrow] = useState(false);
  useEffect(() => {
    const query = window.matchMedia("(max-width: 600px)");
    const sync = () => setNarrow(query.matches);
    sync();
    query.addEventListener("change", sync);
    return () => query.removeEventListener("change", sync);
  }, []);
  return expanded && !narrow ? 12 : 4;
}

export function StoreWorkspace({ expanded, onClose, title, closeLabel, returnFocusRef, children }: {
  expanded: boolean; onClose: () => void; title: string; closeLabel: string; children: ReactNode;
  returnFocusRef: RefObject<HTMLButtonElement | null>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const wasExpanded = useRef(false);
  useEffect(() => {
    if (expanded) dialog.current?.showModal();
    // The compact entry button is remounted when the dialog closes.
    else if (wasExpanded.current) returnFocusRef.current?.focus();
    wasExpanded.current = expanded;
  }, [expanded, returnFocusRef]);

  if (!expanded) return <>{children}</>;
  return (
    <dialog ref={dialog} className="store-marketplace-dialog" aria-label={title}
      data-workbench-store-workspace="expanded" onCancel={onClose} onClose={onClose}>
      <header className="store-workspace-header">
        <strong>{title}</strong>
        <button type="button" className="ghost-button ghost-button--compact"
          data-workbench-store-workspace-action="close" onClick={onClose}>{closeLabel}</button>
      </header>
      {children}
    </dialog>
  );
}

export function storeWorkspaceCopy(language: string) {
  const zh = language.startsWith("zh");
  const ja = language.startsWith("ja");
  return {
    expand: zh ? "展开商店工作区" : ja ? "ストアを展開" : "Expand store workspace",
    close: zh ? "返回工作台" : ja ? "ワークベンチに戻る" : "Back to Workbench",
    details: zh ? "详情" : ja ? "詳細" : "Details",
    back: zh ? "返回目录" : ja ? "カタログに戻る" : "Back to catalog",
    allSources: zh ? "全部来源" : ja ? "すべてのソース" : "All sources",
    allDomains: zh ? "全部领域" : ja ? "すべての分野" : "All domains",
    filters: zh ? "来源与领域筛选" : ja ? "ソース・分野フィルター" : "Source and domain filters",
    source: zh ? "来源" : ja ? "ソース" : "Source",
    version: zh ? "版本" : ja ? "バージョン" : "Version",
    domain: zh ? "领域" : ja ? "分野" : "Domain",
    reference: zh ? "包引用" : ja ? "パッケージ参照" : "Package reference",
    target: zh ? "运行目标" : ja ? "実行先" : "Runtime target",
    unknown: zh ? "未提供" : ja ? "未提供" : "Not provided",
    conflict: zh ? "项目已引用其他来源或版本，请先移除旧引用" : ja ? "別のソースまたはバージョンが追加済みです。先に削除してください。" : "Another source or version is staged. Remove that reference first.",
    stageOnly: zh ? "加入项目只保存资产引用，不会下载、验签或安装可执行包。" : ja ? "追加は資産の参照のみ保存します。実行パッケージのダウンロード・署名検証・インストールは行いません。" : "Adding to a project saves a reference only. It does not download, verify or install an executable package.",
    builtin: zh ? "内置目录引用" : ja ? "組み込みカタログ参照" : "Built-in catalog reference",
    downloadPending: zh ? "外部包：下载与安装链路尚未接通" : ja ? "外部パッケージ：ダウンロード・インストールは未接続" : "External package: download and installation are not connected yet",
    trust: zh ? "目录展示不代表兼容性验证、签名验证或科研精度认证。" : ja ? "掲載は互換性・署名・科学的精度の検証を意味しません。" : "A catalog listing is not proof of compatibility, signature verification or scientific accuracy.",
    scope: zh ? "自部署源不需要中心账号；中心商店账号与下载授权尚待接入。" : ja ? "セルフホストのソースに中央アカウントは不要です。中央ストア認証は未接続です。" : "Self-hosted sources need no central account. Hosted-store identity and download authorization are not connected yet.",
  };
}
