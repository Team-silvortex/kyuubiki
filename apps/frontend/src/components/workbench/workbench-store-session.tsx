"use client";
import { useEffect, useRef, useState } from "react";
import { requestStoreIdentity, validIdentityConfig, type StoreIdentityConfig, type StoreIdentityRequest, type StoreIdentityStatus } from "../../../../desktop-shared/src/store-identity-bridge";
import { storeSessionCopy } from "./workbench-store-session-copy";
import { storeDownloadCopy } from "./workbench-store-download-copy";

const key = "kyuubiki-store-public-login-settings-v1";
const defaults = { client_id: "", callback_port: 47831, ca_pem: "", ca_sha256: "" };
const signedOut: StoreIdentityStatus = { status: "signed_out", issuer: null, subject: null, display_name: null, expires_at: null, can_refresh: false, error: null };

export function StoreSessionPanel({ issuer, language, onDownloads }: { issuer: string; language: string; onDownloads?: () => void }) {
  const copy = storeSessionCopy(language);
  const [settings, setSettings] = useState(defaults);
  const [saved, setSaved] = useState("");
  const [status, setStatus] = useState(signedOut);
  const [desktop, setDesktop] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<"stored" | "invalid" | "failed" | "storage" | null>(null);
  const revision = useRef(0);
  const config: StoreIdentityConfig = { ...settings, issuer };
  const serialized = JSON.stringify(config);
  const valid = validIdentityConfig(config);
  const connected = ["signed_in", "expired", "refreshing"].includes(status.status);
  const pending = ["preparing", "awaiting_browser", "exchanging"].includes(status.status);

  useEffect(() => {
    const native = window.parent !== window && new URLSearchParams(window.location.search).get("desktopLayout") === "1";
    setDesktop(native);
    try {
      const raw = localStorage.getItem(key);
      if (raw) {
        const stored: unknown = JSON.parse(raw);
        if (!validIdentityConfig(stored)) throw new Error("Invalid public settings");
        setSettings({ client_id: stored.client_id, callback_port: stored.callback_port, ca_pem: stored.ca_pem, ca_sha256: stored.ca_sha256 });
        setSaved(JSON.stringify(stored));
      }
    } catch { setNotice("storage"); }
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      const current = revision.current;
      let next = 2000;
      try {
        const view = await requestStoreIdentity({ action: "status" }, window, 8000);
        if (!disposed && current === revision.current) setStatus(view);
        if (["preparing", "awaiting_browser", "exchanging", "refreshing"].includes(view.status)) next = 1000;
      } catch { /* An older shell has no identity bridge; explicit actions report the failure. */ }
      if (!disposed) timer = setTimeout(() => void poll(), next);
    }
    if (native) void poll();
    return () => { disposed = true; revision.current++; clearTimeout(timer); };
  }, []);

  function save() {
    if (!valid) { setNotice("invalid"); return; }
    try { localStorage.setItem(key, serialized); setSaved(serialized); setNotice("stored"); }
    catch { setNotice("storage"); }
  }
  async function act(request: StoreIdentityRequest) {
    const current = ++revision.current;
    setBusy(true); setNotice(null);
    if (request.action === "begin") setStatus({ ...signedOut, status: "preparing", issuer });
    try {
      const view = await requestStoreIdentity(request);
      if (current === revision.current) setStatus(view);
    } catch {
      if (current === revision.current) {
        setNotice("failed");
        try {
          const view = await requestStoreIdentity({ action: "status" }, window, 8000);
          if (current === revision.current) setStatus(view);
        } catch { /* Keep the last known state, never invent a successful login. */ }
      }
    } finally { if (current === revision.current) setBusy(false); }
  }
  return <section className="store-account-session" data-store-identity data-store-identity-status={status.status}>
    <div className="card-head"><h3>{copy.title}</h3><span className="status-pill" role="status">{copy.states[status.status]}</span></div>
    <p className="card-copy">{copy.memory}</p>
    {!desktop ? <p className="card-copy">{copy.desktop}</p> : null}
    <details className="store-identity-settings" open={!saved || undefined}>
      <summary>{copy.settings}</summary>
      <p className="card-copy">{copy.hint}</p>
      <fieldset disabled={busy || pending || connected} className="store-identity-fields">
        <label className="form-field"><span>{copy.client}</span><input className="text-input" data-store-identity-client autoComplete="off" spellCheck={false}
          maxLength={256} value={settings.client_id} onChange={event => setSettings({ ...settings, client_id: event.target.value })} /></label>
        <label className="form-field"><span>{copy.port}</span><input className="text-input" data-store-identity-port type="number" min={1024} max={65535}
          value={settings.callback_port} onChange={event => setSettings({ ...settings, callback_port: Number(event.target.value) })} /></label>
        <div className="store-account-destination"><span>{copy.callback}</span><code>{`http://127.0.0.1:${settings.callback_port}/kyuubiki/account/callback`}</code></div>
        <details><summary>{copy.trust}</summary><p className="card-copy">{copy.trustHint}</p>
          <label className="form-field"><span>{copy.ca}</span><textarea className="text-input" rows={4} maxLength={16384} spellCheck={false} autoComplete="off"
            value={settings.ca_pem} onChange={event => setSettings({ ...settings, ca_pem: event.target.value })} /></label>
          <label className="form-field"><span>{copy.fingerprint}</span><input className="text-input" maxLength={64} spellCheck={false} autoComplete="off"
            value={settings.ca_sha256} onChange={event => setSettings({ ...settings, ca_sha256: event.target.value })} /></label>
        </details>
        <button type="button" className="ghost-button ghost-button--compact" data-store-identity-save disabled={!valid || saved === serialized} onClick={save}>{copy.save}</button>
      </fieldset>
    </details>
    {status.subject ? <div className="store-account-destination"><strong>{copy.identity}: {status.display_name || status.subject}</strong>
      <code>{status.issuer} / {status.subject}</code>
      {status.expires_at ? <span>{copy.expires}: {new Date(status.expires_at * 1000).toLocaleString(language)}</span> : null}</div> : null}
    <div className="button-row">
      {connected && onDownloads ? <button type="button" className="ghost-button ghost-button--compact" data-store-identity-downloads disabled={busy || status.status !== "signed_in"}
        onClick={onDownloads}>{storeDownloadCopy(language).tab}</button> : null}
      {!connected && !pending ? <button type="button" className="ghost-button ghost-button--compact" data-store-identity-connect disabled={!desktop || busy || !valid || saved !== serialized}
        onClick={() => void act({ action: "begin", config })}>{copy.connect}</button> : null}
      {pending ? <button type="button" className="ghost-button ghost-button--compact" data-store-identity-cancel onClick={() => void act({ action: "cancel" })}>{copy.cancel}</button> : null}
      {connected ? <><button type="button" className="ghost-button ghost-button--compact" data-store-identity-refresh disabled={busy || !status.can_refresh}
        onClick={() => void act({ action: "refresh" })}>{copy.refresh}</button>
        <button type="button" className="ghost-button ghost-button--compact" data-store-identity-logout onClick={() => void act({ action: "logout" })}>{copy.logout}</button></> : null}
    </div>
    {notice ? <p role="status" className={notice === "stored" ? "card-copy" : "warning-copy"}>{copy[notice]}</p> : null}
    {status.error ? <p role="status" className="warning-copy">{copy.failed} <code>{status.error}</code></p> : null}
    <p className="card-copy">{copy.scope}</p>
  </section>;
}
