"use client";

import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { createElectrostaticToHeatBridgeContract } from "@/components/workbench/workflow/workbench-workflow-bridge-contract";

type WorkbenchWorkflowTemplateChainActionsProps = {
  labels: WorkflowSidebarLabels;
  selectedSourceNodeId?: string | null;
  onInsertTemplateChain: (
    templates: WorkflowNodeTemplateSelection[],
    sourceNodeId?: string | null,
  ) => void;
};

export function WorkbenchWorkflowTemplateChainActions({
  labels,
  selectedSourceNodeId,
  onInsertTemplateChain,
}: WorkbenchWorkflowTemplateChainActionsProps) {
  return (
    <div className="button-row">
      <button
        onClick={() =>
          onInsertTemplateChain(
            [
              { kind: "solve", operatorId: "solve.frame_3d" },
              { kind: "extract", operatorId: "extract.result_summary" },
              { kind: "export", operatorId: "export.summary_json" },
            ],
            selectedSourceNodeId,
          )
        }
        type="button"
      >
        {labels.insertSolveExtractExportLabel}
      </button>
      <button
        onClick={() =>
          onInsertTemplateChain(
            [
              { kind: "solve", operatorId: "solve.heat_plane_quad_2d" },
              { kind: "transform", operatorId: "bridge.temperature_field_to_thermo_quad_2d" },
              { kind: "solve", operatorId: "solve.thermal_plane_quad_2d" },
            ],
            selectedSourceNodeId,
          )
        }
        type="button"
      >
        {labels.insertHeatBridgeThermoLabel}
      </button>
      <button
        onClick={() =>
          onInsertTemplateChain(
            [
              { kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" },
              {
                kind: "transform",
                operatorId: "bridge.electrostatic_field_to_heat_quad_2d",
                config: { contract: createElectrostaticToHeatBridgeContract() },
              },
              { kind: "solve", operatorId: "solve.heat_plane_quad_2d" },
            ],
            selectedSourceNodeId,
          )
        }
        type="button"
      >
        {labels.insertElectrostaticBridgeHeatLabel}
      </button>
      <button
        onClick={() =>
          onInsertTemplateChain(
            [
              { kind: "solve", operatorId: "solve.electrostatic_plane_quad_2d" },
              {
                kind: "extract",
                operatorId: "extract.result_summary",
                config: {
                  fields: ["max_potential", "max_electric_field", "max_flux_density"],
                },
              },
              { kind: "export", operatorId: "export.summary_json" },
            ],
            selectedSourceNodeId,
          )
        }
        type="button"
      >
        {labels.insertElectrostaticSolveExportLabel}
      </button>
    </div>
  );
}
