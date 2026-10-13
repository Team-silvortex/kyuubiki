"use client";

import { useEffect, useRef, useState } from "react";
import { parseDirectMeshEndpoints } from "@/lib/workbench/helpers";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import type { DurableDirectIndex, DurableDirectCancellation } from "@/lib/api/durable-direct-controls";
import type { DurableDirectInspection } from "@/lib/workbench/durable-direct-actions";
import type { DurableDirectRecoveryHandle } from "@/lib/workbench/durable-direct-recovery";
import { durableDirectRecoveryCopy } from "./durable-direct-recovery-copy";

type Props = {
  language: string;
  endpointsText: string;
  backendTarget: string;
  accessToken: string;
  invoke: (action: string, payload?: Record<string, unknown>) => Promise<unknown>;
};

export function DurableDirectRecoveryPanel({ language, endpointsText, backendTarget, accessToken, invoke }: Props) {
  const copy = durableDirectRecoveryCopy(language);
  const endpoints = parseDirectMeshEndpoints(endpointsText);
  const [endpoint, setEndpoint] = useState(endpoints[0] ?? "");
  const [executionId, setExecutionId] = useState("");
  const [index, setIndex] = useState<DurableDirectIndex | null>(null);
  const [handles, setHandles] = useState<DurableDirectRecoveryHandle[]>([]);
  const [receipt, setReceipt] = useState<DurableDirectInspection | null>(null);
  const [notice, setNotice] = useState("");
  const [pending, setPending] = useState(false);
  const sequence = useRef(0), busy = useRef(false);
  useEffect(() => {
    sequence.current++;
    busy.current = false;
    setPending(false); setIndex(null); setHandles([]); setReceipt(null); setNotice(""); setExecutionId("");
    setEndpoint(parseDirectMeshEndpoints(endpointsText)[0] ?? "");
    return () => { sequence.current++; };
  }, [endpointsText, backendTarget, accessToken]);

  function selectAgent(value: string) {
    sequence.current++;
    setEndpoint(value); setIndex(null); setHandles([]); setReceipt(null); setNotice(""); setExecutionId("");
  }
  function selectExecution(value: string) {
    sequence.current++;
    setExecutionId(value); setReceipt(null); setNotice("");
  }

  async function run(kind: "pending" | "list" | "inspect" | "cancel", after: string | null = null) {
    if (busy.current) return;
    busy.current = true; setPending(true); setNotice("");
    const ticket = ++sequence.current;
    let ownsAuthority = () => true;
    const selected = receipt;
    setReceipt(null);
    if (kind === "list" || kind === "pending") { setIndex(null); setHandles([]); setExecutionId(""); }
    try {
      const current = captureWorkbenchApiAuthority("/api/direct-mesh/tasks/inspect");
      ownsAuthority = () => { try { return current(); } catch { return false; } };
      const payload: Record<string, unknown> = kind === "list" ? { endpoint, after } : kind === "pending" ? { endpoint } : { endpoint, executionId };
      if (kind === "cancel") {
        if (selected?.status !== "running" || selected.execution_id !== executionId) throw new Error(copy.unchecked);
        payload.processInstanceId = selected.process_instance_id; payload.requestDigest = selected.request_digest;
      }
      const value = await invoke(`directExecution/${kind}`, payload) as Record<string, unknown>;
      if (ticket !== sequence.current || !ownsAuthority()) return;
      if (value?.ok !== true || value.endpoint !== endpoint) throw new Error("Invalid direct execution action response.");
      if (kind === "list") setIndex(value.index as DurableDirectIndex);
      if (kind === "pending") { setHandles(value.handles as DurableDirectRecoveryHandle[]); if (!(value.handles as unknown[]).length) setNotice(copy.empty); }
      if (kind === "inspect") { setReceipt(value.execution as DurableDirectInspection); if (value.journal_cleanup_pending) setNotice(copy.cleanup); }
      if (kind === "cancel") {
        const status = (value.cancellation as DurableDirectCancellation).status;
        setNotice(status === "requested" ? copy.cancelSent : status === "not_running" ? copy.noRun : copy.noOwner);
      }
    } catch (error) {
      if (ticket === sequence.current && ownsAuthority()) setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      if (ticket === sequence.current) { busy.current = false; setPending(false); }
    }
  }

  return <details className="sidebar-card sidebar-card--compact" data-durable-direct-recovery>
    <summary>{copy.title}</summary>
    <p className="card-copy">{copy.hint}</p>
    <div className="form-grid compact">
      <label><span>{copy.endpoint}</span><select value={endpoint} disabled={pending} onChange={event => selectAgent(event.target.value)}>
        {endpoints.map(value => <option key={value} value={value}>{value}</option>)}
      </select></label>
      <div className="button-row">
        <button type="button" disabled={pending || !endpoint} onClick={() => void run("pending")}>{copy.localPending}</button>
        <button type="button" disabled={pending || !endpoint} onClick={() => void run("list")}>{copy.refresh}</button>
        <button type="button" disabled={pending || !index?.next_cursor} onClick={() => void run("list", index?.next_cursor)}>{copy.next}</button>
      </div>
      {handles.length ? <label><span>{copy.localPending}</span><select value={executionId} disabled={pending} onChange={event => selectExecution(event.target.value)}>
        <option value="">{copy.choose}</option>
        {handles.map(handle => <option key={handle.execution_id} value={handle.execution_id}>{handle.study_kind} · {handle.execution_id}</option>)}
      </select></label> : null}
      {index ? index.executions.length ? <label><span>{copy.choose}</span>
        <select value={executionId} disabled={pending} onChange={event => selectExecution(event.target.value)}>
          <option value="">{copy.choose}</option>
          {index.executions.map(row => <option key={row.execution_id} value={row.execution_id}>{row.method} · {row.execution_id}</option>)}
        </select>
      </label> : <p className="card-copy">{copy.empty}</p> : null}
      <label><span>{copy.id}</span><input value={executionId} maxLength={32} disabled={pending} spellCheck={false} autoComplete="off"
        onChange={event => selectExecution(event.target.value)} /></label>
      <div className="button-row">
        <button type="button" disabled={pending || !endpoint || !/^[0-9a-f]{32}$/u.test(executionId)} onClick={() => void run("inspect")}>{copy.inspect}</button>
        <button type="button" disabled={pending || receipt?.status !== "running"} onClick={() => void run("cancel")}>{copy.cancel}</button>
      </div>
    </div>
    <p className="card-copy" role="status">{pending ? copy.pending : notice || (receipt ? copy.states[receipt.status] : copy.unchecked)}</p>
    {receipt?.has_result ? <p className="card-copy">{copy.result}</p> : null}
    {receipt?.error_code ? <p className="card-copy" style={{ overflowWrap: "anywhere" }}>{receipt.error_code}: {receipt.error_message}</p> : null}
    <p className="card-copy">{copy.safety}</p>
  </details>;
}
