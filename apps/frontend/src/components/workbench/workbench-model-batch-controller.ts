import type { Frame2dJobInput, Truss2dJobInput, Truss3dJobInput } from "@/lib/api";
import { applyModelBatch, type ModelBatchRequest } from "@/lib/workbench/model-batch-commands";
import { inspectBatchSelection, selectBatchNodes, ModelBatchError, type BatchModel, type BatchQuery } from "@/lib/workbench/model-batch-selection";

type BatchControllerDeps = {
  studyKind: string;
  trussModel: Truss2dJobInput;
  truss3dModel: Truss3dJobInput;
  frameModel: Frame2dJobInput;
  selectedNode: number | null;
  selectedTruss3dNodes: number[];
  setTrussModel: (model: Truss2dJobInput) => void;
  setTruss3dModel: (model: Truss3dJobInput) => void;
  setFrameModel: (model: Frame2dJobInput) => void;
  setSelectedNode: (index: number | null) => void;
  setSelectedElement: (index: number | null) => void;
  setSelectedTruss3dNodes: (indices: number[]) => void;
  setMemberDraftNodes: (indices: number[]) => void;
  recordHistory: (label: string) => void;
  resetActiveResult: () => void;
  historyLabel: string;
};

export function createWorkbenchModelBatchController(deps: BatchControllerDeps) {
  const spatial = deps.studyKind === "truss_3d";
  const frame = deps.studyKind === "frame_2d";
  const model: BatchModel | null = spatial ? deps.truss3dModel : frame ? deps.frameModel
    : deps.studyKind === "truss_2d" ? deps.trussModel : null;
  const selection = spatial && deps.selectedTruss3dNodes.length > 0 ? deps.selectedTruss3dNodes
    : deps.selectedNode === null ? [] : [deps.selectedNode];
  let committed = false;

  return {
    model, spatial, frame, selection, studyKind: deps.studyKind,
    select3d: (query: BatchQuery) => {
      if (!model || !spatial) throw new ModelBatchError("unsupported_study");
      if (committed) throw new ModelBatchError("stale_model");
      const indices = selectBatchNodes(model, query, selection);
      // Selection changes neither model history nor solver observations.
      committed = true;
      deps.setSelectedTruss3dNodes(indices);
      deps.setSelectedNode(indices[0] ?? null);
      deps.setSelectedElement(null);
      deps.setMemberDraftNodes([]);
      return { selectedNodes: indices.length };
    },
    inspect: (query: BatchQuery) => {
      if (!model) throw new ModelBatchError("unsupported_study");
      if (committed) throw new ModelBatchError("stale_model");
      return inspectBatchSelection(model, query, selection);
    },
    apply: (request: ModelBatchRequest) => {
      if (!model) throw new ModelBatchError("unsupported_study");
      if (committed) throw new ModelBatchError("stale_model");
      // Complete validation and construction before touching results or the single undo checkpoint.
      const result = applyModelBatch(model, request, selection);
      if (!result.summary.changed) return result.summary;
      // Overlapping script calls must not overwrite each other before React publishes the next model.
      committed = true;
      deps.recordHistory(deps.historyLabel);
      deps.resetActiveResult();
      if (spatial) deps.setTruss3dModel(result.model as Truss3dJobInput);
      else if (frame) deps.setFrameModel(result.model as Frame2dJobInput);
      else deps.setTrussModel(result.model);
      deps.setSelectedTruss3dNodes(spatial ? result.nextSelection : []);
      deps.setSelectedNode(result.nextSelection[0] ?? null);
      deps.setSelectedElement(null);
      deps.setMemberDraftNodes([]);
      return result.summary;
    },
  };
}

export type WorkbenchModelBatchController = ReturnType<typeof createWorkbenchModelBatchController>;
