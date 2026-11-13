"use client";
import { useEffect, useRef, useState } from "react";
import { requestStoreIdentity } from "../../../../desktop-shared/src/store-identity-bridge";
import { requestStoreDownload, validDownloadSelection, type OperatorVariantPlatform, type StoreDownloadSelection, type StoreDownloadView, type StoreDownloadRequest } from "../../../../desktop-shared/src/store-download-bridge";
import { operatorVariantCopy } from "./workbench-operator-variant-copy";
import { downloadError, storeDownloadCopy } from "./workbench-store-download-copy";
import { StoreHostedCatalog, storeInstallCopy } from "./workbench-store-hosted-catalog";
import { projectOperatorCopy } from "./workbench-project-operator-copy";

const kinds = ["operator", "workflow_template", "frontend_dsl_template", "language_pack"];
const platforms = ["macos-aarch64", "macos-x86_64", "linux-aarch64", "linux-x86_64", "windows-aarch64", "windows-x86_64", "any"];
const active = (state: string) => ["authorizing", "downloading", "cancelling"].includes(state);
const initial = { kind: "operator", asset_id: "", version: "", platform: "", key_id: "", key_sha256: "" };
const settingKey = "kyuubiki-store-public-download-settings-v1";

export function StoreDownloadsPanel({ language, onAccount, onProject }: { language: string; onAccount: () => void; onProject?: (platform?:OperatorVariantPlatform) => void }) {
  const copy = storeDownloadCopy(language);
  const installCopy = storeInstallCopy(language);
  const variantCopy=operatorVariantCopy(language);
  const installStatus = (state?: string | null) => state === "variant_staged" || state === "variant_staging" || state === "variant_stage_failed" ? variantCopy[state]
    : state === "installed" ? installCopy.installed : state === "installing" ? installCopy.pending : installCopy.failedInstall;
  const fields = useRef<HTMLDetailsElement>(null);
  const [selection, setSelection] = useState<StoreDownloadSelection>(initial);
  const [records, setRecords] = useState<StoreDownloadView[]>([]);
  const [account, setAccount] = useState<{ issuer: string; name: string } | null>(null);
  const [desktop, setDesktop] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busyRef = useRef(false), revision = useRef(0), observed = useRef("");
  const running = records.some(row => active(row.state));

  useEffect(() => {
    const native = window.parent !== window && new URLSearchParams(window.location.search).get("desktopLayout") === "1";
    setDesktop(native);
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      const current = revision.current;
      try {
        if (busyRef.current) return;
        const identity = await requestStoreIdentity({ action: "status" }, window, 8000);
        if (disposed || current !== revision.current) return;
        if (identity.status !== "signed_in" || !identity.issuer || !identity.subject) { setAccount(null); setRecords([]); return; }
        setAccount({ issuer: identity.issuer, name: identity.display_name || identity.subject });
        if (observed.current !== identity.issuer) {
          observed.current = identity.issuer;
          setSelection(initial);
          try {
            const saved = JSON.parse(localStorage.getItem(settingKey) || "null");
            if (saved?.issuer === identity.issuer && validDownloadSelection(saved.selection)) setSelection(saved.selection);
          } catch { /* Only optional public settings; never restore credentials or authorize on mount. */ }
        }
        const views = await requestStoreDownload({ action: "list" }, window, 8000);
        if (!disposed && current === revision.current) setRecords(views);
      } catch (reason) {
        if (!disposed && current === revision.current) setError(reason instanceof Error ? reason.message : "download_operation_failed");
      } finally { if (!disposed) timer = setTimeout(() => void poll(), 2000); }
    }
    if (native) void poll();
    return () => { disposed = true; revision.current++; clearTimeout(timer); };
  }, []);

  async function act(request: StoreDownloadRequest) {
    if (busyRef.current) return;
    busyRef.current = true; setBusy(true); setError(null);
    const current = ++revision.current;
    try {
      const views = await requestStoreDownload(request);
      if (current !== revision.current) return;
      setRecords(views);
      if (request.action === "prepare" && account) {
        try { localStorage.setItem(settingKey, JSON.stringify({ issuer: account.issuer, selection })); } catch { /* Non-essential preferences only. */ }
      }
    } catch (reason) {
      if (current === revision.current) {
        const code = reason instanceof Error ? reason.message : "download_operation_failed";
        setError(code);
        if (["login_required", "session_changed"].includes(code)) { setAccount(null); setRecords([]); }
      }
    } finally { busyRef.current = false; if (current === revision.current) setBusy(false); }
  }
  const formatSize = (bytes: number) => {
    const scale = bytes < 1024 ? 1 : bytes < 1024 * 1024 ? 1024 : 1024 * 1024;
    return `${(bytes / scale).toLocaleString(language, { maximumFractionDigits: 2 })} ${scale === 1 ? "B" : scale === 1024 ? "KiB" : "MiB"}`;
  };
  return <section className="sidebar-card sidebar-card--compact store-account store-downloads" data-store-downloads aria-busy={busy}>
    <div className="card-head"><h2>{copy.title}</h2><button type="button" className="ghost-button ghost-button--compact" onClick={onAccount}>{copy.account}</button></div>
    <p className="card-copy">{copy.intro}</p>
    {!desktop ? <p className="card-copy">{copy.desktop}</p> : !account ? <p className="warning-copy">{copy.login}</p> : null}
    {account ? <div className="store-account-destination"><strong>{copy.source}: {account.name}</strong><code>{account.issuer}</code></div> : null}
    {account ? <StoreHostedCatalog key={account.issuer} language={language} issuer={account.issuer} disabled={busy || running}
      onSelect={(item,platform)=>{setSelection(current=>({...current,kind:item.kind,asset_id:item.asset_id,version:item.latest_version,platform}));setError(null);if(fields.current){fields.current.open=true;fields.current.scrollIntoView({block:"nearest"});}}} /> : null}
    <details ref={fields} className="store-identity-settings" open={records.length === 0 || undefined}>
    <summary>{copy.selection}</summary>
    <fieldset className="store-identity-fields" disabled={!desktop || !account || busy || running}>
      <div className="store-download-fields">
        <label className="form-field"><span>{copy.kind}</span><select className="text-input" value={selection.kind} onChange={event => setSelection({ ...selection, kind: event.target.value })}>
          {kinds.map((kind, index) => <option key={kind} value={kind}>{copy.kinds[index]}</option>)}</select></label>
        <label className="form-field"><span>{copy.asset}</span><input className="text-input" data-store-download-asset maxLength={200} autoComplete="off" spellCheck={false}
          value={selection.asset_id} onChange={event => setSelection({ ...selection, asset_id: event.target.value })} /></label>
        <label className="form-field"><span>{copy.version}</span><input className="text-input" data-store-download-version maxLength={200} autoComplete="off" spellCheck={false}
          value={selection.version} onChange={event => setSelection({ ...selection, version: event.target.value })} /></label>
        <label className="form-field"><span>{copy.platform}</span><select className="text-input" data-store-download-platform value={selection.platform} onChange={event => setSelection({ ...selection, platform: event.target.value })}>
          <option value="">{copy.choose}</option>{platforms.map(platform => <option key={platform}>{platform}</option>)}</select></label>
      </div>
      <details className="store-identity-settings" open>
        <summary>{copy.trust}</summary><p className="card-copy">{copy.trustHint}</p>
        <label className="form-field"><span>{copy.key}</span><input className="text-input" data-store-download-key maxLength={200} autoComplete="off" spellCheck={false}
          value={selection.key_id} onChange={event => setSelection({ ...selection, key_id: event.target.value })} /></label>
        <label className="form-field"><span>{copy.fingerprint}</span><input className="text-input" data-store-download-pin maxLength={64} autoComplete="off" spellCheck={false}
          value={selection.key_sha256} onChange={event => setSelection({ ...selection, key_sha256: event.target.value.trim().toLowerCase() })} /></label>
      </details>
      <button type="button" className="ghost-button ghost-button--compact" data-store-download-prepare disabled={!validDownloadSelection(selection)}
        onClick={() => void act({ action: "prepare", selection })}>{copy.prepare}</button>
    </fieldset></details>
    <div className="card-head"><h3>{copy.history}</h3><button type="button" className="ghost-button ghost-button--compact" data-store-download-refresh disabled={!desktop || busy}
      onClick={() => void act({ action: "list" })}>{copy.refresh}</button></div>
    {busy ? <p className="card-copy" role="status">{copy.working}</p> : null}
    {error ? <p className="warning-copy" role="status">{downloadError(copy, error)} <code>{error}</code></p> : null}
    {!records.length ? <p className="card-copy">{copy.empty}</p> : null}
    <div className="store-download-list">{records.map(row => <article key={row.id} className="store-download-record" data-store-download-id={row.id} data-store-download-state={row.state}>
      <div className="card-head"><strong>{row.selection.asset_id} · {row.selection.version}</strong><span className="status-pill">{row.installation ? installStatus(row.installation) : copy.states[row.state]}</span></div>
      <code>{row.issuer} · {row.selection.platform}</code>
      <div className="store-workspace-context"><span>{copy.files}: {row.file_count}</span><span>{copy.size}: {formatSize(row.total_bytes)}</span>
        <span>{copy.quota}: {row.unlimited ? copy.unlimited : row.remaining ?? "?"}</span></div>
      {row.state === "prepared" ? <p className="card-copy">{copy.preview}</p> : <p className="card-copy">{row.charged === null ? copy.unknown : row.charged ? copy.charged : copy.notCharged}</p>}
      {active(row.state) ? <progress aria-label={copy.states[row.state]} max={row.total_bytes} value={row.received_bytes} /> : null}
      {row.error ? <p className="warning-copy" role="status">{downloadError(copy, row.error)} <code>{row.error}</code></p> : null}
      {row.directory ? <div className="store-account-destination"><code>{row.directory}</code><p className="card-copy">{row.installation === "variant_staged" ? variantCopy.receipt : row.installation === "installed" ? installCopy.receipt : row.installation ? installStatus(row.installation) : copy.noInstall}</p></div> : null}
      {row.installation ? <p className="card-copy" role="status" data-store-installation={row.installation}>{installStatus(row.installation)}</p> : null}
      <div className="button-row">
        {["installed","variant_staged"].includes(row.installation??"") && onProject ? <button type="button" className="ghost-button ghost-button--compact" data-store-download-project
          onClick={()=>onProject(row.installation==="variant_staged"?row.selection.platform as OperatorVariantPlatform:undefined)}>{projectOperatorCopy(language).title}</button> : null}
        {row.state === "downloaded" && row.selection.kind === "operator" && ["linux-x86_64","linux-aarch64"].includes(row.selection.platform) ?
          <button type="button" className="ghost-button ghost-button--compact" data-store-download-stage-variant disabled={busy || running || !account}
            onClick={()=>{if(window.confirm(variantCopy.confirm))void act({action:"stage_variant",id:row.id});}}>{variantCopy.stage}</button> : null}
        {row.state === "downloaded" && row.selection.kind === "operator" ? <button type="button" className="ghost-button ghost-button--compact" data-store-download-install disabled={busy || running || !account}
          onClick={()=>{if(window.confirm(installCopy.confirm))void act({action:"install",id:row.id});}}>{row.installation ? installCopy.retry : installCopy.install}</button> : null}
        {!active(row.state) && row.state !== "downloaded" ? <button type="button" className="ghost-button ghost-button--compact" data-store-download-start disabled={busy || running || !account}
          onClick={() => void act({ action: "start", id: row.id })}>{row.state === "prepared" ? row.metered ? copy.confirm : copy.free : copy.retry}</button> : null}
        {active(row.state) ? <button type="button" className="ghost-button ghost-button--compact" data-store-download-cancel disabled={busy || row.state === "cancelling"}
          onClick={() => void act({ action: "cancel", id: row.id })}>{copy.cancel}</button> : null}
        {!active(row.state) ? <button type="button" className="ghost-button ghost-button--compact" data-store-download-remove disabled={busy || running || !account}
          onClick={() => { if (window.confirm(copy.removeConfirm)) void act({ action: "remove", id: row.id }); }}>{copy.remove}</button> : null}
      </div>
    </article>)}</div>
    <p className="card-copy">{copy.recovery}</p><p className="card-copy">{copy.cap}</p>
    <p className="card-copy">{variantCopy.unsupported}</p>
  </section>;
}
