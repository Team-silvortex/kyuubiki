"use client";

import type {
  HeadlessActionContract,
  HeadlessActionGuidanceNote,
  HeadlessInputPort,
  HeadlessLocalizedText,
  HeadlessOutputPort,
} from "@/components/workbench/workbench-headless-workflow-contract";
import { localizeHeadlessText } from "@/components/workbench/workbench-headless-workflow-contract";
import type { WorkbenchRecordedMacroDraft, WorkbenchScriptLanguage } from "@/lib/scripting/workbench-script-runtime";

export type HeadlessWorkflowExportAction = {
  id: string;
  risk: HeadlessActionContract["risk"];
  summary: HeadlessLocalizedText;
  inputSchema: HeadlessInputPort[];
  outputSchema: HeadlessOutputPort[];
  guidanceNotes: HeadlessActionGuidanceNote[];
};

export type HeadlessWorkflowExportDocument = {
  schema_version: "kyuubiki.headless-workflow/v1";
  exported_at: string;
  language: WorkbenchScriptLanguage;
  workflow: WorkbenchRecordedMacroDraft;
  actions: HeadlessWorkflowExportAction[];
};

export type HeadlessExecutionBinding = {
  kind: "step_result";
  step: number;
  output: string;
};

export type HeadlessExecutionValue =
  | { kind: "literal"; value: unknown }
  | { kind: "binding"; source: HeadlessExecutionBinding }
  | { kind: "object"; fields: Record<string, HeadlessExecutionValue> }
  | { kind: "array"; items: HeadlessExecutionValue[] };

export type HeadlessWorkflowExecutionBatch = {
  schema_version: "kyuubiki.headless-execution-batch/v1";
  exported_at: string;
  language: WorkbenchScriptLanguage;
  workflow_id: string;
  steps: Array<{
    index: number;
    action: string;
    risk: HeadlessActionContract["risk"];
    payload: HeadlessExecutionValue;
    guidanceNotes: HeadlessActionGuidanceNote[];
  }>;
  warnings: string[];
};

function uniqueActions(draft: WorkbenchRecordedMacroDraft, actionMap: Map<string, HeadlessActionContract>) {
  const seen = new Set<string>();
  return draft.steps.flatMap((step) => {
    if (seen.has(step.action)) return [];
    seen.add(step.action);
    const contract = actionMap.get(step.action);
    if (!contract) return [];
    return [
      {
        id: contract.id,
        risk: contract.risk,
        summary: contract.summary,
        inputSchema: contract.inputSchema,
        outputSchema: contract.outputSchema,
        guidanceNotes: contract.guidanceNotes ?? [],
      },
    ];
  });
}

const STEP_RESULT_TEMPLATE_RE = /^\{\{\s*steps\.(\d+)\.result\.([a-zA-Z0-9_]+)\s*\}\}$/;

function compileExecutionValue(value: unknown, warnings: string[]): HeadlessExecutionValue {
  if (typeof value === "string") {
    const exact = value.match(STEP_RESULT_TEMPLATE_RE);
    if (exact) {
      return {
        kind: "binding",
        source: {
          kind: "step_result",
          step: Number(exact[1]),
          output: exact[2],
        },
      };
    }
    if (value.includes("{{")) {
      warnings.push(`Unresolved inline template kept as literal: ${value}`);
    }
    return { kind: "literal", value };
  }

  if (Array.isArray(value)) {
    return {
      kind: "array",
      items: value.map((entry) => compileExecutionValue(entry, warnings)),
    };
  }

  if (value && typeof value === "object") {
    return {
      kind: "object",
      fields: Object.fromEntries(
        Object.entries(value).map(([key, entry]) => [key, compileExecutionValue(entry, warnings)]),
      ),
    };
  }

  return { kind: "literal", value };
}

export function buildHeadlessWorkflowExportDocument({
  actionMap,
  draft,
  language,
}: {
  actionMap: Map<string, HeadlessActionContract>;
  draft: WorkbenchRecordedMacroDraft;
  language: WorkbenchScriptLanguage;
}): HeadlessWorkflowExportDocument {
  return {
    schema_version: "kyuubiki.headless-workflow/v1",
    exported_at: new Date().toISOString(),
    language,
    workflow: draft,
    actions: uniqueActions(draft, actionMap),
  };
}

export function buildHeadlessWorkflowExecutionBatch({
  actionMap,
  draft,
  language,
}: {
  actionMap: Map<string, HeadlessActionContract>;
  draft: WorkbenchRecordedMacroDraft;
  language: WorkbenchScriptLanguage;
}): HeadlessWorkflowExecutionBatch {
  const warnings: string[] = [];
  return {
    schema_version: "kyuubiki.headless-execution-batch/v1",
    exported_at: new Date().toISOString(),
    language,
    workflow_id: draft.id,
    steps: draft.steps.map((step, index) => {
      const contract = actionMap.get(step.action);
      return {
        index: index + 1,
        action: step.action,
        risk: contract?.risk ?? "normal",
        payload: compileExecutionValue(step.payload ?? {}, warnings),
        guidanceNotes: contract?.guidanceNotes ?? [],
      };
    }),
    warnings,
  };
}

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function localize(language: WorkbenchScriptLanguage, value: HeadlessLocalizedText) {
  return escapeHtml(localizeHeadlessText(language, value));
}

function renderPorts(title: string, ports: Array<HeadlessInputPort | HeadlessOutputPort>) {
  if (ports.length === 0) {
    return `<section><h4>${escapeHtml(title)}</h4><p>--</p></section>`;
  }
  return [
    `<section><h4>${escapeHtml(title)}</h4><ul>`,
    ...ports.map((port) => {
      const required = "required" in port && port.required ? " required" : "";
      const bindable = "bindable" in port && port.bindable ? " bindable" : "";
      return `<li><strong>${escapeHtml(port.label)}</strong><code>${escapeHtml(port.key)}</code><span>${escapeHtml(`${required}${bindable}`.trim() || "field")}</span></li>`;
    }),
    "</ul></section>",
  ].join("");
}

function renderGuidance(language: WorkbenchScriptLanguage, notes: HeadlessActionGuidanceNote[]) {
  if (notes.length === 0) {
    return "<section><h4>Governance</h4><p>--</p></section>";
  }
  return [
    "<section><h4>Governance</h4><ul>",
    ...notes.map((note) => `<li><strong>${localize(language, note.label)}</strong><p>${localize(language, note.value)}</p></li>`),
    "</ul></section>",
  ].join("");
}

export function buildHeadlessWorkflowExportHtml(document: HeadlessWorkflowExportDocument) {
  const workflowSteps = document.workflow.steps
    .map((step, index) => `<li><strong>${index + 1}. ${escapeHtml(step.action)}</strong><pre>${escapeHtml(JSON.stringify(step.payload ?? {}, null, 2))}</pre></li>`)
    .join("");
  const actionSections = document.actions
    .map(
      (action) => `
        <article class="action-card">
          <header>
            <h3>${escapeHtml(action.id)}</h3>
            <span class="risk risk--${escapeHtml(action.risk)}">${escapeHtml(action.risk)}</span>
          </header>
          <p>${localize(document.language, action.summary)}</p>
          ${renderGuidance(document.language, action.guidanceNotes)}
          ${renderPorts("Inputs", action.inputSchema)}
          ${renderPorts("Outputs", action.outputSchema)}
        </article>
      `,
    )
    .join("");

  return `<!doctype html>
<html lang="${escapeHtml(document.language)}">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${escapeHtml(document.workflow.id)} headless workflow</title>
    <style>
      :root { color-scheme: dark; --bg: #171a1f; --panel: #232831; --panel-soft: #1d222a; --line: #3a4250; --text: #e8edf5; --muted: #aab3c2; --accent: #f1993a; }
      * { box-sizing: border-box; }
      body { margin: 0; font: 14px/1.55 "SF Mono", "IBM Plex Sans", monospace; background: linear-gradient(180deg, #111418, #1a1f27); color: var(--text); }
      main { max-width: 1100px; margin: 0 auto; padding: 32px 20px 56px; display: grid; gap: 20px; }
      section, article { border: 1px solid var(--line); border-radius: 14px; background: linear-gradient(180deg, rgba(255,255,255,0.03), rgba(0,0,0,0.16)); }
      .hero { padding: 22px; background: linear-gradient(135deg, rgba(241,153,58,0.18), rgba(0,0,0,0.1)), var(--panel); }
      .hero h1 { margin: 0 0 10px; font-size: 24px; }
      .meta, .workflow, .contracts { padding: 18px 20px; }
      .meta-grid { display: grid; gap: 10px; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); }
      .meta-grid div, .action-card section { background: var(--panel-soft); border: 1px solid var(--line); border-radius: 10px; padding: 10px 12px; }
      .workflow ol, .contracts ul { margin: 10px 0 0; padding-left: 20px; }
      pre { overflow: auto; padding: 12px; border-radius: 10px; background: #0f1318; border: 1px solid #2b3442; color: #d9e3f0; }
      .contracts { display: grid; gap: 14px; }
      .action-card { padding: 16px 18px; display: grid; gap: 12px; background: var(--panel); }
      .action-card header { display: flex; justify-content: space-between; gap: 12px; align-items: center; }
      .action-card h3, h2, h4 { margin: 0; }
      .risk { padding: 4px 8px; border-radius: 999px; border: 1px solid var(--line); color: var(--muted); }
      .risk--destructive { color: #ff9d86; border-color: #8d4c41; }
      .risk--sensitive { color: #ffd27a; border-color: #7d6531; }
      code { margin-left: 8px; color: var(--accent); }
      p { margin: 0; color: var(--muted); }
      li { margin: 8px 0; }
      strong { color: var(--text); }
    </style>
  </head>
  <body>
    <main>
      <section class="hero">
        <h1>${escapeHtml(document.workflow.id)}</h1>
        <p>Headless workflow contract export with governance and install guidance aligned to the workbench runtime model.</p>
      </section>
      <section class="meta">
        <h2>Export metadata</h2>
        <div class="meta-grid">
          <div><strong>Schema</strong><p>${escapeHtml(document.schema_version)}</p></div>
          <div><strong>Language</strong><p>${escapeHtml(document.language)}</p></div>
          <div><strong>Exported at</strong><p>${escapeHtml(document.exported_at)}</p></div>
          <div><strong>Actions</strong><p>${escapeHtml(String(document.actions.length))}</p></div>
        </div>
      </section>
      <section class="workflow">
        <h2>Workflow steps</h2>
        <ol>${workflowSteps}</ol>
      </section>
      <section class="contracts">
        <h2>Action contracts</h2>
        ${actionSections}
      </section>
    </main>
  </body>
</html>`;
}

export function parseHeadlessWorkflowImportDocument(value: unknown): WorkbenchRecordedMacroDraft {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid headless workflow document.");
  }

  const candidate = value as {
    id?: unknown;
    steps?: unknown;
    schema_version?: unknown;
    workflow?: unknown;
  };

  if (candidate.schema_version === "kyuubiki.headless-workflow/v1") {
    const workflow = candidate.workflow;
    if (!workflow || typeof workflow !== "object") {
      throw new Error("Headless workflow contract is missing its workflow body.");
    }
    return parseHeadlessWorkflowImportDocument(workflow);
  }

  if (typeof candidate.id !== "string" || !Array.isArray(candidate.steps) || candidate.steps.length === 0) {
    throw new Error("Headless workflow document does not contain a valid workflow draft.");
  }

  const steps = candidate.steps.flatMap((step) => {
    if (!step || typeof step !== "object") return [];
    const entry = step as { action?: unknown; payload?: unknown };
    if (typeof entry.action !== "string" || !entry.action.trim()) return [];
    if (entry.payload === undefined) {
      return [{ action: entry.action }];
    }
    if (!entry.payload || typeof entry.payload !== "object" || Array.isArray(entry.payload)) {
      throw new Error("Headless workflow draft contains an invalid payload.");
    }
    return [{ action: entry.action, payload: entry.payload as Record<string, unknown> }];
  });

  if (steps.length === 0) {
    throw new Error("Headless workflow draft does not contain any valid steps.");
  }

  return {
    id: candidate.id.trim() || "macro/imported-headless-workflow",
    steps,
  };
}
