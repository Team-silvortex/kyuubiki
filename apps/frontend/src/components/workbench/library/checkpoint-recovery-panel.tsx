"use client";

import { useEffect, useRef, useState } from "react";
import type { CheckpointIntent } from "@/lib/workbench/checkpoint-journal";
import type { CheckpointReceipt } from "@/lib/api/project-types";
import { checkpointRecoveryCopy } from "./checkpoint-recovery-copy";

export type CheckpointRecoveryInvoker = (action: string, payload?: Record<string, unknown>) => Promise<Record<string, unknown>>;

export function CheckpointRecoveryPanel({ language, invoke }: { language: string; invoke: CheckpointRecoveryInvoker }) {
  const copy = checkpointRecoveryCopy(language);
  const latest = useRef(invoke);
  latest.current = invoke;
  const [entries, setEntries] = useState<CheckpointIntent[]>([]);
  const [key, setKey] = useState("");
  const [receipt, setReceipt] = useState<CheckpointReceipt | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const active = entries.find((entry) => entry.key === key) ?? entries[0];
  useEffect(() => { setReceipt(null); }, [active?.key]);

  useEffect(() => {
    let live = true;
    let reads = 0;
    const refresh = async () => {
      const token = ++reads;
      try {
        const result = await latest.current("model/listPendingSaves");
        if (live && token === reads) { setEntries(result.pending as CheckpointIntent[]); setError(""); }
      } catch (failure) {
        if (live && token === reads) { setEntries([]); setReceipt(null); setError(String(failure)); }
      }
    };
    void refresh();
    window.addEventListener("focus", refresh);
    window.addEventListener("kyuubiki-checkpoint-journal", refresh);
    return () => {
      live = false;
      generation.current += 1;
      window.removeEventListener("focus", refresh);
      window.removeEventListener("kyuubiki-checkpoint-journal", refresh);
    };
  }, []);

  async function run(action: string) {
    if (!active || busy) return;
    const token = ++generation.current;
    setBusy(true);
    setError("");
    try {
      const result = await latest.current(action, { key: active.key });
      if (token !== generation.current) return;
      setReceipt(result.checkpoint as CheckpointReceipt);
      if (action === "model/acknowledgeSave") {
        setEntries((values) => values.filter((entry) => entry.key !== active.key));
        setReceipt(null);
      }
    } catch (failure) {
      if (token === generation.current) { setReceipt(null); setError(String(failure)); }
    } finally { if (token === generation.current) setBusy(false); }
  }

  if (!entries.length && !error) return null;
  return <details className="sidebar-card" data-checkpoint-recovery-panel>
    <summary>{copy.title} ({entries.length}/64)</summary>
    <p className="muted">{copy.hint}</p>
    {active && <>
      <select aria-label={copy.title} disabled={busy} value={active.key} onChange={(event) => {
        generation.current += 1; setKey(event.target.value); setReceipt(null); setError("");
      }} style={{ width: "100%", minWidth: 0 }}>
        {entries.map((entry, index) => <option key={entry.key} value={entry.key}>
          {index + 1}. {new Date(entry.createdAt).toISOString().slice(0, 19).replace("T", " ")} UTC · {entry.parentId.slice(0, 16)}
        </option>)}
      </select>
      <p role="status" data-checkpoint-recovery-status={receipt?.status ?? "unchecked"}>
        {receipt ? copy[receipt.status] : copy.check}
      </p>
      <div className="button-row" style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
        <button type="button" disabled={busy} onClick={() => void run("model/checkPendingSave")}>{copy.check}</button>
        <button type="button" disabled={busy || receipt?.status !== "committed"} onClick={() => void run("model/openRecoveredSave")}>{copy.open}</button>
        <button type="button" disabled={busy || !receipt || receipt.status === "unknown"} onClick={() => void run("model/acknowledgeSave")}>{copy.acknowledge}</button>
      </div>
    </>}
    {error && <p role="alert" style={{ overflowWrap: "anywhere" }}>{error}</p>}
  </details>;
}
