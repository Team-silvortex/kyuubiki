"use client";
import { useEffect, useRef, useState } from "react";
import { requestProjectOperator, type ProjectOperatorOperation, type ProjectOperatorStatus } from "../../../../desktop-shared/src/store-download-bridge";
import { workbenchDurableDirectRunService as runs } from "@/lib/workbench/durable-direct-run";
import { DurableDirectSubmissionUncertain, type DurableDirectRecoveryHandle } from "@/lib/workbench/durable-direct-recovery";
import type { DurableDirectEnvelope } from "@/lib/api/durable-direct-client";
import { captureWorkbenchApiAuthority, readFrontendRuntimeModeFromWindow } from "@/lib/api/auth-context";
import { downloadTextFile } from "./workbench-file-helpers";
import { projectOperatorCopy, projectDeploymentCopy } from "./workbench-project-operator-copy";
import { directExecutionGatewayIdentity } from "@/lib/workbench/durable-direct-run";
import { checkpointDigest } from "@/lib/workbench/checkpoint-journal";
import { createProjectOperatorDeployment, type OperatorDeploymentStatus } from "@/lib/workbench/project-operator-deployment";

export function ProjectOperatorsPanel({ language, projectId }: { language: string; projectId: string | null }) {
  const c = projectOperatorCopy(language);
  const d = projectDeploymentCopy(language);
  const [remote,setRemote]=useState<OperatorDeploymentStatus|null>(null);
  const [status, setStatus] = useState<ProjectOperatorStatus | null>(null), [installed, setInstalled] = useState(""), [enabled, setEnabled] = useState("");
  const [operator, setOperator] = useState(""), [input, setInput] = useState("{}"), [config, setConfig] = useState("{}"), [endpoint, setEndpoint] = useState("");
  const [notice, setNotice] = useState(""), [pending, setPending] = useState<DurableDirectRecoveryHandle[]>([]);
  const [handle, setHandle] = useState<DurableDirectRecoveryHandle | null>(null), [receipt, setReceipt] = useState<DurableDirectEnvelope | null>(null);
  const [busy, setBusy] = useState(false), [desktop, setDesktop] = useState(false);
  const revision = useRef(0), active = useRef(false);
  useEffect(() => { setDesktop(window.parent !== window && new URLSearchParams(window.location.search).get("desktopLayout") === "1"); return () => { revision.current++; }; }, []);
  const selected = status?.enabled.find(e => e.package.package_id === enabled);
  const chosen = status?.installed.find(p => p.package_id === installed);
  function invalidateTask() { setStatus(s => s ? { ...s, task: null } : s); }
  async function act(operation: ProjectOperatorOperation | "run" | "pending" | "inspect" | "ack" | "inspect-package" | "deploy-package" | "abort-package") {
    if (active.current || !projectId) return;
    active.current = true; setBusy(true); setNotice("");
    const ticket = ++revision.current;
    let current = () => ticket === revision.current;
    try {
      const authority = captureWorkbenchApiAuthority("/api/direct-mesh/tasks", "/api/direct-mesh/tasks/inspect", "/api/direct-mesh/tasks/cancel", "/api/direct-mesh/operator-package");
      current = () => ticket === revision.current && authority();
      const scope = await checkpointDigest(directExecutionGatewayIdentity());
      if (!current()) throw new Error(c.stale);
      if (operation === "pending") { const p = await runs.pending(); if (current()) setPending(p.filter(h => h.study_kind === "operator_task")); return; }
      if (operation === "inspect" && handle) { const r = await runs.inspect(handle); if (current()) setReceipt(r); return; }
      if (operation === "ack" && handle && receipt) { await runs.acknowledge(handle, receipt, current); if (current()) { setHandle(null); setReceipt(null); setPending(p => p.filter(h => h.execution_id !== handle.execution_id)); } return; }
      if(operation==="run" || operation==="inspect-package" || operation==="deploy-package" || operation==="abort-package") {
        if(readFrontendRuntimeModeFromWindow()!=="direct_mesh_gui")throw new Error(c.route);
        if(!selected)throw new Error(c.changed);
        const service=createProjectOperatorDeployment({read:async offset=>{
          const result=await requestProjectOperator(projectId,scope,{action:"read_deployment",package:selected.package,offset});
          if(!result.deployment)throw new Error("deployment_source_unavailable");return result.deployment;
        }});
        const progress=(s:OperatorDeploymentStatus)=>{if(current())setRemote(s);};
        const result=operation==="deploy-package" ? await service.deploy(selected.package,endpoint,current,progress)
          : operation==="abort-package" ? await service.abort(selected.package,endpoint,current)
          : await service.inspect(selected.package,endpoint,current);
        progress(result);
        if(operation!=="run")return;
        if(result.state!=="ready")throw new Error(d[result.state]);
      }
      let op: ProjectOperatorOperation;
      if (operation === "run") {
        if (readFrontendRuntimeModeFromWindow() !== "direct_mesh_gui") throw new Error(c.route);
        if (!selected || selected.state !== "ready") throw new Error(c.changed);
        op = { action: "prepare", package: selected.package, operator_id: operator, task_id: crypto.randomUUID(), input: JSON.parse(input), config: JSON.parse(config) };
      } else if (typeof operation === "object") op = operation;
      else return;
      const value = await requestProjectOperator(projectId, scope, op);
      if (!current()) throw new Error(c.stale);
      setStatus(value);
      if (operation === "run") {
        if (!value.task) throw new Error(c.changed);
        const submitted = await runs.submit("operator_task", { mode: "execute", task_ir: value.task }, endpoint, current);
        if (current()) { setHandle(submitted.handle); setReceipt(submitted.envelope); }
      } else if (op.action === "prepare") setNotice(c.prepared);
    } catch (e) {
      if (current()) {
        if (e instanceof DurableDirectSubmissionUncertain) { setHandle(e.handle); setReceipt(null); }
        setNotice(`${c.error} ${e instanceof Error ? e.message : "project_operator_failed"}`);
      }
    } finally { active.current = false; if (ticket === revision.current) setBusy(false); }
  }
  return <section className="sidebar-card sidebar-card--compact store-account" data-project-operators aria-busy={busy}>
    <h2>{c.title}</h2><p className="card-copy">{c.intro}</p><strong>{projectId ?? c.project}</strong>
    {!desktop ? <p className="card-copy">{c.desktop}</p> : null}
    <fieldset className="store-identity-fields" disabled={busy || !desktop || !projectId}>
      <div className="button-row"><button type="button" className="ghost-button ghost-button--compact" data-project-operator-refresh onClick={() => void act({ action: "list" })}>{c.refresh}</button></div>
      {status?.installed_error ? <p className="warning-copy">{c.changed} <code>{status.installed_error}</code></p> : null}
      <label className="form-field"><span>{c.choose}</span><select value={installed} data-project-operator-installed onChange={e => setInstalled(e.target.value)}><option value="">--</option>
        {status?.installed.map(p => <option key={p.package_id} value={p.package_id}>{p.package_id} · {p.package_version}</option>)}</select></label>
      <button type="button" className="ghost-button ghost-button--compact" data-project-operator-enable disabled={!chosen || status?.enabled.some(e => e.package.package_id === chosen.package_id)}
        onClick={() => { if (chosen && window.confirm(c.confirm)) void act({ action: "enable", package: chosen }); }}>{c.enable}</button>
      <label className="form-field"><span>{c.enabled}</span><select data-project-operator-enabled value={enabled} onChange={e => { setEnabled(e.target.value); setOperator(""); setRemote(null); invalidateTask(); }}><option value="">--</option>
        {status?.enabled.map(e => <option key={e.package.package_id} value={e.package.package_id}>{e.package.package_id} · {e.package.package_version}</option>)}</select></label>
      {!status?.enabled.length ? <p className="card-copy">{c.empty}</p> : null}
      {selected ? <><p className="card-copy">{selected.state === "ready" ? c.ready : c.changed}</p>
        <details><summary>SHA-256</summary><code style={{ overflowWrap: "anywhere" }}>{selected.package.entrypoint_sha256}</code></details>
        <button type="button" className="ghost-button ghost-button--compact" data-project-operator-disable onClick={() => { if (window.confirm(c.confirm)) void act({ action: "disable", package: selected.package }); }}>{c.disable}</button>
        <label className="form-field"><span>{c.operator}</span><select data-project-operator-id value={operator} onChange={e => { setOperator(e.target.value); invalidateTask(); }}><option value="">--</option>
          {selected.package.operators.map(o => <option key={o.operator_id}>{o.operator_id}</option>)}</select></label>
        <div className="store-download-fields"><label className="form-field"><span>{c.input}</span><textarea className="text-input" data-project-operator-input rows={4} maxLength={32768} value={input} onChange={e => { setInput(e.target.value); invalidateTask(); }} /></label>
          <label className="form-field"><span>{c.config}</span><textarea className="text-input" rows={4} maxLength={32768} value={config} onChange={e => { setConfig(e.target.value); invalidateTask(); }} /></label></div>
        <div className="button-row"><button type="button" className="ghost-button ghost-button--compact" data-project-operator-prepare disabled={!operator || selected.state !== "ready"} onClick={() => {
          try { void act({ action: "prepare", package: selected.package, operator_id: operator, task_id: crypto.randomUUID(), input: JSON.parse(input), config: JSON.parse(config) }); } catch { setNotice(c.error); }
        }}>{c.prepare}</button><button type="button" className="ghost-button ghost-button--compact" disabled={!status?.task} onClick={() => downloadTextFile("project-operator.task.json", JSON.stringify(status?.task, null, 2))}>{c.export}</button></div>
        <p className="card-copy">{c.route}</p><label className="form-field"><span>{c.endpoint}</span><input className="text-input" data-project-operator-endpoint value={endpoint} maxLength={255} placeholder="127.0.0.1:5001" onChange={e => {setEndpoint(e.target.value);setRemote(null);}} /></label>
        <div className="button-row">
          <button type="button" className="ghost-button ghost-button--compact" data-project-operator-inspect-target disabled={!endpoint} onClick={()=>void act("inspect-package")}>{d.inspect}</button>
          <button type="button" className="ghost-button ghost-button--compact" data-project-operator-deploy disabled={!endpoint||selected.state!=="ready"} onClick={()=>{if(window.confirm(d.confirm))void act("deploy-package");}}>{d.deploy}</button>
          <button type="button" className="ghost-button ghost-button--compact" data-project-operator-abort disabled={!endpoint} onClick={()=>{if(window.confirm(d.abortConfirm))void act("abort-package");}}>{d.abort}</button>
        </div>
        {remote?<p className="card-copy" data-project-operator-target-state={remote.state}>{d[remote.state]} {remote.state==="uploading"?`${remote.received_bytes} / ${remote.total_bytes} B`:""}</p>:null}
        <button type="button" className="ghost-button ghost-button--compact" data-project-operator-run disabled={!operator || !endpoint || selected.state !== "ready" || !!handle} onClick={() => { if (window.confirm(c.runConfirm)) void act("run"); }}>{c.run}</button>
      </> : null}
      {status ? <details><summary>{c.root}</summary><code style={{ overflowWrap: "anywhere" }}>{status.packages_root}</code></details> : null}
      <details open={!!handle}><summary>{c.pending}</summary><p className="card-copy">{c.recovery}</p>
        <button type="button" className="ghost-button ghost-button--compact" onClick={() => void act("pending")}>{c.pending}</button>
        {pending.map(h => <button type="button" className="ghost-button ghost-button--compact" key={h.execution_id} onClick={() => { setHandle(h); setReceipt(null); }}>{h.endpoint} · {h.execution_id}</button>)}
        {handle ? <><code style={{ overflowWrap: "anywhere" }}>{handle.endpoint} · {handle.execution_id}</code><div className="button-row">
          <button type="button" className="ghost-button ghost-button--compact" onClick={() => void act("inspect")}>{c.check}</button>
          <button type="button" className="ghost-button ghost-button--compact" disabled={!receipt || !["completed", "failed"].includes(receipt.execution.status)} onClick={() => void act("ack")}>{c.ack}</button></div></> : null}
        {receipt ? <pre style={{ maxHeight: "16rem", overflow: "auto", whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>{JSON.stringify(receipt.execution, null, 2).slice(0, 65536)}</pre> : null}
      </details>
    </fieldset><p className="card-copy" role="status">{notice}</p>
  </section>;
}
