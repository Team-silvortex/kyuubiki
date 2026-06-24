export function formatAssistantAuditTime(value) {
  const timestamp = new Date(String(value || "").trim());
  if (Number.isNaN(timestamp.getTime())) {
    return String(value || "").trim();
  }

  return timestamp.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function renderHubAssistantAuditEntries({
  appendTextElement,
  assistantAuditList,
  assistantDeliveryStateClass,
  assistantRiskStateClass,
  assistantStatusStateClass,
  entries,
  renderEmptyHistoryState,
}) {
  if (!assistantAuditList) {
    return;
  }

  assistantAuditList.innerHTML = "";
  if (!entries.length) {
    renderEmptyHistoryState(assistantAuditList, "No assistant actions recorded in this session.");
    return;
  }

  entries.forEach((entry) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    const header = document.createElement("div");
    header.className = "desktop-shell-section-header";
    appendTextElement(header, "strong", entry.action);
    const badges = document.createElement("div");
    badges.className = "desktop-shell-action-row";
    appendTextElement(badges, "span", entry.risk, assistantRiskStateClass(entry.risk));
    appendTextElement(badges, "span", entry.status, assistantStatusStateClass(entry.status));
    appendTextElement(badges, "span", entry.delivery || "local", assistantDeliveryStateClass(entry.delivery || "local"));
    header.appendChild(badges);
    article.appendChild(header);
    appendTextElement(
      article,
      "p",
      `${formatAssistantAuditTime(entry.createdAt)} · ${entry.source}${entry.note ? ` · ${entry.note}` : ""}`,
      "desktop-shell-note",
    );
    assistantAuditList.appendChild(article);
  });
}

export function rememberHubAssistantAuditEntry(entry, {
  auditLimit,
  loadHubAssistantAudit,
  mirrorHubAssistantAuditToSecurityEvents,
  persistHubAssistantAudit,
  renderHubAssistantAudit,
}) {
  const normalized = {
    auditId: String(entry?.auditId || `hub-assistant-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`),
    action: String(entry?.action || "").trim(),
    risk: String(entry?.risk || "low").trim(),
    status: String(entry?.status || "idle").trim(),
    source: String(entry?.source || "assistant").trim(),
    note: String(entry?.note || "").trim(),
    createdAt: new Date().toISOString(),
    delivery: String(entry?.delivery || "local").trim(),
  };

  if (!normalized.action) {
    return loadHubAssistantAudit();
  }

  const next = [normalized, ...loadHubAssistantAudit()].slice(0, auditLimit);
  persistHubAssistantAudit(next);
  renderHubAssistantAudit(next);
  if (entry?.sync !== false) {
    void mirrorHubAssistantAuditToSecurityEvents(normalized);
  }
  return next;
}

export function updateHubAssistantAuditDeliveryEntry(auditId, delivery, noteSuffix, {
  loadHubAssistantAudit,
  persistHubAssistantAudit,
  renderHubAssistantAudit,
}) {
  const entries = loadHubAssistantAudit();
  const next = entries.map((entry) => {
    if (entry.auditId !== auditId) {
      return entry;
    }
    return {
      ...entry,
      delivery,
      note: noteSuffix ? `${entry.note}${entry.note ? " · " : ""}${noteSuffix}` : entry.note,
    };
  });
  persistHubAssistantAudit(next);
  renderHubAssistantAudit(next);
}

export async function mirrorHubAssistantAuditToSecurityEvents(entry, {
  currentAssistantAuditContext,
  currentOrchestratorBaseUrl,
  fetchImpl = fetch,
  updateHubAssistantAuditDelivery,
}) {
  const baseUrl = currentOrchestratorBaseUrl().replace(/\/+$/, "");
  try {
    const response = await fetchImpl(`${baseUrl}/api/v1/security-events`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        event_id: entry.auditId,
        event_type: "hub.assistant.action",
        source: "hub-assistant",
        action: entry.action,
        risk: entry.risk,
        status: entry.status,
        note: entry.note || null,
        context: {
          ...currentAssistantAuditContext(),
          assistant_source: entry.source,
          delivery: "hub-session",
        },
        occurred_at: entry.createdAt,
      }),
    });

    if (!response.ok) {
      throw new Error(`control-plane sync failed (${response.status})`);
    }

    updateHubAssistantAuditDelivery(entry.auditId, "synced");
  } catch (error) {
    updateHubAssistantAuditDelivery(
      entry.auditId,
      "sync_failed",
      error instanceof Error ? error.message : String(error),
    );
  }
}
