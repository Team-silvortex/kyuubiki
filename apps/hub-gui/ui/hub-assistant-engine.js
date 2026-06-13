export function assistantTrustHostOrigin(baseUrl) {
  try {
    return new URL(baseUrl).origin.toLowerCase();
  } catch {
    return "";
  }
}

export function assistantHostRequiresTrust(baseUrl) {
  try {
    const parsed = new URL(baseUrl);
    const protocol = parsed.protocol.toLowerCase();
    const hostname = parsed.hostname.toLowerCase();
    const isLoopback =
      hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
    return protocol === "https:" && !isLoopback;
  } catch {
    return false;
  }
}

export function ensureAssistantHostTrust(baseUrl, apiKey, options) {
  if (!assistantHostRequiresTrust(baseUrl)) {
    return true;
  }

  const origin = assistantTrustHostOrigin(baseUrl);
  if (!origin) {
    return false;
  }
  if (options.trustedHosts.has(origin)) {
    return true;
  }

  const approved = options.confirm(
    `This assistant request will send${apiKey ? " your API key and" : ""} the prompt directly to ${origin}.\n\nOnly continue if you trust this host.`,
  );
  if (!approved) {
    return false;
  }

  options.trustedHosts.add(origin);
  options.persistTrustedHosts(options.trustedHosts);
  return true;
}

export function ensureRemoteHostTrust(baseUrl, label, options) {
  if (!assistantHostRequiresTrust(baseUrl)) {
    return true;
  }

  const origin = assistantTrustHostOrigin(baseUrl);
  if (!origin) {
    return false;
  }
  if (options.trustedHosts.has(origin)) {
    return true;
  }

  const approved = options.confirm(
    `${label} will contact ${origin} directly.\n\nOnly continue if you trust this remote host.`,
  );
  if (!approved) {
    return false;
  }

  options.trustedHosts.add(origin);
  options.persistTrustedHosts(options.trustedHosts);
  return true;
}

export async function requestHubAssistantPlan(options) {
  const baseUrl = options.assistantBaseUrl?.value?.trim() || "";
  const model = options.assistantModelName?.value?.trim() || "";
  const prompt = options.assistantPrompt?.value?.trim() || "";
  const apiKey = options.assistantApiKey?.value?.trim() || "";
  const baseUrlValidation = options.validateAssistantBaseUrl(baseUrl);

  if (!baseUrlValidation.ok || !model) {
    throw new Error(baseUrlValidation.reason || "Fill in the assistant base URL and model before requesting a plan.");
  }

  if (!ensureAssistantHostTrust(baseUrlValidation.normalized, apiKey, {
    trustedHosts: options.assistantTrustedHosts,
    persistTrustedHosts: options.persistAssistantTrustedHosts,
    confirm: options.confirm,
  })) {
    throw new Error("assistant request cancelled before contacting the configured host");
  }

  const response = await fetch(`${baseUrlValidation.normalized.replace(/\/+$/u, "")}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}),
    },
    body: JSON.stringify({
      model,
      temperature: 0.2,
      response_format: { type: "json_object" },
      messages: [
        {
          role: "system",
          content:
            "You are the Kyuubiki Hub assistant. Return strict JSON with keys summary, rationale, suggested_actions. suggested_actions must be an array of objects with action, payload, reason. Only suggest actions from the provided Hub action catalog. Keep it concise, safe, and onboarding-oriented.",
        },
        {
          role: "user",
          content: JSON.stringify(
            {
              prompt,
              snapshot: options.currentAssistantSnapshot(),
              action_catalog: options.assistantActions,
              local_hints: options.buildHubAssistantLocalCards().map((card) => ({
                id: card.id,
                title: card.title,
                summary: card.summary,
                actionLabel: card.actionLabel,
              })),
            },
            null,
            2,
          ),
        },
      ],
    }),
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`assistant request failed (${response.status}): ${body.slice(0, 240)}`);
  }

  const payload = await response.json();
  const content = payload?.choices?.[0]?.message?.content;
  if (!content) {
    throw new Error("assistant response did not include a message body");
  }

  const parsed = JSON.parse(options.extractAssistantJsonBlock(content));
  return {
    summary: String(parsed?.summary || ""),
    rationale: String(parsed?.rationale || ""),
    suggested_actions: Array.isArray(parsed?.suggested_actions)
      ? parsed.suggested_actions.map((entry) => ({
          action: String(entry?.action || ""),
          payload: entry && typeof entry.payload === "object" && entry.payload ? entry.payload : {},
          reason: String(entry?.reason || ""),
        }))
      : [],
  };
}

export function renderHubAssistantPlan(options) {
  if (!options.assistantPlanActions) {
    return;
  }

  const plan = options.assistantPlan;
  options.assistantPlanActions.innerHTML = "";
  if (!plan) {
    options.renderEmptyHistoryState(options.assistantPlanActions, options.hubDynamic("assistantNoPlan"));
    return;
  }

  const summaryCard = document.createElement("article");
  summaryCard.className = "hub-list__card";
  options.appendAssistantCardHeader(
    summaryCard,
    plan.summary || options.hubDynamic("modelPlanTitle"),
    `${plan.suggested_actions.length} actions`,
  );
  options.appendTextElement(
    summaryCard,
    "p",
    plan.rationale || options.hubDynamic("noRationale"),
    "desktop-shell-note",
  );
  options.assistantPlanActions.appendChild(summaryCard);

  if (!plan.suggested_actions.length) {
    options.renderEmptyHistoryState(options.assistantPlanActions, options.hubDynamic("assistantNoExecutable"));
    return;
  }

  plan.suggested_actions.forEach((entry) => {
    const article = document.createElement("article");
    article.className = "hub-list__card";
    const risk = options.assistantRiskLevel(entry.action);
    options.appendAssistantCardHeader(
      article,
      entry.action,
      risk,
      options.assistantRiskStateClass(risk),
    );
    options.appendTextElement(article, "p", entry.reason || options.hubDynamic("noRationale"), "desktop-shell-note");
    options.appendTextElement(article, "code", JSON.stringify(entry.payload || {}, null, 2));
    const row = document.createElement("div");
    row.className = "desktop-shell-action-row";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "desktop-shell-button-ghost";
    button.textContent = options.hubDynamic("actionRun");
    button.addEventListener("click", () => {
      void options.executeHubAssistantAction(entry.action, entry.payload || {});
    });
    row.appendChild(button);
    article.appendChild(row);
    options.assistantPlanActions.appendChild(article);
  });
}

export function confirmHubAssistantAction(action, source = "assistant", options) {
  const risk = options.assistantRiskLevel(action);
  if (risk === "low") {
    return true;
  }

  const note = source === "plan" ? "model plan action" : "assistant action";
  options.rememberHubAssistantAudit({ action, risk, status: "prompted", source, note });
  const message =
    risk === "high"
      ? `High-risk ${note}: ${action}\n\nThis may launch builds or rewrite bundle outputs.\n\nContinue?`
      : `Sensitive ${note}: ${action}\n\nPlease confirm before the Hub continues.\n\nContinue?`;
  const approved = options.confirm(message);
  options.rememberHubAssistantAudit({
    action,
    risk,
    status: approved ? "confirmed" : "cancelled",
    source,
    note,
  });
  return approved;
}

export function applyAssistantBundlePayload(payload, options) {
  if (typeof payload?.path === "string") {
    options.projectBundlePath.value = payload.path;
  }
  if (typeof payload?.comparePath === "string" || typeof payload?.rightPath === "string") {
    options.projectBundleComparePath.value = String(payload.comparePath ?? payload.rightPath ?? "");
  }
  if (typeof payload?.out === "string") {
    options.projectBundleOutPath.value = payload.out;
  }
}

export async function executeHubAssistantAction(action, payload = {}, source = "assistant", options) {
  const risk = options.assistantRiskLevel(action);
  if (!confirmHubAssistantAction(action, source, options)) {
    options.setAssistantOutput(options.hubDynamic("assistantCancelled", { action }));
    return;
  }

  switch (action) {
    case "hub/focusSection":
      options.setSection(typeof payload.section === "string" ? payload.section : "projects");
      options.setAssistantOutput(
        options.hubDynamic("assistantFocusedSection", {
          section: typeof payload.section === "string" ? payload.section : "projects",
        }),
      );
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "focused Hub section" });
      return;
    case "hub/openWorkbench":
      await options.runActionWithOptions("open-workbench", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Workbench shell" });
      return;
    case "hub/openInstaller":
      await options.runActionWithOptions("open-installer", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Installer shell" });
      return;
    case "hub/openDocsIndex":
      await options.runActionWithOptions("open-docs-index", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened docs index" });
      return;
    case "hub/openCurrentLineDoc":
      await options.runActionWithOptions("open-current-line-doc", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened current-line document" });
      return;
    case "hub/openOperationsDoc":
      await options.runActionWithOptions("open-operations-doc", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened operations guide" });
      return;
    case "hub/openTroubleshootingDoc":
      await options.runActionWithOptions("open-troubleshooting-doc", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened troubleshooting guide" });
      return;
    case "hub/startLocal":
      await options.runActionWithOptions("start-local", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "started local stack" });
      return;
    case "hub/validateEnv":
      await options.runActionWithOptions("validate-env", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated environment" });
      return;
    case "hub/desktopStage":
      await options.runActionWithOptions("open-installer", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Installer for desktop staging work" });
      return;
    case "hub/desktopBuildHost":
      await options.runActionWithOptions("open-installer", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Installer for host-bundle build work" });
      return;
    case "hub/desktopVerify":
      await options.runActionWithOptions("open-installer", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "opened Installer for desktop verification work" });
      return;
    case "hub/setBundleContext":
      applyAssistantBundlePayload(payload, options);
      options.renderAssistantContext();
      options.setAssistantOutput(options.hubDynamic("assistantUpdatedBundle"));
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "updated bundle inputs" });
      return;
    case "hub/projectInspect":
      applyAssistantBundlePayload(payload, options);
      await options.runActionWithOptions("project-inspect", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "inspected project bundle" });
      return;
    case "hub/projectValidate":
      applyAssistantBundlePayload(payload, options);
      await options.runActionWithOptions("project-validate", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "validated project bundle" });
      return;
    case "hub/projectNormalize":
      applyAssistantBundlePayload(payload, options);
      await options.runActionWithOptions("project-normalize", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "normalized project bundle" });
      return;
    case "hub/projectUnpack":
      applyAssistantBundlePayload(payload, options);
      await options.runActionWithOptions("project-unpack", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "unpacked project bundle" });
      return;
    case "hub/projectPack":
      applyAssistantBundlePayload(payload, options);
      await options.runActionWithOptions("project-pack", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "packed project bundle" });
      return;
    case "hub/projectDiff":
      applyAssistantBundlePayload(payload, options);
      await options.runActionWithOptions("project-diff", { skipConfirmation: true });
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "diffed project bundles" });
      return;
    default:
      options.rememberHubAssistantAudit({ action, risk, status: "failed", source, note: "unknown assistant action" });
      throw new Error(`Unknown assistant action: ${action}`);
  }
}

export async function executeHubAssistantPlan(options) {
  if (!options.assistantPlan?.suggested_actions?.length) {
    options.setAssistantOutput(options.hubDynamic("assistantNoPlanToExecute"));
    return;
  }

  if (!options.assistantApprovePlan?.checked) {
    options.setAssistantOutput(options.hubDynamic("assistantReviewFirst"));
    return;
  }

  for (const entry of options.assistantPlan.suggested_actions) {
    try {
      await options.executeHubAssistantAction(entry.action, entry.payload || {}, "plan");
    } catch (error) {
      options.rememberHubAssistantAudit({
        action: entry.action,
        risk: options.assistantRiskLevel(entry.action),
        status: "failed",
        source: "plan",
        note: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  options.setAssistantOutput(
    options.hubDynamic("assistantExecuteCount", {
      count: options.assistantPlan.suggested_actions.length,
    }),
  );
}
