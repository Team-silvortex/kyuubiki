"use client";

import { useEffect, useRef, useState } from "react";
import {
  AssistantConnectionError, assistantTransport, assistantTrustedOrigins, filterAssistantModels,
  listAssistantModels, setAssistantEndpointTrusted, validateAssistantEndpoint,
  type AssistantConnectionCode,
} from "@/lib/assistant/model-connection";
import { modelConnectionCopy, modelConnectionError } from "./workbench-model-connection-copy";

type Props = {
  language: string; baseUrl: string; apiKey: string; model: string;
  onBaseUrl: (value: string) => void; onApiKey: (value: string) => void;
  onModel: (value: string) => void; onReady: () => void;
};

export function WorkbenchModelConnection({ language, baseUrl, apiKey, model, onBaseUrl, onApiKey, onModel, onReady }: Props) {
  const t = modelConnectionCopy(language), endpoint = validateAssistantEndpoint(baseUrl);
  const [models, setModels] = useState<string[]>([]);
  const [notice, setNotice] = useState<"ready" | "empty" | AssistantConnectionCode | null>(null);
  const message = notice === "ready" || notice === "empty" ? t[notice]
    : notice ? modelConnectionError(new AssistantConnectionError(notice), language) : "";
  const [editingConnection, setEditingConnection] = useState(true);
  const [pending, setPending] = useState(false), [showKey, setShowKey] = useState(false);
  const [trusted, setTrusted] = useState(false), [expanded, setExpanded] = useState(true);
  const [transport, setTransport] = useState<"browser" | "native" | null>(null);
  const [status, setStatus] = useState<"idle" | "connected" | "failed">("idle");
  const [picker, setPicker] = useState<"catalog" | "manual">("manual");
  const [query, setQuery] = useState("");
  const request = useRef<AbortController | null>(null);
  const current = useRef({ baseUrl, apiKey }); current.current = { baseUrl, apiKey };
  const filtered = filterAssistantModels(models, query);
  const permitted = endpoint.ok && (!endpoint.requiresTrust || trusted);

  useEffect(() => {
    request.current?.abort(); request.current = null;
    setModels([]); setNotice(null); setPending(false); setShowKey(false); setEditingConnection(true);
    setTransport(null); setStatus("idle"); setPicker("manual"); setQuery("");
    const validation = validateAssistantEndpoint(baseUrl);
    setTrusted(Boolean(validation.origin && assistantTrustedOrigins().has(validation.origin)));
    return () => { request.current?.abort(); request.current = null; };
  }, [baseUrl, apiKey]);

  const cancel = () => {
    request.current?.abort(); request.current = null;
    setPending(false); setModels([]); setPicker("manual"); setStatus("idle"); setTransport(null);
  };
  const check = async () => {
    request.current?.abort(); const controller = new AbortController(); request.current = controller;
    // A previous list is not evidence for a new connection attempt.
    setPending(true); setNotice(null); setModels([]); setPicker("manual"); setStatus("idle"); setQuery(""); setTransport(null);
    const active = () => request.current === controller && !controller.signal.aborted
      && current.current.baseUrl === baseUrl && current.current.apiKey === apiKey;
    try {
      const route = await assistantTransport();
      if (!active()) return;
      setTransport(route);
      const found = await listAssistantModels({ baseUrl, apiKey, trustedOrigins:assistantTrustedOrigins(), signal:controller.signal, transport:route });
      if (active()) {
        setModels(found); setStatus("connected"); setPicker(found.length ? "catalog" : "manual");
        setNotice(found.length ? "ready" : "empty"); setEditingConnection(false); setShowKey(false);
      }
    } catch (error) {
      if (active()) { setStatus("failed"); setNotice(error instanceof AssistantConnectionError ? error.code : "invalid_response"); }
    } finally {
      if (request.current === controller) { setPending(false); request.current = null; }
    }
  };

  return <section className="sidebar-card assistant-model-connection" data-testid="assistant-model-connection">
    <details open={expanded} onToggle={event => setExpanded(event.currentTarget.open)}>
      <summary className="assistant-model-connection__summary">
        <span><strong>{t.title}</strong><small>{expanded ? t.protocol : model || t.notSelected}</small></span>
        <span className="assistant-model-connection__status" data-status={pending ? "pending" : status}>{pending ? t.loading : t[status]}</span>
      </summary>
      <div className="assistant-model-connection__grid">
        <fieldset className="assistant-model-connection__section">
          <legend><b>01</b> {t.connectionSection}</legend>
          {status === "connected" || !editingConnection ? <button type="button" className="ghost-button ghost-button--compact" aria-expanded={editingConnection} onClick={() => { setEditingConnection(value => !value); setShowKey(false); }}>{editingConnection ? t.hideConnection : t.editConnection}</button> : null}
          {!editingConnection ? <p className="card-copy assistant-model-connection__origin">{endpoint.origin}</p> : null}
          <div className="assistant-model-connection__fields" hidden={!editingConnection}>
          <div className="assistant-model-connection__presets">
            <button type="button" className="ghost-button ghost-button--compact" onClick={() => onBaseUrl("https://api.openai.com/v1")}>OpenAI</button>
            <button type="button" className="ghost-button ghost-button--compact" onClick={() => onBaseUrl("http://127.0.0.1:1234/v1")}>{t.presetLocal}</button>
          </div>
          <label><span>{t.base}</span><input type="url" value={baseUrl} onChange={event => onBaseUrl(event.target.value)} autoComplete="off" spellCheck={false} placeholder="https://your-provider.example/v1" /></label>
          {endpoint.ok ? endpoint.normalized !== baseUrl.trim() && <button type="button" className="ghost-button ghost-button--compact" onClick={() => onBaseUrl(endpoint.normalized!)}>{t.normalize}</button>
            : <p className="card-copy" role="status">{modelConnectionError(new AssistantConnectionError("endpoint"), language)}</p>}
          <label><span>{t.key}</span><input type={showKey ? "text" : "password"} value={apiKey} onChange={event => onApiKey(event.target.value)} autoComplete="off" spellCheck={false} maxLength={8192} /></label>
          <div className="assistant-model-connection__key-actions">
            <small>{t.sessionOnly}</small>
            <button type="button" className="ghost-button ghost-button--compact" aria-pressed={showKey} onClick={() => setShowKey(value => !value)}>{showKey ? t.hide : t.show}</button>
            <button type="button" className="ghost-button ghost-button--compact" disabled={!apiKey} onClick={() => onApiKey("")}>{t.clear}</button>
          </div>
          {endpoint.requiresTrust ? <label className="assistant-model-connection__trust"><input type="checkbox" checked={trusted} onChange={event => {
            setTrusted(event.target.checked); setAssistantEndpointTrusted(baseUrl, event.target.checked);
            if (!event.target.checked) { cancel(); setNotice(null); setShowKey(false); }
          }} /><span>{t.trust} <strong>{endpoint.origin}</strong></span></label> : null}
          <div className="assistant-model-connection__test">
            <button type="button" className="ghost-button" disabled={pending || !permitted} onClick={() => void check()}>{pending ? t.loading : t.test}</button>
            {pending ? <button type="button" className="ghost-button" onClick={() => { cancel(); setNotice("cancelled"); }}>{t.cancel}</button> : null}
          </div>
          <p className="card-copy">{t.scopeShort}</p>
          </div>
          {message ? <p className="assistant-model-connection__message" data-status={status} role="status" aria-live="polite">{message}</p> : null}
        </fieldset>

        <fieldset className="assistant-model-connection__section">
          <legend><b>02</b> {t.modelSection}</legend>
          {models.length > 0 ? <div className="panel-tabs assistant-model-connection__picker" role="group" aria-label={t.modelSection}>
            <button type="button" className={`panel-tab${picker === "catalog" ? " panel-tab--active" : ""}`} aria-pressed={picker === "catalog"} onClick={() => setPicker("catalog")}>{t.catalogMode}</button>
            <button type="button" className={`panel-tab${picker === "manual" ? " panel-tab--active" : ""}`} aria-pressed={picker === "manual"} onClick={() => setPicker("manual")}>{t.manualMode}</button>
          </div> : null}
          {models.length > 0 && picker === "catalog" ? <>
            <label><span>{t.search}</span><input type="search" value={query} onChange={event => setQuery(event.target.value)} autoComplete="off" spellCheck={false} placeholder={t.searchHint} /></label>
            <small role="status">{t.count.replace("{shown}", String(filtered.items.length)).replace("{matched}", String(filtered.matched))}</small>
            {filtered.items.length ? <label><span>{t.models}</span><select aria-label={t.models} size={Math.min(5, filtered.items.length + 1)} value={filtered.items.includes(model) ? model : ""} onChange={event => { if (event.target.value) onModel(event.target.value); }}>
              <option value="" disabled>{t.select}</option>{filtered.items.map(id => <option key={id} value={id}>{id}</option>)}
            </select></label> : <p className="card-copy">{t.noMatches}</p>}
          </> : <label><span>{t.model}</span><input value={model} onChange={event => onModel(event.target.value)} autoComplete="off" maxLength={256} spellCheck={false} placeholder={t.select} /></label>}
          <p className="card-copy">{t.modelHint}</p>
        </fieldset>
      </div>
      <details className="assistant-model-connection__advanced"><summary>{t.help}</summary>
        {endpoint.ok ? <code className="assistant-model-connection__url">{endpoint.normalized}/chat/completions</code> : null}
        {transport ? <p className="card-copy">{transport === "native" ? t.native : t.browser}</p> : null}
        <p className="card-copy">{t.hint}</p><p className="card-copy">{t.scope}</p>
      </details>
      <footer className="assistant-model-connection__footer">
        <div><small>{t.current}</small><strong data-testid="assistant-current-model">{model || t.notSelected}</strong></div>
        <button className="ghost-button" type="button" disabled={pending || !model.trim() || !permitted} onClick={() => { setShowKey(false); setExpanded(false); requestAnimationFrame(onReady); }}>{t.useModel}</button>
      </footer>
    </details>
  </section>;
}
