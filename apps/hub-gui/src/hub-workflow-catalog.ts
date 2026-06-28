type WorkflowCatalogEntry = {
  id?: string;
  name?: string;
  summary?: string;
  version?: string;
  entry_inputs?: Record<string, unknown>;
  output_artifacts?: unknown[];
};

type WorkflowSuggestion = {
  entry: WorkflowCatalogEntry;
  score: number;
  matchedTerms: string[];
};

type WorkflowSearchMetadata = {
  fields: string[];
  weightedFields: Array<[string, number]>;
};

type WorkflowState = {
  workflowCatalog: WorkflowCatalogEntry[];
  workflowCatalogBusy: boolean;
};

type DesktopStateTarget = Element | null | undefined;

type WorkflowMessage = (template: string, values?: Record<string, unknown>) => string;
type WorkflowLabel = (key: string) => string;

type WorkflowJobOptions = {
  timeoutMs?: number;
  intervalMs?: number;
  currentOrchestratorBaseUrl: () => string;
  hubMessage: WorkflowMessage;
  localizedWorkflowCatalogLabel: WorkflowLabel;
  setWorkflowCatalogOutput: (value: string) => void;
};

type WorkflowSampleOptions = WorkflowJobOptions & {
  state: WorkflowState;
  applyDesktopState: (element: DesktopStateTarget, state: string, options?: Record<string, unknown>) => void;
  actionState: DesktopStateTarget;
  currentWorkflowCatalogUrl: () => string;
  setOperationOutput: (value: string) => void;
  formatHubOperatorError: (error: unknown, options?: Record<string, unknown>) => string;
  renderWorkflowCatalog: () => void;
};

type FetchWorkflowCatalogOptions = {
  silent?: boolean;
  state: WorkflowState;
  renderWorkflowCatalog: () => void;
  setWorkflowCatalogOutput: (value: string) => void;
  setOperationOutput: (value: string) => void;
  applyDesktopState: (element: DesktopStateTarget, state: string, options?: Record<string, unknown>) => void;
  actionState: DesktopStateTarget;
  currentWorkflowCatalogUrl: () => string;
  hubMessage: WorkflowMessage;
  localizedWorkflowCatalogLabel: WorkflowLabel;
  formatHubOperatorError: (error: unknown, options?: Record<string, unknown>) => string;
};

type RenderWorkflowCatalogOptions = {
  workflowCatalogList?: HTMLElement | null;
  workflowCatalogBusy?: boolean;
  workflowCatalogQuery?: string;
  renderEmptyHistoryState: (container: HTMLElement, message: string) => void;
  localizedWorkflowCatalogLabel: WorkflowLabel;
  appendTextElement: (parent: HTMLElement, tagName: string, text: unknown, className?: string) => HTMLElement;
  hubMessage: WorkflowMessage;
  runWorkflowCatalogSample: (entry: WorkflowCatalogEntry) => void | Promise<void>;
};

type JsonObject = Record<string, unknown>;

function asObject(value: unknown): JsonObject {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value) ? value as JsonObject : {};
}

export function builtInWorkflowSampleInputArtifacts(workflowId: unknown): JsonObject | null {
  switch (workflowId) {
    case "workflow.heat-to-thermo-quad-2d":
      return {
        heat_model: {
          nodes: [
            { id: "h0", x: 0.0, y: 0.0, fix_temperature: true, temperature: 100.0, heat_load: 0.0 },
            { id: "h1", x: 1.0, y: 0.0, fix_temperature: false, temperature: 0.0, heat_load: 0.0 },
            { id: "h2", x: 1.0, y: 1.0, fix_temperature: true, temperature: 20.0, heat_load: 0.0 },
            { id: "h3", x: 0.0, y: 1.0, fix_temperature: true, temperature: 20.0, heat_load: 0.0 },
          ],
          elements: [
            {
              id: "hq0",
              node_i: 0,
              node_j: 1,
              node_k: 2,
              node_l: 3,
              thickness: 0.02,
              conductivity: 45.0,
            },
          ],
        },
      };
    default:
      return null;
  }
}

function workflowCatalogTokens(value: unknown): string[] {
  return String(value || "")
    .toLowerCase()
    .split(/[^a-z0-9]+/u)
    .map((token) => token.trim())
    .filter(Boolean);
}

function workflowCatalogSearchMetadata(entry: WorkflowCatalogEntry): WorkflowSearchMetadata {
  const inputs = Object.keys(entry?.entry_inputs || {});
  const outputs = Array.isArray(entry?.output_artifacts) ? entry.output_artifacts : [];
  const fields = [
    String(entry?.id || "").toLowerCase(),
    String(entry?.name || "").toLowerCase(),
    String(entry?.summary || "").toLowerCase(),
    String(entry?.version || "").toLowerCase(),
    inputs.join(" ").toLowerCase(),
    outputs.join(" ").toLowerCase(),
  ];
  const weightedFields: Array<[string, number]> = [
    [String(entry?.id || "").toLowerCase(), 5],
    [String(entry?.name || "").toLowerCase(), 4],
    [String(entry?.summary || "").toLowerCase(), 2],
    [inputs.join(" ").toLowerCase(), 2],
    [outputs.join(" ").toLowerCase(), 2],
    [String(entry?.version || "").toLowerCase(), 1],
  ];
  return { fields, weightedFields };
}

function scoreWorkflowCatalogEntry(entry: WorkflowCatalogEntry, tokens: string[]): { score: number; matchedTerms: string[] } {
  const metadata = workflowCatalogSearchMetadata(entry);
  const matchedTerms = tokens.filter((token) => metadata.fields.some((field) => field.includes(token)));
  if (tokens.length > 0 && matchedTerms.length !== tokens.length) {
    return { score: 0, matchedTerms: [] };
  }
  const score = tokens.reduce((total, token) => {
    const best = metadata.weightedFields.reduce((max, [field, weight]) => {
      if (field === token) return Math.max(max, weight * 6);
      if (field.startsWith(token)) return Math.max(max, weight * 4);
      if (field.includes(token)) return Math.max(max, weight);
      return max;
    }, 0);
    return total + best;
  }, 0);
  return { score, matchedTerms };
}

export function suggestWorkflowCatalogEntries(
  entries: WorkflowCatalogEntry[],
  query: unknown,
  limit = 8,
): WorkflowSuggestion[] {
  const tokens = workflowCatalogTokens(query);
  if (!tokens.length) {
    return entries.map((entry) => ({ entry, score: 0, matchedTerms: [] }));
  }
  return entries
    .map((entry) => {
      const { score, matchedTerms } = scoreWorkflowCatalogEntry(entry, tokens);
      return { entry, score, matchedTerms };
    })
    .filter((candidate) => candidate.score > 0)
    .sort((left, right) => right.score - left.score || String(left.entry.id || "").localeCompare(String(right.entry.id || "")))
    .slice(0, limit);
}

export function describeWorkflowSummary(resultPayload: unknown): string {
  const result = asObject(asObject(resultPayload).result);
  const artifacts = asObject(result.artifacts);
  const exported = asObject(artifacts["json_output.json"]);
  if (!exported?.content) {
    return "no exported summary";
  }

  try {
    const summary = JSON.parse(String(exported.content));
    return Object.entries(summary)
      .slice(0, 3)
      .map(([key, value]) => `${key}=${value}`)
      .join(" · ");
  } catch (_error) {
    return "exported summary is not valid JSON";
  }
}

export async function waitForWorkflowJob(jobId: string, options: WorkflowJobOptions): Promise<unknown> {
  const timeoutMs = Number(options.timeoutMs || 30_000);
  const intervalMs = Number(options.intervalMs || 700);
  const baseUrl = options.currentOrchestratorBaseUrl().replace(/\/+$/u, "");
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    const response = await fetch(`${baseUrl}/api/v1/jobs/${encodeURIComponent(jobId)}`);
    if (!response.ok) {
      throw new Error(`workflow job lookup failed (${response.status})`);
    }

    const payload = await response.json() as unknown;
    const payloadObject = asObject(payload);
    const job = asObject(payloadObject.job);
    const result = asObject(payloadObject.result);
    const status = String(job.status || "").trim();
    if (status === "completed") {
      return payload;
    }
    if (status === "failed" || status === "cancelled") {
      throw new Error(
        options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogFailed"), {
          workflow: result.workflow_id || job.job_type || "workflow",
          status,
        }),
      );
    }

    options.setWorkflowCatalogOutput(
      options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogPolling"), {
        job: jobId,
      }),
    );
    await new Promise((resolve) => window.setTimeout(resolve, intervalMs));
  }

  throw new Error(`workflow job ${jobId} timed out`);
}

export async function runWorkflowCatalogSample(entry: WorkflowCatalogEntry, options: WorkflowSampleOptions): Promise<void> {
  const workflowId = String(entry?.id || "").trim();
  const inputArtifacts = builtInWorkflowSampleInputArtifacts(workflowId);
  if (!workflowId || !inputArtifacts) {
    options.setWorkflowCatalogOutput(
      options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogUnsupported"), {
        workflow: workflowId || "unknown",
      }),
    );
    return;
  }

  options.state.workflowCatalogBusy = true;
  options.renderWorkflowCatalog();
  options.applyDesktopState(options.actionState, "running", { kind: "activity" });

  try {
    const response = await fetch(
      `${options.currentWorkflowCatalogUrl().replace(/\/+$/u, "")}/${encodeURIComponent(workflowId)}/jobs`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ input_artifacts: inputArtifacts }),
      },
    );

    if (!response.ok) {
      throw new Error(`workflow submit failed (${response.status})`);
    }

    const payload = await response.json() as unknown;
    const job = asObject(asObject(payload).job);
    const jobId = String(job.job_id || "").trim();
    const queuedMessage = options.hubMessage(
      options.localizedWorkflowCatalogLabel("workflowCatalogQueued"),
      {
        workflow: workflowId,
        job: jobId || "--",
      },
    );
    options.setWorkflowCatalogOutput(queuedMessage);
    options.setOperationOutput(queuedMessage);

    if (!jobId) {
      throw new Error("workflow submit returned no job id");
    }

    const resultPayload = await waitForWorkflowJob(jobId, options);
    const result = asObject(asObject(resultPayload).result);
    const completedNodes = Array.isArray(result.completed_nodes)
      ? result.completed_nodes
      : [];
    const completedMessage = options.hubMessage(
      options.localizedWorkflowCatalogLabel("workflowCatalogCompleted"),
      {
        workflow: workflowId,
        count: completedNodes.length,
        summary: describeWorkflowSummary(resultPayload),
      },
    );
    options.setWorkflowCatalogOutput(completedMessage);
    options.setOperationOutput(completedMessage);
    options.applyDesktopState(options.actionState, "ready", { kind: "activity" });
  } catch (error) {
    options.setWorkflowCatalogOutput(
      options.formatHubOperatorError(error, {
        actionLabel: "Running this workflow sample",
      }),
    );
    options.applyDesktopState(options.actionState, "failed", { kind: "activity" });
  } finally {
    options.state.workflowCatalogBusy = false;
    options.renderWorkflowCatalog();
  }
}

export async function fetchWorkflowCatalog(options: FetchWorkflowCatalogOptions): Promise<void> {
  const silent = options?.silent === true;
  if (options.state.workflowCatalogBusy) {
    return;
  }

  options.state.workflowCatalogBusy = true;
  options.renderWorkflowCatalog();
  if (!silent) {
    options.setWorkflowCatalogOutput(options.localizedWorkflowCatalogLabel("workflowCatalogLoading"));
    options.applyDesktopState(options.actionState, "running", { kind: "activity" });
  }

  try {
    const response = await fetch(options.currentWorkflowCatalogUrl());
    if (!response.ok) {
      throw new Error(`workflow catalog fetch failed (${response.status})`);
    }

    const payload = await response.json() as unknown;
    const workflows = asObject(payload).workflows;
    options.state.workflowCatalog = Array.isArray(workflows) ? workflows as WorkflowCatalogEntry[] : [];
    options.renderWorkflowCatalog();
    if (!silent) {
      const loadedMessage = options.hubMessage(
        options.localizedWorkflowCatalogLabel("workflowCatalogLoaded"),
        {
          count: options.state.workflowCatalog.length,
        },
      );
      options.setWorkflowCatalogOutput(loadedMessage);
      options.setOperationOutput(loadedMessage);
      options.applyDesktopState(options.actionState, "ready", { kind: "activity" });
    }
  } catch (error) {
    if (!silent) {
      options.setWorkflowCatalogOutput(
        options.formatHubOperatorError(error, {
          actionLabel: "Loading the workflow catalog",
        }),
      );
      options.applyDesktopState(options.actionState, "failed", { kind: "activity" });
    }
  } finally {
    options.state.workflowCatalogBusy = false;
    options.renderWorkflowCatalog();
  }
}

export function renderWorkflowCatalog(entries: WorkflowCatalogEntry[], options: RenderWorkflowCatalogOptions): void {
  const workflowCatalogList = options.workflowCatalogList;
  if (!workflowCatalogList) {
    return;
  }

  workflowCatalogList.innerHTML = "";
  if (!entries.length) {
    options.renderEmptyHistoryState(
      workflowCatalogList,
      options.localizedWorkflowCatalogLabel("workflowCatalogEmpty"),
    );
    return;
  }
  const query = String(options.workflowCatalogQuery || "").trim();
  const suggestedEntries = suggestWorkflowCatalogEntries(entries, query);
  if (query && !suggestedEntries.length) {
    options.renderEmptyHistoryState(
      workflowCatalogList,
      options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogNoSearchMatches"), {
        query,
      }),
    );
    return;
  }

  suggestedEntries.forEach(({ entry, score, matchedTerms }) => {
    const shell = document.createElement("div");
    shell.className = "hub-history-item";

    const summary = document.createElement("div");
    summary.className = "hub-history-item__summary";
    const heading = document.createElement("div");
    heading.className = "hub-history-item__heading";
    options.appendTextElement(heading, "strong", entry.name || entry.id || "workflow");
    const meta = document.createElement("div");
    meta.className = "hub-history-item__meta";
    options.appendTextElement(meta, "span", entry.id || "--", "desktop-shell-chip");
    options.appendTextElement(meta, "span", entry.version || "v1", "desktop-shell-chip");
    if (query) {
      options.appendTextElement(
        meta,
        "span",
        options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogSuggestedBadge"), {
          score,
        }),
        "desktop-shell-chip",
      );
    }
    heading.appendChild(meta);
    summary.appendChild(heading);
    options.appendTextElement(summary, "span", entry.summary || "named workflow");
    if (query && matchedTerms.length) {
      options.appendTextElement(
        summary,
        "span",
        options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogMatchedTerms"), {
          terms: matchedTerms.join(", "),
        }),
        "hub-history-item__alias",
      );
    }
    options.appendTextElement(
      summary,
      "span",
      options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogEntryInputs"), {
        inputs: Object.keys(entry.entry_inputs || {}).join(", ") || "--",
      }),
      "hub-history-item__alias",
    );
    options.appendTextElement(
      summary,
      "span",
      options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogOutputs"), {
        outputs: Array.isArray(entry.output_artifacts) ? entry.output_artifacts.join(", ") : "--",
      }),
      "hub-history-item__provenance",
    );

    const controls = document.createElement("div");
    controls.className = "hub-history-item__controls";
    const runButton = document.createElement("button");
    runButton.type = "button";
    runButton.className = "desktop-shell-button-ghost";
    runButton.textContent = options.localizedWorkflowCatalogLabel("workflowCatalogRun");
    runButton.disabled = options.workflowCatalogBusy === true;
    runButton.addEventListener("click", () => {
      void options.runWorkflowCatalogSample(entry);
    });
    controls.appendChild(runButton);

    shell.append(summary, controls);
    workflowCatalogList.appendChild(shell);
  });
}
