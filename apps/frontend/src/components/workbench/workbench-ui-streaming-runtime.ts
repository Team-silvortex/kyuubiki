import {
  WORKBENCH_UI_STREAMING_CONTRACT_VERSION,
  resolveWorkbenchUiChunkLoadDecision,
  resolveWorkbenchUiChunkLoadPlan,
  resolveWorkbenchUiStreamingState,
} from "@/components/workbench/workbench-ui-streaming";
import type {
  WorkbenchUiChunkId,
  WorkbenchUiChunkLoadDecision,
} from "@/components/workbench/workbench-ui-streaming";
import type { SidebarSection } from "@/components/workbench/workbench-types";

export type WorkbenchUiChunkRuntimeAttrs = {
  "data-workbench-ui-chunk": WorkbenchUiChunkId;
  "data-workbench-ui-chunk-phase": WorkbenchUiChunkLoadDecision["phase"];
  "data-workbench-ui-chunk-priority": number;
  "data-workbench-ui-chunk-reason": string;
  "data-workbench-ui-render-policy": WorkbenchUiChunkLoadDecision["renderPolicy"];
};

export type WorkbenchUiStreamingRuntime = {
  activeSection: SidebarSection;
  shellAttrs: Record<string, string>;
  chunkAttrs: (chunkId: WorkbenchUiChunkId) => WorkbenchUiChunkRuntimeAttrs;
};

export function createWorkbenchUiStreamingRuntime(
  activeSection: SidebarSection,
): WorkbenchUiStreamingRuntime {
  const state = resolveWorkbenchUiStreamingState(activeSection);
  const plan = resolveWorkbenchUiChunkLoadPlan(activeSection);

  return {
    activeSection,
    shellAttrs: {
      "data-workbench-ui-streaming-contract": WORKBENCH_UI_STREAMING_CONTRACT_VERSION,
      "data-workbench-active-ui-chunks": state.activeChunks.join(" "),
      "data-workbench-prefetch-ui-chunks": state.prefetchChunks.join(" "),
      "data-workbench-evictable-ui-chunks": state.evictableChunks.join(" "),
      "data-workbench-ui-chunk-budget": state.budgetStatus,
      "data-workbench-ui-load-now": plan.loadNow.join(" "),
      "data-workbench-ui-prefetch-idle": plan.prefetchWhenIdle.join(" "),
      "data-workbench-ui-parked": plan.parked.join(" "),
      "data-workbench-ui-evictable": plan.evictable.join(" "),
    },
    chunkAttrs: (chunkId) => buildWorkbenchUiChunkRuntimeAttrs(activeSection, chunkId),
  };
}

export function buildWorkbenchUiChunkRuntimeAttrs(
  activeSection: SidebarSection,
  chunkId: WorkbenchUiChunkId,
): WorkbenchUiChunkRuntimeAttrs {
  const decision = resolveWorkbenchUiChunkLoadDecision(activeSection, chunkId);

  return {
    "data-workbench-ui-chunk": chunkId,
    "data-workbench-ui-chunk-phase": decision.phase,
    "data-workbench-ui-chunk-priority": decision.priority,
    "data-workbench-ui-chunk-reason": decision.reason,
    "data-workbench-ui-render-policy": decision.renderPolicy,
  };
}
