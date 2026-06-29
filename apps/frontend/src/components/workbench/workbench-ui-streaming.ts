import type { SidebarSection } from "@/components/workbench/workbench-types";

export const WORKBENCH_UI_STREAMING_CONTRACT_VERSION =
  "kyuubiki.workbench-ui-streaming/v1";

export type WorkbenchUiChunkId =
  | "shell.rail"
  | "shell.sidebar"
  | "workspace.viewport"
  | "workspace.inspector"
  | "workspace.console"
  | "overlay.assistant"
  | "section.study"
  | "section.model"
  | "section.workflow"
  | "section.store"
  | "section.library"
  | "section.system"
  | "renderer.truss3d"
  | "runtime.wasm-python";

export type WorkbenchUiChunkMode = "eager" | "active" | "prefetch" | "evictable";

export type WorkbenchUiChunkContract = {
  id: WorkbenchUiChunkId;
  label: string;
  group: "shell" | "workspace" | "section" | "renderer" | "runtime";
  tags: string[];
  mode: WorkbenchUiChunkMode;
  priority: number;
  retainWhenInactive: boolean;
  estimatedWeight: number;
};

export type WorkbenchUiChunkLoadPhase = "load" | "prefetch" | "park" | "evict";

export type WorkbenchUiChunkLoadDecision = {
  id: WorkbenchUiChunkId;
  phase: WorkbenchUiChunkLoadPhase;
  priority: number;
  reason: string;
  renderPolicy: "visible" | "idle" | "parked" | "hidden";
};

export type WorkbenchUiChunkLoadPlan = {
  activeSection: SidebarSection;
  decisions: WorkbenchUiChunkLoadDecision[];
  loadNow: WorkbenchUiChunkId[];
  prefetchWhenIdle: WorkbenchUiChunkId[];
  parked: WorkbenchUiChunkId[];
  evictable: WorkbenchUiChunkId[];
};

export type WorkbenchUiStreamingState = {
  activeSection: SidebarSection;
  activeChunks: WorkbenchUiChunkId[];
  prefetchChunks: WorkbenchUiChunkId[];
  evictableChunks: WorkbenchUiChunkId[];
  retainedChunks: WorkbenchUiChunkId[];
  estimatedActiveWeight: number;
  budgetStatus: "ok" | "warn";
};

const ACTIVE_WEIGHT_BUDGET = 14;

const CHUNK_CONTRACTS: WorkbenchUiChunkContract[] = [
  chunk("shell.rail", "App rail", "shell", ["navigation"], "eager", 100, true, 1),
  chunk("shell.sidebar", "Sidebar frame", "shell", ["navigation"], "eager", 96, true, 2),
  chunk("workspace.viewport", "Main viewport", "workspace", ["viewport"], "eager", 92, true, 4),
  chunk("workspace.inspector", "Inspector panels", "workspace", ["results"], "active", 72, false, 3),
  chunk("workspace.console", "Console and logs", "workspace", ["logs"], "prefetch", 44, false, 2),
  chunk("overlay.assistant", "Assistant overlay", "workspace", ["assistant"], "prefetch", 36, false, 3),
  chunk("section.study", "Study controls", "section", ["study"], "active", 80, false, 3),
  chunk("section.model", "Model tools", "section", ["model"], "active", 82, false, 4),
  chunk("section.workflow", "Workflow builder", "section", ["workflow"], "active", 86, false, 5),
  chunk("section.store", "Workspace Store", "section", ["store", "assets"], "active", 70, false, 3),
  chunk("section.library", "Library and history", "section", ["library"], "active", 68, false, 4),
  chunk("section.system", "System controls", "section", ["system"], "active", 74, false, 4),
  chunk("renderer.truss3d", "3D renderer", "renderer", ["model", "viewport"], "prefetch", 48, false, 5),
  chunk("runtime.wasm-python", "wasm Python automation", "runtime", ["automation"], "prefetch", 52, false, 6),
];

const SECTION_CHUNKS: Record<SidebarSection, WorkbenchUiChunkId> = {
  study: "section.study",
  model: "section.model",
  workflow: "section.workflow",
  store: "section.store",
  library: "section.library",
  system: "section.system",
};

const SECTION_PREFETCH: Record<SidebarSection, WorkbenchUiChunkId[]> = {
  study: ["section.model", "workspace.inspector"],
  model: ["renderer.truss3d", "section.study", "workspace.inspector"],
  workflow: ["runtime.wasm-python", "workspace.console"],
  store: ["section.workflow", "workspace.console"],
  library: ["section.workflow", "workspace.console"],
  system: ["workspace.console", "overlay.assistant"],
};

export function listWorkbenchUiChunkContracts(): WorkbenchUiChunkContract[] {
  return CHUNK_CONTRACTS.map((contract) => ({ ...contract, tags: [...contract.tags] }));
}

export function resolveWorkbenchUiStreamingState(
  activeSection: SidebarSection,
): WorkbenchUiStreamingState {
  const activeChunks = uniqueChunks([
    ...CHUNK_CONTRACTS.filter((contract) => contract.mode === "eager").map(
      (contract) => contract.id,
    ),
    SECTION_CHUNKS[activeSection],
  ]);
  const prefetchChunks = uniqueChunks(
    SECTION_PREFETCH[activeSection].filter((chunkId) => !activeChunks.includes(chunkId)),
  );
  const retainedChunks = CHUNK_CONTRACTS.filter((contract) => contract.retainWhenInactive).map(
    (contract) => contract.id,
  );
  const evictableChunks = CHUNK_CONTRACTS.map((contract) => contract.id).filter(
    (chunkId) => !activeChunks.includes(chunkId) && !retainedChunks.includes(chunkId),
  );
  const estimatedActiveWeight = activeChunks.reduce(
    (total, chunkId) => total + resolveChunkWeight(chunkId),
    0,
  );

  return {
    activeSection,
    activeChunks,
    prefetchChunks,
    evictableChunks,
    retainedChunks,
    estimatedActiveWeight,
    budgetStatus: estimatedActiveWeight <= ACTIVE_WEIGHT_BUDGET ? "ok" : "warn",
  };
}

export function resolveWorkbenchUiChunkLoadPlan(
  activeSection: SidebarSection,
): WorkbenchUiChunkLoadPlan {
  const state = resolveWorkbenchUiStreamingState(activeSection);
  const decisions = CHUNK_CONTRACTS.map((contract) => {
    if (state.activeChunks.includes(contract.id)) {
      return decision(contract, "load", "active section path");
    }
    if (state.prefetchChunks.includes(contract.id)) {
      return decision(contract, "prefetch", "nearby workflow path");
    }
    if (state.retainedChunks.includes(contract.id)) {
      return decision(contract, "park", "retained shell surface");
    }
    return decision(contract, "evict", "inactive optional surface");
  }).sort((left, right) => right.priority - left.priority || left.id.localeCompare(right.id));

  return {
    activeSection,
    decisions,
    loadNow: idsForPhase(decisions, "load"),
    prefetchWhenIdle: idsForPhase(decisions, "prefetch"),
    parked: idsForPhase(decisions, "park"),
    evictable: idsForPhase(decisions, "evict"),
  };
}

export function resolveWorkbenchUiChunkLoadDecision(
  activeSection: SidebarSection,
  chunkId: WorkbenchUiChunkId,
): WorkbenchUiChunkLoadDecision {
  const plan = resolveWorkbenchUiChunkLoadPlan(activeSection);
  return (
    plan.decisions.find((entry) => entry.id === chunkId) ?? {
      id: chunkId,
      phase: "evict",
      priority: 0,
      reason: "unknown chunk",
      renderPolicy: "hidden",
    }
  );
}

function chunk(
  id: WorkbenchUiChunkId,
  label: string,
  group: WorkbenchUiChunkContract["group"],
  tags: string[],
  mode: WorkbenchUiChunkMode,
  priority: number,
  retainWhenInactive: boolean,
  estimatedWeight: number,
): WorkbenchUiChunkContract {
  return { id, label, group, tags, mode, priority, retainWhenInactive, estimatedWeight };
}

function uniqueChunks(chunkIds: WorkbenchUiChunkId[]): WorkbenchUiChunkId[] {
  return [...new Set(chunkIds)];
}

function resolveChunkWeight(chunkId: WorkbenchUiChunkId): number {
  return CHUNK_CONTRACTS.find((contract) => contract.id === chunkId)?.estimatedWeight ?? 0;
}

function decision(
  contract: WorkbenchUiChunkContract,
  phase: WorkbenchUiChunkLoadPhase,
  reason: string,
): WorkbenchUiChunkLoadDecision {
  return {
    id: contract.id,
    phase,
    priority: contract.priority,
    reason,
    renderPolicy: resolveRenderPolicy(phase),
  };
}

function idsForPhase(
  decisions: WorkbenchUiChunkLoadDecision[],
  phase: WorkbenchUiChunkLoadPhase,
): WorkbenchUiChunkId[] {
  return decisions.filter((entry) => entry.phase === phase).map((entry) => entry.id);
}

function resolveRenderPolicy(
  phase: WorkbenchUiChunkLoadPhase,
): WorkbenchUiChunkLoadDecision["renderPolicy"] {
  if (phase === "load") return "visible";
  if (phase === "prefetch") return "idle";
  if (phase === "park") return "parked";
  return "hidden";
}
