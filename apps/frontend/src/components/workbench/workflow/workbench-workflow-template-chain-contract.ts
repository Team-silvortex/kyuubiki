import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";

export const WORKFLOW_TEMPLATE_CHAIN_STEP_LIMIT = 64;
export const WORKFLOW_TEMPLATE_CHAIN_CONNECTION_LIMIT = 256;

export type WorkflowTemplateChainTopologyConnection = {
  from: number;
  to: number;
  fromPort?: string;
  toPort?: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asOptionalNonEmptyString(value: unknown): string | undefined | null {
  if (value === undefined) return undefined;
  return typeof value === "string" && value.trim().length > 0 ? value : null;
}

export function asWorkflowTemplateChainSelections(
  value: unknown,
): WorkflowNodeTemplateSelection[] | null {
  if (
    !Array.isArray(value) ||
    value.length === 0 ||
    value.length > WORKFLOW_TEMPLATE_CHAIN_STEP_LIMIT
  ) {
    return null;
  }
  const selections: WorkflowNodeTemplateSelection[] = [];
  for (const entry of value) {
    if (!isRecord(entry)) return null;
    const kind = asOptionalNonEmptyString(entry.kind);
    const operatorId = asOptionalNonEmptyString(entry.operatorId);
    if (kind === null || operatorId === null || (!kind && !operatorId)) return null;
    if (entry.config !== undefined && !isRecord(entry.config)) return null;
    const selection: WorkflowNodeTemplateSelection = {};
    if (kind) selection.kind = kind;
    if (operatorId) selection.operatorId = operatorId;
    if (entry.config !== undefined) {
      selection.config = structuredClone(entry.config as Record<string, unknown>);
    }
    selections.push(selection);
  }
  return selections;
}

function isAcyclic(
  templateCount: number,
  connections: WorkflowTemplateChainTopologyConnection[],
): boolean {
  const outgoing = Array.from({ length: templateCount }, () => new Set<number>());
  const indegree = Array.from({ length: templateCount }, () => 0);
  for (const connection of connections) {
    if (outgoing[connection.from]!.has(connection.to)) continue;
    outgoing[connection.from]!.add(connection.to);
    indegree[connection.to] += 1;
  }
  const ready = indegree.flatMap((degree, index) => degree === 0 ? [index] : []);
  let visited = 0;
  while (ready.length > 0) {
    const index = ready.pop()!;
    visited += 1;
    for (const target of outgoing[index]!) {
      indegree[target] -= 1;
      if (indegree[target] === 0) ready.push(target);
    }
  }
  return visited === templateCount;
}

export function asWorkflowTemplateChainConnections(
  value: unknown,
  templateCount: number,
): WorkflowTemplateChainTopologyConnection[] | undefined | null {
  if (value === undefined) return undefined;
  if (
    !Number.isSafeInteger(templateCount) ||
    templateCount <= 0 ||
    templateCount > WORKFLOW_TEMPLATE_CHAIN_STEP_LIMIT ||
    !Array.isArray(value) ||
    value.length > WORKFLOW_TEMPLATE_CHAIN_CONNECTION_LIMIT
  ) {
    return null;
  }
  const connections: WorkflowTemplateChainTopologyConnection[] = [];
  const connectionKeys = new Set<string>();
  for (const entry of value) {
    if (!isRecord(entry) || !Number.isSafeInteger(entry.from) || !Number.isSafeInteger(entry.to)) {
      return null;
    }
    const from = entry.from as number;
    const to = entry.to as number;
    const fromPort = asOptionalNonEmptyString(entry.fromPort);
    const toPort = asOptionalNonEmptyString(entry.toPort);
    if (
      from < 0 ||
      to < 0 ||
      from >= templateCount ||
      to >= templateCount ||
      from === to ||
      fromPort === null ||
      toPort === null
    ) {
      return null;
    }
    const key = `${from}:${to}:${fromPort ?? ""}:${toPort ?? ""}`;
    if (connectionKeys.has(key)) return null;
    connectionKeys.add(key);
    const connection: WorkflowTemplateChainTopologyConnection = { from, to };
    if (fromPort) connection.fromPort = fromPort;
    if (toPort) connection.toPort = toPort;
    connections.push(connection);
  }
  return isAcyclic(templateCount, connections) ? connections : null;
}

export function isWorkflowTemplateChainTopologyValid(
  templateCount: number,
  connections: WorkflowTemplateChainTopologyConnection[],
): boolean {
  return asWorkflowTemplateChainConnections(connections, templateCount) !== null;
}
