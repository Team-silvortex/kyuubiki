export function builtInWorkflowSampleInputArtifacts(workflowId) {
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

export function describeWorkflowSummary(resultPayload) {
  const exported = resultPayload?.result?.artifacts?.["json_output.json"];
  if (!exported?.content) {
    return "no exported summary";
  }

  try {
    const summary = JSON.parse(exported.content);
    return Object.entries(summary)
      .slice(0, 3)
      .map(([key, value]) => `${key}=${value}`)
      .join(" · ");
  } catch (_error) {
    return "exported summary is not valid JSON";
  }
}

export async function waitForWorkflowJob(jobId, options = {}) {
  const timeoutMs = Number(options.timeoutMs || 30_000);
  const intervalMs = Number(options.intervalMs || 700);
  const baseUrl = options.currentOrchestratorBaseUrl().replace(/\/+$/u, "");
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    const response = await fetch(`${baseUrl}/api/v1/jobs/${encodeURIComponent(jobId)}`);
    if (!response.ok) {
      throw new Error(`workflow job lookup failed (${response.status})`);
    }

    const payload = await response.json();
    const status = String(payload?.job?.status || "").trim();
    if (status === "completed") {
      return payload;
    }
    if (status === "failed" || status === "cancelled") {
      throw new Error(
        options.hubMessage(options.localizedWorkflowCatalogLabel("workflowCatalogFailed"), {
          workflow: payload?.result?.workflow_id || payload?.job?.job_type || "workflow",
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

export async function runWorkflowCatalogSample(entry, options) {
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

    const payload = await response.json();
    const jobId = String(payload?.job?.job_id || "").trim();
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
    const completedNodes = Array.isArray(resultPayload?.result?.completed_nodes)
      ? resultPayload.result.completed_nodes
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

export async function fetchWorkflowCatalog(options) {
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

    const payload = await response.json();
    options.state.workflowCatalog = Array.isArray(payload?.workflows) ? payload.workflows : [];
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

export function renderWorkflowCatalog(entries, options) {
  if (!options.workflowCatalogList) {
    return;
  }

  options.workflowCatalogList.innerHTML = "";
  if (!entries.length) {
    options.renderEmptyHistoryState(
      options.workflowCatalogList,
      options.localizedWorkflowCatalogLabel("workflowCatalogEmpty"),
    );
    return;
  }

  entries.forEach((entry) => {
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
    heading.appendChild(meta);
    summary.appendChild(heading);
    options.appendTextElement(summary, "span", entry.summary || "named workflow");
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
    options.workflowCatalogList.appendChild(shell);
  });
}
