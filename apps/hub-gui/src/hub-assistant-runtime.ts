import { assistantRuntimeCopy, type AssistantRuntimeCopy } from "./hub-assistant-runtime-copy.js";

export type AssistantRuntimeReport = { rendered: string; summary: unknown; failed?: boolean };
type RuntimeState = "running" | "stopped" | "blocked" | "starting" | "disabled" | "unknown";
type Service = { label: string; status: RuntimeState };
type Field = { label: string; value: string };

function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown> : null;
}

function runtimeState(value: unknown): RuntimeState {
  return ["running", "stopped", "blocked", "starting", "disabled"].includes(String(value))
    ? value as RuntimeState : "unknown";
}

export function assistantRuntimeModel(report: AssistantRuntimeReport | null) {
  const summary = report?.failed ? null : record(report?.summary);
  const services: Service[] = summary ? [
    { label: "orchestrator", status: runtimeState(summary.orchestrator_status) },
    { label: "frontend", status: runtimeState(summary.frontend_status) },
    ...(Array.isArray(summary.agents) ? summary.agents.flatMap((entry): Service[] => {
      const agent = record(entry);
      return agent ? [{ label: typeof agent.label === "string" ? agent.label : "agent",
        status: runtimeState(agent.status) }] : [];
    }) : []),
  ] : [];
  // Keep native keys intact, split only at the first colon (paths and URLs contain colons).
  const fields: Field[] = (report?.rendered || "").split(/\r?\n/u).flatMap((line) => {
    const match = /^\s*([a-z][\w-]*(?:\[[^\]\r\n]+\])?):\s*(.*)$/u.exec(line);
    if (!match || /^(?:orchestrator|frontend|agent\[[^\]]+\])$/u.test(match[1])) return [];
    return [{ label: match[1], value: match[2] }];
  });
  return { services, fields, running: services.filter((service) => service.status === "running").length,
    ready: services.some((service) => service.status === "running") &&
      services.every((service) => service.status === "running" || service.status === "disabled"),
    available: Boolean(summary), pending: report === null, raw: report?.rendered || "" };
}

function text(root: HTMLElement, selector: string, value: string): void {
  const element = root.querySelector(selector);
  if (element && element.textContent !== value) element.textContent = value;
}

function serviceRow(document: Document, service: Service, copy: AssistantRuntimeCopy): HTMLLIElement {
  const row = document.createElement("li");
  const label = document.createElement("code");
  label.textContent = service.label;
  label.dir = "auto";
  const status = document.createElement("span");
  status.className = "hub-assistant-runtime__state";
  status.dataset.state = service.status;
  status.textContent = copy[service.status];
  row.append(label, status);
  return row;
}

function fieldRow(document: Document, field: Field): HTMLDivElement {
  const row = document.createElement("div");
  const label = document.createElement("dt");
  const value = document.createElement("dd");
  label.textContent = field.label;
  value.textContent = field.value;
  value.dir = "auto";
  row.append(label, value);
  return row;
}

export function createAssistantRuntimeView(root: HTMLElement | null) {
  let previous = "";
  return (report: AssistantRuntimeReport | null, language: string): void => {
    if (!root) return;
    const copy = assistantRuntimeCopy(language);
    const model = assistantRuntimeModel(report);
    const key = JSON.stringify([model, language]);
    if (key === previous) return;
    previous = key;
    text(root, "#assistant-context-runtime", model.available
      ? `${model.running}/${model.services.length} ${copy.running}`
      : model.pending ? copy.pending : copy.unavailable);
    text(root, "[data-runtime-services-label]", copy.services);
    text(root, "[data-runtime-configuration-label]", copy.configuration);
    text(root, "[data-runtime-raw-label]", copy.raw);
    text(root, "[data-runtime-raw]", model.raw || copy.pending);
    const services = root.querySelector<HTMLElement>("[data-runtime-services]");
    services?.replaceChildren(...model.services.map((service) => serviceRow(root.ownerDocument, service, copy)));
    const fields = root.querySelector<HTMLElement>("[data-runtime-fields]");
    fields?.replaceChildren(...model.fields.map((field) => fieldRow(root.ownerDocument, field)));
    // Do not replace the details/summary elements: refreshes must preserve open state and focus.
    const group = root.querySelector<HTMLElement>("[data-runtime-configuration]");
    if (group) group.hidden = model.fields.length === 0;
    const list = root.querySelector<HTMLElement>("[data-runtime-service-group]");
    if (list) list.hidden = model.services.length === 0;
  };
}
