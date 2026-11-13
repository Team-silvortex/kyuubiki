"use client";
import { useEffect, useRef, useState } from "react";
import { requestStoreCatalog, type StoreCatalogItem, type StoreCatalogPage } from "../../../../desktop-shared/src/store-download-bridge";
import { operatorVariantCopy } from "./workbench-operator-variant-copy";

export function storeInstallCopy(language: string) {
  return language.startsWith("zh") ? {
    catalog:"账号商店目录", search:"搜索资源", load:"查询目录", next:"下一页", first:"回到第一页", choose:"选择此版本",
    hint:"目录 → 预检确切版本 → 确认授权下载 → 原生安装。目录查询不消耗下载额度。", empty:"此页暂无资源。",
    changed:"目录版本已变化，请从第一页重新查询。", failed:"目录查询失败，请重试；没有发起下载授权。",
    install:"安装到本机算子库", retry:"核验或重试安装", confirm:"将此确切版本安装到本机管理的算子库？不会自动启用、执行或分发给 Agent，也不会覆盖其他已安装版本。安装不再扣下载额度。",
    installed:"安装回执已确认；未自动启用", pending:"安装结果待核对", failedInstall:"安装未确认，请核验或重试",
    receipt:"原生 Installer 已确认这份文件的安装回执。这不是实时运行状态，也不代表已在项目或 Agent 中启用。移除下载缓存不会卸载算子。",
    unsupported:"当前仅支持本机平台的最小 Rust 算子包安装；模板、语言包和其他平台文件保留为已验证下载。",
  } : language.startsWith("ja") ? {
    catalog:"アカウントのストア一覧", search:"アセットを検索", load:"一覧を取得", next:"次のページ", first:"最初のページ", choose:"このバージョンを選択",
    hint:"一覧 → 正確なリリースを確認 → 認可して取得 → ネイティブインストール。一覧取得は枠を消費しません。", empty:"このページにアセットはありません。",
    changed:"一覧が更新されました。最初のページから再取得してください。", failed:"一覧の取得に失敗しました。ダウンロード認可は行っていません。",
    install:"本機のオペレーター庫にインストール", retry:"インストールを確認・再試行", confirm:"このバージョンを本機の管理対象ライブラリにインストールしますか？自動有効化、実行、Agent 配布、既存バージョンの上書きは行いません。枠は再消費しません。",
    installed:"インストール記録を確認・未有効化", pending:"インストール結果の確認が必要", failedInstall:"インストール未確認・確認または再試行",
    receipt:"ネイティブ Installer がこのファイルのインストール記録を確認しました。実行状態ではなく、プロジェクトや Agent では未有効化です。ダウンロード削除はアンインストールではありません。",
    unsupported:"本機向けの最小 Rust オペレーターパッケージのみインストールできます。テンプレート、言語パック、別プラットフォームのファイルは検証済みダウンロードとして保持します。",
  } : {
    catalog:"Account store catalog", search:"Search assets", load:"Browse catalog", next:"Next page", first:"First page", choose:"Select this version",
    hint:"Catalog → exact release preview → authorize download → native install. Browsing does not use download credits.", empty:"No assets on this page.",
    changed:"The catalog changed. Restart from the first page.", failed:"Catalog lookup failed. No download was authorized; retry the lookup.",
    install:"Install into this device's operator library", retry:"Verify or retry installation", confirm:"Install this exact release into the managed library on this device? This does not activate, execute, distribute to Agents, or replace an existing version. Installation does not use another download credit.",
    installed:"Installation receipt confirmed; not activated", pending:"Installation outcome needs checking", failedInstall:"Installation unconfirmed; verify or retry",
    receipt:"Native Installer confirmed installation of these exact files. This is a receipt, not live runtime status or project/Agent activation. Removing the download cache does not uninstall the operator.",
    unsupported:"Only minimal Rust operator packages for this device can be installed here. Templates, language packs and other-platform files remain verified downloads.",
  };
}

export function StoreHostedCatalog({language, issuer, disabled, onSelect}: {
  language:string; issuer:string; disabled:boolean; onSelect:(item:StoreCatalogItem, platform:string)=>void;
}) {
  const copy = storeInstallCopy(language);
  const [query,setQuery] = useState("");
  const [platform,setPlatform]=useState("");
  const variantCopy=operatorVariantCopy(language);
  const [page,setPage] = useState<StoreCatalogPage|null>(null);
  const [busy,setBusy] = useState(false), [error,setError] = useState("");
  const revision = useRef(0), pending = useRef(false);
  useEffect(()=>()=>{revision.current++;},[]);
  async function load(cursor: string|null) {
    if(pending.current || disabled) return;
    pending.current=true;setBusy(true);setError("");const current=++revision.current;
    try {
      const next=await requestStoreCatalog({action:"catalog",query,cursor});
      if(current!==revision.current) return;
      if(next.issuer!==issuer) throw new Error("store_identity_mismatch");
      if(cursor && (next.catalog_revision!==page?.catalog_revision || next.next_cursor===cursor)) {
        setPage(null);setError(copy.changed);return;
      }
      setPage(next);
    } catch {
      if(current===revision.current) {setPage(null);setError(copy.failed);}
    } finally {pending.current=false;if(current===revision.current)setBusy(false);}
  }
  return <section className="store-hosted-catalog" data-store-hosted-catalog aria-busy={busy}>
    <h3>{copy.catalog}</h3><p className="card-copy">{copy.hint}</p>
    <form className="store-catalog-search" onSubmit={event=>{event.preventDefault();void load(null);}}>
      <label className="form-field"><span>{copy.search}</span><input className="text-input" value={query} maxLength={200} disabled={busy || disabled}
        onChange={event=>{setQuery(event.target.value);setPage(null);}} /></label>
      <button className="ghost-button ghost-button--compact" data-store-catalog-load disabled={busy || disabled}>{copy.load}</button>
    </form>
    {error ? <p className="warning-copy" role="status">{error}</p> : null}
    <label className="form-field"><span>{variantCopy.platform}</span><select className="text-input" data-store-catalog-platform value={platform} disabled={busy||disabled} onChange={e=>setPlatform(e.target.value)}>
      <option value="">{variantCopy.local}</option><option value="linux-x86_64">Linux x86_64</option><option value="linux-aarch64">Linux arm64</option></select></label>
    {page ? <><ul className="store-hosted-catalog-list">{page.items.map(item=><li key={`${item.kind}/${item.asset_id}`}>
      <div><strong>{item.title}</strong><code>{item.kind} · {item.asset_id} · {item.latest_version}</code><p className="card-copy">{item.summary}</p></div>
      <button className="ghost-button ghost-button--compact" data-store-catalog-select disabled={busy || disabled}
        onClick={()=>onSelect(item, item.kind==="operator" ? platform || page.host_platform : "any")}>{copy.choose}</button>
    </li>)}</ul>
    {!page.items.length ? <p className="card-copy">{copy.empty}</p> : null}
    <div className="button-row"><button className="ghost-button ghost-button--compact" disabled={busy || disabled} onClick={()=>void load(null)}>{copy.first}</button>
      <button className="ghost-button ghost-button--compact" disabled={busy || disabled || !page.next_cursor} onClick={()=>void load(page.next_cursor)}>{copy.next}</button></div></> : null}
  </section>;
}
