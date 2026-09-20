import { assistantRuntimeCopy } from "./hub-assistant-runtime-copy.js";
function record(value) {
    return value && typeof value === "object" && !Array.isArray(value)
        ? value : null;
}
function runtimeState(value) {
    return ["running", "stopped", "blocked", "starting", "disabled"].includes(String(value))
        ? value : "unknown";
}
export function assistantRuntimeModel(report) {
    const summary = report?.failed ? null : record(report?.summary);
    const services = summary ? [
        { label: "orchestrator", status: runtimeState(summary.orchestrator_status) },
        { label: "frontend", status: runtimeState(summary.frontend_status) },
        ...(Array.isArray(summary.agents) ? summary.agents.flatMap((entry) => {
            const agent = record(entry);
            return agent ? [{ label: typeof agent.label === "string" ? agent.label : "agent",
                    status: runtimeState(agent.status) }] : [];
        }) : []),
    ] : [];
    // Keep native keys intact, split only at the first colon (paths and URLs contain colons).
    const fields = (report?.rendered || "").split(/\r?\n/u).flatMap((line) => {
        const match = /^\s*([a-z][\w-]*(?:\[[^\]\r\n]+\])?):\s*(.*)$/u.exec(line);
        if (!match || /^(?:orchestrator|frontend|agent\[[^\]]+\])$/u.test(match[1]))
            return [];
        return [{ label: match[1], value: match[2] }];
    });
    return { services, fields, running: services.filter((service) => service.status === "running").length,
        ready: services.some((service) => service.status === "running") &&
            services.every((service) => service.status === "running" || service.status === "disabled"),
        available: Boolean(summary), pending: report === null, raw: report?.rendered || "" };
}
function text(root, selector, value) {
    const element = root.querySelector(selector);
    if (element && element.textContent !== value)
        element.textContent = value;
}
function serviceRow(document, service, copy) {
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
function fieldRow(document, field) {
    const row = document.createElement("div");
    const label = document.createElement("dt");
    const value = document.createElement("dd");
    label.textContent = field.label;
    value.textContent = field.value;
    value.dir = "auto";
    row.append(label, value);
    return row;
}
export function createAssistantRuntimeView(root) {
    let previous = "";
    return (report, language) => {
        if (!root)
            return;
        const copy = assistantRuntimeCopy(language);
        const model = assistantRuntimeModel(report);
        const key = JSON.stringify([model, language]);
        if (key === previous)
            return;
        previous = key;
        text(root, "#assistant-context-runtime", model.available
            ? `${model.running}/${model.services.length} ${copy.running}`
            : model.pending ? copy.pending : copy.unavailable);
        text(root, "[data-runtime-services-label]", copy.services);
        text(root, "[data-runtime-configuration-label]", copy.configuration);
        text(root, "[data-runtime-raw-label]", copy.raw);
        text(root, "[data-runtime-raw]", model.raw || copy.pending);
        const services = root.querySelector("[data-runtime-services]");
        services?.replaceChildren(...model.services.map((service) => serviceRow(root.ownerDocument, service, copy)));
        const fields = root.querySelector("[data-runtime-fields]");
        fields?.replaceChildren(...model.fields.map((field) => fieldRow(root.ownerDocument, field)));
        // Do not replace the details/summary elements: refreshes must preserve open state and focus.
        const group = root.querySelector("[data-runtime-configuration]");
        if (group)
            group.hidden = model.fields.length === 0;
        const list = root.querySelector("[data-runtime-service-group]");
        if (list)
            list.hidden = model.services.length === 0;
    };
}
