import type {
  WorkflowGraphDefinition,
  WorkflowGraphEdge,
  WorkflowGraphNode,
} from "@/lib/api";

export const FRONTEND_KERNEL_CONTRACT_VERSION = "kyuubiki.frontend-kernel/v1";

export type FrontendKernelBackend = "typescript" | "wasm";

export type FrontendKernelWorkload = {
  workflowNodeCount?: number;
  workflowEdgeCount?: number;
  materialCandidateCount?: number;
  layoutRectCount?: number;
  repeated?: boolean;
  wasmReady?: boolean;
  wasmWarmed?: boolean;
};

export type FrontendKernelBackendDecision = {
  backend: FrontendKernelBackend;
  coldStartProtected: boolean;
  hotPathScore: number;
  reason: string;
};

export type WorkflowGraphKernelIndex = {
  nodeById: Map<string, WorkflowGraphNode>;
  nodeIds: string[];
  edgeIds: string[];
  outgoing: Map<string, WorkflowGraphEdge[]>;
  incoming: Map<string, WorkflowGraphEdge[]>;
  duplicateNodeIds: string[];
  missingEdgeNodeIds: string[];
};

export type WorkflowGraphTopologyAnalysis = {
  index: WorkflowGraphKernelIndex;
  hasCycle: boolean;
  cycleNodeIds: string[];
  topologicalNodeIds: string[];
};

const WASM_NODE_THRESHOLD = 512;
const WASM_EDGE_THRESHOLD = 1024;
const WASM_MATERIAL_THRESHOLD = 2048;
const WASM_LAYOUT_THRESHOLD = 768;
const HOT_PATH_SCORE_THRESHOLD = 1500;

export function selectFrontendKernelBackend(
  workload: FrontendKernelWorkload,
): FrontendKernelBackendDecision {
  const workflowNodeCount = positiveCount(workload.workflowNodeCount);
  const workflowEdgeCount = positiveCount(workload.workflowEdgeCount);
  const materialCandidateCount = positiveCount(workload.materialCandidateCount);
  const layoutRectCount = positiveCount(workload.layoutRectCount);
  const hotPathScore =
    workflowNodeCount +
    workflowEdgeCount +
    materialCandidateCount * 2 +
    layoutRectCount * 2 +
    (workload.repeated ? 1000 : 0);

  const isHotPath =
    hotPathScore >= HOT_PATH_SCORE_THRESHOLD ||
    workflowNodeCount >= WASM_NODE_THRESHOLD ||
    workflowEdgeCount >= WASM_EDGE_THRESHOLD ||
    materialCandidateCount >= WASM_MATERIAL_THRESHOLD ||
    layoutRectCount >= WASM_LAYOUT_THRESHOLD;

  if (!isHotPath) {
    return {
      backend: "typescript",
      coldStartProtected: true,
      hotPathScore,
      reason: "workload is below the wasm hot-path threshold",
    };
  }

  if (!workload.wasmReady) {
    return {
      backend: "typescript",
      coldStartProtected: true,
      hotPathScore,
      reason: "wasm backend is not available for this runtime",
    };
  }

  if (!workload.wasmWarmed) {
    return {
      backend: "typescript",
      coldStartProtected: true,
      hotPathScore,
      reason: "wasm backend is cold, so the first pass stays on TypeScript",
    };
  }

  return {
    backend: "wasm",
    coldStartProtected: false,
    hotPathScore,
    reason: "workload is hot and the wasm backend is already warmed",
  };
}

export function buildWorkflowGraphKernelIndex(
  graph: Pick<WorkflowGraphDefinition, "nodes" | "edges">,
): WorkflowGraphKernelIndex {
  const nodeById = new Map<string, WorkflowGraphNode>();
  const outgoing = new Map<string, WorkflowGraphEdge[]>();
  const incoming = new Map<string, WorkflowGraphEdge[]>();
  const duplicateNodeIds: string[] = [];
  const missingEdgeNodeIds = new Set<string>();

  for (const node of graph.nodes) {
    if (nodeById.has(node.id)) {
      duplicateNodeIds.push(node.id);
      continue;
    }
    nodeById.set(node.id, node);
    outgoing.set(node.id, []);
    incoming.set(node.id, []);
  }

  for (const edge of graph.edges ?? []) {
    const fromExists = nodeById.has(edge.from.node);
    const toExists = nodeById.has(edge.to.node);
    if (!fromExists) missingEdgeNodeIds.add(edge.from.node);
    if (!toExists) missingEdgeNodeIds.add(edge.to.node);
    if (fromExists) outgoing.get(edge.from.node)?.push(edge);
    if (toExists) incoming.get(edge.to.node)?.push(edge);
  }

  return {
    nodeById,
    nodeIds: [...nodeById.keys()],
    edgeIds: (graph.edges ?? []).map((edge) => edge.id),
    outgoing,
    incoming,
    duplicateNodeIds,
    missingEdgeNodeIds: [...missingEdgeNodeIds],
  };
}

export function analyzeWorkflowGraphTopology(
  graph: Pick<WorkflowGraphDefinition, "nodes" | "edges">,
): WorkflowGraphTopologyAnalysis {
  const index = buildWorkflowGraphKernelIndex(graph);
  const inDegree = new Map<string, number>(
    index.nodeIds.map((nodeId) => [nodeId, 0] as const),
  );

  for (const edge of graph.edges ?? []) {
    if (index.nodeById.has(edge.from.node) && index.nodeById.has(edge.to.node)) {
      inDegree.set(edge.to.node, (inDegree.get(edge.to.node) ?? 0) + 1);
    }
  }

  const queue = index.nodeIds.filter((nodeId) => (inDegree.get(nodeId) ?? 0) === 0);
  const topologicalNodeIds: string[] = [];

  for (let cursor = 0; cursor < queue.length; cursor += 1) {
    const nodeId = queue[cursor];
    topologicalNodeIds.push(nodeId);
    for (const edge of index.outgoing.get(nodeId) ?? []) {
      const nextCount = (inDegree.get(edge.to.node) ?? 0) - 1;
      inDegree.set(edge.to.node, nextCount);
      if (nextCount === 0) queue.push(edge.to.node);
    }
  }

  const cycleNodeIds = index.nodeIds.filter((nodeId) => (inDegree.get(nodeId) ?? 0) > 0);

  return {
    index,
    hasCycle: cycleNodeIds.length > 0,
    cycleNodeIds,
    topologicalNodeIds,
  };
}

function positiveCount(value: number | undefined): number {
  return Number.isFinite(value) && value && value > 0 ? Math.floor(value) : 0;
}
