"use client";

import { useEffect, useRef, useState, type MouseEvent } from "react";
import { requestStoreAccountPortal, storeAccountPortalUrl } from "../../../../desktop-shared/src/store-account-bridge";
import { storeAccountCopy } from "./workbench-store-account-copy";
import { StoreSessionPanel } from "./workbench-store-session";

const preferenceKey = "kyuubiki-store-account-origin";
type Notice = "saved" | "storage" | "next" | "failed" | null;

export function StoreAccountPanel({ language, onBack, onDownloads }: { language: string; onBack: () => void; onDownloads?: () => void }) {
  const copy = storeAccountCopy(language);
  const [origin, setOrigin] = useState("");
  const [savedOrigin, setSavedOrigin] = useState("");
  const [notice, setNotice] = useState<Notice>(null);
  const [opening, setOpening] = useState(false);
  const [hydrated, setHydrated] = useState(false);
  const request = useRef(0);
  let portal = "";
  try { portal = storeAccountPortalUrl(origin, language); } catch { /* Incomplete input is not a destination. */ }
  const normalizedOrigin = portal ? new URL(portal).origin : "";
  const ready = !!portal && normalizedOrigin === savedOrigin;

  useEffect(() => {
    try {
      const stored = localStorage.getItem(preferenceKey);
      if (stored) {
        const normalized = new URL(storeAccountPortalUrl(stored, "en")).origin;
        setOrigin(normalized);
        setSavedOrigin(normalized);
      }
    } catch { setNotice("storage"); }
    setHydrated(true);
    return () => { request.current++; };
  }, []);

  function save() {
    if (!portal) return;
    try {
      localStorage.setItem(preferenceKey, normalizedOrigin);
      setOrigin(normalizedOrigin);
      setSavedOrigin(normalizedOrigin);
      setNotice("saved");
    } catch { setNotice("storage"); }
  }

  function clear() {
    request.current++;
    setOpening(false);
    setOrigin("");
    setSavedOrigin("");
    try { localStorage.removeItem(preferenceKey); setNotice(null); }
    catch { setNotice("storage"); }
  }

  async function open(event: MouseEvent<HTMLAnchorElement>) {
    if (!ready || opening) { event.preventDefault(); return; }
    const native = window.parent !== window && new URLSearchParams(window.location.search).get("desktopLayout") === "1";
    if (!native) { setNotice("next"); return; }
    event.preventDefault();
    const current = ++request.current;
    setOpening(true);
    setNotice(null);
    try {
      await requestStoreAccountPortal(savedOrigin, language);
      if (current === request.current) setNotice("next");
    } catch { if (current === request.current) setNotice("failed"); }
    finally { if (current === request.current) setOpening(false); }
  }

  return <section className="sidebar-card sidebar-card--compact store-account" data-store-account aria-busy={!hydrated}>
    <div className="card-head"><h2>{copy.title}</h2><span className="status-pill">{copy.badge}</span></div>
    <p className="card-copy">{copy.intro}</p>
    <label className="form-field">
      <span>{copy.origin}</span>
      <input className="text-input" data-store-account-origin type="url" autoComplete="off" spellCheck={false} maxLength={2048}
        placeholder={copy.placeholder} value={origin} aria-invalid={!!origin && !portal}
        aria-describedby="store-account-origin-hint" disabled={opening || !hydrated}
        onChange={event => { request.current++; setOrigin(event.target.value); setNotice(null); }} />
    </label>
    <p id="store-account-origin-hint" className={origin && !portal ? "warning-copy" : "card-copy"}>{origin && !portal ? copy.invalid : copy.originHint}</p>
    <div className="button-row">
      <button type="button" className="ghost-button ghost-button--compact" data-store-account-save disabled={!portal || ready || opening} onClick={save}>{copy.save}</button>
      <button type="button" className="ghost-button ghost-button--compact" data-store-account-clear disabled={!origin && !savedOrigin && !notice} onClick={clear}>{copy.clear}</button>
    </div>
    {ready ? <div className="store-account-destination">
      <span className="card-copy">{copy.destination}</span>
      <code data-store-account-destination>{portal}</code>
      <a className="ghost-button ghost-button--compact" data-store-account-open href={portal} target="_blank"
        rel="noopener noreferrer" referrerPolicy="no-referrer" aria-disabled={opening} onClick={event => void open(event)}>
        {opening ? copy.opening : copy.open}
      </a>
    </div> : null}
    {notice ? <p role="status" className={notice === "storage" || notice === "failed" ? "warning-copy" : "card-copy"}>{copy[notice]}</p> : null}
    <ol className="store-account-steps">{copy.steps.map(([title, hint]) => <li key={title}><strong>{title}</strong><p className="card-copy">{hint}</p></li>)}</ol>
    <StoreSessionPanel issuer={savedOrigin} language={language} onDownloads={onDownloads} />
    <p className="card-copy">{copy.certificate}</p>
    <button type="button" className="ghost-button ghost-button--compact" data-store-account-back onClick={onBack}>{copy.back}</button>
  </section>;
}
