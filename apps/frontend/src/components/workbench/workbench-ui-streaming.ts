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
  retainWhenInactive: boolean;
  estimatedWeight: number;
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
  chunk("shell.rail", "App rail", "shell", ["navigation"], "eager", true, 1),
  chunk("shell.sidebar", "Sidebar frame", "shell", ["navigation"], "eager", true, 2),
  chunk("workspace.viewport", "Main viewport", "workspace", ["viewport"], "eager", true, 4),
  chunk("workspace.inspector", "Inspector panels", "workspace", ["results"], "active", false, 3),
  chunk("workspace.console", "Console and logs", "workspace", ["logs"], "prefetch", false, 2),
  chunk("overlay.assistant", "Assistant overlay", "workspace", ["assistant"], "prefetch", false, 3),
  chunk("section.study", "Study controls", "section", ["study"], "active", false, 3),
  chunk("section.model", "Model tools", "section", ["model"], "active", false, 4),
  chunk("section.workflow", "Workflow builder", "section", ["workflow"], "active", false, 5),
  chunk("section.library", "Library and history", "section", ["library"], "active", false, 4),
  chunk("section.system", "System controls", "section", ["system"], "active", false, 4),
  chunk("renderer.truss3d", "3D renderer", "renderer", ["model", "viewport"], "prefetch", false, 5),
  chunk("runtime.wasm-python", "wasm Python automation", "runtime", ["automation"], "prefetch", false, 6),
];

const SECTION_CHUNKS: Record<SidebarSection, WorkbenchUiChunkId> = {
  study: "section.study",
  model: "section.model",
  workflow: "section.workflow",
  library: "section.library",
  system: "section.system",
};

const SECTION_PREFETCH: Record<SidebarSection, WorkbenchUiChunkId[]> = {
  study: ["section.model", "workspace.inspector"],
  model: ["renderer.truss3d", "section.study", "workspace.inspector"],
  workflow: ["runtime.wasm-python", "workspace.console"],
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

function chunk(
  id: WorkbenchUiChunkId,
  label: string,
  group: WorkbenchUiChunkContract["group"],
  tags: string[],
  mode: WorkbenchUiChunkMode,
  retainWhenInactive: boolean,
  estimatedWeight: number,
): WorkbenchUiChunkContract {
  return { id, label, group, tags, mode, retainWhenInactive, estimatedWeight };
}

function uniqueChunks(chunkIds: WorkbenchUiChunkId[]): WorkbenchUiChunkId[] {
  return [...new Set(chunkIds)];
}

function resolveChunkWeight(chunkId: WorkbenchUiChunkId): number {
  return CHUNK_CONTRACTS.find((contract) => contract.id === chunkId)?.estimatedWeight ?? 0;
}
