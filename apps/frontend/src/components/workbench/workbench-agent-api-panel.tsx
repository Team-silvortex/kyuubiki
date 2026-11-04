"use client";

import { useState } from "react";
import { agentApiAccessSummary, clearAgentApiAccess, configureAgentApiAccess } from "@/lib/api/agent-api-access";
import { resolveWorkbenchApiUrl } from "@/lib/api/backend-target";
import { fetchDirectMeshAgents } from "@/lib/api/runtime-client";

export function WorkbenchAgentApiPanel({ language, endpointsText, directMode }: { language: string; endpointsText: string; directMode: boolean }) {
  const zh = language.startsWith("zh");
  const copy = zh ? {
    title: "Agent API 安全连接", hint: "独立于控制面和 Mesh 网关令牌。原生 Agent 使用 TLS 1.3；凭证仅保留在当前页面内存，不写入项目或浏览器存储。",
    endpoint: "绑定的 Agent 地址（host:port）", name: "证书中的服务器名称", ca: "受信 CA / 自签名证书（PEM）", token: "Agent API token", apply: "应用到当前会话", clear: "清除凭证", applied: "当前会话已配置；请刷新 Agent 状态验证连接。", cleared: "已清除当前会话凭证。", file: "选择证书文件", required: "原生 Agent 必须配置 token、TLS 证书和私钥；此处只配置客户端，不修改服务器。只对一个明确绑定的 Agent 发送凭证。",
  } : {
    title: "Agent API secure connection", hint: "Separate from control-plane and Mesh gateway tokens. Native Agent uses TLS 1.3; credentials stay in page memory, never project or browser storage.",
    endpoint: "Pinned Agent address (host:port)", name: "Certificate server name", ca: "Trusted CA / self-signed certificate (PEM)", token: "Agent API token", apply: "Apply to this session", clear: "Clear credentials", applied: "Session configured; refresh Agent status to verify the connection.", cleared: "Session credentials cleared.", file: "Choose certificate file", required: "Configure token, TLS certificate and private key on the native Agent first. This form configures the client only and sends credentials to one pinned Agent.",
  };
  const [endpoint, setEndpoint] = useState(() => agentApiAccessSummary().endpoint ?? endpointsText.split(/[\n,]/u)[0]?.trim() ?? "");
  const [serverName, setServerName] = useState(() => agentApiAccessSummary().serverName ?? "");
  const [caPem, setCaPem] = useState("");
  const [token, setToken] = useState("");
  const [message, setMessage] = useState("");
  const [configured, setConfigured] = useState(() => agentApiAccessSummary().configured);
  const [checking, setChecking] = useState(false);
  const verify = async () => {
    const pinned = agentApiAccessSummary().endpoint;
    if (!pinned) return;
    setChecking(true);
    try {
      const response = await fetchDirectMeshAgents([pinned]);
      const access = response.agents.length === 1 ? response.agents[0].descriptor?.connection_access : undefined;
      setMessage(access?.authenticated === true && access.encrypted === true
        ? access.control_allowed ? (zh ? "TLS 身份验证通过，Agent 允许控制请求。" : "TLS authentication verified; Agent permits control requests.")
          : (zh ? "TLS 身份验证通过，但 Agent 尚未允许远程控制。" : "TLS authentication verified; remote control is not permitted by Agent policy.")
        : (zh ? "验证未通过；检查证书信任、服务器名称和令牌。没有提交计算任务。" : "Verification failed; check certificate trust, server name and token. No task submitted."));
    } catch { setMessage(zh ? "连接验证失败或配置已改变，没有提交计算任务。" : "Connection verification failed or configuration changed. No task submitted."); }
    finally { setChecking(false); }
  };
  const apply = () => {
    try {
      const target = resolveWorkbenchApiUrl("/");
      configureAgentApiAccess({ endpoint: endpoint.trim(), serverName: serverName.trim(), caPem, token }, target === "/" ? window.location.origin : target);
      setToken(""); setConfigured(true); setMessage(copy.applied);
    } catch { setMessage(zh ? "配置无效：检查地址、证书、令牌；网关必须使用 HTTPS 或本机回环地址。" : "Invalid configuration: check address, certificate and token; gateway must use HTTPS or loopback."); }
  };
  return <section className="sidebar-card sidebar-card--compact"><details data-testid="agent-api-security-panel">
    <summary>{copy.title}{configured ? " · TLS" : ""}</summary>
    <p>{copy.hint}</p><p>{copy.required}</p>
    <div className="form-grid compact">
    <label>{copy.endpoint}<input value={endpoint} onChange={(event) => setEndpoint(event.target.value)} placeholder="127.0.0.1:5001" autoComplete="off" /></label>
    <label>{copy.name}<input value={serverName} onChange={(event) => setServerName(event.target.value)} placeholder="agent.example" autoComplete="off" /></label>
    <label>{copy.file}<input type="file" accept=".pem,.crt,.cer" onChange={(event) => {
      const file = event.target.files?.[0]; event.target.value = "";
      if (!file) return;
      if (file.size > 32 * 1024) { setMessage(zh ? "证书文件不能超过 32 KiB。" : "Certificate file must not exceed 32 KiB."); return; }
      void file.text().then(setCaPem).catch(() => setMessage(zh ? "无法读取证书。" : "Cannot read certificate."));
    }} /></label>
    <label>{copy.ca}<textarea rows={4} value={caPem} onChange={(event) => setCaPem(event.target.value)} spellCheck={false} /></label>
    <label>{copy.token}<input type="password" value={token} onChange={(event) => setToken(event.target.value)} autoComplete="off" maxLength={256} /></label>
    <div className="button-row"><button type="button" onClick={apply}>{copy.apply}</button><button type="button" onClick={() => { clearAgentApiAccess(); setToken(""); setConfigured(false); setMessage(copy.cleared); }}>{copy.clear}</button></div>
    <button type="button" disabled={!configured || !directMode || checking} onClick={() => void verify()}>{zh ? "验证 Agent 连接（直连模式）" : "Verify Agent connection (direct mode)"}</button>
    <p role="status">{message}</p>
    </div>
  </details></section>;
}
