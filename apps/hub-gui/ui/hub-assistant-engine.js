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
    const hostname = parsed.hostname.toLowerCase();
    const isLoopback =
      hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
    return !isLoopback;
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

const HUB_ASSISTANT_DESKTOP_ROUTES = Object.freeze({
  "hub/openWorkbench": ["open-workbench", "opened Workbench shell"],
  "hub/openInstaller": ["open-installer", "opened Installer shell"],
  "hub/openDocsIndex": ["open-docs-index", "opened docs index"],
  "hub/openCurrentLineDoc": ["open-current-line-doc", "opened current-line document"],
  "hub/openOperationsDoc": ["open-operations-doc", "opened operations guide"],
  "hub/openTroubleshootingDoc": ["open-troubleshooting-doc", "opened troubleshooting guide"],
  "hub/startLocal": ["start-local", "started local stack"],
  "hub/validateEnv": ["validate-env", "validated environment"],
  "hub/desktopStage": ["open-installer", "opened Installer for desktop staging work"],
  "hub/desktopBuildHost": ["open-installer", "opened Installer for host-bundle build work"],
  "hub/desktopVerify": ["open-installer", "opened Installer for desktop verification work"],
  "hub/projectCreate": ["project-create", "created project bundle and activated its path", true],
  "hub/projectInspect": ["project-inspect", "inspected project bundle", true],
  "hub/projectValidate": ["project-validate", "validated project bundle", true],
  "hub/projectNormalize": ["project-normalize", "normalized project bundle", true],
  "hub/projectUnpack": ["project-unpack", "unpacked project bundle", true],
  "hub/projectPack": ["project-pack", "packed project bundle", true],
  "hub/projectDiff": ["project-diff", "diffed project bundles", true],
});

const HUB_ACTION_TERMINAL_STATUSES = new Set([
  "completed",
  "blocked",
  "cancelled",
  "failed",
  "missing",
]);

function normalizeHubActionOutcome(action, outcome) {
  const status = String(outcome?.status || "failed");
  return {
    action,
    status: HUB_ACTION_TERMINAL_STATUSES.has(status) ? status : "failed",
  };
}

async function executeHubAssistantDesktopRoute(action, payload, source, risk, route, options) {
  const [desktopAction, completedNote, appliesBundlePayload = false] = route;
  if (appliesBundlePayload) {
    applyAssistantBundlePayload(payload, options);
  }

  const outcome = normalizeHubActionOutcome(
    action,
    await options.runActionWithOptions(desktopAction, { skipConfirmation: true }),
  );
  options.rememberHubAssistantAudit({
    action,
    risk,
    status: outcome.status,
    source,
    note: outcome.status === "completed" ? completedNote : `${desktopAction} ended as ${outcome.status}`,
  });
  return outcome;
}

export async function executeHubAssistantAction(action, payload = {}, source = "assistant", options) {
  const risk = options.assistantRiskLevel(action);
  if (!confirmHubAssistantAction(action, source, options)) {
    options.setAssistantOutput(options.hubDynamic("assistantCancelled", { action }));
    return { action, status: "cancelled" };
  }

  const desktopRoute = HUB_ASSISTANT_DESKTOP_ROUTES[action];
  if (desktopRoute) {
    return executeHubAssistantDesktopRoute(action, payload, source, risk, desktopRoute, options);
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
      return { action, status: "completed" };
    case "hub/setBundleContext":
      applyAssistantBundlePayload(payload, options);
      options.renderAssistantContext();
      options.setAssistantOutput(options.hubDynamic("assistantUpdatedBundle"));
      options.rememberHubAssistantAudit({ action, risk, status: "completed", source, note: "updated bundle inputs" });
      return { action, status: "completed" };
    default:
      options.rememberHubAssistantAudit({ action, risk, status: "failed", source, note: "unknown assistant action" });
      return { action, status: "failed" };
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
    let outcome;
    try {
      outcome = await options.executeHubAssistantAction(entry.action, entry.payload || {}, "plan");
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
    if (outcome?.status !== "completed") {
      throw new Error(`Assistant action ${entry.action} did not complete (${outcome?.status || "failed"}).`);
    }
  }

  options.setAssistantOutput(
    options.hubDynamic("assistantExecuteCount", {
      count: options.assistantPlan.suggested_actions.length,
    }),
  );
}
